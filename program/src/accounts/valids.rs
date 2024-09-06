use {
    super::{cast_slice, cast_slice_mut, Code, Data},
    crate::error::Result,
    solana_program::account_info::AccountInfo,
    std::cell::{Ref, RefMut},
};

pub struct Valids {}

impl Data for Valids {
    type Item<'a> = Ref<'a, [u8]>;
    type ItemMut<'a> = RefMut<'a, [u8]>;

    fn from_account<'a>(info: &'a AccountInfo) -> Result<Self::Item<'a>> {
        cast_slice(info, Self::offset(info), Self::size(info))
    }
    fn from_account_mut<'a>(info: &'a AccountInfo) -> Result<Self::ItemMut<'a>> {
        cast_slice_mut(info, Self::offset(info), Self::size(info))
    }
    fn offset(info: &AccountInfo) -> usize {
        Code::offset(info) + Code::size(info)
    }
    fn size(info: &AccountInfo) -> usize {
        let offset = Self::offset(info);
        let len = info.data_len();
        assert!(len >= offset);

        len - offset
    }
}
