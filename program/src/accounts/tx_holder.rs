use {
    super::{
        AccountType, {cast, cast_mut, Data, Ver},
    },
    crate::{
        error::{Result, RomeProgramError::*},
    },
    evm::H256,
    solana_program::{account_info::AccountInfo},
    std::{
        cell::{Ref, RefMut},
        mem::size_of,
    },
};

#[repr(C, packed)]
pub struct TxHolder {
    pub hash: H256,
    pub iter_cnt: u16,
}

impl TxHolder {
    pub fn init(info: &AccountInfo) -> Result<()> {
        Ver::init(info, AccountType::TxHolder)?;

        let mut tx_holder = TxHolder::from_account_mut(info)?;
        tx_holder.hash = H256::default();
        tx_holder.iter_cnt = 0;

        Ok(())
    }
    pub fn check_hash(info: &AccountInfo, ix_hash: H256, rlp_hash: H256) -> Result<()> {
        let tx_holder = TxHolder::from_account(info)?;
        if tx_holder.hash == ix_hash && tx_holder.hash == rlp_hash {
            Ok(())
        } else {
            Err(InvalidHolderHash(*info.key))
        }
    }
    pub fn reset(info:&AccountInfo, hash: H256) -> Result<()> {
        let mut tx_holder = TxHolder::from_account_mut(info)?;
        tx_holder.hash = hash;
        tx_holder.iter_cnt = 0;

        Ok(())
    }
    pub fn inc_iteration(info: &AccountInfo) -> Result<()> {
        let mut tx_holder = TxHolder::from_account_mut(info)?;
        tx_holder.iter_cnt = tx_holder
            .iter_cnt
            .checked_add(1)
            .ok_or(CalculationOverflow)?;

        Ok(())
    }
}

impl Data for TxHolder {
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
