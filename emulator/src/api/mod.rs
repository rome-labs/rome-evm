mod confirm_tx_iterative;
mod deposit;
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
pub use deposit::deposit;
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

use {
    crate::{
        state::{Item, Slots, State}, context::ContextIt,
    },
    rome_evm::{
        accounts::{AccountState, AccountType, Data},
        error::{Result, RomeProgramError::*},
        ExitReason, H160, SIG_VERIFY_COST, NUMBER_OPCODES_PER_TX, StateHolder,
    },
    solana_program::{
        account_info::IntoAccountInfo, msg, pubkey::Pubkey,
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
        lamports_fee: u64,
        lamports_refund: u64,
        is_gas_estimate: bool,
        context: Option<&ContextIt>
    ) -> Result<Self> {
        let is_atomic = steps_executed <= NUMBER_OPCODES_PER_TX
            && alloc <= MAX_PERMITTED_DATA_INCREASE
            && syscalls < 64;

        let gas = Emulation::gas(
            state,
            is_atomic,
            is_gas_estimate,
            lamports_fee,
            lamports_refund,
            context,
        )?;

        let lock_overrides = Emulation::cast_overrides(state, lock_overrides)?;

        msg!(">> emulation results:");
        msg!("steps_executed: {}", steps_executed);
        msg!("number of iterations: {}", iter_count);
        msg!("allocated: {}", alloc);
        msg!("deallocated: {}", dealloc);
        msg!("allocated_payed: {}", alloc_payed);
        msg!("deallocated_payed: {}", dealloc_payed);
        msg!("exit_reason: {:?}", exit_reason);
        msg!("lock_overrides: {:?}", lock_overrides);
        msg!("syscalls: {}", syscalls);
        msg!("lamports_fee: {}", lamports_fee);
        msg!("lamports_refund: {}", lamports_refund);
        msg!("gas: {:?}", gas);
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
                    item.account.writable,
                    item.signer,
                    type_,
                    item.account.data.len(),
                    address,
                )
            } else {
                msg!(
                    "{} {} {} {} {}",
                    key,
                    item.account.writable,
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
            gas: 0,
            lock_overrides: vec![],
            syscalls: state.pda.syscall.count(),
            is_atomic: true,
        })
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

    pub fn gas(state: &State, is_atomic: bool, is_gas_estimate: bool, fee: u64, refund: u64, context: Option<&ContextIt>) -> Result<u64> {

        let actual_fee = if is_gas_estimate {
            let context = context.unwrap();

            if is_atomic {
                let mut bind = state.info_state_holder(context.holder, false)?;
                let info = bind.into_account_info();

                let iter_cnt = StateHolder::from_account(&info)?.iter_cnt;
                let extra_fee = (iter_cnt - 1) * SIG_VERIFY_COST;
                fee.checked_sub(extra_fee).ok_or(CalculationUnderflow)?
            } else {
                fee.checked_add(SIG_VERIFY_COST * 10).ok_or(CalculationOverflow)?
            }
        } else {
            fee
        };

        let gas = actual_fee.saturating_sub(refund);

        Ok(21_000.max(gas))
    }
}

pub mod fake {
    solana_program::declare_id!("11111111111111111111111111111112");
}
