use {
    crate::state::State,
    rome_evm::{
        error::{Result, RomeProgramError::*},
        info::Info,
        origin::Origin,
        AddressTable, Code, Data, Storage, H160, H256, U256,
    },
    solana_program::{
        account_info::IntoAccountInfo, clock::Slot, pubkey::Pubkey, sysvar::recent_blockhashes,
    },
    std::{
        cmp::Ordering::{Greater, Less},
        mem::size_of,
    },
};

impl Info for State<'_> {}

impl Origin for State<'_> {
    fn program_id(&self) -> &Pubkey {
        self.program_id
    }
    fn nonce(&self, address: &H160) -> Result<u64> {
        let mut bind = self.info_addr(address, false)?;
        let info = bind.into_account_info();
        Info::nonce(self, &info)
    }
    fn balance(&self, address: &H160) -> Result<U256> {
        let mut bind = self.info_addr(address, false)?;
        let info = bind.into_account_info();
        Info::balance(self, &info)
    }
    fn code(&self, address: &H160) -> Result<Vec<u8>> {
        let mut bind = self.info_addr(address, false)?;
        let info = bind.into_account_info();
        Info::code(self, &info)
    }
    fn valids(&self, address: &H160) -> Result<Vec<u8>> {
        let mut bind = self.info_addr(address, false)?;
        let info = bind.into_account_info();
        Info::valids(self, &info)
    }
    fn storage(&self, address: &H160, slot: &U256) -> Result<Option<U256>> {
        let (mut bind, sub_index) = self.info_slot(address, slot, false)?;
        let info = bind.into_account_info();
        Info::storage(self, &info, sub_index)
    }
    fn inc_nonce(&self, address: &H160) -> Result<()> {
        let mut bind = self.info_addr(address, true)?;
        let info = bind.into_account_info();
        Info::inc_nonce(self, &info)?;
        self.update(bind)
    }
    fn add_balance(&self, address: &H160, balance: &U256) -> Result<()> {
        let mut bind = self.info_addr(address, true)?;
        let info = bind.into_account_info();
        Info::add_balance(self, &info, balance)?;
        self.update(bind)
    }
    fn sub_balance(&self, address: &H160, balance: &U256) -> Result<()> {
        let mut bind = self.info_addr(address, true)?;
        let info = bind.into_account_info();
        Info::sub_balance(self, &info, balance, address)?;
        self.update(bind)
    }
    fn set_code(&self, address: &H160, code: &[u8], valids: &[u8]) -> Result<()> {
        let mut bind = self.info_addr(address, true)?;

        let len = bind.1.data.len();
        let offset = Code::offset(&bind.into_account_info());
        let required = offset + code.len() + valids.len();

        match len.cmp(&required) {
            Less => {
                self.realloc(&mut bind, required)?;
            }
            Greater => {
                // TODO: implement deallocation of the unused contract space
                return Err(Unimplemented(
                    format!(
                        "the contract deployment space must be deallocated according to size of the contract \
                        pubkey: {}, available: {}, required: {}",
                        bind.0,
                        len,
                        required
                    ))
                );
            }
            _ => {}
        }

        let info = bind.into_account_info();
        Info::set_code(self, &info, code, valids, address)?;
        self.update(bind)
    }
    fn set_storage(&self, address: &H160, slot: &U256, value: &U256) -> Result<()> {
        let (mut bind, sub_index) = self.info_slot(address, slot, true)?;

        let allocate = {
            let info = bind.into_account_info();
            let table = AddressTable::from_account(&info)?;
            let index = table.get(sub_index) as usize;
            index == 0
        };

        if allocate {
            let new_len = bind.1.data.len() + size_of::<Storage>();
            self.realloc(&mut bind, new_len)?;
            let info = bind.into_account_info();
            let len = Storage::from_account(&info)?.len();
            let mut table = AddressTable::from_account_mut(&info)?;
            table.set(sub_index, len);
        }

        let info = bind.into_account_info();
        Info::set_storage(self, &info, sub_index, value)?;
        self.update(bind)
    }
    fn block_hash(&self, block: U256, slot: Slot) -> Result<H256> {
        let mut bind = self
            .load(&recent_blockhashes::ID, None)?
            .ok_or(AccountNotFound(recent_blockhashes::ID))?;
        let info = bind.into_account_info();
        Info::block_hash(self, &info, block, slot)
    }

    fn allocated(&self) -> usize {
        *self.alloc.borrow()
    }

    fn deallocated(&self) -> usize {
        *self.dealloc.borrow()
    }
}
