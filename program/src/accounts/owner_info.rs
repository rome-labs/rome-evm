use {
    super::{AccountType, Data},
    crate::{
        accounts::{cast_slice, cast_slice_mut, slise_len},
        error::{Result, RomeProgramError::UnregisteredChainId,},
        H160,
    },
    solana_program::{account_info::AccountInfo, pubkey::Pubkey},
    std::cell::{Ref, RefMut},
};

#[derive(Clone, Default)]
#[repr(C, packed)]
pub struct OwnerInfo {
    pub key: Pubkey,
    pub chain: u64,
    pub mint_address: Option<H160>,
}

impl OwnerInfo {
    pub fn get_mut<'a>(
        info: &'a AccountInfo,
        key: &Pubkey,
        chain: u64,
    ) -> Result<Option<RefMut<'a, Self>>> {
        let reg = OwnerInfo::from_account_mut(info)?;

        for (ix, owner) in reg.iter().enumerate() {
            if owner.key == *key && owner.chain == chain {
                let owner_ref = RefMut::map(reg, |a| &mut a[ix]);
                return Ok(Some(owner_ref));
            }
        }
        Ok(None)
    }

    pub fn is_owned(info: &AccountInfo, chain: u64) -> Result<bool> {
        let reg = OwnerInfo::from_account(info)?;

        for owner in reg.iter() {
            if owner.chain == chain {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn check_chain(info: &AccountInfo, chain: u64) -> Result<()> {
        if !OwnerInfo::is_owned(info, chain)? {
            return Err(UnregisteredChainId(chain));
        }

        Ok(())
    }
}

impl Data for OwnerInfo {
    type Item<'a> = Ref<'a, [Self]>;
    type ItemMut<'a> = RefMut<'a, [Self]>;

    fn from_account<'a>(info: &'a AccountInfo) -> Result<Self::Item<'a>> {
        cast_slice(info, Self::offset(info), Self::size(info))
    }
    fn from_account_mut<'a>(info: &'a AccountInfo) -> Result<Self::ItemMut<'a>> {
        cast_slice_mut(info, Self::offset(info), Self::size(info))
    }
    fn offset(info: &AccountInfo) -> usize {
        // account_type | reg_owner
        AccountType::offset(info) + AccountType::size(info)
    }
    fn size(info: &AccountInfo) -> usize {
        slise_len::<Self>(info)
    }
}
