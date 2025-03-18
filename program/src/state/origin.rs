use {
    crate::{
        accounts::{AccountState, Data},
        context::account_lock::AccountLock,
        error::{Result, RomeProgramError::*},
        info::Info,
        state::{base::Base, State},
        Code, Account, EVENT_LOG, pda::Seed,
    },
    evm::{H160, H256, U256},
    solana_program::{
        clock::Slot, instruction::Instruction, log::sol_log_data, program::{
            invoke_signed_unchecked, invoke,
        },
        pubkey::Pubkey, sysvar::recent_blockhashes,
    },
    std::cmp::Ordering::*,
};

pub trait Origin {
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
    fn base(&self) -> &Base;
    fn account(&self, key: &Pubkey) -> Result<Account>;
    fn invoke_signed(&self, ix: &Instruction, seed: Seed) -> Result<()>;
    fn signer(&self) -> Pubkey;
}

impl Origin for State<'_> {
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

    fn base(&self) -> &Base {
        &self.base
    }

    fn account(&self, key: &Pubkey) -> Result<Account> {
        let info = self.all()
            .get(key)
            .ok_or(AccountNotFound(*key))?;

        let acc = if info.executable {
            Account::new_executable()
        } else {
            Account::from_account_info(info)
        };

        Ok(acc)
    }

    fn invoke_signed(&self, ix: &Instruction, seed: Seed) -> Result<()> {
        let f = |key: Pubkey| {
            self
                .all()
                .get(&key)
                .map(|&x| x.clone())
                .ok_or(AccountNotFound(key))
        };

        #[allow(unused_assignments)]
        let mut infos = Vec::with_capacity(ix.accounts.len() + 1);

        infos = ix
            .accounts
            .iter()
            .map(|a| f(a.pubkey))
            .collect::<Result<Vec<_>>>()?;

        infos.push(f(ix.program_id)?);

        if seed.items.is_empty() {
            invoke(ix, infos.as_slice())?;
        } else {
            invoke_signed_unchecked(ix, infos.as_slice(), &[seed.cast().as_slice()])?;
        }

        Ok(())
    }
    fn signer(&self) -> Pubkey {
        *self.signer.key
    }
}
