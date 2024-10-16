use {
    super::{vm::Vm, Execute},
    crate::{
        accounts::Iterations,
        config::NUMBER_OPCODES_PER_TX,
        context::{account_lock::AccountLock, Context},
        error::{Result, RomeProgramError::*},
        origin::Origin,
        state::Allocate,
        JournaledState,
    },
    solana_program::msg,
};

pub enum MachineIterative {
    FromStateHolder,
    Lock,
    Init,
    Execute,
    IntoTrap,
    GasTransfer,
    Serialize(Box<Self>),
    AllocateHolder,
    Allocate,
    Unlock,
    NextIteration(Box<Self>),
    Error,
    Commit,
    Exit,
}

use MachineIterative::*;

impl From<Iterations> for MachineIterative {
    fn from(iter: Iterations) -> Self {
        match iter {
            Iterations::Lock => Lock,
            Iterations::Start => Init,
            Iterations::Execute => Execute,
            Iterations::AllocateHolder => AllocateHolder,
            Iterations::Allocate => Allocate,
            Iterations::Commit => Commit,
            Iterations::Unlock => Unlock,
            Iterations::Error => Error,
        }
    }
}
impl From<&MachineIterative> for Iterations {
    fn from(machine: &MachineIterative) -> Self {
        match machine {
            Lock => Iterations::Lock,
            Init => Iterations::Start,
            Execute => Iterations::Execute,
            AllocateHolder => Iterations::AllocateHolder,
            Allocate => Iterations::Allocate,
            Commit => Iterations::Commit,
            Unlock => Iterations::Unlock,
            Error => Iterations::Error,
            _ => panic!("VmFault: MachineIterativeative to Iterations cast error"),
        }
    }
}

impl<T: Origin + Allocate, L: AccountLock + Context> Execute<MachineIterative>
    for Vm<'_, T, MachineIterative, L>
{
    fn advance(&mut self) -> Result<()> {
        let state_machine = self
            .state_machine
            .take()
            .unwrap_or_else(|| panic!("vm state machine fault"));

        let state_machine = match state_machine {
            FromStateHolder => {
                msg!("FromStateHolder");
                // found allocations (it can be holder account) in current iteration
                if self.handler.state.allocated() > 0 {
                    msg!("allocated: {}", self.handler.state.allocated());
                    NextIteration(Box::new(Lock))
                } else {
                    // state_holder stores hash of the current tx
                    if self.context.is_tx_binded_to_holder()? {
                        let iteration = self.context.restore_iteration()?;
                        iteration.into()
                    } else {
                        //start execution from beginnig
                        msg!("tx in not binded");
                        Lock
                    }
                }
            }
            Lock => {
                msg!("Lock");
                self.context.lock()?;
                // save tx_hash to state_holder
                self.context.bind_tx_to_holder()?;
                Init
            }
            Init => {
                msg!("Init");
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else {
                    let snapshot = self.snapshot_from_tx()?;
                    self.add_snapshot(snapshot);
                    Serialize(Box::new(Execute))
                }
            }
            Serialize(to) => {
                msg!("Serialize");
                match self.context.serialize_vm(self) {
                    Err(IoError(io)) => {
                        // not enough space
                        match io.kind() {
                            // holder.data is invalid, it cannot be used. The state is lost.
                            // We need to start from the beginning
                            std::io::ErrorKind::WriteZero => AllocateHolder,
                            _ => return Err(IoError(io)),
                        }
                    }
                    Err(e) => return Err(e),
                    Ok(()) => NextIteration(to),
                }
            }
            AllocateHolder => {
                msg!("AllocateHolder");
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else {
                    self.context.allocate_holder()?;
                    NextIteration(Box::new(Init))
                }
            }
            Execute => {
                msg!("Execute");
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else {
                    self.context.deserialize_vm(self)?;
                    IntoTrap
                }
            }
            IntoTrap => {
                msg!("IntoTrap");
                let steps_left = NUMBER_OPCODES_PER_TX.saturating_sub(self.steps_executed);

                // TODO:  go to error if reason is not success
                if let Some((return_value, reason)) = self.execute(steps_left)? {
                    self.return_value = Some(return_value);
                    self.exit_reason = Some(reason);
                    Serialize(Box::new(Allocate))
                } else if NUMBER_OPCODES_PER_TX.saturating_sub(self.steps_executed) > 0 {
                    IntoTrap
                } else {
                    Serialize(Box::new(Execute))
                }
            }
            Allocate => {
                msg!("Allocate");
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else {
                    self.context.deserialize_vm(self)?;
                    let is_allocated = self.handler.allocate()?;
                    self.context.lock_new_one()?; // lock new allocated accounts

                    if is_allocated {
                        GasTransfer
                    } else {
                        Exit
                    }
                }
            }
            GasTransfer => {
                msg!("GasTransfer");
                self.gas_transfer()?;
                Serialize(Box::new(Commit))
            }
            Commit => {
                msg!("Commit");
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else {
                    self.context.deserialize_vm(self)?;
                    self.handler.commit()?;
                    self.log_gas_transfer();
                    self.log_exit_reason()?;
                    NextIteration(Box::new(Unlock))
                }
            }
            Unlock => {
                msg!("Unlock");
                self.context.unlock()?;
                // todo: deallocate holders?
                NextIteration(Box::new(Error))
            }
            Error => {
                msg!("UnnecessaryIteration: {}", self.context.tx_hash());
                return Err(UnnecessaryIteration(self.context.tx_hash()));
            }
            NextIteration(to) => {
                self.context.save_iteration((&*to).into())?;
                Exit
            }
            Exit => unreachable!(),
        };
        self.state_machine = Some(state_machine);
        Ok(())
    }

    fn consume(&mut self, machine: MachineIterative) -> Result<()> {
        self.state_machine = Some(machine);

        loop {
            self.advance()?;
            if let Some(Exit) = self.state_machine.as_ref() {
                msg!("Exit");
                break;
            }
        }

        Ok(())
    }
}

impl<'a, T: Origin + Allocate, L: AccountLock + Context> Vm<'a, T, MachineIterative, L> {
    #[allow(dead_code)]
    pub fn new_iterative(state: &'a T, context: &'a L) -> Result<Box<Self>> {
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
