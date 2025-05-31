use {
    super::{cast_slice, cast_slice_mut, Data, TxHolder, StateHolder, AccountType},
    crate::{
        error::{Result, RomeProgramError::IncorrectChainId},
        tx::tx::Tx,
        H256,
    },
    solana_program::{
        account_info::AccountInfo, keccak
    },
    std::cell::{Ref, RefMut},
};

#[repr(C, packed)]
pub struct Holder {}

impl Holder {
    pub fn rlp<'a>(info: &'a AccountInfo, ix_hash: H256, chain: u64) -> Result<Ref<'a, [u8]>> {
        let rlp = Holder::from_account(info)?;
        let rlp_hash = H256::from(keccak::hash(&rlp).to_bytes());

        TxHolder::check_hash(info, ix_hash, rlp_hash)?;

        let rpl_chain_id = Tx::chain_id_from_rlp(&rlp)?;

        if rpl_chain_id == chain {
            Ok(rlp)
        } else {
            Err(IncorrectChainId(Some((rpl_chain_id, chain))))
        }
    }

    pub fn fill(info: &AccountInfo, from: usize, to: usize, tx: &[u8]) -> Result<()> {
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
        let typ = AccountType::from_account(info)
            .expect("invalid argument Holder::offset");

        match *typ {
            AccountType::TxHolder => TxHolder::offset(info) + TxHolder::size(info),
            AccountType::StateHolder => StateHolder::offset(info) + StateHolder::size(info),
            _ => unreachable!()
        }
    }
}
