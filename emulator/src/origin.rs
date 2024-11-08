use rome_evm::pda::{Pda, Seed};
use {
    crate::state::State,
    rome_evm::{
        context::account_lock::AccountLock,
        error::{Result, RomeProgramError::*},
        info::Info,
        origin::Origin,
        Code, Data, H160, H256, U256,
    },
    solana_program::{
        account_info::IntoAccountInfo, clock::Slot, pubkey::Pubkey, sysvar::recent_blockhashes,
    },
    std::cmp::Ordering::{Greater, Less},
};

impl Info for State<'_> {}

impl Origin for State<'_> {
    fn program_id(&self) -> &Pubkey {
        self.program_id
    }
    fn nonce(&self, address: &H160) -> Result<u64> {
        let mut bind = self.info_addr(address, false)?;
        let info = bind.into_account_info();
        Info::nonce(self, &info)
    }
    fn balance(&self, address: &H160) -> Result<U256> {
        let mut bind = self.info_addr(address, false)?;
        let info = bind.into_account_info();
        Info::balance(self, &info)
    }
    fn code(&self, address: &H160) -> Result<Vec<u8>> {
        let mut bind = self.info_addr(address, false)?;
        let info = bind.into_account_info();
        Info::code(self, &info)
    }
    fn valids(&self, address: &H160) -> Result<Vec<u8>> {
        let mut bind = self.info_addr(address, false)?;
        let info = bind.into_account_info();
        Info::valids(self, &info)
    }
    fn storage(&self, address: &H160, slot: &U256) -> Result<Option<U256>> {
        let (mut bind, sub_ix) = self.info_slot(address, slot, false)?;
        let info = bind.into_account_info();
        Info::storage(self, &info, sub_ix)
    }
    fn inc_nonce<L: AccountLock>(&self, address: &H160, context: &L) -> Result<()> {
        let mut bind = self.info_addr(address, true)?;
        let info = bind.into_account_info();
        Info::inc_nonce(self, &info, context)?;
        self.update(bind)
    }
    fn add_balance<L: AccountLock>(
        &self,
        address: &H160,
        balance: &U256,
        context: &L,
    ) -> Result<()> {
        let mut bind = self.info_addr(address, true)?;
        let info = bind.into_account_info();
        Info::add_balance(self, &info, balance, context)?;
        self.update(bind)
    }
    fn sub_balance<L: AccountLock>(
        &self,
        address: &H160,
        balance: &U256,
        context: &L,
    ) -> Result<()> {
        let mut bind = self.info_addr(address, true)?;
        let info = bind.into_account_info();
        Info::sub_balance(self, &info, balance, address, context)?;
        self.update(bind)
    }
    fn set_code<L: AccountLock>(
        &self,
        address: &H160,
        code: &[u8],
        valids: &[u8],
        context: &L,
    ) -> Result<()> {
        let mut bind = self.info_addr(address, true)?;

        let len = bind.1.data.len();
        let offset = Code::offset(&bind.into_account_info());
        let required = offset + code.len() + valids.len();

        match len.cmp(&required) {
            Less => {
                self.realloc(&mut bind, required)?;
            }
            Greater => {
                // TODO: implement deallocation of the unused contract space
                return Err(Unimplemented(
                    format!(
                        "the contract deployment space must be deallocated according to size of the contract \
                        pubkey: {}, available: {}, required: {}",
                        bind.0,
                        len,
                        required
                    ))
                );
            }
            _ => {}
        }

        let info = bind.into_account_info();
        Info::set_code(self, &info, code, valids, address, context)?;
        self.update(bind)
    }
    fn set_storage<L: AccountLock>(
        &self,
        address: &H160,
        slot: &U256,
        value: &U256,
        context: &L,
    ) -> Result<()> {
        let (mut bind, sub_ix) = self.info_slot(address, slot, true)?;
        let info = bind.into_account_info();
        Info::set_storage(self, &info, sub_ix, value, context)?;
        self.update(bind)
    }
    fn block_hash(&self, block: U256, slot: Slot) -> Result<H256> {
        let mut bind = self
            .load(&recent_blockhashes::ID, None)?
            .ok_or(SystemAccountNotFound(recent_blockhashes::ID))?;
        let info = bind.into_account_info();
        Info::block_hash(self, &info, block, slot)
    }

    fn allocated(&self) -> usize {
        *self.alloc.borrow()
    }

    fn deallocated(&self) -> usize {
        *self.dealloc.borrow()
    }

    fn chain_id(&self) -> u64 {
        self.chain
    }

    fn serialize_pda(&self, into: &mut &mut [u8]) -> Result<()> {
        self.pda.serialize(into)
    }

    fn deserialize_pda(&self, from: &mut &[u8]) -> Result<()> {
        self.pda.deserialize(from)
    }

    fn slot_to_key(&self, address: &H160, slot: &U256) -> (Pubkey, Seed, u8) {
        let (index_be, sub_ix) = Pda::storage_index(slot);
        let (base, _) = self.pda.balance_key(address);
        let (key, seed) = self.pda.storage_key(&base, index_be);

        (key, seed, sub_ix)
    }
    fn syscalls(&self) -> u64 {
        self.syscall.count()
    }
}
