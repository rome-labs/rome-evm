mod create_balance;
mod do_tx;
pub mod do_tx_holder;
pub mod do_tx_holder_iterative;
pub mod do_tx_iterative;
mod eth_call;
mod eth_get_balance;
mod eth_get_code;
mod eth_get_tx_count;
mod reg_signer;
mod transmit_tx;

pub use create_balance::create_balance;
pub use do_tx::do_tx;
pub use do_tx_holder::do_tx_holder;
pub use do_tx_holder_iterative::do_tx_holder_iterative;
pub use do_tx_iterative::do_tx_iterative;
pub use eth_call::eth_call;
pub use eth_get_balance::eth_get_balance;
pub use eth_get_code::eth_get_code;
pub use eth_get_tx_count::eth_get_tx_count;
pub use reg_signer::reg_signer;
pub use transmit_tx::transmit_tx;

use {
    crate::state::{Item, State},
    rome_evm::{
        accounts::{AccountState, AccountType, Data},
        error::{Result, RomeProgramError::*},
        state::origin::Origin,
        ExitReason, SIG_VERIFY_COST,
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
    pub vm: Option<Vm>,
    pub allocated: usize,
    pub deallocated: usize,
    pub allocated_state: usize,
    pub deallocated_state: usize,
    pub gas: u64,
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
        alloc_state: usize,
        dealloc_state: usize,
    ) -> Result<Self> {
        msg!(">> emulation results:");
        msg!("steps_executed: {}", steps_executed);
        msg!("nubmer of iterations: {}", iter_count);
        msg!("allocated: {}", alloc);
        msg!("deallocated: {}", dealloc);
        msg!("allocated_state: {}", alloc_state);
        msg!("deallocated_state: {}", dealloc_state);
        msg!("exit_reason: {:?}", exit_reason);
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
                    item.writable,
                    item.signer,
                    type_,
                    item.account.data.len(),
                    address,
                )
            } else {
                msg!(
                    "{} {} {} {} {}",
                    key,
                    item.writable,
                    item.signer,
                    type_,
                    item.account.data.len(),
                )
            }
        }

        let vm = Vm {
            exit_reason: exit_reason.ok_or(VmFault("exit_reason expected".to_string()))?,
            return_value,
            steps_executed,
            iteration_count: iter_count,
        };

        let gas = Emulation::gas(alloc_state, dealloc_state, iter_count)?;

        Ok(Self {
            accounts: state.accounts.borrow().clone(),
            vm: Some(vm),
            allocated: alloc,
            deallocated: dealloc,
            allocated_state: alloc_state,
            deallocated_state: dealloc_state,
            gas,
        })
    }

    pub fn without_vm(state: &State) -> Result<Self> {
        let alloc_state = *state.alloc_state.borrow();
        let dealloc_state = *state.dealloc_state.borrow();
        let gas = Emulation::gas(alloc_state, dealloc_state, 1)?;

        Ok(Self {
            accounts: state.accounts.borrow().clone(),
            vm: None,
            allocated: state.allocated(),
            deallocated: state.deallocated(),
            allocated_state: alloc_state,
            deallocated_state: dealloc_state,
            gas,
        })
    }

    pub fn gas(alloc_state: usize, dealloc_state: usize, iter_count: u64) -> Result<u64> {
        let space_to_pay = alloc_state.saturating_sub(dealloc_state);
        let rent = Rent::get()?.minimum_balance(space_to_pay);

        Ok(rent + SIG_VERIFY_COST * iter_count)
    }
}
