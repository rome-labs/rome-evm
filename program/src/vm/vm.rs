use {
    super::Snapshot,
    crate::{
        config::{EXIT_REASON, GAS_RECIPIENT, GAS_VALUE, REVERT_ERROR, REVERT_PANIC},
        context::{account_lock::AccountLock, Context},
        error::{Result, RomeProgramError::*},
        origin::Origin,
        state::{
            handler::{CallInterrupt, CreateInterrupt},
            Allocate, Diff, JournaledState,
        },
        tx::tx::Tx,
        vm::Reason,
    },
    evm::{Capture, ExitFatal, ExitReason, Handler, Resolve, H160, U256},
    solana_program::{log::sol_log_data, msg},
    std::mem::size_of,
};

pub struct Vm<'a, T: Origin + Allocate, M, L: AccountLock + Context> {
    pub snapshot: Option<Box<Snapshot>>,
    pub handler: JournaledState<'a, T>,
    pub state_machine: Option<M>,
    pub return_value: Option<Vec<u8>>,
    pub exit_reason: Option<ExitReason>,
    pub context: &'a L,
    pub steps_executed: u64,
}

impl<'a, T: Origin + Allocate, M: 'static, L: AccountLock + Context> Vm<'a, T, M, L> {
    pub fn is_mut(&self) -> bool {
        if let Some(snapshot) = self.snapshot.as_ref() {
            return snapshot.is_mut();
        }

        true
    }

    pub fn call_from_tx(&self, tx: &mut Tx) -> Result<CallInterrupt> {
        let to = tx.to().unwrap();
        let context = evm::Context {
            address: to,
            caller: tx.from(),
            apparent_value: tx.value(),
        };

        let transfer = if !tx.value().is_zero() {
            Some(evm::Transfer {
                source: tx.from(),
                target: to,
                value: tx.value(),
            })
        } else {
            None
        };

        Ok(CallInterrupt {
            code_address: to,
            transfer,
            input: tx.data().unwrap(),
            is_static: false,
            context,
        })
    }

    pub fn create_from_tx(&self, tx: &mut Tx) -> Result<CreateInterrupt> {
        let address_scheme = evm::CreateScheme::Legacy { caller: tx.from() };
        let to = self.handler.build_address(address_scheme)?;
        let context = evm::Context {
            address: to,
            caller: tx.from(),
            apparent_value: tx.value(),
        };

        let transfer = if !tx.value().is_zero() {
            Some(evm::Transfer {
                source: tx.from(),
                target: to,
                value: tx.value(),
            })
        } else {
            None
        };

        Ok(CreateInterrupt {
            context,
            address: to,
            transfer,
            init_code: tx.data().unwrap(),
        })
    }

    pub fn call_snapshot(&mut self, call: CallInterrupt) -> Result<Box<Snapshot>> {
        msg!(
            "Call: from {}, to {}",
            &hex::encode(call.context.caller),
            &hex::encode(call.context.address)
        );
        let code = self.handler.code(call.code_address);
        let valids = self.handler.valids(call.code_address);
        let runtime = evm::Runtime::new(code, valids, call.input, call.context);

        self.handler.new_page();
        if let Some(transfer) = call.transfer {
            self.handler
                .transfer(&transfer.source, &transfer.target, &transfer.value)?;
        }
        let snapshot = Snapshot {
            evm: runtime,
            reason: Reason::Call,
            mutable: (!call.is_static) && self.is_mut(),
            parent: None,
        };

        Ok(Box::new(snapshot))
    }

    pub fn create_snapshot(&mut self, create: CreateInterrupt) -> Result<Box<Snapshot>> {
        msg!(
            "Create: from {}, contract {}",
            &hex::encode(create.context.caller),
            &hex::encode(create.context.address)
        );
        let valids = evm::Valids::compute(&create.init_code);
        let to = create.address;
        let runtime = evm::Runtime::new(create.init_code, valids, vec![], create.context);

        self.handler.new_page();
        if evm::CONFIG.create_increase_nonce {
            self.handler.journal.get_mut(&to).push(Diff::NonceChange);
        }
        if let Some(transfer) = create.transfer {
            self.handler
                .transfer(&transfer.source, &transfer.target, &transfer.value)?;
        }
        let snapshot = Snapshot {
            evm: runtime,
            reason: Reason::Create(to),
            mutable: self.is_mut(),
            parent: None,
        };

        Ok(Box::new(snapshot))
    }

    pub fn snapshot_from_tx(&mut self) -> Result<Box<Snapshot>> {
        let mut tx = self.context.tx()?;
        let from = tx.from();
        msg!("from {}", &hex::encode(from));

        // TODO add test to eliminate the possibility of repeated transaction execution
        if self.context.check_nonce() {
            let nonce = self.handler.nonce(from);
            if nonce != tx.nonce().into() {
                return Err(InvalidTxNonce(from, tx.nonce(), nonce.as_u64()));
            }
        }

        let snapshot = if tx.to().is_some() {
            let call = self.call_from_tx(&mut tx)?;
            self.call_snapshot(call)?
        } else {
            let create = self.create_from_tx(&mut tx)?;
            self.create_snapshot(create)?
        };

        //todo: remove it for the eth_call
        self.handler
            .journal
            .get_mut(&tx.from())
            .push(Diff::NonceChange); // manual increment caller.nonce
        self.handler.origin = Some(tx.from());
        self.handler.gas_limit = Some(tx.gas_limit());
        self.handler.gas_price = Some(tx.gas_price());
        self.handler.gas_recipient = self.context.fee_recipient();

        Ok(snapshot)
    }

    pub fn commit_exit(
        &mut self,
        snapshot: Box<Snapshot>,
        reason: ExitReason,
    ) -> Option<(Vec<u8>, ExitReason)> {
        if !reason.is_succeed() {
            self.handler.revert_diff();
        }

        match snapshot.reason {
            Reason::Call => self.commit_exit_call(snapshot, reason),
            Reason::Create(address) => self.commit_exit_create(snapshot, reason, address),
        }
    }

    pub fn commit_exit_call(
        &mut self,
        snapshot: Box<Snapshot>,
        reason: ExitReason,
    ) -> Option<(Vec<u8>, ExitReason)> {
        // TODO: check return_value in case of revert
        let return_value = snapshot.evm.machine().return_value();

        if let Some(snapshot) = self.snapshot.as_mut() {
            match evm::save_return_value::<JournaledState<'a, T>>(
                &mut snapshot.evm,
                reason,
                return_value,
            ) {
                evm::Control::Continue => None,
                evm::Control::Exit(reason) => Some((vec![], reason)),
                _ => unreachable!(),
            }
        } else {
            Some((return_value, reason))
        }
    }
    pub fn commit_exit_create(
        &mut self,
        snapshot: Box<Snapshot>,
        reason: ExitReason,
        address: H160,
    ) -> Option<(Vec<u8>, ExitReason)> {
        if reason.is_succeed() {
            let return_value = snapshot.evm.machine().return_value();
            // TODO: static flag ?
            self.handler.set_code(address, return_value);
        }

        let snapshot = if let Some(snapshot) = self.snapshot.as_mut() {
            snapshot
        } else {
            let return_value = if let ExitReason::Revert(_) = reason {
                snapshot.evm.machine().return_value()
            } else {
                vec![]
            };
            return Some((return_value, reason));
        };

        match evm::save_created_address::<JournaledState<'a, T>>(
            &mut snapshot.evm,
            reason,
            Some(address),
        ) {
            evm::Control::Continue => None,
            evm::Control::Exit(reason) => Some((vec![], reason)),
            _ => unreachable!(),
        }
    }

    pub fn add_snapshot(&mut self, mut new: Box<Snapshot>) {
        new.parent = self.snapshot.take();
        self.snapshot = Some(new);
        // TODO: remove mutable and from fields from JournaledState,
        // TODO: implement Handler trait for this one:
        // struct {
        //    handler: JournaledState,
        //    snapshot: Snanpshot,
        // }
        //
        self.handler.mutable = self.is_mut();
    }

    pub fn remove_snapshot(&mut self) -> Option<Box<Snapshot>> {
        if let Some(mut snapshot) = self.snapshot.take() {
            self.snapshot = snapshot.parent.take();
            self.handler.mutable = self.is_mut();

            return Some(snapshot);
        }

        None
    }

    pub fn log_exit_reason(&self) -> Result<()> {
        assert!(self.exit_reason.is_some());
        let exit_reason = self.exit_reason.unwrap();

        let code = match exit_reason {
            ExitReason::Succeed(_) => 0x0_u8,
            ExitReason::Error(_) => 0x1,
            ExitReason::Revert(_) => {
                self.log_revert_msg()?;
                0x2
            }
            ExitReason::Fatal(_) => 0x3,
            ExitReason::StepLimitReached => panic!("vm state machine fault: StepLimitReached"),
        };

        let mut return_value = &vec![];
        if let Some(value) = self.return_value.as_ref() {
            return_value = value;
        };

        let msg = format!("{:?}", exit_reason);
        let msg_len = msg.len();
        sol_log_data(&[
            EXIT_REASON,
            &[code],
            &msg_len.to_le_bytes(),
            msg.as_bytes(),
            return_value,
        ]);
        Ok(())
    }

    pub fn log_revert_msg(&self) -> Result<()> {
        let return_value = if let Some(value) = &self.return_value {
            value
        } else {
            return Ok(());
        };

        if return_value.starts_with(REVERT_ERROR) {
            let msg = &return_value[REVERT_ERROR.len()..];
            let left = 64_usize;
            let offset = 32_usize;

            if msg.len() >= left {
                let found = U256::from_big_endian(&msg[0..size_of::<U256>()]) == offset.into();
                if found {
                    let len = U256::from_big_endian(&msg[offset..left]).as_usize();
                    let right = left.checked_add(len).ok_or(CalculationOverflow)?;
                    if let Some(msg) = msg.get(left..right) {
                        let msg = std::str::from_utf8(msg).unwrap_or("str::from_utf8() error");
                        msg!("Revert: {:?}", msg);
                    }
                }
            }
        } else if return_value.starts_with(REVERT_PANIC) {
            let msg = &return_value[REVERT_ERROR.len()..];
            let len = size_of::<U256>();

            if msg.len() == len {
                let msg = U256::from_big_endian(&msg[0..len]);
                msg!("Revert panic: {:?})", msg);
            }
        }

        Ok(())
    }

    fn capture_to_trap(capture: Capture<ExitReason, Resolve<JournaledState<T>>>) -> Option<Trap> {
        match capture {
            Capture::Trap(trap) => {
                match trap {
                    Resolve::Call(call, runtime) => {
                        std::mem::forget(runtime); // todo: remove it from evm.run() result
                        Some(Trap::Call(call))
                    }
                    Resolve::Create(create, runtime) => {
                        std::mem::forget(runtime);
                        Some(Trap::Create(create))
                    }
                }
            }
            Capture::Exit(ExitReason::StepLimitReached) => None,
            Capture::Exit(reason) => Some(Trap::Reason(reason)),
        }
    }

    pub fn execute(&mut self, steps: u64) -> Result<Option<(Vec<u8>, ExitReason)>> {
        let trap = if let Some(snapshot) = self.snapshot.as_mut() {
            let (steps, capture) = snapshot.evm.run(steps, &mut self.handler);
            self.steps_executed += steps;
            if let Some(trap) = Self::capture_to_trap(capture) {
                trap
            } else {
                return Ok(None);
            }
        } else {
            return Ok(Some((vec![], ExitReason::Fatal(ExitFatal::NotSupported))));
        };

        match trap {
            Trap::Call(call) => {
                let snapshot = self.call_snapshot(call)?;
                self.add_snapshot(snapshot);
                Ok(None)
            }
            Trap::Create(create) => {
                let snapshot = self.create_snapshot(create)?;
                self.add_snapshot(snapshot);
                Ok(None)
            }
            Trap::Reason(reason) => {
                if let Some(snapshot) = self.remove_snapshot() {
                    Ok(self.commit_exit(snapshot, reason))
                } else {
                    Ok(Some((vec![], ExitReason::Fatal(ExitFatal::NotSupported))))
                }
            }
        }
    }

    pub fn log_gas_transfer(&self) {
        let mut sum_be = [0_u8; 32];
        self.handler.gas_limit.unwrap().to_big_endian(&mut sum_be);

        sol_log_data(&[GAS_VALUE, &sum_be]);
        if let Some(to) = self.handler.gas_recipient {
            sol_log_data(&[GAS_RECIPIENT, to.as_bytes()]);
        }
    }

    pub fn gas_transfer(&mut self) -> Result<()> {
        let gas_used = self.handler.gas_limit.unwrap();

        if let Some(to) = self.handler.gas_recipient {
            let gas_price = self.handler.gas_price.unwrap();
            let from = self.handler.origin.unwrap();

            let tx_price = gas_used.checked_mul(gas_price).ok_or(CalculationOverflow)?;

            self.handler.transfer(&from, &to, &tx_price)?;
        }

        Ok(())
    }
}

pub enum Trap {
    Call(CallInterrupt),
    Create(CreateInterrupt),
    Reason(ExitReason),
}
