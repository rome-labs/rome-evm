use {
    crate::{
        accounts::{AccountState, Data},
        error::{Result, RomeProgramError::*},
        info::Info,
        state::State,
        AddressTable, Code, Storage, EVENT_LOG,
    },
    evm::{H160, H256, U256},
    solana_program::{
        clock::Slot, entrypoint::MAX_PERMITTED_DATA_INCREASE, log::sol_log_data, msg,
        pubkey::Pubkey, sysvar::recent_blockhashes,
    },
    std::{cmp::Ordering::*, mem::size_of},
};

pub trait Origin {
    fn program_id(&self) -> &Pubkey;
    fn nonce(&self, address: &H160) -> Result<u64>;
    fn balance(&self, address: &H160) -> Result<U256>;
    fn code(&self, address: &H160) -> Result<Vec<u8>>;
    fn valids(&self, address: &H160) -> Result<Vec<u8>>;
    fn storage(&self, address: &H160, slot: &U256) -> Result<Option<U256>>;

    fn inc_nonce(&self, address: &H160) -> Result<()>;
    fn add_balance(&self, address: &H160, balance: &U256) -> Result<()>;
    fn sub_balance(&self, address: &H160, balance: &U256) -> Result<()>;
    fn set_code(&self, address: &H160, code: &[u8], valids: &[u8]) -> Result<()>;
    fn set_storage(&self, address: &H160, slot: &U256, value: &U256) -> Result<()>;
    fn block_hash(&self, block: U256, slot: Slot) -> Result<H256>;
    fn set_logs(&self, address: &H160, topics: &[H256], data: &[u8]) -> Result<()> {
        match topics.len() {
            0 => sol_log_data(&[EVENT_LOG, address.as_bytes(), &[0_u8], data]),
            1 => sol_log_data(&[
                EVENT_LOG,
                address.as_bytes(),
                &[1_u8],
                topics[0].as_bytes(),
                data,
            ]),
            2 => sol_log_data(&[
                EVENT_LOG,
                address.as_bytes(),
                &[2_u8],
                topics[0].as_bytes(),
                topics[1].as_bytes(),
                data,
            ]),
            3 => sol_log_data(&[
                EVENT_LOG,
                address.as_bytes(),
                &[3_u8],
                topics[0].as_bytes(),
                topics[1].as_bytes(),
                topics[2].as_bytes(),
                data,
            ]),
            4 => sol_log_data(&[
                EVENT_LOG,
                address.as_bytes(),
                &[4_u8],
                topics[0].as_bytes(),
                topics[1].as_bytes(),
                topics[2].as_bytes(),
                topics[3].as_bytes(),
                data,
            ]),
            _ => panic!("vm fault, event logs topics.len > 4"),
        }

        Ok(())
    }
    fn allocated(&self) -> usize;
    fn deallocated(&self) -> usize;
    fn available_for_allocation(&self) -> usize {
        MAX_PERMITTED_DATA_INCREASE.saturating_sub(self.allocated())
    }
}

impl Origin for State<'_> {
    fn program_id(&self) -> &Pubkey {
        self.program_id
    }
    fn nonce(&self, address: &H160) -> Result<u64> {
        let info = self.info_addr(address, false)?;
        Info::nonce(self, info)
    }

    fn balance(&self, address: &H160) -> Result<U256> {
        let info = self.info_addr(address, false)?;
        Info::balance(self, info)
    }

    fn code(&self, address: &H160) -> Result<Vec<u8>> {
        let info = self.info_addr(address, false)?;
        Info::code(self, info)
    }
    fn valids(&self, address: &H160) -> Result<Vec<u8>> {
        let info = self.info_addr(address, false)?;
        Info::valids(self, info)
    }

    fn storage(&self, address: &H160, slot: &U256) -> Result<Option<U256>> {
        let (info, sub_index) = self.info_slot(address, slot, false)?;
        msg!("slot storage_key {} {}", slot, info.key);
        Info::storage(self, info, sub_index)
    }

    fn inc_nonce(&self, address: &H160) -> Result<()> {
        let info = self.info_addr(address, true)?;
        Info::inc_nonce(self, info)
    }

    fn add_balance(&self, address: &H160, balance: &U256) -> Result<()> {
        let info = self.info_addr(address, true)?;
        Info::add_balance(self, info, balance)
    }

    fn sub_balance(&self, address: &H160, balance: &U256) -> Result<()> {
        let info = self.info_addr(address, true)?;
        Info::sub_balance(self, info, balance, address)
    }

    fn set_code(&self, address: &H160, code: &[u8], valids: &[u8]) -> Result<()> {
        let info = self.info_addr(address, true)?;
        if AccountState::is_contract(info)? {
            return Err(DeployContractToExistingAccount(*address));
        }

        let len = info.data_len();
        let offset = Code::offset(info);
        let required = offset + code.len() + valids.len();

        msg!("set_code data.len(): {}, required: {}", len, required);
        match len.cmp(&required) {
            Less => {
                self.realloc(info, required)?;
            }
            Greater => {
                // TODO: implement deallocation of the unused contract space
                return Err(Unimplemented(
                    format!(
                        "the contract deployment space must be deallocated according to size of the contract \
                        pubkey: {}, available: {}, required: {}",
                        info.key,
                        len,
                        required
                    ))
                );
            }
            _ => {}
        }

        Info::set_code(self, info, code, valids, address)
    }

    fn set_storage(&self, address: &H160, slot: &U256, value: &U256) -> Result<()> {
        let (info, sub_index) = self.info_slot(address, slot, true)?;

        let allocate = {
            let table = AddressTable::from_account(info)?;
            let index = table.get(sub_index) as usize;
            index == 0
        };

        if allocate {
            self.realloc(info, info.data_len() + size_of::<Storage>())?;
            let len = Storage::from_account(info)?.len();
            let mut table = AddressTable::from_account_mut(info)?;
            table.set(sub_index, len);
        }

        Info::set_storage(self, info, sub_index, value)
    }

    fn block_hash(&self, block: U256, slot: Slot) -> Result<H256> {
        let sysvar = self.info_sys(&recent_blockhashes::ID)?;
        Info::block_hash(self, sysvar, block, slot)
    }

    fn allocated(&self) -> usize {
        *self.allocated.borrow()
    }

    fn deallocated(&self) -> usize {
        *self.deallocated.borrow()
    }
}
