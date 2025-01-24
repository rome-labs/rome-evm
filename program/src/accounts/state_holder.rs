use {
    super::{
        AccountType, {cast, cast_mut, Data, Ver},
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
#[derive(Clone, Debug)]
pub enum Iterations {
    Lock = 1,
    Start = 2,
    Execute = 3,
    Allocate = 5,
    MergeSlots = 6,
    AllocateStorage = 7,
    Commit = 8,
    Unlock = 9,
    Unnecessary = 10, // UnnecessaryIteration
}

impl Iterations {
    pub fn is_complete(&self) -> bool {
        if !matches!(self, Iterations::Unlock | Iterations::Unnecessary) {
            solana_program::msg!("not enough iterations, last iteration: {:?}", self);
            return false;
        }

        true
    }
}
#[repr(C, packed)]
pub struct StateHolder {
    pub iteration: Iterations,
    pub hash: H256,
    pub session: u64,
}

impl StateHolder {
    pub fn init(info: &AccountInfo) -> Result<()> {
        Ver::init(info, AccountType::StateHolder)?;

        let mut holder = StateHolder::from_account_mut(info)?;
        holder.iteration = Iterations::Lock;
        holder.hash = H256::default();
        holder.session = 0;

        Ok(())
    }
    pub fn is_linked(info: &AccountInfo, hash: H256, session: u64) -> Result<bool> {
        let state_holder = StateHolder::from_account(info)?;
        Ok(state_holder.hash == hash && state_holder.session == session)
    }
    pub fn set_link(info: &AccountInfo, hash: H256, session: u64) -> Result<()> {
        let mut state_holder = StateHolder::from_account_mut(info)?;
        state_holder.hash = hash;
        state_holder.session = session;
        Ok(())
    }
    pub fn set_iteration(info: &AccountInfo, iteration: Iterations) -> Result<()> {
        let mut state_holder = StateHolder::from_account_mut(info)?;
        state_holder.iteration = iteration;
        Ok(())
    }
    pub fn get_iteration(info: &AccountInfo) -> Result<Iterations> {
        let state_holder = StateHolder::from_account(info)?;
        Ok(state_holder.iteration.clone())
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
        Ver::offset(info) + Ver::size(info)
    }
}
