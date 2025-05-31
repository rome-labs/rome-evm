use solana_program::pubkey::Pubkey;
use {
    super::AccountLock,
    crate::{
        error::{Result, RomeProgramError::AccountLocked, },
        state::State, Data, Lock,
    },
    solana_program::account_info::AccountInfo,
};

pub struct ContextAt<'a, 'b> {
    pub state: &'b State<'a>,
}
impl<'a, 'b> ContextAt<'a, 'b> {
    pub fn new(state: &'b State<'a>) -> Self {
        Self {
            state,
        }
    }
}

impl<'a, 'b> AccountLock for ContextAt<'a, 'b> {
    fn lock(&self) -> Result<()> {
        for &info in self.state.all().values() {
            lock_impl(info, self.state.program_id)?;
        }

        Ok(())
    }
    fn locked(&self) -> Result<bool> {
        unreachable!()
    }
    fn unlock(&self) -> Result<()> {
        unreachable!()
    }
    fn lock_new_one(&self, _info: &AccountInfo) -> Result<()> {
        Ok(())
    }
    fn check_writable(&self, _info: &AccountInfo) -> Result<()> {
        Ok(())
    }
}

pub fn lock_impl(info: &AccountInfo, program_id: &Pubkey) -> Result<()> {
    // existings locks can only affect writable accounts of the atomic tx
    if Lock::is_managed(info, program_id)? && info.is_writable {
        let lock = Lock::from_account_mut(info)?;
        if lock.get()?.is_some() {
            return Err(AccountLocked(*info.key, lock.lock));
        }
    }

    Ok(())
}
