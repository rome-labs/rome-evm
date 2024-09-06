use {
    crate::{
        accounts::{cast_slice, cast_slice_mut, AddressTable, Data},
        error::Result,
    },
    solana_program::account_info::AccountInfo,
    std::{
        cell::{Ref, RefMut},
        mem::size_of,
    },
};

#[derive(Clone)]
#[repr(C, packed)]
pub struct Storage(pub [u8; 32]);

impl Data for Storage {
    type Item<'a> = Ref<'a, [Self]>;
    type ItemMut<'a> = RefMut<'a, [Self]>;

    fn from_account<'a>(info: &'a AccountInfo) -> Result<Self::Item<'a>> {
        cast_slice(info, Self::offset(info), Self::size(info))
    }
    fn from_account_mut<'a>(info: &'a AccountInfo) -> Result<Self::ItemMut<'a>> {
        cast_slice_mut(info, Self::offset(info), Self::size(info))
    }
    fn offset(info: &AccountInfo) -> usize {
        // account_type | address_tabe | values
        AddressTable::offset(info) + AddressTable::size(info)
    }
    fn size(info: &AccountInfo) -> usize {
        let offset = Self::offset(info);
        let mut len = info.data.borrow().len();
        assert!(len >= offset);
        len -= offset;
        assert!(len % size_of::<Self>() == 0);
        len / size_of::<Self>()
    }
}
