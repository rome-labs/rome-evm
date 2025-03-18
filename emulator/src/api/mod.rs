mod confirm_tx_iterative;
mod create_balance;
mod do_tx;
pub mod do_tx_holder;
pub mod do_tx_holder_iterative;
pub mod do_tx_iterative;
mod eth_call;
mod eth_estimate_gas;
mod eth_get_balance;
mod eth_get_code;
mod eth_get_storage_at;
mod eth_get_tx_count;
mod get_rollups;
mod reg_owner;
mod transmit_tx;

pub use confirm_tx_iterative::confirm_tx_iterative;
pub use create_balance::create_balance;
pub use do_tx::do_tx;
pub use do_tx_holder::do_tx_holder;
pub use do_tx_holder_iterative::do_tx_holder_iterative;
pub use do_tx_iterative::do_tx_iterative;
pub use eth_call::eth_call;
pub use eth_estimate_gas::eth_estimate_gas;
pub use eth_get_balance::eth_get_balance;
pub use eth_get_code::eth_get_code;
pub use eth_get_storage_at::eth_get_storage_at;
pub use eth_get_tx_count::eth_get_tx_count;
pub use get_rollups::get_rollups;
pub use reg_owner::reg_owner;
use solana_program::entrypoint::MAX_PERMITTED_DATA_INCREASE;
pub use transmit_tx::transmit_tx;

use rome_evm::NUMBER_OPCODES_PER_TX;
use {
    crate::state::{Item, Slots, State},
    rome_evm::{
        accounts::{AccountState, AccountType, Data},
        error::{Result, RomeProgramError::*},
        ExitReason, H160, SIG_VERIFY_COST,
    },
    solana_program::{
        account_info::IntoAccountInfo, msg, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
    },
    std::collections::BTreeMap,
};

pub struct Vm {
    pub exit_reason: ExitReason,
    pub return_value: Option<Vec<u8>>,
    pub steps_executed: u64,
    pub iteration_count: u64,
}

pub struct Emulation {
    pub accounts: BTreeMap<Pubkey, Item>,
    pub storage: BTreeMap<H160, Slots>,
    pub vm: Option<Vm>,
    pub alloc: usize,
    pub dealloc: usize,
    pub alloc_payed: usize,
    pub dealloc_payed: usize,
    pub gas: u64,
    pub lock_overrides: Vec<u8>,
    pub syscalls: u64,
    pub is_atomic: bool,
}

impl Emulation {
    #[allow(clippy::too_many_arguments)]
    pub fn with_vm(
        state: &State,
        exit_reason: Option<ExitReason>,
        return_value: Option<Vec<u8>>,
        steps_executed: u64,
        iter_count: u64,
        alloc: usize,
        dealloc: usize,
        alloc_payed: usize,
        dealloc_payed: usize,
        lock_overrides: Vec<Pubkey>,
        syscalls: u64,
    ) -> Result<Self> {
        let is_atomic = steps_executed <= NUMBER_OPCODES_PER_TX
            && alloc <= MAX_PERMITTED_DATA_INCREASE
            && syscalls < 64;

        let gas = Emulation::gas(alloc_payed, dealloc_payed, iter_count, is_atomic)?;

        let lock_overrides = Emulation::cast_overrides(state, lock_overrides)?;

        msg!(">> emulation results:");
        msg!("steps_executed: {}", steps_executed);
        msg!("nubmer of iterations: {}", iter_count);
        msg!("allocated: {}", alloc);
        msg!("deallocated: {}", dealloc);
        msg!("allocated_payed: {}", alloc_payed);
        msg!("deallocated_payed: {}", dealloc_payed);
        msg!("exit_reason: {:?}", exit_reason);
        msg!("gas: {:?}", gas);
        msg!("lock_overrides: {:?}", lock_overrides);
        msg!("syscalls: {}", syscalls);
        msg!("is_atomic: {}", is_atomic);

        Emulation::log_accounts(state)?;

        let vm = Vm {
            exit_reason: exit_reason.ok_or(VmFault("exit_reason expected".to_string()))?,
            return_value,
            steps_executed,
            iteration_count: iter_count,
        };

        Ok(Self {
            accounts: state.accounts.borrow().clone(),
            storage: state.storage.borrow().clone(),
            vm: Some(vm),
            alloc,
            dealloc,
            alloc_payed,
            dealloc_payed,
            gas,
            lock_overrides,
            syscalls,
            is_atomic,
        })
    }

    fn log_accounts(state: &State) -> Result<()> {
        msg!("accounts:");
        msg!("Pubkey | is_writable | is_signer | AccountType | data.len() | {address} ");
        for (key, item) in state.accounts.borrow().iter() {
            let mut bind = (*key, item.account.clone());
            let info = bind.into_account_info();
            let is_pda = AccountType::check_owner(&info, state.program_id).is_ok();

            let type_ = if is_pda {
                let typ = AccountType::from_account(&info)?;
                let mut is_contract = "".to_string();

                if *typ == AccountType::Balance && AccountState::from_account(&info)?.is_contract {
                    is_contract = "(contract)".to_string();
                }
                format!("{:?}{}", typ, is_contract)
            } else {
                "System".to_string()
            };

            if let Some(address) = item.address {
                msg!(
                    "{} {} {} {} {} {}",
                    key,
                    item.account.writeable,
                    item.signer,
                    type_,
                    item.account.data.len(),
                    address,
                )
            } else {
                msg!(
                    "{} {} {} {} {}",
                    key,
                    item.account.writeable,
                    item.signer,
                    type_,
                    item.account.data.len(),
                )
            }
        }

        Ok(())
    }

    pub fn without_vm(state: &State) -> Result<Self> {
        let alloc = state.alloc();
        let dealloc = state.dealloc();
        let alloc_payed = state.alloc_payed();
        let dealloc_payed = state.dealloc_payed();

        let gas = Emulation::gas(alloc_payed, dealloc_payed, 1, true)?;

        msg!(">> emulation results:");
        msg!("allocated: {}", alloc);
        msg!("deallocated: {}", dealloc);
        msg!("allocated_payed: {}", alloc_payed);
        msg!("deallocated_payed: {}", dealloc_payed);
        Emulation::log_accounts(state)?;

        Ok(Self {
            accounts: state.accounts.borrow().clone(),
            storage: state.storage.borrow().clone(),
            vm: None,
            alloc,
            dealloc,
            alloc_payed,
            dealloc_payed,
            gas,
            lock_overrides: vec![],
            syscalls: state.pda.syscall.count(),
            is_atomic: true,
        })
    }

    pub fn gas(
        alloc_payed: usize,
        dealloc_payed: usize,
        iter_count: u64,
        is_atomic: bool,
    ) -> Result<u64> {
        let space_to_pay = alloc_payed.saturating_sub(dealloc_payed);

        let rent = if space_to_pay > 0 {
            Rent::get()?.minimum_balance(space_to_pay)
        } else {
            0
        };

        let sig_veify_cost = if is_atomic {
            SIG_VERIFY_COST
        } else {
            SIG_VERIFY_COST * iter_count
        };

        Ok(21_000.max(rent + sig_veify_cost))
    }

    pub fn cast_overrides(state: &State, overrides: Vec<Pubkey>) -> Result<Vec<u8>> {
        state
            .accounts
            .borrow()
            .iter()
            .enumerate()
            .filter(|(_, (key, _))| overrides.iter().any(|locked| locked == *key))
            .map(|(ix, _)| {
                if ix > u8::MAX as usize {
                    Err(Custom("too  many accounts".to_string()))
                } else {
                    Ok(ix as u8)
                }
            })
            .collect::<Result<Vec<_>>>()
    }
}

pub mod fake {
    solana_program::declare_id!("11111111111111111111111111111112");
}
