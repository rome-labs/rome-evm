use {
    super::{vm::Vm, Execute},
    crate::{
        context::AccountLock,
        error::Result,
        origin::Origin,
        state::Allocate,
        config::SIG_VERIFY_COST,
        H160,
        tx::tx::Tx,
    },
    solana_program::msg,
};

pub enum MachineAt {
    Lock,
    Init,
    Execute,
    Commit,
    GasTransfer,
    Exit,
}

use MachineAt::*;

pub struct VmAt<'a, T: Origin + Allocate, L: AccountLock> {
    pub vm: Vm<'a, T>,
    pub state_machine: Option<MachineAt>,
    tx: Tx,
    fee_addr: Option<H160>,
    context: &'a L,
}

impl<'a, T: Origin + Allocate, L: AccountLock> VmAt<'a, T, L> {
    pub fn new(state: &'a T, rlp: &'a[u8], fee_addr: Option<H160>, context: &'a L) -> Result<Box<Self>> {
        let atomic = Self {
            vm: Vm::new(state)?,
            state_machine: None,
            tx: Tx::from_instruction(rlp)?,
            fee_addr,
            context,
        };

        Ok(Box::new(atomic))
    }
}

impl<T: Origin + Allocate, L: AccountLock> Execute<MachineAt> for VmAt<'_, T, L> {
    fn advance(&mut self) -> Result<()> {
        let state_machine = self
            .state_machine
            .take()
            .unwrap_or_else(|| panic!("vm state machine fault"));

        let state_machine = match state_machine {
            Lock => {
                msg!("Lock");
                self.context.lock()?;
                Init
            }
            Init => {
                msg!("Init");
                if let Some((value, reason)) = self.vm.init(&mut self.tx, true, self.fee_addr)? {
                    self.vm.set_exit_reason(reason, value);
                    if reason.is_succeed() {
                        Commit
                    } else {
                        Exit
                    }
                } else {
                    Execute
                }
            }
            Execute => {
                msg!("Execute");
                if let Some((return_value, reason)) = self.vm.execute(u64::MAX) {
                    self.vm.set_exit_reason(reason, return_value);
                    if reason.is_succeed() {
                        Commit 
                    } else {
                        Exit
                    }
                } else {
                    Execute
                }
            }
            Commit=> {
                msg!("Commit");
                self.vm.handler.alloc_slots_unchecked()?;
                self.vm.handler.commit(self.context)?;
                self.vm.handler.revert_all();
                self.vm.log_exit_reason()?;
                GasTransfer
            }
            GasTransfer => {
                // TODO: create test for gas payment in case of Revert
                self.vm.handler.state.base().add_fee(SIG_VERIFY_COST)?;
                let (fee, refund) = self.vm.handler.state.base().get_fees();

                // fee_recipient account will be created at the operator's expense.
                // TODO: remove this len from alloc_payed, remove this cost from lamports_fee 
                self.vm.gas_transfer(fee, refund)?;
                self.vm.handler.commit(self.context)?;
                Exit
            }
            Exit => {
                msg!("Exit");
                Exit
            }
        };
        self.state_machine = Some(state_machine);
        Ok(())
    }

    fn consume(&mut self, machine: MachineAt) -> Result<()> {
        self.state_machine = Some(machine);
        loop {
            self.advance()?;

            if let Some(Exit) = self.state_machine.as_ref() {
                break;
            }
        }

        Ok(())
    }
}
