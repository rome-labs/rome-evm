use {
    crate::{
        accounts::{cast_slice, cast_slice_mut, slise_len, Data, Storage},
        error::Result,
        STORAGE_LEN,
    },
    solana_program::account_info::AccountInfo,
    std::cell::{Ref, RefMut},
};

#[derive(Clone)]
#[repr(C, packed)]
pub struct Slot {
    pub ix: u8,
    pub value: [u8; 32],
}

impl Data for Slot {
    type Item<'a> = Ref<'a, [Self]>;
    type ItemMut<'a> = RefMut<'a, [Self]>;

    fn from_account<'a>(info: &'a AccountInfo) -> Result<Self::Item<'a>> {
        cast_slice(info, Self::offset(info), Self::size(info))
    }
    fn from_account_mut<'a>(info: &'a AccountInfo) -> Result<Self::ItemMut<'a>> {
        cast_slice_mut(info, Self::offset(info), Self::size(info))
    }
    fn offset(info: &AccountInfo) -> usize {
        // account_type | Ver | Lock | Storage | [Slot]
        Storage::offset(info) + Storage::size(info)
    }
    fn size(info: &AccountInfo) -> usize {
        let cnt = slise_len::<Self>(info);
        assert!(cnt <= STORAGE_LEN);
        cnt
    }
}
