use {
    super::{Diff, Journal},
    crate::{error::RomeProgramError::*, error::*, origin::Origin, state::Allocate},
    borsh::{BorshDeserialize, BorshSerialize},
    evm::{Handler, H160, H256, U256},
    solana_program::{
        clock::Clock,
        keccak::{hash, hashv},
        sysvar::Sysvar,
    },
};

/// JournalState is internal EVM state that is used to contain state and track changes to that state.
/// It contains journal of changes that happened to state so that they can be reverted.
pub struct JournaledState<'a, T: Origin + Allocate> {
    /// initial state (from Solana)
    pub state: &'a T,
    /// [EIP-1153[(https://eips.ethereum.org/EIPS/eip-1153) transient storage that is discarded after every transactions
    // pub transient_storage: TransientStorage,

    /// logs
    // pub logs: Vec<Log>,
    // journal with changes that happened between calls.
    pub journal: Journal,
    pub mutable: bool,
    pub block_number: U256,
    pub block_timestamp: U256,
    pub slot: u64,
    pub origin: Option<H160>,
    pub gas_limit: Option<U256>,
    pub gas_price: Option<U256>,
    pub gas_recipient: Option<H160>,
}

impl<'a, T: Origin + Allocate> JournaledState<'a, T> {
    #[allow(dead_code)]
    pub fn new(state: &'a T) -> Result<Self> {
        let clock = Clock::get()?;
        let journaled_state = Self {
            state,
            journal: Journal::new(),
            mutable: true,
            block_number: clock.slot.into(),
            block_timestamp: clock.unix_timestamp.into(),
            slot: clock.slot,
            origin: None,
            gas_limit: None,
            gas_price: None,
            gas_recipient: None,
        };

        Ok(journaled_state)
    }

    pub fn new_page(&mut self) {
        let mut journal = Journal::new();
        std::mem::swap(&mut self.journal, &mut journal); // todo: swap uses mem copy. Replace tree to Vec
        self.journal = Journal::next_page(Box::new(journal));
    }

    pub fn transfer(&mut self, from: &H160, to: &H160, balance: &U256) -> Result<()> {
        if balance.is_zero() {
            return Ok(());
        }

        if self.balance(*from) < *balance {
            Err(InsufficientFunds(*from, *balance)) // todo: remove this check?
        } else {
            self.journal
                .get_mut(from)
                .push(Diff::TransferFrom { balance: *balance });
            self.journal
                .get_mut(to)
                .push(Diff::TransferTo { balance: *balance });
            Ok(())
        }
    }

    // todo: track the depth
    pub fn revert_diff(&mut self) {
        self.journal = if let Some(parent) = self.journal.parent.take() {
            *parent
        } else {
            Journal::new()
        }
    }

    pub fn set_code(&mut self, address: H160, code: Vec<u8>) {
        let valids = evm::Valids::compute(&code);
        let diff = Diff::CodeChange { code, valids };
        self.journal.get_mut(&address).push(diff);
    }

    pub fn build_address(&self, scheme: evm::CreateScheme) -> Result<H160> {
        let address = match scheme {
            evm::CreateScheme::Legacy { caller } => {
                let nonce = self.nonce(caller); //
                let mut rlp = rlp::RlpStream::new_list(2);
                rlp.append(&caller).append(&nonce);
                let data = rlp.out();
                let hash = hash(&data);
                let h256 = H256::from(hash.to_bytes());
                h256.into()
            }
            evm::CreateScheme::Create2 {
                caller,
                code_hash,
                salt,
            } => {
                let data: Vec<&[u8]> = vec![&[0xff_u8], &caller[..], &salt[..], &code_hash[..]];
                let hash = hashv(data.as_slice());
                let h256 = H256::from(hash.to_bytes());
                h256.into()
            }
            evm::CreateScheme::Fixed(new) => new,
        };

        let nonce = self.nonce(address);
        let size = self.code_size(address);

        // TODO:  figure out about nonce. Existing balance account can become contract account.
        if nonce.is_zero() && size.is_zero() {
            Ok(address)
        } else {
            Err(DeployContractToExistingAccount(address))
        }
    }

    pub fn block_hash(&self, block: U256) -> Result<H256> {
        let slot = self.slot;
        self.state.block_hash(block, slot)
    }

    pub fn commit(&mut self) -> Result<()> {
        self.journal.commit(self.state)
    }

    pub fn serialize(&self, into: &mut &mut [u8]) -> Result<()> {
        self.journal.serialize(into)?;
        self.mutable.serialize(into)?;
        self.block_number.serialize(into)?;
        self.block_timestamp.serialize(into)?;
        self.slot.serialize(into)?;
        self.origin.serialize(into)?;
        self.gas_limit.serialize(into)?;
        self.gas_price.serialize(into)?;
        self.gas_recipient.serialize(into)?;
        Ok(())
    }

    pub fn deserialize(from: &mut &[u8], state: &'a T) -> Result<Self> {
        let journal = Journal::deserialize(from)?;
        let mutable: bool = BorshDeserialize::deserialize(from)?;
        let block_number: U256 = BorshDeserialize::deserialize(from)?;
        let block_timestamp: U256 = BorshDeserialize::deserialize(from)?;
        let slot: u64 = BorshDeserialize::deserialize(from)?;
        let origin: Option<H160> = BorshDeserialize::deserialize(from)?;
        let gas_limit: Option<U256> = BorshDeserialize::deserialize(from)?;
        let gas_price: Option<U256> = BorshDeserialize::deserialize(from)?;
        let gas_recipient: Option<H160> = BorshDeserialize::deserialize(from)?;

        Ok(Self {
            state,
            journal,
            mutable,
            block_number,
            block_timestamp,
            slot,
            origin,
            gas_limit,
            gas_price,
            gas_recipient,
        })
    }

    pub fn allocate(&self) -> Result<bool> {
        self.journal.allocate(self.state)
    }
}
