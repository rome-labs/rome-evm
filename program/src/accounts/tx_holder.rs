use {
    super::{
        AccountType, {cast, cast_mut, Data, Ver},
    },
    crate::{
        error::{Result, RomeProgramError::*},
        Holder,
    },
    evm::H256,
    solana_program::{account_info::AccountInfo, keccak},
    std::{
        cell::{Ref, RefMut},
        mem::size_of,
        ops::Deref,
    },
};

#[repr(C, packed)]
pub struct TxHolder {
    pub hash: H256,
    reserved: [u8; 9],
}

impl TxHolder {
    pub fn init(info: &AccountInfo) -> Result<()> {
        Ver::init(info, AccountType::TxHolder)?;

        let mut tx_holder = TxHolder::from_account_mut(info)?;
        tx_holder.hash = H256::default();
        Ok(())
    }
    pub fn check_hash(&self, info: &AccountInfo, ix_hash: H256) -> Result<()> {
        let tx = Holder::from_account(info)?;
        let tx_hash = H256::from(keccak::hash(tx.deref()).to_bytes());
        if self.hash == tx_hash && ix_hash == tx_hash {
            Ok(())
        } else {
            Err(InvalidHolderHash(*info.key))
        }
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
