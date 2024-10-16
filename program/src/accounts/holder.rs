use {
    super::{cast_slice, cast_slice_mut, Data, TxHolder},
    crate::{
        error::{Result, RomeProgramError::IncorrectChainId},
        tx::tx::Tx,
        H256,
    },
    solana_program::account_info::AccountInfo,
    std::cell::{Ref, RefMut},
};

#[repr(C, packed)]
pub struct Holder {}

impl Holder {
    pub fn tx(info: &AccountInfo, hash: H256, chain: u64) -> Result<Tx> {
        let holder = TxHolder::from_account(info)?;
        holder.check_hash(info, hash)?;
        let rlp = Holder::from_account(info)?;
        let tx = Tx::from_instruction(&rlp)?;
        if tx.chain_id() == chain {
            Ok(tx)
        } else {
            Err(IncorrectChainId(Some((tx.chain_id(), chain))))
        }
    }

    pub fn fill(info: &AccountInfo, hash: H256, from: usize, to: usize, tx: &[u8]) -> Result<()> {
        TxHolder::from_account_mut(info)?.hash = hash;
        let holder = Holder::from_account_mut(info)?;
        let mut location = RefMut::map(holder, |a| &mut a[from..to]);
        location.copy_from_slice(tx);
        Ok(())
    }
}

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
