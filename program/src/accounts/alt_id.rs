use {
    super::{cast, cast_mut, AccountType, Data, Ver,},
    crate::error::Result,
    solana_program::account_info::AccountInfo,
    std::{
        cell::{Ref, RefMut,}, mem::size_of,
    },
};

#[derive(Clone, Default)]
#[repr(C, packed)]
pub struct AltId {
    pub session_id: u64,
}

impl AltId {
    pub fn init(info: &AccountInfo) -> Result<()> {
        Ver::init(info, AccountType::AltSlots)?;

        let len = AltId::offset(info) + AltId::size(info);
        assert_eq!(len, info.data_len());

        let mut alt_id = AltId::from_account_mut(info)?;
        alt_id.session_id = 0;

        Ok(())
    }
    pub fn has_session(info: &AccountInfo, id: u64) -> Result<bool> {
        let alt_id = AltId::from_account(info)?;
        Ok(alt_id.session_id == id)
    }
    pub fn set_session(info: &AccountInfo, id: u64) -> Result<()> {
        let mut alt_id = AltId::from_account_mut(info)?;
        alt_id.session_id = id;
        Ok(())
    }
}

impl Data for AltId {
    type Item<'a> = Ref<'a, Self>;
    type ItemMut<'a> = RefMut<'a, Self>;

    fn from_account<'a>(info: &'a AccountInfo) -> Result<Self::Item<'a>> {
        cast(info, Self::offset(info), Self::size(info))
    }
    fn from_account_mut<'a>(info: &'a AccountInfo) -> Result<Self::ItemMut<'a>> {
        cast_mut(info, Self::offset(info), Self::size(info))
    }
    fn offset(info: &AccountInfo) -> usize {
        Ver::offset(info) + Ver::size(info)
    }
    // account_type | ver | alt
    fn size(_info: &AccountInfo) -> usize {
        size_of::<Self>()
    }
}