use {
    super::{
        AccountType::{self, *},
        Ver, {cast, cast_mut, Data},
    },
    crate::{
        config::LOCK_DURATION,
        error::{Result, RomeProgramError::*},
    },
    solana_program::{account_info::AccountInfo, clock::Clock, pubkey::Pubkey, sysvar::Sysvar},
    std::{
        cell::{Ref, RefMut},
        fmt::{self, Debug, Formatter},
        mem::size_of,
    },
};

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum LockType {
    Ro = 0,
    Rw([u8; 32]) = 1,
}

impl Debug for LockType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LockType::Ro => write!(f, "ro"),
            LockType::Rw(bin) => {
                let key = Pubkey::from(*bin);
                write!(f, "rw: {}", key)
            }
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Lock {
    pub lock: Option<LockType>,
    timestamp: i64,
}

impl Lock {
    pub fn init(info: &AccountInfo, typ: AccountType) -> Result<()> {
        Ver::init(info, typ)?;

        let mut lock = Lock::from_account_mut(info)?;
        *lock = Lock {
            lock: None,
            timestamp: 0,
        };

        Ok(())
    }
    pub fn is_managed(info: &AccountInfo, program_id: &Pubkey) -> Result<bool> {
        if AccountType::check_owner(info, program_id).is_ok() {
            let typ = AccountType::from_account(info)?;
            return Ok(*typ == Balance || *typ == Storage);
        }

        Ok(false)
    }
    fn is_expired(&self) -> Result<bool> {
        let expired = Clock::get()?
            .unix_timestamp
            .checked_sub(self.timestamp)
            .ok_or(CalculationUnderflow)?
            >= LOCK_DURATION;

        Ok(expired)
    }
    pub fn get(&self) -> Result<Option<LockType>> {
        let mut lock = None;

        if self.lock.is_some() && !self.is_expired()? {
            lock = self.lock
        }

        Ok(lock)
    }
    pub fn ro_lock(&mut self) -> Result<()> {
        self.lock = Some(LockType::Ro);
        self.timestamp = Clock::get()?.unix_timestamp;
        Ok(())
    }
    pub fn rw_lock(&mut self, holder: &Pubkey) -> Result<()> {
        self.lock = Some(LockType::Rw(holder.to_bytes()));
        self.timestamp = Clock::get()?.unix_timestamp;
        Ok(())
    }
    pub fn unlock(&mut self) {
        self.lock = None
    }
    pub fn update(&mut self) -> Result<()> {
        self.timestamp = Clock::get()?.unix_timestamp;
        Ok(())
    }
    pub fn is_new_one(&self) -> bool {
        self.lock.is_none() && self.timestamp == 0
    }
}

impl Data for Lock {
    type Item<'a> = Ref<'a, Self>;
    type ItemMut<'a> = RefMut<'a, Self>;

    fn from_account<'a>(info: &'a AccountInfo) -> Result<Self::Item<'a>> {
        cast(info, Self::offset(info), Self::size(info))
    }
    fn from_account_mut<'a>(info: &'a AccountInfo) -> Result<Self::ItemMut<'a>> {
        cast_mut(info, Self::offset(info), Self::size(info))
    }
    fn size(_info: &AccountInfo) -> usize {
        size_of::<Self>()
    }
    fn offset(info: &AccountInfo) -> usize {
        Ver::offset(info) + Ver::size(info)
    }
}
