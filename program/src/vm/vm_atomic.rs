use {
    super::{vm::Vm, Execute},
    crate::{
        context::{account_lock::AccountLock, Context},
        error::Result,
        origin::Origin,
        state::Allocate,
        ExitReason, JournaledState,
    },
    solana_program::msg,
};

pub enum MachineAtomic {
    Lock,
    Init,
    Execute,
    Commit(Vec<u8>, ExitReason),
    Exit,
}

use MachineAtomic::*;

impl<'a, T: Origin + Allocate, L: AccountLock + Context> Vm<'a, T, MachineAtomic, L> {
    #[allow(dead_code)]
    pub fn new_atomic(state: &'a T, context: &'a L) -> Result<Box<Self>> {
        let handler = JournaledState::new(state)?;

        Ok(Box::new(Self {
            snapshot: None,
            handler,
            state_machine: None,
            return_value: None,
            exit_reason: None,
            context,
            steps_executed: 0,
        }))
    }
}

impl<T: Origin + Allocate, L: AccountLock + Context> Execute<MachineAtomic>
    for Vm<'_, T, MachineAtomic, L>
{
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
                msg!("FromTx");
                let snapshot = self.snapshot_from_tx()?;
                self.add_snapshot(snapshot);
                Execute
            }
            Execute => {
                msg!("Execute");
                if let Some((return_value, reason)) = self.execute(u64::MAX)? {
                    self.gas_transfer()?;
                    Commit(return_value, reason)
                } else {
                    Execute
                }
            }
            Commit(return_value, reason) => {
                msg!("Commit");
                self.return_value = Some(return_value);
                self.exit_reason = Some(reason);
                self.handler.alloc_slots_unchecked()?;
                self.handler.commit(self.context)?;
                self.log_gas_transfer();
                self.log_exit_reason()?;
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

    fn consume(&mut self, machine: MachineAtomic) -> Result<()> {
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
