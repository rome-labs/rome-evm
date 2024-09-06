use {
    crate::{
        accounts::Data,
        error::{Result, RomeProgramError::*},
        state::State,
        AccountState, AddressTable, Code, Storage, Valids,
    },
    evm::{H160, H256, U256},
    solana_program::{account_info::AccountInfo, clock::Slot, msg},
    std::{cell::RefMut, convert::TryInto},
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
    fn storage<'a>(&self, info: &'a AccountInfo<'a>, sub_index: usize) -> Result<Option<U256>> {
        let table = AddressTable::from_account(info)?;
        let index = table.get(sub_index) as usize;

        if index == 0 {
            Ok(None)
        } else {
            let values = Storage::from_account(info)?;
            assert!(index - 1 < values.len());
            let value = &values[index - 1];
            Ok(Some(U256::from_big_endian(&value.0[..])))
        }
    }
    fn inc_nonce<'a>(&self, info: &'a AccountInfo<'a>) -> Result<()> {
        let mut state = AccountState::from_account_mut(info)?;
        state.nonce = state.nonce.checked_add(1).ok_or(CalculationOverflow)?;
        Ok(())
    }
    fn add_balance<'a>(&self, info: &'a AccountInfo<'a>, balance: &U256) -> Result<()> {
        let mut state = AccountState::from_account_mut(info)?;
        state.balance = state
            .balance
            .checked_add(*balance)
            .ok_or(CalculationOverflow)?;
        Ok(())
    }
    fn sub_balance<'a>(
        &self,
        info: &'a AccountInfo<'a>,
        balance: &U256,
        address: &H160,
    ) -> Result<()> {
        let mut state = AccountState::from_account_mut(info)?;
        state.balance = state
            .balance
            .checked_sub(*balance)
            .ok_or(InsufficientFunds(*address, *balance))?;
        Ok(())
    }

    fn set_code<'a>(
        &self,
        info: &'a AccountInfo<'a>,
        code: &[u8],
        valids: &[u8],
        address: &H160,
    ) -> Result<()> {
        msg!("set_code");
        msg!("contract key {}", info.key);
        assert_eq!(valids.len(), evm::Valids::size_needed(code.len()));

        if AccountState::is_contract(info)? {
            return Err(DeployContractToExistingAccount(*address));
        }
        {
            let mut code_mut = Code::from_account_mut(info)?;
            msg!(
                "code_mut.len(), code.len()  {} {}",
                code_mut.len(),
                code.len()
            );
            assert!(code_mut.len() == code.len());
            code_mut.copy_from_slice(code);
        }
        {
            msg!("set_valids");
            let mut valids_mut = Valids::from_account_mut(info)?;
            msg!(
                "valids.len(), valids.len(): {} {}",
                valids_mut.len(),
                valids.len()
            );
            assert!(valids_mut.len() == valids.len());
            valids_mut.copy_from_slice(valids);
        }
        {
            let mut state = AccountState::from_account_mut(info)?;
            state.is_contract = true;
        }
        Ok(())
    }
    fn set_storage<'a>(
        &self,
        info: &'a AccountInfo<'a>,
        sub_index: usize,
        value: &U256,
    ) -> Result<()> {
        let index = {
            let table = AddressTable::from_account(info)?;
            table.get(sub_index) as usize
        };
        assert!(index > 0);
        let len = Storage::from_account(info)?.len();
        assert!(index - 1 < len);
        let storage = Storage::from_account_mut(info)?;
        let mut location = RefMut::map(storage, |a| &mut a[index - 1]);
        value.to_big_endian(&mut location.0[..]);

        Ok(())
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
