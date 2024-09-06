use {
    crate::context::ContextIterative,
    rome_evm::{
        accounts::{Data, Lock, LockType, RoLock},
        context::account_lock::AccountLock,
        error::{Result, RomeProgramError::*},
    },
    solana_program::account_info::IntoAccountInfo,
    std::mem::size_of,
};

impl AccountLock for ContextIterative<'_, '_> {
    fn lock(&self) -> Result<()> {
        let mut ro_count = 0;
        let state_holder = self.state.info_state_holder(self.holder, false)?;

        let accounts = self.state.accounts.borrow().clone();
        for (key, item) in accounts.iter() {
            let mut bind = (*key, item.account.clone());
            let mut info = bind.into_account_info();
            info.is_writable = item.writable;

            let mut managed = false;
            if Lock::is_managed(&info, self.state.program_id)? {
                let mut lock = Lock::from_account_mut(&info)?;
                // ro-lock is required
                if !info.is_writable {
                    ro_count += 1;

                    let mut ro_lock_bind = self.state.info_ro_lock(info.key, true)?;

                    let allocate_len = match lock.get()? {
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

                    // add ro-lock
                    lock.ro_lock()?;

                    // push holder.key to ro-lock-info
                    if allocate_len > 0 {
                        self.state.realloc(&mut ro_lock_bind, allocate_len)?;

                        let ro_lock_info = ro_lock_bind.into_account_info();
                        let mut ro_lock = RoLock::from_account_mut(&ro_lock_info)?;
                        assert!(!ro_lock.is_empty());
                        ro_lock.last_mut().unwrap().0 = state_holder.0.to_bytes();
                    }

                    self.state.update(ro_lock_bind)?;
                } else {
                    // writable lock is required
                    if let Some(lock) = lock.get()? {
                        return Err(AccountLocked(*info.key, Some(lock)));
                    }
                    // add rw-lock
                    lock.rw_lock(&state_holder.0)?;
                }

                managed = true;
            }

            if managed {
                self.state.update(bind)?;
            }
        }

        // ALT supports up to 256 accounts
        if ro_count >= 256 {
            return Err(Custom(format!("too many read-only accounts: {}", ro_count)));
        };

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
    fn lock_new_one(&self) -> Result<()> {
        Ok(())
    }
}
