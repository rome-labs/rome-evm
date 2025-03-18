use solana_program::pubkey::Pubkey;
use {
    crate::{context::ContextIterative, state::State, Bind},
    rome_evm::{
        accounts::{Data, Lock, LockType, RoLock},
        context::account_lock::AccountLock,
        error::{Result, RomeProgramError::*},
    },
    solana_program::account_info::{AccountInfo, IntoAccountInfo},
    std::mem::size_of,
};

#[allow(dead_code)]
pub fn add_ro_lock(state: &State, info: &AccountInfo, state_holder: &Bind) -> Result<()> {
    let mut lock = Lock::from_account_mut(info)?;
    let mut ro_lock_bind = state.info_ro_lock(info.key, true)?;

    let len = match lock.get()? {
        Some(LockType::Ro) => {
            // allocate ro-lock-info
            let ro_lock_info = ro_lock_bind.into_account_info();
            if !RoLock::found(&ro_lock_info, &state_holder.0)? {
                ro_lock_info.data_len() + size_of::<RoLock>()
            } else {
                0
            }
        }
        Some(LockType::Rw(_)) => return Err(AccountLocked(*info.key, lock.lock)),
        None => {
            // allocate/deallocate ro-lock-info
            let ro_lock_info = ro_lock_bind.into_account_info();
            RoLock::offset(&ro_lock_info) + size_of::<RoLock>()
        }
    };

    // push holder.key to ro-lock-info
    if len > 0 {
        state.realloc(&mut ro_lock_bind, len)?;
        let ro_lock_info = ro_lock_bind.into_account_info();
        RoLock::add_preallocated(&ro_lock_info, &state_holder.0)?;
    }
    // ro_lock account must be writable so that it can be deallocated during unlock iteration
    state.update(ro_lock_bind);
    // add ro-lock
    lock.ro_lock()
}

pub fn add_rw_lock(info: &AccountInfo, state_holder: &Bind) -> Result<()> {
    let mut lock = Lock::from_account_mut(info)?;

    if let Some(lock) = lock.get()? {
        return Err(AccountLocked(*info.key, Some(lock)));
    }
    // add rw-lock
    lock.rw_lock(&state_holder.0)
}

pub fn iterative_lock(state: &State, holder: u64) -> Result<Vec<Pubkey>> {
    let state_holder = state.info_state_holder(holder, false)?;
    let lock_overrides = vec![];

    let accounts = state.accounts.borrow().clone();

    for (key, item) in accounts.iter() {
        let mut bind = (*key, item.account.clone());
        let info = bind.into_account_info();

        if Lock::is_managed(&info, state.program_id)? {
            // TODO: enable ro-lock after the ALT is implemented
            add_rw_lock(&info, &state_holder)?;
            // if item.writable {
            //     add_rw_lock(&info, &state_holder)?;
            // } else {
            //     add_ro_lock(state, &info, &state_holder)?;
            //     lock_overrides.push(*key);
            // }
            // all accounts must be writable
            state.update(bind);
        }
    }

    Ok(lock_overrides)
}

impl AccountLock for ContextIterative<'_, '_> {
    fn lock(&self) -> Result<()> {
        let mut lock_overrides = self.lock_overrides.borrow_mut();
        *lock_overrides = iterative_lock(self.state, self.holder)?;
        Ok(())
    }
    fn locked(&self) -> Result<bool> {
        // during transaction emulation accounts are not locked
        Ok(true)
    }
    fn unlock(&self) -> Result<()> {
        // it doesn't make sense for emulation
        Ok(())
    }
    fn lock_new_one(&self, _info: &AccountInfo) -> Result<()> {
        unreachable!()
    }
    fn check_writable(&self, _info: &AccountInfo) -> Result<()> {
        Ok(())
    }
}
