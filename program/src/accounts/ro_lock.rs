use {
    super::{cast_slice, cast_slice_mut, slise_len, AccountType, Data, Ver},
    crate::error::Result,
    solana_program::{account_info::AccountInfo, pubkey::Pubkey},
    std::cell::{Ref, RefMut},
};

#[derive(Clone, Default)]
#[repr(C, packed)]
pub struct RoLock(pub [u8; 32]);

impl RoLock {
    pub fn init(info: &AccountInfo) -> Result<()> {
        Ver::init(info, AccountType::RoLock)?;

        let ro_lock = RoLock::from_account_mut(info)?;
        assert!(ro_lock.is_empty());

        Ok(())
    }

    pub fn found(info: &AccountInfo, holder: &Pubkey) -> Result<bool> {
        let ro_lock = RoLock::from_account(info)?;
        let mut found = false;
        for key in ro_lock.iter() {
            if key.0 == holder.to_bytes() {
                assert!(!found);
                found = true;
            }
        }

        Ok(found)
    }
    pub fn remove(info: &AccountInfo, holder: &Pubkey) -> Result<bool> {
        let mut ro_lock = RoLock::from_account_mut(info)?;

        let last = if let Some(last) = ro_lock.last() {
            last.0
        } else {
            return Ok(false);
        };

        let mut found = false;
        for key in ro_lock.iter_mut() {
            if key.0 == holder.to_bytes() {
                key.0 = last;
                assert!(!found);
                found = true;
            }
        }

        if found {
            *ro_lock.last_mut().unwrap() = RoLock::default()
        }

        Ok(found)
    }

    pub fn add_preallocated(info: &AccountInfo, holder: &Pubkey) -> Result<()> {
        let mut ro_lock = RoLock::from_account_mut(info)?;
        assert!(!ro_lock.is_empty());
        ro_lock.last_mut().unwrap().0 = holder.to_bytes();
        Ok(())
    }
}

impl Data for RoLock {
    type Item<'a> = Ref<'a, [Self]>;
    type ItemMut<'a> = RefMut<'a, [Self]>;

    fn from_account<'a>(info: &'a AccountInfo) -> Result<Self::Item<'a>> {
        cast_slice(info, Self::offset(info), Self::size(info))
    }
    fn from_account_mut<'a>(info: &'a AccountInfo) -> Result<Self::ItemMut<'a>> {
        cast_slice_mut(info, Self::offset(info), Self::size(info))
    }
    fn offset(info: &AccountInfo) -> usize {
        // account_type | ver | lock_overrides
        Ver::offset(info) + Ver::size(info)
    }
    fn size(info: &AccountInfo) -> usize {
        slise_len::<Self>(info)
    }
}
