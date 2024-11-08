use {
    crate::{
        accounts::{cast, cast_mut, Data, Slot},
        error::Result,
        AccountType, Lock, STORAGE_LEN,
    },
    evm::U256,
    solana_program::account_info::AccountInfo,
    std::{
        cell::{Ref, RefMut},
        mem::size_of,
    },
};

#[repr(C, packed)]
pub struct Storage {
    len: u16,
}

impl Storage {
    pub fn init(info: &AccountInfo) -> Result<()> {
        Lock::init(info, AccountType::Storage)?;

        let len = Storage::offset(info) + Storage::size(info);
        assert_eq!(len, info.data_len());

        let mut storage = Storage::from_account_mut(info)?;
        storage.len = 0;

        Ok(())
    }

    pub fn get(info: &AccountInfo, ix: u8) -> Result<Option<U256>> {
        assert!((ix as usize) < STORAGE_LEN);

        let len = Storage::from_account(info)?.len as usize;

        let slots = Slot::from_account(info)?;
        assert!(len <= slots.len());

        let res = slots[..len]
            .iter()
            .filter(|slot| slot.ix == ix)
            .collect::<Vec<&Slot>>();

        assert!(res.is_empty() || res.len() == 1);

        Ok(res.first().map(|a| U256::from_big_endian(&a.value)))
    }

    /// private fn
    fn push_or_update(info: &AccountInfo, value: &U256, ix: u8) -> Result<bool> {
        let len = Storage::from_account(info)?.len as usize;
        let mut slots = Slot::from_account_mut(info)?;

        assert!(len <= slots.len());

        let mut res = slots[..len]
            .iter_mut()
            .filter(|slot| slot.ix == ix)
            .collect::<Vec<&mut Slot>>();

        assert!(res.is_empty() || res.len() == 1);

        let push = if res.is_empty() {
            assert!(len < slots.len());
            let slot = slots.get_mut(len).unwrap();
            slot.ix = ix;
            value.to_big_endian(&mut slot.value);

            true
        } else {
            let slot = res.get_mut(0).unwrap();
            value.to_big_endian(&mut slot.value);

            false
        };

        Ok(push)
    }

    pub fn set(info: &AccountInfo, value: &U256, ix: u8) -> Result<()> {
        assert!((ix as usize) < STORAGE_LEN);

        if Storage::push_or_update(info, value, ix)? {
            let mut storage = Storage::from_account_mut(info)?;
            storage.len += 1;
        }

        Ok(())
    }

    pub fn unused_len(info: &AccountInfo) -> Result<usize> {
        let len = Storage::from_account(info)?.len as usize;
        let allocated = Slot::size(info);
        assert!(len <= allocated);

        Ok(allocated - len)
    }

    pub fn available(info: &AccountInfo, to_alloc: usize) -> Result<()> {
        let allocated = Slot::size(info);
        assert!(allocated + to_alloc <= STORAGE_LEN);
        Ok(())
    }
}

impl Data for Storage {
    type Item<'a> = Ref<'a, Self>;
    type ItemMut<'a> = RefMut<'a, Self>;

    fn from_account<'a>(info: &'a AccountInfo) -> Result<Self::Item<'a>> {
        cast(info, Self::offset(info), Self::size(info))
    }
    fn from_account_mut<'a>(info: &'a AccountInfo) -> Result<Self::ItemMut<'a>> {
        cast_mut(info, Self::offset(info), Self::size(info))
    }
    fn offset(info: &AccountInfo) -> usize {
        // account_type | Ver | Lock | Storage | Slot
        Lock::offset(info) + Lock::size(info)
    }
    fn size(_info: &AccountInfo) -> usize {
        size_of::<Self>()
    }
}
