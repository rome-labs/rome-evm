use {
    super::{
        AccountType, {cast, cast_mut, Data, Ver},
    },
    crate::error::Result,
    evm::H256,
    solana_program::{account_info::AccountInfo, msg},
    std::{
        cell::{Ref, RefMut},
        mem::size_of,
    },
};
use crate::error::RomeProgramError::CalculationOverflow;

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
    UnlockFailedTx = 10,
    Completed = 11, 
    Failed = 12, 
}

impl Iterations {
    pub fn is_complete(&self) -> bool {
        match self {
            Iterations::Unlock | Iterations::Completed => true,
            Iterations::UnlockFailedTx | Iterations::Failed => {
                msg!("transaction failed, last iteration: {:?}", self);
                false
            },
            _ => {
                msg!("not enough iterations, last iteration: {:?}", self);
                false
            }
        }
    }
}
#[repr(C, packed)]
pub struct StateHolder {
    pub iteration: Iterations,
    pub hash: H256,
    pub session: u64,
    pub lamports_fee: u64,
    pub lamports_refund: u64,
    pub iter_cnt: u64,
}

impl StateHolder {
    pub fn init(info: &AccountInfo) -> Result<()> {
        Ver::init(info, AccountType::StateHolder)?;

        let mut holder = StateHolder::from_account_mut(info)?;
        holder.iteration = Iterations::Lock;
        holder.hash = H256::default();
        holder.session = 0;
        holder.lamports_fee = 0;
        holder.lamports_refund = 0;
        holder.iter_cnt = 0;

        Ok(())
    }
    pub fn has_session(info: &AccountInfo, hash: H256, session: u64) -> Result<bool> {
        let state_holder = StateHolder::from_account(info)?;
        Ok(state_holder.hash == hash && state_holder.session == session)
    }
    pub fn set_session(info: &AccountInfo, hash: H256, session: u64) -> Result<()> {
        let mut state_holder = StateHolder::from_account_mut(info)?;
        state_holder.hash = hash;
        state_holder.session = session;
        state_holder.lamports_fee = 0;
        state_holder.lamports_refund = 0;
        state_holder.iter_cnt = 0;
        
        Ok(())
    }
    pub fn set_iteration(info: &AccountInfo, iteration: Iterations) -> Result<()> {
        let mut state_holder = StateHolder::from_account_mut(info)?;
        state_holder.iteration = iteration;
        state_holder.iter_cnt += 1;
        Ok(())
    }
    pub fn get_iteration(info: &AccountInfo) -> Result<Iterations> {
        let state_holder = StateHolder::from_account(info)?;
        Ok(state_holder.iteration.clone())
    }

    pub fn collect_fees(info: &AccountInfo, fee: u64, refund: u64) -> Result<()> {
        let mut state_holder = StateHolder::from_account_mut(info)?;
        state_holder.lamports_fee = state_holder
            .lamports_fee
            .checked_add(fee)
            .ok_or(CalculationOverflow)?;

        state_holder.lamports_refund = state_holder
            .lamports_refund
            .checked_add(refund)
            .ok_or(CalculationOverflow)?;

        Ok(())
    }

    pub fn fees(info: &AccountInfo) -> Result<(u64, u64)> {
        let state_holder = StateHolder::from_account(info)?;

        Ok((state_holder.lamports_fee, state_holder.lamports_refund))
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
