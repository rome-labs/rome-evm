use {
    super::{
        vm::Vm,
        Execute,
        MachineEthCall::{self, *},
    },
    crate::{
        context::{account_lock::AccountLock, Context},
        error::Result,
        origin::Origin,
        state::Allocate,
        JournaledState,
    },
    solana_program::msg,
};

// use MachineEthCall::*;

impl<'a, T: Origin + Allocate, L: AccountLock + Context> Vm<'a, T, MachineEthCall, L> {
    #[allow(dead_code)]
    pub fn new_eth_call(state: &'a T, context: &'a L) -> Result<Box<Self>> {
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

impl<T: Origin + Allocate, L: AccountLock + Context> Execute<MachineEthCall>
    for Vm<'_, T, MachineEthCall, L>
{
    fn advance(&mut self) -> Result<()> {
        let state_machine = self
            .state_machine
            .take()
            .unwrap_or_else(|| panic!("vm state machine fault"));

        let state_machine = match state_machine {
            Init => {
                msg!("FromTx");
                let snapshot = self.snapshot_from_tx()?;
                self.add_snapshot(snapshot);
                Execute
            }
            Execute => {
                msg!("Execute");
                if let Some((return_value, reason)) = self.execute(u64::MAX)? {
                    self.return_value = Some(return_value);
                    self.exit_reason = Some(reason);
                    self.log_exit_reason()?;
                    Exit
                } else {
                    Execute
                }
            }
            Exit => {
                msg!("Exit");
                Exit
            }
        };
        self.state_machine = Some(state_machine);
        Ok(())
    }

    fn consume(&mut self, machine: MachineEthCall) -> Result<()> {
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
