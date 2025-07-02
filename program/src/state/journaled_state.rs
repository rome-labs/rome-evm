use {
    super::{Diff, Journal},
    crate::{
        context::AccountLock, error::RomeProgramError::*, error::*, origin::Origin,
        pda::Seed, state::Allocate, NUMBER_ALLOC_DIFF_PER_TX,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    evm::{Handler, H160, H256, U256},
    solana_program::{
        clock::Clock,
        keccak::{hash, hashv, Hash},
        pubkey::Pubkey,
        sysvar::Sysvar,
    },
    std::collections::{BTreeMap, BTreeSet, HashMap, HashSet,},
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
    pub merged_slots: BTreeMap<H160, HashSet<U256>>,
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
            merged_slots: BTreeMap::new(),
        };

        Ok(journaled_state)
    }

    pub fn new_page(&mut self) {
        let mut journal = Journal::new();
        std::mem::swap(&mut self.journal, &mut journal); // todo: swap uses mem copy. Replace tree to Vec
        self.journal = Journal::next_page(Box::new(journal));
    }

    pub fn transfer(&mut self, from: &H160, to: &H160, balance: &U256){
        if balance.is_zero() {
            return;
        }

        self.journal.get_mut(from).push(Diff::TransferFrom { balance: *balance });
        self.journal.get_mut(to).push(Diff::TransferTo { balance: *balance });
    }

    pub fn revert_page(&mut self) {
        self.journal = self.journal.revert_page(self.journal.page)
    }

    pub fn revert_all(&mut self) {
        self.journal = Journal::new()
    }

    pub fn get_and_revert_all(&mut self) -> Journal {
        let mut journal = Journal::new();
        std::mem::swap(&mut self.journal, &mut journal);
        journal
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

    pub fn commit<L: AccountLock>(&mut self, context: &'a L) -> Result<()> {
        self.journal.commit(self.state, context)
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
        self.merged_slots.serialize(into)?;
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
        let merged_slots: BTreeMap<H160, HashSet<U256>> = BorshDeserialize::deserialize(from)?;

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
            merged_slots,
        })
    }

    pub fn allocate<L: AccountLock>(&self, context: &'a L) -> Result<bool> {
        self.journal.alloc_balances(self.state, context)
    }

    pub fn merge_slots(&mut self) -> Result<()> {
        // very heavy in terms of CU consumption
        self.merged_slots = self.journal.merge_slots(self.state)?;
        Ok(())
    }

    pub fn alloc_slots<L: AccountLock>(&mut self, context: &'a L) -> Result<bool> {
        let keys = self.storage_keys(&self.merged_slots)?;

        for (key, (seed, count, address)) in keys.iter() {
            let base = self.state.base();
            if base.syscall.count() >= NUMBER_ALLOC_DIFF_PER_TX || base.alloc_limit() < 500 {
                return Ok(false);
            }

            if !self
                .state
                .alloc_slots(key, seed, *count, context, address)?
            {
                return Ok(false);
            }
        }

        // no need to serialize/deserialize temporary data
        self.merged_slots.clear();

        Ok(true)
    }

    pub fn alloc_slots_unchecked(&self) -> Result<()> {
        let slots = self.journal.merge_slots(self.state)?;
        let keys = self.storage_keys(&slots)?;

        for (key, (seed, count, address)) in keys.iter() {
            self.state
                .alloc_slots_unchecked(key, seed, *count, address)?;
        }

        Ok(())
    }

    pub fn storage_keys(
        &self,
        merged_slots: &BTreeMap<H160, HashSet<U256>>,
    ) -> Result<HashMap<Pubkey, (Seed, usize, H160)>> {
        let mut keys_new_slots: HashMap<Pubkey, (Seed, usize, H160)> = HashMap::new();

        for (address, set) in merged_slots.iter() {
            for slot in set {
                let (key, seed, _) = self.state.base().slot_to_key(address, slot);
                keys_new_slots
                    .entry(key)
                    .and_modify(|(_, len, _)| *len += 1)
                    .or_insert((seed, 1, *address));
            }
        }

        Ok(keys_new_slots)
    }

    pub fn hash_journaled_accounts(&self, journal: &Journal) -> Result<Hash> {
        let mut accs = journal.journaled_accs();
        let mut slots = journal.journaled_slots();

        // this journal doesn't include the gas_transfer
        if let Some(fee_recipient) = self.gas_recipient {
            accs.insert(fee_recipient);
        }

        accs
            .iter()
            .for_each(|addr| {
                if slots.get(addr).is_none() {
                    slots.insert(*addr, BTreeSet::new());
                }
        });

        let mut hashes = vec![];

        for (addr, slots_) in slots.iter() {
            let nonce = self.state.nonce(addr)?.to_le_bytes();

            let balance = self.state.balance(addr)?;
            let mut balance_be = [0; 32];
            balance.to_big_endian(&mut balance_be);

            let code = self.state.code(addr)?;
            
            let values_be = self.slots_to_values_be(addr, &slots_)?;
            let mut values_be_ref = values_be
                .iter()
                .map(|x| x.as_slice())
                .collect::<Vec<_>>();

            let mut slices = vec![
                &addr[..],
                nonce.as_slice(),
                balance_be.as_slice(),
                code.as_slice()
            ];
            
            slices.append(&mut values_be_ref);
            hashes.push(hashv(&slices));
        }

        let hash_vec = hashes.iter().map(|x| x.as_ref()).collect::<Vec<_>>();
        
        Ok(hashv(&hash_vec))
    }

    fn slots_to_values_be(&self, addr: &H160, slots: &BTreeSet<U256>) -> Result<Vec<[u8; 32]>> {
        let values = slots
            .iter()
            .map(|slot| self.state.storage(addr, slot))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .map(|opt| opt.expect("storage slot expected"))
            .collect::<Vec<_>>();
        
        let values_be = values
            .iter()
            .map(|item| {
                let mut buf = [0_u8; 32];
                item.to_big_endian(&mut buf);
                buf
            }).
            collect::<Vec<_>>();
        
        Ok(values_be)
    }
}
