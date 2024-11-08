use {
    super::{cast, cast_mut, Data},
    crate::{error::Result, AccountType},
    solana_program::account_info::AccountInfo,
    std::{
        cell::{Ref, RefMut},
        mem::size_of,
    },
};

#[repr(C, packed)]
pub struct Ver(u8);

impl Ver {
    pub fn init(info: &AccountInfo, typ: AccountType) -> Result<()> {
        AccountType::init(info, typ)?;

        let mut ver = Ver::from_account_mut(info)?;
        ver.0 = 0;

        Ok(())
    }
}

impl Data for Ver {
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
        AccountType::offset(info) + AccountType::size(info)
    }
}
