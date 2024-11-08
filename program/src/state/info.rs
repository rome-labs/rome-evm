use {
    crate::{
        accounts::Data,
        context::account_lock::AccountLock,
        error::{Result, RomeProgramError::*},
        state::State,
        AccountState, Code, Storage, Valids,
    },
    evm::{H160, H256, U256},
    solana_program::{account_info::AccountInfo, clock::Slot},
    std::convert::TryInto,
};

pub trait Info {
    fn nonce<'a>(&self, info: &'a AccountInfo<'a>) -> Result<u64> {
        AccountState::from_account(info).map(|a| a.nonce)
    }
    fn balance<'a>(&self, info: &'a AccountInfo<'a>) -> Result<U256> {
        AccountState::from_account(info).map(|a| a.balance)
    }
    fn code<'a>(&self, info: &'a AccountInfo<'a>) -> Result<Vec<u8>> {
        let state = AccountState::from_account(info)?;
        if state.is_contract {
            Code::from_account(info).map(|a| a.to_vec())
        } else {
            Ok(vec![])
        }
    }
    fn valids<'a>(&self, info: &'a AccountInfo<'a>) -> Result<Vec<u8>> {
        let state = AccountState::from_account(info)?;
        if state.is_contract {
            Valids::from_account(info).map(|a| a.to_vec())
        } else {
            Ok(vec![])
        }
    }
    fn storage<'a>(&self, info: &'a AccountInfo<'a>, sub_ix: u8) -> Result<Option<U256>> {
        Storage::get(info, sub_ix)
    }
    fn inc_nonce<'a, L: AccountLock>(&self, info: &'a AccountInfo<'a>, context: &L) -> Result<()> {
        context.check_writable(info)?;
        let mut state = AccountState::from_account_mut(info)?;
        state.nonce = state.nonce.checked_add(1).ok_or(CalculationOverflow)?;
        Ok(())
    }
    fn add_balance<'a, L: AccountLock>(
        &self,
        info: &'a AccountInfo<'a>,
        balance: &U256,
        context: &L,
    ) -> Result<()> {
        context.check_writable(info)?;
        let mut state = AccountState::from_account_mut(info)?;
        state.balance = state
            .balance
            .checked_add(*balance)
            .ok_or(CalculationOverflow)?;
        Ok(())
    }
    fn sub_balance<'a, L: AccountLock>(
        &self,
        info: &'a AccountInfo<'a>,
        balance: &U256,
        address: &H160,
        context: &L,
    ) -> Result<()> {
        context.check_writable(info)?;
        let mut state = AccountState::from_account_mut(info)?;
        state.balance = state
            .balance
            .checked_sub(*balance)
            .ok_or(InsufficientFunds(*address, *balance))?;
        Ok(())
    }

    fn set_code<'a, L: AccountLock>(
        &self,
        info: &'a AccountInfo<'a>,
        code: &[u8],
        valids: &[u8],
        address: &H160,
        context: &L,
    ) -> Result<()> {
        context.check_writable(info)?;

        assert_eq!(valids.len(), evm::Valids::size_needed(code.len()));

        AccountState::check_no_contract(info, address)?;
        {
            let mut code_mut = Code::from_account_mut(info)?;
            assert!(code_mut.len() == code.len());
            code_mut.copy_from_slice(code);
        }
        {
            let mut valids_mut = Valids::from_account_mut(info)?;
            assert!(valids_mut.len() == valids.len());
            valids_mut.copy_from_slice(valids);
        }
        {
            let mut state = AccountState::from_account_mut(info)?;
            state.is_contract = true;
        }
        Ok(())
    }
    fn set_storage<'a, L: AccountLock>(
        &self,
        info: &'a AccountInfo<'a>,
        sub_ix: u8,
        value: &U256,
        context: &L,
    ) -> Result<()> {
        context.check_writable(info)?;
        Storage::set(info, value, sub_ix)
    }

    fn block_hash<'a>(&self, sysvar: &'a AccountInfo<'a>, block: U256, slot: Slot) -> Result<H256> {
        if block >= slot.into() {
            return Ok(H256::default());
        }
        let offset: usize = (8 + (slot - 1 - block.as_u64()) * 40).try_into().unwrap();
        let data = sysvar.try_borrow_data()?;
        if offset + 32 > data.len() {
            return Ok(H256::default());
        }
        Ok(H256::from_slice(&data[offset..][..32]))
    }
}

impl Info for State<'_> {}
