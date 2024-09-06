use {
    super::{AccountType, Data},
    crate::{
        accounts::{cast, cast_mut},
        error::Result,
    },
    evm::H160,
    solana_program::account_info::AccountInfo,
    std::cell::{Ref, RefMut},
    std::mem::size_of,
};

#[derive(Clone, Default)]
#[repr(C, packed)]
pub struct SignerInfo {
    pub address: H160,
}

impl SignerInfo {
    pub fn init(info: &AccountInfo) -> Result<()> {
        AccountType::init(info, AccountType::SignerInfo)?;

        let mut signer_info = SignerInfo::from_account_mut(info)?;
        signer_info.address = H160::default();
        Ok(())
    }
}

impl Data for SignerInfo {
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
        // account_type | signer_info
        AccountType::offset(info) + AccountType::size(info)
    }
}
