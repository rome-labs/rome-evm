use {
    super::{vm::Vm, Execute},
    crate::{
        accounts::Iterations,
        config::{NUMBER_OPCODES_PER_TX, SIG_VERIFY_COST, HASH},
        context::{AccountLock, Context},
        error::{Result, RomeProgramError::*},
        origin::Origin,
        state::Allocate,
    },
    evm::{Handler, U256,},
    solana_program::{msg,  log::sol_log_data}
};

pub enum MachineIt {
    FromStateHolder,
    Lock,
    Init,
    InitLocked,
    Execute,
    IntoTrap,
    Serialize(Box<Self>),
    AllocateHolder(Box<Self>),
    Allocate,
    MergeSlots,
    AllocateStorage,
    Unlock,
    UnlockFailedTx,
    NextIteration(Box<Self>),
    NextIterationUnchecked(Box<Self>),
    Completed,
    Failed,
    Commit,
    Exit,
}

use MachineIt::*;

impl From<Iterations> for MachineIt {
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
            Iterations::UnlockFailedTx => UnlockFailedTx,
            Iterations::Completed => Completed,
            Iterations::Failed => Failed,
        }
    }
}
impl From<&MachineIt> for Iterations {
    fn from(machine: &MachineIt) -> Self {
        match machine {
            Lock => Iterations::Lock,
            Init => Iterations::Start,
            Execute => Iterations::Execute,
            Allocate => Iterations::Allocate,
            MergeSlots => Iterations::MergeSlots,
            AllocateStorage => Iterations::AllocateStorage,
            Commit => Iterations::Commit,
            Unlock => Iterations::Unlock,
            UnlockFailedTx => Iterations::UnlockFailedTx,
            Completed => Iterations::Completed,
            Failed => Iterations::Failed,
            _ => panic!("VmFault: MachineIterativeative to Iterations cast error"),
        }
    }
}

pub struct VmIt<'a, T: Origin + Allocate, L: AccountLock + Context> {
    pub vm: Vm<'a, T>,
    pub state_machine: Option<MachineIt>,
    pub context: &'a L,
}

impl<'a, T: Origin + Allocate, L: AccountLock + Context> VmIt<'a, T, L> {
    pub fn new(state: &'a T, context: &'a L) -> Result<Box<Self>> {
        let vm_it = Self {
            vm: Vm::new(state)?,
            state_machine: None,
            context,
        };

        Ok(Box::new(vm_it))
    }

    pub fn verify_balance_and_gas(&self) -> Result<()> {
        if self.vm.handler.gas_recipient.is_some() {
            let gas_limit = self.vm.handler.gas_limit.unwrap();
            let gas_price = self.vm.handler.gas_price.unwrap();
            let from = self.vm.handler.origin.unwrap();

            let wei = gas_limit.checked_mul(gas_price).ok_or(CalculationOverflow)?;
            if self.vm.handler.balance(from) < wei {
                return Err(InsufficientFunds(from, wei))
            }
        }

        Ok(())
    }

    pub fn verify_balance(&self) -> Result<()> {
        if self.vm.handler.gas_recipient.is_some() {
            let from = self.vm.handler.origin.unwrap();
            let gas_limit = self.vm.handler.gas_limit.unwrap();
            let gas_price = self.vm.handler.gas_price.unwrap();

            let (fee, refund) = self.context.fees()?;
            let lamports: U256 = fee.saturating_sub(refund).into();

            if lamports > gas_limit {
                return Err(InsufficientGas(gas_limit, lamports))
            }

            let wei = lamports.checked_mul(gas_price).ok_or(CalculationOverflow)?;

            if self.vm.handler.balance(from) < wei {
                return Err(InsufficientFunds(from, wei))
            }
        }

        Ok(())
    }

    pub fn collect_fees(&self) -> Result<()> {
        let (fee, refund) = self.vm.handler.state.base().get_fees();
        self.vm.handler.state.base().reset_fees();
        self.context.collect_fees(fee, refund)
    }
}


impl<T: Origin + Allocate, L: AccountLock + Context> Execute<MachineIt> for VmIt<'_, T, L> {
    fn advance(&mut self) -> Result<()> {
        let state_machine = self
            .state_machine
            .take()
            .unwrap_or_else(|| panic!("vm state machine fault"));

        let state_machine = match state_machine {
            FromStateHolder => {
                msg!("FromStateHolder");
                // state_holder stores tx_hash and session_id
                if self.context.has_session()? {
                    self.context.get_iteration()?.into()
                } else {
                    self.context.new_session()?;

                    if self.context.state_holder_len()? == 0 {
                        AllocateHolder(Box::new(Lock))
                    } else {
                        Lock    //start execution from the very beginning
                    }
                }
            }
            Lock => {
                msg!("Lock");
                self.context.lock()?;
                InitLocked
            }
            Init => {
                msg!("Init");
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else {
                    InitLocked
                }
            }
            InitLocked => {
                msg!("InitLocked");
                let mut tx = self.context.tx()?;
                let fee_addr = self.context.fee_recipient();
                let check_nonce = !self.context.is_gas_estimate();

                let state =  if let Some((value, reason)) = self.vm.init(&mut tx, check_nonce, fee_addr)? {
                    self.vm.set_exit_reason(reason, value);
                    if reason.is_succeed() || reason.is_revert() {
                        Commit
                    } else {
                        UnlockFailedTx // skip Commit
                    }
                } else {
                    Execute
                };

                self.verify_balance_and_gas()?;
                Serialize(Box::new(state))
            }
            Serialize(to) => {
                msg!("Serialize");
                match self.context.serialize(&self.vm) {
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
                self.context.deserialize(&mut self.vm)?;
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else {
                    IntoTrap
                }
            }
            IntoTrap => {
                msg!("IntoTrap");
                let steps_left = NUMBER_OPCODES_PER_TX.saturating_sub(self.vm.steps_executed);

                if let Some((return_value, reason)) = self.vm.execute(steps_left) {
                    self.vm.set_exit_reason(reason, return_value);
                    let next_step = if reason.is_succeed() {
                        Allocate
                    } else {
                        if reason.is_revert() {
                            Commit // skip Allocate
                        } else {
                            UnlockFailedTx // skip Commit
                        }
                    };
                    Serialize(Box::new(next_step))

                } else if NUMBER_OPCODES_PER_TX.saturating_sub(self.vm.steps_executed) > 0 {
                    IntoTrap
                } else {
                    Serialize(Box::new(Execute))
                }
            }
            Allocate => {
                msg!("Allocate");
                self.context.deserialize(&mut self.vm)?;
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else if self.vm.handler.allocate(self.context)? {
                    if self.vm.handler.journal.found_storage() {
                        Serialize(Box::new(MergeSlots))
                    } else {
                        // skip merge slots, allocate slots
                        Serialize(Box::new(Commit))
                    }
                } else {
                    Serialize(Box::new(Allocate))
                }
            }
            MergeSlots => {
                msg!("MergeSlots");
                self.context.deserialize(&mut self.vm)?;
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else {
                    self.vm.handler.merge_slots()?;
                    Serialize(Box::new(AllocateStorage))
                }
            }
            AllocateStorage => {
                msg!("AllocateStorage");
                self.context.deserialize(&mut self.vm)?;
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else if self.vm.handler.alloc_slots(self.context)? {
                    Serialize(Box::new(Commit))
                } else {
                    Serialize(Box::new(AllocateStorage))
                }
            }
            Commit => {
                msg!("Commit");
                self.context.deserialize(&mut self.vm)?;
                if !self.context.locked()? {
                    NextIteration(Box::new(Lock))
                } else {
                    self.vm.handler.commit(self.context)?;
                    self.vm.handler.revert_all();
                    
                    self.collect_fees()?;
                    let (fee, refund) = self.context.fees()?;

                    // fee_recipient account will be created at the operator's expense.
                    // otherwise it is necessary to include this cost in gas_estimate for each tx. 
                    // TODO: remove this len from alloc_payed
                    self.vm.gas_transfer(fee, refund)?; 
                    self.vm.handler.commit(self.context)?;

                    self.vm.log_exit_reason()?;
                    NextIterationUnchecked(Box::new(Unlock))
                }
            }
            Unlock => {
                msg!("Unlock");
                self.context.deserialize(&mut self.vm)?;
                if self.context.locked()? {
                    let hash = self.vm.handler.hash_journaled_accounts(&self.vm.handler.journal)?;
                    sol_log_data(&[HASH,  hash.as_ref()]);
                }

                self.context.unlock()?;
                NextIterationUnchecked(Box::new(Completed))
            }
            Completed => {
                msg!("UnnecessaryIteration: {}", self.context.tx_hash());
                return Err(UnnecessaryIteration(self.context.tx_hash()));
            }
            UnlockFailedTx => {
                msg!("UnlockFailedTx");
                self.context.deserialize(&mut self.vm)?;
                msg!("reason: {:?}", self.vm.exit_reason.unwrap());

                self.context.unlock()?;
                NextIterationUnchecked(Box::new(Failed))
            }
            Failed => {
                msg!("TxFailed: {}", self.context.tx_hash());
                return Err(UnnecessaryIteration(self.context.tx_hash()))
            }
            NextIteration(to) => {
                let fee = match *to {
                    Commit => SIG_VERIFY_COST * 3, // fee for current_iteration + Commit + Unlock
                    _ => SIG_VERIFY_COST
                };
                
                self.vm.handler.state.base().add_fee(fee)?;
                self.collect_fees()?;
                self.verify_balance()?;
                NextIterationUnchecked(to)
            }
            NextIterationUnchecked(to) => {
                self.context.set_iteration((&*to).into())?;
                Exit
            }
            Exit => unreachable!(),
        };
        self.state_machine = Some(state_machine);
        Ok(())
    }

    fn consume(&mut self, machine: MachineIt) -> Result<()> {
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
