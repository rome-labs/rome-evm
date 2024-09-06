use {
    super::{cast, cast_mut, Data, Lock},
    crate::{error::Result, AccountType},
    evm::U256,
    solana_program::{account_info::AccountInfo, pubkey::Pubkey},
    std::{
        cell::{Ref, RefMut},
        mem::size_of,
    },
};

#[repr(C, packed)]
pub struct AccountState {
    pub nonce: u64,
    pub balance: U256,
    pub is_contract: bool,
}

impl AccountState {
    pub fn init(info: &AccountInfo) -> Result<()> {
        AccountType::init(info, AccountType::Balance)?;
        Lock::init(info)?;

        let len = AccountState::offset(info) + AccountState::size(info);
        assert_eq!(len, info.data_len());

        let mut state = AccountState::from_account_mut(info)?;

        *state = AccountState {
            nonce: 0,
            balance: U256::zero(),
            is_contract: false,
        };

        Ok(())
    }
    pub fn is_ok<'a>(info: &'a AccountInfo<'a>, program_id: &Pubkey) -> Result<()> {
        AccountType::is_ok(info, AccountType::Balance, program_id)?;
        AccountType::from_account(info)?;
        Ok(())
    }
    pub fn is_contract<'a>(info: &'a AccountInfo<'a>) -> Result<bool> {
        let state = AccountState::from_account(info)?;
        Ok(state.is_contract)
    }
}

impl Data for AccountState {
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
        Lock::offset(info) + Lock::size(info)
    }
}
