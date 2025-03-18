use {
    crate::state::State,
    rome_evm::{
        context::account_lock::AccountLock,
        error::{Result, RomeProgramError::*},
        pda::Seed,
        state::allocate::Allocate,
        AccountState, AccountType, Code, Data, Slot, Storage, H160,
    },
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey},
    std::mem::size_of,
};

impl Allocate for State<'_> {
    fn alloc_balance<L: AccountLock>(&self, address: &H160, _context: &L) -> Result<()> {
        let _ = self.info_addr(address, true)?;

        Ok(())
    }
    fn alloc_contract<L: AccountLock>(
        &self,
        address: &H160,
        code: &[u8],
        valids: &[u8],
        _context: &L,
    ) -> Result<bool> {
        let mut bind = self.info_addr(address, true)?;

        let diff = {
            let info = bind.into_account_info();
            AccountState::check_no_contract(&info, address)?;

            let req = Code::offset(&info) + code.len() + valids.len();
            // TODO: implement deallocation
            if info.data_len() > req {
                return Err(Unimplemented("the contract deployment space must be deallocated according to size of the contract".to_string()));
            }

            req.saturating_sub(info.data_len())
        };

        let limit = self.alloc_limit();
        let len = diff.min(limit);

        let new_len = bind.1.data.len() + len;
        self.realloc(&mut bind, new_len)?;
        self.update(bind);

        Ok(diff <= limit)
    }

    fn alloc_slots<L: AccountLock>(
        &self,
        key: &Pubkey,
        _: &Seed,
        new: usize,
        _context: &L,
        address: &H160,
    ) -> Result<bool> {
        let mut bind = self.info_pda(key, AccountType::Storage, Some(*address), true)?;
        let limit = self.alloc_limit() / size_of::<Slot>();

        let (len, diff) = {
            let info = bind.into_account_info();
            let unused = Storage::unused_len(&info)?;
            let diff = new.saturating_sub(unused);

            Storage::available(&info, diff)?;

            let diff_limited = diff.min(limit);
            let len = info.data_len() + diff_limited * size_of::<Slot>();

            (len, diff)
        };

        self.realloc(&mut bind, len)?;
        self.update(bind);
        msg!("allocate slots {}, diff {}", key, diff);

        Ok(diff <= limit)
    }

    fn alloc_slots_unchecked(
        &self,
        key: &Pubkey,
        _: &Seed,
        new: usize,
        address: &H160,
    ) -> Result<()> {
        let mut bind = self.info_pda(key, AccountType::Storage, Some(*address), true)?;

        let (len, diff) = {
            let info = bind.into_account_info();
            let unused = Storage::unused_len(&info)?;
            let diff = new.saturating_sub(unused);

            Storage::available(&info, diff)?;

            let len = info.data_len() + diff * size_of::<Slot>();
            (len, diff)
        };

        self.realloc(&mut bind, len)?;
        self.update(bind);
        msg!("allocate slots {}, diff {}", key, diff);

        Ok(())
    }
}
