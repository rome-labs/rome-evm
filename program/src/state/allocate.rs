use {
    super::State,
    crate::{
        context::account_lock::AccountLock,
        error::{Result, RomeProgramError::*},
        pda::Seed,
        AccountState, AccountType, Code, Data, Slot, Storage,
    },
    evm::H160,
    solana_program::{msg, pubkey::Pubkey},
    std::mem::size_of,
};

pub trait Allocate {
    fn alloc_balance<L: AccountLock>(&self, address: &H160, context: &L) -> Result<()>;
    fn alloc_slots<L: AccountLock>(
        &self,
        key: &Pubkey,
        seed: &Seed,
        count: usize,
        context: &L,
        address: &H160,
    ) -> Result<bool>;
    fn alloc_slots_unchecked(
        &self,
        key: &Pubkey,
        seed: &Seed,
        count: usize,
        address: &H160,
    ) -> Result<()>;
    fn alloc_contract<L: AccountLock>(
        &self,
        address: &H160,
        code: &[u8],
        valids: &[u8],
        context: &L,
    ) -> Result<bool>;
}

impl Allocate for State<'_> {
    fn alloc_balance<L: AccountLock>(&self, address: &H160, context: &L) -> Result<()> {
        let info = self.info_addr(address, true)?;
        context.check_writable(info)?;
        context.lock_new_one(info)?;

        Ok(())
    }
    fn alloc_contract<L: AccountLock>(
        &self,
        address: &H160,
        code: &[u8],
        valids: &[u8],
        context: &L,
    ) -> Result<bool> {
        let info = self.info_addr(address, true)?;

        context.check_writable(info)?;
        context.lock_new_one(info)?;

        AccountState::check_no_contract(info, address)?;

        let req = Code::offset(info) + code.len() + valids.len();
        // TODO: implement deallocation
        if info.data_len() > req {
            return Err(Unimplemented("the contract deployment space must be deallocated according to size of the contract".to_string()));
        }

        let diff = req.saturating_sub(info.data_len());
        let limit = self.alloc_limit();
        let len = diff.min(limit);

        self.realloc(info, info.data_len() + len)?;

        Ok(diff <= limit)
    }

    fn alloc_slots<L: AccountLock>(
        &self,
        key: &Pubkey,
        seed: &Seed,
        new: usize,
        context: &L,
        _: &H160,
    ) -> Result<bool> {
        let info = self.info_pda(key, seed, AccountType::Storage, true)?;

        context.check_writable(info)?;
        context.lock_new_one(info)?;

        let unused = Storage::unused_len(info)?;
        let diff = new.saturating_sub(unused);
        Storage::available(info, diff)?;

        let limit = self.alloc_limit() / size_of::<Slot>();
        let diff_limited = diff.min(limit);

        let len = info.data_len() + diff_limited * size_of::<Slot>();
        self.realloc(info, len)?;

        msg!("allocate slots {}, slots {}", key, diff_limited);
        Ok(diff <= limit)
    }

    fn alloc_slots_unchecked(&self, key: &Pubkey, seed: &Seed, new: usize, _: &H160) -> Result<()> {
        let info = self.info_pda(key, seed, AccountType::Storage, true)?;

        let unused = Storage::unused_len(info)?;
        let diff = new.saturating_sub(unused);
        Storage::available(info, diff)?;

        let len = info.data_len() + diff * size_of::<Slot>();
        msg!("allocate slots {}, slots {}", key, diff);
        self.realloc(info, len)
    }
}
