use {
    super::{cast_slice, cast_slice_mut, Data, TxHolder},
    crate::error::Result,
    solana_program::account_info::AccountInfo,
    std::cell::{Ref, RefMut},
};

#[repr(C, packed)]
pub struct Holder {}
impl Data for Holder {
    type Item<'a> = Ref<'a, [u8]>;
    type ItemMut<'a> = RefMut<'a, [u8]>;

    fn from_account<'a>(info: &'a AccountInfo) -> Result<Self::Item<'a>> {
        cast_slice(info, Self::offset(info), Self::size(info))
    }
    fn from_account_mut<'a>(info: &'a AccountInfo) -> Result<Self::ItemMut<'a>> {
        cast_slice_mut(info, Self::offset(info), Self::size(info))
    }
    fn size(info: &AccountInfo) -> usize {
        assert!(info.data_len() >= Self::offset(info));
        info.data_len() - Self::offset(info)
    }
    fn offset(info: &AccountInfo) -> usize {
        // size_of::<TxHolder> == size_of<StateHolder>
        TxHolder::offset(info) + TxHolder::size(info)
    }
}
