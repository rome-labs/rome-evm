use {
    super::{cast, cast_mut, Data},
    crate::error::{Result, RomeProgramError::*},
    solana_program::{account_info::AccountInfo, pubkey::Pubkey},
    std::{
        cell::{Ref, RefMut},
        mem::size_of,
    },
};

#[repr(u8)]
#[derive(PartialEq, Clone, Debug)]
pub enum AccountType {
    New = 0,
    Balance = 1,
    Storage = 2,
    TxHolder = 3,
    StateHolder = 4,
    RoLock = 5,
    OwnerInfo = 6,
}

impl AccountType {
    pub fn init(info: &AccountInfo, new: AccountType) -> Result<()> {
        let mut old = AccountType::from_account_mut(info)?;

        // TODO: create_pda must fill 0
        if *old != AccountType::New {
            return Err(AccountInitialized(*info.key));
        }
        assert!(new != AccountType::New);

        *old = new;
        Ok(())
    }

    pub fn check_owner(info: &AccountInfo, program_id: &Pubkey) -> Result<()> {
        // TODO
        if info.owner != program_id || info.data_len() == 0 {
            return Err(InvalidOwner(*info.key));
        }

        Ok(())
    }
    pub fn is_ok(info: &AccountInfo, typ: Self, program_id: &Pubkey) -> Result<()> {
        AccountType::check_owner(info, program_id)?;

        if *AccountType::from_account(info)? == typ {
            Ok(())
        } else {
            Err(InvalidAccountType(*info.key))
        }
    }
}

impl Data for AccountType {
    type Item<'a> = Ref<'a, Self>;
    type ItemMut<'a> = RefMut<'a, Self>;

    fn from_account<'a>(info: &'a AccountInfo) -> Result<Self::Item<'a>> {
        cast(info, Self::offset(info), Self::size(info))
    }
    fn from_account_mut<'a>(info: &'a AccountInfo) -> Result<Self::ItemMut<'a>> {
        cast_mut(info, Self::offset(info), Self::size(info))
    }
    fn offset(_info: &AccountInfo) -> usize {
        0
    }
    fn size(_info: &AccountInfo) -> usize {
        size_of::<Self>()
    }
}
