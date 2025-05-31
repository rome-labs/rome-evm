use {
    super::Snapshot,
    crate::{
        config::{
            EXIT_REASON, GAS_RECIPIENT, GAS_VALUE, REVERT_ERROR, REVERT_PANIC, RSOL_DECIMALS,
            GAS_PRICE,
        },
        error::{Result, RomeProgramError::*},
        origin::Origin,
        state::{
            handler::{CallInterrupt, CreateInterrupt},
            Allocate, Diff, JournaledState,
        },
        tx::tx::Tx,
        vm::Reason,
    },
    evm::{Capture, ExitReason, Handler, Resolve, H160, U256},
    solana_program::{log::sol_log_data, msg},
    std::mem::size_of,
};

pub enum Trap {
    Call(CallInterrupt),
    Create(CreateInterrupt),
    ExitFromSnapshot(ExitReason),
    ExitNoShapshot(Vec<u8>, ExitReason),
}

pub struct Vm<'a, T: Origin + Allocate> {
    pub snapshot: Option<Box<Snapshot>>,
    pub handler: JournaledState<'a, T>,
    pub return_value: Option<Vec<u8>>,
    pub exit_reason: Option<ExitReason>,
    pub steps_executed: u64,
}

impl<'a, T: Origin + Allocate> Vm<'a, T> {
    pub fn new(state: &'a T) -> Result<Self> {
        let vm = Self {
            snapshot: None,
            handler: JournaledState::new(state)?,
            return_value: None,
            exit_reason: None,
            steps_executed: 0,
        };

        Ok(vm)
    }
    pub fn is_mut(&self) -> bool {
        if let Some(snapshot) = self.snapshot.as_ref() {
            return snapshot.is_mut();
        }

        true
    }

    pub fn call_from_tx(&mut self, tx: &mut Tx) -> Capture<(ExitReason, Vec<u8>), CallInterrupt> {
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

        let input = tx.data().unwrap();

        self
            .handler
            .call(to, transfer, input, None, false, context)
    }

    pub fn push_call_snapshot(&mut self, call: CallInterrupt) {
        msg!(
            "Call: from {}, to {}",
            &hex::encode(call.context.caller),
            &hex::encode(call.context.address)
        );
        let code = self.handler.code(call.code_address);
        let valids = self.handler.valids(call.code_address);
        let runtime = evm::Runtime::new(code, valids, call.input, call.context);

        self.handler.new_page();

        if self.snapshot.is_none() {
            let from = self.handler.origin.unwrap();
            self.handler.journal.get_mut(&from).push(Diff::NonceChange);
        }

        if let Some(transfer) = call.transfer {
            self.handler.transfer(&transfer.source, &transfer.target, &transfer.value);
        }
        let snapshot = Snapshot {
            evm: runtime,
            reason: Reason::Call,
            mutable: (!call.is_static) && self.is_mut(),
            parent: None,
        };

        self.push_snapshot(snapshot);
    }

    pub fn push_create_snapshot(&mut self, create: CreateInterrupt) {
        msg!(
            "Create: from {}, contract {}",
            &hex::encode(create.context.caller),
            &hex::encode(create.context.address)
        );
        let valids = evm::Valids::compute(&create.init_code);
        let to = create.address;
        let caller = create.context.caller;
        let runtime = evm::Runtime::new(create.init_code, valids, vec![], create.context);

        self.handler.new_page();
        self.handler.journal.get_mut(&caller).push(Diff::NonceChange);

        if evm::CONFIG.create_increase_nonce {
            self.handler.journal.get_mut(&to).push(Diff::NonceChange);
        }
        if let Some(transfer) = create.transfer {
            self.handler.transfer(&transfer.source, &transfer.target, &transfer.value);
        }
        let snapshot = Snapshot {
            evm: runtime,
            reason: Reason::Create(to),
            mutable: self.is_mut(),
            parent: None,
        };

        self.push_snapshot(snapshot);
    }

    pub fn push_snapshot(&mut self, mut new: Snapshot) {
        self.handler.mutable = new.mutable;
        new.parent = self.snapshot.take();
        self.snapshot = Some(Box::new(new));
        // TODO: remove "mutable" and "from" fields from JournaledState,
        // TODO: implement Handler trait for the struct:
        // struct {
        //    handler: JournaledState,
        //    snapshot: Snanpshot,
        // }
    }

    pub fn pop_snapshot(&mut self) -> Option<Box<Snapshot>> {
        if let Some(mut snapshot) = self.snapshot.take() {
            self.snapshot = snapshot.parent.take();
            self.handler.mutable = self.is_mut();

            return Some(snapshot);
        }

        None
    }

    pub fn verify_gas_price(&self) -> Result<()> {
        if self.handler.gas_recipient.is_some() {
            if self.handler.gas_price.unwrap() < U256::exp10(RSOL_DECIMALS - 9) {
                return Err(InvalidGasPrice)
            }
        }

        Ok(())
    }

    pub fn init(
        &mut self,
        tx: &mut Tx,
        check_nonce: bool,
        fee_recipient: Option<H160>
    ) -> Result<Option<(Vec<u8>, ExitReason)>> {

        let from = tx.from();
        msg!("from {}", &hex::encode(from));

        // TODO add test to eliminate the possibility of repeated transaction execution
        if check_nonce {
            let nonce = self.handler.nonce(from);
            if nonce != tx.nonce().into() {
                return Err(InvalidTxNonce(from, tx.nonce(), nonce.as_u64()));
            }
        }
        self.handler.origin = Some(from);
        self.handler.gas_limit = Some(tx.gas_limit());
        self.handler.gas_price = Some(tx.gas_price());
        self.handler.gas_recipient = fee_recipient;
        self.verify_gas_price()?;

        let trap = if tx.to().is_some() {
            match self.call_from_tx(tx) {
                Capture::Trap(call) => Trap::Call(call),
                Capture::Exit((reason, value)) => Trap::ExitNoShapshot(value, reason)
            }
        } else {
            let capture = self
                .handler
                .create(
                    tx.from(),
                    evm::CreateScheme::Legacy { caller: tx.from() },
                    tx.value(),
                    tx.data().unwrap(),
                    None,
                );

            match capture {
                Capture::Trap(create) => Trap::Create(create),
                Capture::Exit((reason, _, value)) => Trap::ExitNoShapshot(value, reason)
            }
        };

        Ok(self.trap(trap))
    }

    pub fn commit_exit(
        &mut self,
        snapshot: Box<Snapshot>,
        reason: ExitReason,
    ) -> Option<(Vec<u8>, ExitReason)> {
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
        let return_value = snapshot.evm.machine().return_value();

        if self.snapshot.is_none() {
            return Some((return_value, reason))
        }

        let latest = self.snapshot.as_mut().unwrap();

        match evm::save_return_value::<JournaledState<'a, T>>(
            &mut latest.evm,
            reason,
            return_value,
        ) {
            evm::Control::Continue => None,
            evm::Control::Exit(reason) => {
                assert!(reason.is_fatal());
                Some((vec![], reason))
            },
            _ => unreachable!(),
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
            assert!(self.handler.mutable);
            self.handler.set_code(address, return_value);
        }

        if self.snapshot.is_none() {
            return if reason.is_revert() {
                Some((snapshot.evm.machine().return_value(), reason))
            } else {
                Some((vec![], reason))
            };
        }

        let latest = self.snapshot.as_mut().unwrap();

        match evm::save_created_address::<JournaledState<'a, T>>(
            &mut latest.evm,
            reason,
            Some(address),
        ) {
            evm::Control::Continue => None,
            evm::Control::Exit(reason) => {
                assert!(reason.is_fatal());
                Some((vec![], reason))
            },
            _ => unreachable!(),
        }
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

    fn to_trap(capture: Capture<ExitReason, Resolve<JournaledState<T>>>) -> Option<Trap> {
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
            Capture::Exit(reason) => Some(Trap::ExitFromSnapshot(reason)),
        }
    }

    pub fn trap(&mut self, trap: Trap) -> Option<(Vec<u8>, ExitReason)> {
        match trap {
            Trap::Call(call) => {
                self.push_call_snapshot(call);
                None
            }
            Trap::Create(create) => {
                self.push_create_snapshot(create);
                None
            }
            Trap::ExitFromSnapshot(reason) => {
                let snapshot = self.pop_snapshot().expect("vm fault");
                let exit = self.commit_exit(snapshot, reason);

                if let Some((_, reason)) = exit.as_ref() {
                    if !reason.is_succeed() {
                        self.handler.revert_all();  // fatal error or there is no parent snapshot
                    }
                    return exit
                }

                if !reason.is_succeed() {
                    self.handler.revert_page()
                } else {
                    self.handler.journal.merge_page();
                }

                exit
            }
            Trap::ExitNoShapshot(value, reason) => {
                if reason.is_succeed() { 
                    let from = self.handler.origin.unwrap();
                    self.handler.journal.get_mut(&from).push(Diff::NonceChange);
                }
                // no need to revert diff, it was done in handler.call()
                
                Some((value, reason))
            }
        }
    }

    pub fn execute(&mut self, steps: u64) -> Option<(Vec<u8>, ExitReason)> {
        let snapshot = self.snapshot.as_mut().expect("vm fault");
        let (steps, capture) = snapshot.evm.run(steps, &mut self.handler);
        self.steps_executed += steps;

        Self::to_trap(capture).and_then(|trap| self.trap(trap))
    }
 
    pub fn gas_transfer(&mut self, fee:u64, refund: u64) -> Result<()> {
        let mut buf_limit = [0_u8; 32];
        let mut buf_price = [0_u8; 32];

        if let Some(to) = self.handler.gas_recipient {
            let gas_limit = self.handler.gas_limit.unwrap();
            let gas_price = self.handler.gas_price.unwrap();

            let from = self.handler.origin.unwrap();
            let lamports: U256 = fee.saturating_sub(refund).into();

            if lamports > gas_limit {
                return Err(InsufficientGas(gas_limit, lamports))
            }

            let wei = lamports.checked_mul(gas_price).ok_or(CalculationOverflow)?;
            self.handler.transfer(&from, &to, &wei);

            lamports.to_big_endian(&mut buf_limit);
            gas_price.to_big_endian(&mut buf_price);
            
            sol_log_data(&[GAS_RECIPIENT, to.as_bytes()]);
        }
        
        sol_log_data(&[GAS_VALUE, &buf_limit]);
        sol_log_data(&[GAS_PRICE, &buf_price]);

        Ok(())
    }

    pub fn set_exit_reason(&mut self, reason: ExitReason, value: Vec<u8>) {
        self.exit_reason = Some(reason);
        self.return_value = Some(value);
    }
}
