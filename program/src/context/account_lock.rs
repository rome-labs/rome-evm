use {
    super::{ContextAtomic, ContextIterative},
    crate::{
        accounts::{Data, Lock, LockType, RoLock},
        error::{Result, RomeProgramError::*},
    },
    solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey},
    std::mem::size_of,
};

pub trait AccountLock {
    fn lock(&self) -> Result<()>;
    fn locked(&self) -> Result<bool>;
    fn unlock(&self) -> Result<()>;
    fn lock_new_one(&self, info: &AccountInfo) -> Result<()>;
    fn check_writable(&self, info: &AccountInfo) -> Result<()>;
}

impl AccountLock for ContextAtomic<'_, '_> {
    fn lock(&self) -> Result<()> {
        for &info in self.state.all().values() {
            // existings locks can only affect writable accounts of the atomic tx
            if Lock::is_managed(info, self.state.program_id)? && info.is_writable {
                let lock = Lock::from_account_mut(info)?;
                if lock.get()?.is_some() {
                    return Err(AccountLocked(*info.key, lock.lock));
                }
            }
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
        unreachable!()
    }
    fn check_writable(&self, _info: &AccountInfo) -> Result<()> {
        Ok(())
    }
}

impl AccountLock for ContextIterative<'_, '_> {
    fn lock(&self) -> Result<()> {
        assert!(self.origin_accounts.len() <= 256); // ALT supports up to 256 accounts

        // TODO: there may be allocation and deallocation together (ro_lock)
        for (index, info) in self.origin_accounts.iter().enumerate() {
            if Lock::is_managed(info, self.state.program_id)? {
                let mut lock = Lock::from_account_mut(info)?;
                // ro-lock is required
                if self.lock_overrides.iter().any(|&ro| ro == index as u8) {
                    let ro_lock_info = self.state.info_ro_lock(info.key, true)?;
                    let push = match lock.get()? {
                        Some(LockType::Ro) => {
                            // allocate/resize ro-lock-info
                            if !RoLock::found(ro_lock_info, self.state_holder.key)? {
                                let len = ro_lock_info.data_len() + size_of::<RoLock>();
                                self.state.realloc(ro_lock_info, len)?;
                                true
                            } else {
                                false
                            }
                        }
                        Some(LockType::Rw(_)) => return Err(AccountLocked(*info.key, lock.lock)),
                        None => {
                            // TODO: split allocation and deallocations to different iterations
                            // allocate/deallocate ro-lock-info
                            let len = RoLock::offset(ro_lock_info) + size_of::<RoLock>();
                            self.state.realloc(ro_lock_info, len)?;
                            true
                        }
                    };

                    // add ro-lock
                    lock.ro_lock()?;
                    // push holder.key to ro-lock-info
                    if push {
                        RoLock::add_preallocated(ro_lock_info, self.state_holder.key)?;
                    }
                } else {
                    // writable lock is required
                    if let Some(lock) = lock.get()? {
                        return Err(AccountLocked(*info.key, Some(lock)));
                    }
                    // add rw-lock
                    lock.rw_lock(self.state_holder.key)?;
                }
            }
        }
        Ok(())
    }
    fn locked(&self) -> Result<bool> {
        for info in self.origin_accounts {
            if Lock::is_managed(info, self.state.program_id)? {
                let mut lock = Lock::from_account_mut(info)?;
                match lock.get()? {
                    None => {
                        msg!("account lock not found: {}", info.key);
                        return Ok(false);
                    }
                    Some(LockType::Ro) => {
                        let ro_lock_info = self.state.info_ro_lock(info.key, false)?;
                        if !RoLock::found(ro_lock_info, self.state_holder.key)? {
                            msg!("account ro-lock not found: {}", info.key);
                            return Ok(false);
                        }
                    }
                    Some(LockType::Rw(holder)) => {
                        if holder != self.state_holder.key.to_bytes() {
                            msg!("account is rw-locked by another tx: {}", info.key);
                            return Ok(false);
                        }
                    }
                }
                lock.update()?;
            }
        }

        Ok(true)
    }
    fn unlock(&self) -> Result<()> {
        for info in self.origin_accounts {
            if Lock::is_managed(info, self.state.program_id)? {
                let mut lock = Lock::from_account_mut(info)?;

                match lock.get()? {
                    Some(LockType::Ro) => {
                        let ro_info = self.state.info_ro_lock(info.key, false)?;

                        if !RoLock::remove(ro_info, self.state_holder.key)? {
                            msg!("ro-lock of current tx is absent for account {}", info.key);
                            continue;
                        };

                        // deallocate ro-lock-info
                        let len = ro_info.data_len().saturating_sub(size_of::<RoLock>());
                        assert!(len >= RoLock::offset(ro_info));

                        self.state.realloc(ro_info, len)?;
                        if RoLock::size(ro_info) == 0 {
                            lock.unlock()
                        }
                    }
                    Some(LockType::Rw(holder)) => {
                        // before Unlock iteration the account lock might be expired
                        // and account might be locked by another transaction
                        if holder == self.state_holder.key.to_bytes() {
                            lock.unlock()
                        } else {
                            let key = Pubkey::from(holder);
                            msg!(
                                "account {} is rw-locked by another holder {}",
                                info.key,
                                key
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }
    fn lock_new_one(&self, info: &AccountInfo) -> Result<()> {
        if Lock::is_managed(info, self.state.program_id)? {
            let mut lock = Lock::from_account_mut(info)?;
            if lock.is_new_one() {
                lock.rw_lock(self.state_holder.key)?;
            }
        }

        Ok(())
    }
    /// this is not lock checking (lock can be expired).
    /// It is just checking the type of lock, that was checked earlier.
    fn check_writable(&self, info: &AccountInfo) -> Result<()> {
        let lock = Lock::from_account(info)?;
        if let Some(LockType::Ro) = lock.lock {
            return Err(AttemptWriteRoAccount(*info.key));
        }

        Ok(())
    }
}
