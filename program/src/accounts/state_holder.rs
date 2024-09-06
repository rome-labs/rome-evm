use {
    super::{
        AccountType, {cast, cast_mut, Data},
    },
    crate::error::Result,
    evm::H256,
    solana_program::account_info::AccountInfo,
    std::{
        cell::{Ref, RefMut},
        mem::size_of,
    },
};

#[repr(u8)]
#[derive(Clone)]
pub enum Iterations {
    Lock = 1,
    Start = 2,
    Execute = 3,
    AllocateHolder = 4,
    Allocate = 5,
    Commit = 6,
    Unlock = 7,
    Error = 8,
}

#[repr(C, packed)]
pub struct StateHolder {
    pub iteration: Iterations,
    pub hash: H256,
}

impl StateHolder {
    pub fn init(info: &AccountInfo) -> Result<()> {
        AccountType::init(info, AccountType::StateHolder)?;

        let mut holder = StateHolder::from_account_mut(info)?;
        holder.iteration = Iterations::Lock;
        holder.hash = H256::default();

        Ok(())
    }
}

impl Data for StateHolder {
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
