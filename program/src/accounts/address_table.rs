use {
    crate::{
        accounts::{cast, cast_mut, Data},
        error::Result,
        AccountType, Lock,
    },
    solana_program::account_info::AccountInfo,
    std::{
        cell::{Ref, RefMut},
        mem::size_of,
    },
};

pub const ADDRESS_TABLE_SIZE: usize = 255;
#[repr(C, packed)]
pub struct AddressTable([u8; ADDRESS_TABLE_SIZE]);

impl AddressTable {
    pub fn init(info: &AccountInfo) -> Result<()> {
        AccountType::init(info, AccountType::Storage)?;
        Lock::init(info)?;

        let len = AddressTable::offset(info) + AddressTable::size(info);
        assert_eq!(len, info.data_len());

        let mut table = AddressTable::from_account_mut(info)?;
        table.0.fill(0);

        Ok(())
    }

    pub fn get(&self, index: usize) -> u8 {
        assert!(index < ADDRESS_TABLE_SIZE);
        self.0[index]
    }

    pub fn set(&mut self, index: usize, value: usize) {
        assert!(index < ADDRESS_TABLE_SIZE);
        assert!(value > 0 && value < 256);
        self.0[index] = value as u8;
    }
}

impl Data for AddressTable {
    type Item<'a> = Ref<'a, Self>;
    type ItemMut<'a> = RefMut<'a, Self>;

    fn from_account<'a>(info: &'a AccountInfo) -> Result<Self::Item<'a>> {
        cast(info, Self::offset(info), Self::size(info))
    }
    fn from_account_mut<'a>(info: &'a AccountInfo) -> Result<Self::ItemMut<'a>> {
        cast_mut(info, Self::offset(info), Self::size(info))
    }
    fn offset(info: &AccountInfo) -> usize {
        // account_type | Lock | address_table | Values
        Lock::offset(info) + Lock::size(info)
    }
    fn size(_info: &AccountInfo) -> usize {
        size_of::<Self>()
    }
}
