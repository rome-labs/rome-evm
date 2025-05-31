use {
    rome_evm::{
        vm::{Vm, Execute}, error::Result, origin::Origin, state::Allocate,
        tx::{
            tx::Tx, legacy::Legacy,
        },
    },
    solana_program::msg,
};

pub enum MachineEthCall {
    Init,
    Execute,
    Exit,
}

use MachineEthCall::*;

pub struct VmCall<'a, T: Origin + Allocate> {
    pub vm: Vm<'a, T>,
    state_machine: Option<MachineEthCall>,
    legacy: Option<Legacy>,
}

impl<'a, T: Origin + Allocate> VmCall<'a, T> {
    #[allow(dead_code)]
    pub fn new(state: &'a T, legacy: Legacy, ) -> Result<Box<Self>> {
        let atomic = Self {
            vm: Vm::new(state)?,
            state_machine: None,
            legacy: Some(legacy),
        };

        Ok(Box::new(atomic))
    }
}

impl<T: Origin + Allocate> Execute<MachineEthCall> for VmCall<'_, T> {
    fn advance(&mut self) -> Result<()> {
        let state_machine = self
            .state_machine
            .take()
            .unwrap_or_else(|| panic!("vm state machine fault"));

        let state_machine = match state_machine {
            Init => {
                msg!("Init");
                let mut tx = Tx::from_legacy(self.legacy.take().unwrap());
                if let Some((value, reason)) = self.vm.init(&mut tx, false, None)? {
                    self.vm.set_exit_reason(reason, value);
                    Exit
                } else {
                    Execute
                }
            }
            Execute => {
                msg!("Execute");
                if let Some((return_value, reason)) = self.vm.execute(u64::MAX) {
                    self.vm.set_exit_reason(reason, return_value);
                    Exit
                } else {
                    Execute
                }
            }
            Exit => {
                msg!("Exit");
                self.vm.log_exit_reason()?;
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
