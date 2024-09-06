use {
    super::{cast_slice, cast_slice_mut, AccountState, Data},
    crate::error::Result,
    solana_program::account_info::AccountInfo,
    std::cell::{Ref, RefMut},
};

pub struct Code {}

impl Data for Code {
    type Item<'a> = Ref<'a, [u8]>;
    type ItemMut<'a> = RefMut<'a, [u8]>;

    fn from_account<'a>(info: &'a AccountInfo) -> Result<Self::Item<'a>> {
        cast_slice(info, Self::offset(info), Self::size(info))
    }
    fn from_account_mut<'a>(info: &'a AccountInfo) -> Result<Self::ItemMut<'a>> {
        cast_slice_mut(info, Self::offset(info), Self::size(info))
    }
    fn offset(info: &AccountInfo) -> usize {
        AccountState::offset(info) + AccountState::size(info)
    }
    fn size(info: &AccountInfo) -> usize {
        let offset = Self::offset(info);
        assert!(info.data_len() >= offset);

        let len = info.data_len() - offset;
        if len == 0 {
            return 0;
        }

        let mut res = (8 * len - 7) / 9;
        let rest = (8 * len - 7) % 9;

        if rest > 0 {
            res += 1;
        }
        assert_eq!(len, res + evm::Valids::size_needed(res));

        res
    }
}
