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
    AllocateHolder(Box<Self>),
    Allocate,
    MergeSlots,
    AllocateStorage,
    Unlock,
    NextIteration(Box<Self>),
    Unnecessary,
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
            Iterations::Allocate => Allocate,
            Iterations::MergeSlots => MergeSlots,
            Iterations::AllocateStorage => AllocateStorage,
            Iterations::Commit => Commit,
            Iterations::Unlock => Unlock,
            Iterations::Unnecessary => Unnecessary,
        }
    }
}
impl From<&MachineIterative> for Iterations {
    fn from(machine: &MachineIterative) -> Self {
        match machine {
            Lock => Iterations::Lock,
            Init => Iterations::Start,
            Execute => Iterations::Execute,
            Allocate => Iterations::Allocate,
            MergeSlots => Iterations::MergeSlots,
            AllocateStorage => Iterations::AllocateStorage,
            Commit => Iterations::Commit,
            Unlock => Iterations::Unlock,
            Unnecessary => Iterations::Unnecessary,
            _ => panic!("VmFault: MachineIterativeative to Iterations cast error"),
        }
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

                if self.context.state_holder_len()? == 0 {
                    AllocateHolder(Box::new(Lock))
                } else {
                    // state_holder stores tx_hash and session_id
                    if self.context.exists_session()? {
                        let iteration = self.context.restore_iteration()?;
                        iteration.into()
                    } else {
                        //start execution from the very beginning
                        msg!("session not found");
                        Lock
                    }
                }
            }
            Lock => {
                msg!("Lock");
                self.context.lock()?;
                // save tx_hash to state_holder
                self.context.new_session()?;
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
                match self.context.serialize(self) {
                    Err(IoError(io)) => {
                        // not enough space
                        match io.kind() {
                            // holder.data is invalid, it cannot be used. The state is lost.
                            // We need to start from the beginning
                            std::io::ErrorKind::WriteZero => AllocateHolder(Box::new(Init)),
                            _ => return Err(IoError(io)),
                        }
                    }
                    Err(e) => return Err(e),
                    Ok(()) => NextIteration(to),
                }
            }
            AllocateHolder(to) => {
                msg!("AllocateHolder");
                self.context.allocate_holder()?;
                NextIteration(to)
            }
            Execute => {
                msg!("Execute");
                self.context.deserialize(self)?;
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else {
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
                self.context.deserialize(self)?;
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else if self.handler.allocate(self.context)? {
                    if self.handler.journal.found_storage() {
                        Serialize(Box::new(MergeSlots))
                    } else {
                        // skip merge slots, allocate slots
                        GasTransfer
                    }
                } else {
                    Serialize(Box::new(Allocate))
                }
            }
            MergeSlots => {
                msg!("MergeSlots");
                self.context.deserialize(self)?;
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else {
                    self.handler.merge_slots()?;
                    Serialize(Box::new(AllocateStorage))
                }
            }
            AllocateStorage => {
                msg!("AllocateStorage");
                self.context.deserialize(self)?;
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else if self.handler.alloc_slots(self.context)? {
                    GasTransfer
                } else {
                    Serialize(Box::new(AllocateStorage))
                }
            }
            GasTransfer => {
                msg!("GasTransfer");
                self.gas_transfer()?;
                Serialize(Box::new(Commit))
            }
            Commit => {
                msg!("Commit");
                self.context.deserialize(self)?;
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else {
                    self.handler.commit(self.context)?;
                    self.log_gas_transfer();
                    self.log_exit_reason()?;
                    NextIteration(Box::new(Unlock))
                }
            }
            Unlock => {
                msg!("Unlock");
                self.context.unlock()?;
                // todo: deallocate holders?
                NextIteration(Box::new(Unnecessary))
            }
            Unnecessary => {
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
                break;
            }
        }

        Ok(())
    }
}
