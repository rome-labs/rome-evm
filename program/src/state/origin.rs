use {
    crate::{
        accounts::{AccountState, Data},
        context::account_lock::AccountLock,
        error::{Result, RomeProgramError::*},
        info::Info,
        pda::{Pda, Seed},
        state::State,
        Code, EVENT_LOG,
    },
    evm::{H160, H256, U256},
    solana_program::{
        clock::Slot, entrypoint::MAX_PERMITTED_DATA_INCREASE, log::sol_log_data, pubkey::Pubkey,
        sysvar::recent_blockhashes,
    },
    std::cmp::Ordering::*,
};

pub trait Origin {
    fn program_id(&self) -> &Pubkey;
    fn chain_id(&self) -> u64;
    fn nonce(&self, address: &H160) -> Result<u64>;
    fn balance(&self, address: &H160) -> Result<U256>;
    fn code(&self, address: &H160) -> Result<Vec<u8>>;
    fn valids(&self, address: &H160) -> Result<Vec<u8>>;
    fn storage(&self, address: &H160, slot: &U256) -> Result<Option<U256>>;

    fn inc_nonce<L: AccountLock>(&self, address: &H160, context: &L) -> Result<()>;
    fn add_balance<L: AccountLock>(
        &self,
        address: &H160,
        balance: &U256,
        context: &L,
    ) -> Result<()>;
    fn sub_balance<L: AccountLock>(
        &self,
        address: &H160,
        balance: &U256,
        context: &L,
    ) -> Result<()>;
    fn set_code<L: AccountLock>(
        &self,
        address: &H160,
        code: &[u8],
        valids: &[u8],
        context: &L,
    ) -> Result<()>;
    fn set_storage<L: AccountLock>(
        &self,
        address: &H160,
        slot: &U256,
        value: &U256,
        context: &L,
    ) -> Result<()>;
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
    fn alloc_limit(&self) -> usize {
        MAX_PERMITTED_DATA_INCREASE.saturating_sub(self.allocated())
    }
    fn serialize_pda(&self, into: &mut &mut [u8]) -> Result<()>;
    fn deserialize_pda(&self, from: &mut &[u8]) -> Result<()>;
    fn slot_to_key(&self, address: &H160, slot: &U256) -> (Pubkey, Seed, u8);
    fn syscalls(&self) -> u64;
}

impl Origin for State<'_> {
    fn program_id(&self) -> &Pubkey {
        self.program_id
    }
    fn chain_id(&self) -> u64 {
        self.chain
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
        let (info, sub_ix) = self.info_slot(address, slot, false)?;
        Info::storage(self, info, sub_ix)
    }

    fn inc_nonce<L: AccountLock>(&self, address: &H160, context: &L) -> Result<()> {
        let info = self.info_addr(address, true)?;
        Info::inc_nonce(self, info, context)
    }

    fn add_balance<L: AccountLock>(
        &self,
        address: &H160,
        balance: &U256,
        context: &L,
    ) -> Result<()> {
        let info = self.info_addr(address, true)?;
        Info::add_balance(self, info, balance, context)
    }

    fn sub_balance<L: AccountLock>(
        &self,
        address: &H160,
        balance: &U256,
        context: &L,
    ) -> Result<()> {
        let info = self.info_addr(address, true)?;
        Info::sub_balance(self, info, balance, address, context)
    }

    fn set_code<L: AccountLock>(
        &self,
        address: &H160,
        code: &[u8],
        valids: &[u8],
        context: &L,
    ) -> Result<()> {
        let info = self.info_addr(address, true)?;
        AccountState::check_no_contract(info, address)?;

        let len = info.data_len();
        let offset = Code::offset(info);
        let required = offset + code.len() + valids.len();

        // TODO move allocation to trait for use in the emulator
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

        Info::set_code(self, info, code, valids, address, context)
    }

    fn set_storage<L: AccountLock>(
        &self,
        address: &H160,
        slot: &U256,
        value: &U256,
        context: &L,
    ) -> Result<()> {
        let (info, sub_ix) = self.info_slot(address, slot, true)?;
        Info::set_storage(self, info, sub_ix, value, context)
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

    fn serialize_pda(&self, into: &mut &mut [u8]) -> Result<()> {
        self.pda.serialize(into)
    }

    fn deserialize_pda(&self, from: &mut &[u8]) -> Result<()> {
        self.pda.deserialize(from)
    }

    fn slot_to_key(&self, address: &H160, slot: &U256) -> (Pubkey, Seed, u8) {
        let (index_be, sub_ix) = Pda::storage_index(slot);
        let (base, _) = self.pda.balance_key(address);
        let (key, seed) = self.pda.storage_key(&base, index_be);

        (key, seed, sub_ix)
    }
    fn syscalls(&self) -> u64 {
        self.syscall.count()
    }
}
