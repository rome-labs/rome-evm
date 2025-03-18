use {
    super::{aux::Ix, precompiled_contract, JournaledState},
    crate::{
        origin::Origin,
        precompile::{ non_evm_program,},
        state::{Allocate, Diff},
        non_evm::Program, aux::revert_msg,
    },
    evm::{
        Capture, Context, CreateScheme, ExitError, ExitReason, Handler, Machine, Opcode, Stack,
        Transfer, H160, H256, U256, ExitSucceed::Returned, ExitRevert::Reverted,
    },
    solana_program::{keccak::hash, },
    std::convert::Infallible,
};

pub struct CallInterrupt {
    pub code_address: H160,
    pub transfer: Option<Transfer>,
    pub input: Vec<u8>,
    pub is_static: bool,
    pub context: Context,
}

pub struct CreateInterrupt {
    pub context: Context,
    pub transfer: Option<Transfer>,
    pub address: H160,
    pub init_code: Vec<u8>,
}

impl<T: Origin + Allocate> Handler for JournaledState<'_, T> {
    type CreateInterrupt = CreateInterrupt;
    type CreateFeedback = Infallible;
    type CallInterrupt = CallInterrupt;
    type CallFeedback = Infallible;

    fn keccak256_h256(&self, data: &[u8]) -> H256 {
        let hash = hash(data);
        H256::from(hash.to_bytes())
    }

    fn nonce(&self, address: H160) -> U256 {
        let diff = self.journal.nonce_diff(&address);
        let nonce = self.state.nonce(&address).unwrap_or(0);

        (nonce + diff).into()
    }

    fn balance(&self, address: H160) -> U256 {
        let debet = self.journal.transfer_from(&address);
        let credit = self.journal.transfer_to(&address);
        let base = self.state.balance(&address).unwrap_or(U256::zero());

        base + credit - debet
    }

    fn code_size(&self, address: H160) -> U256 {
        if precompiled_contract(address) {
            U256::one()
        } else {
            if let Some((code, _)) = self.journal.code_valids_diff(&address) {
                return code.len().into();
            }

            self.state.code(&address).map_or(0, |vec| vec.len()).into()
        }
    }

    fn code_hash(&self, address: H160) -> H256 {
        let code = self.code(address);
        let hash = hash(&code);
        H256::from(hash.to_bytes())
    }

    fn code(&self, address: H160) -> Vec<u8> {
        if let Some((code, _)) = self.journal.code_valids_diff(&address) {
            return code.clone();
        }

        self.state.code(&address).unwrap_or_default()
    }

    fn valids(&self, address: H160) -> Vec<u8> {
        if let Some((_, valids)) = self.journal.code_valids_diff(&address) {
            return valids.clone();
        }

        self.state.valids(&address).unwrap_or_default()
    }

    fn storage(&self, address: H160, index: U256) -> U256 {
        if let Some(value) = self.journal.storage_diff(&address, &index) {
            return value;
        }
        self.state
            .storage(&address, &index)
            .map_or(U256::zero(), |opt| opt.unwrap_or(U256::zero()))
    }

    fn gas_left(&self) -> U256 {
        // todo!()
        U256::max_value()
    }

    fn gas_price(&self) -> U256 {
        U256::one() // todo!()
    }

    fn origin(&self) -> H160 {
        self.origin
            .unwrap_or_else(|| panic!("journal_state.origin expected"))
    }

    fn block_hash(&self, number: U256) -> H256 {
        let new: U256 = self.slot.into();
        let diff = new.saturating_sub(self.block_number);

        let block = number.saturating_add(diff);
        self.block_hash(block).unwrap_or_else(|e| panic!("{}", e))
    }

    fn block_number(&self) -> U256 {
        self.block_number
    }

    fn block_coinbase(&self) -> H160 {
        H160::default()
    }

    fn block_timestamp(&self) -> U256 {
        self.block_timestamp
    }

    fn block_difficulty(&self) -> U256 {
        U256::zero()
    }
    fn block_gas_limit(&self) -> U256 {
        U256::max_value()
    }

    fn chain_id(&self) -> U256 {
        U256::from(self.state.base().chain)
    }

    fn set_storage(&mut self, address: H160, index: U256, value: U256) -> Result<(), ExitError> {
        if !self.mutable {
            return Err(ExitError::StaticModeViolation);
        }

        self.journal
            .get_mut(&address)
            .push(Diff::StorageChange { key: index, value });
        Ok(())
    }

    fn log(&mut self, address: H160, topics: Vec<H256>, data: Vec<u8>) -> Result<(), ExitError> {
        if !self.mutable {
            return Err(ExitError::StaticModeViolation);
        }

        self.journal
            .get_mut(&address)
            .push(Diff::Event { topics, data });
        Ok(())
    }

    fn mark_delete(&mut self, address: H160, target: H160) -> Result<(), ExitError> {
        if !self.mutable {
            return Err(ExitError::StaticModeViolation);
        }

        let value = self.balance(address);

        self.transfer(&address, &target, &value)
            .map_err(|_| ExitError::OutOfFund)?;

        if !self.code_size(address).is_zero() {
            let onchain_code_size = if precompiled_contract(address) {
                1
            } else {
                self.state.code(&address).map_or(0, |vec| vec.len())
            };

            if onchain_code_size == 0 {
                self.journal.selfdestruct(&address)
            }
        }

        Ok(())
    }

    fn create(
        &mut self,
        caller: H160,
        scheme: CreateScheme,
        value: U256,
        init_code: Vec<u8>,
        _target_gas: Option<u64>,
    ) -> Capture<(ExitReason, Option<H160>, Vec<u8>), Self::CreateInterrupt> {
        if !self.mutable {
            return Capture::Exit((
                ExitReason::Error(ExitError::StaticModeViolation),
                None,
                vec![],
            ));
        }

        if !value.is_zero() && self.balance(caller) < value {
            return Capture::Exit((ExitReason::Error(ExitError::OutOfFund), None, vec![]));
        }

        let new_addres = self.build_address(scheme);
        self.journal.get_mut(&caller).push(Diff::NonceChange);

        if new_addres.is_err() {
            return Capture::Exit((ExitReason::Error(ExitError::CreateCollision), None, vec![]));
        }
        let new_addres = new_addres.unwrap();

        let context = evm::Context {
            address: new_addres,
            caller,
            apparent_value: value,
        };

        let transfer = Some(Transfer {
            source: caller,
            target: new_addres,
            value,
        });

        let create = CreateInterrupt {
            context,
            transfer,
            address: new_addres,
            init_code,
        };

        Capture::Trap(create)
    }

    fn call(
        &mut self,
        code_address: H160,
        transfer: Option<Transfer>,
        input: Vec<u8>,
        _target_gas: Option<u64>,
        is_static: bool,
        context: Context,
    ) -> Capture<(ExitReason, Vec<u8>), Self::CallInterrupt> {
        if !self.mutable || is_static {
            if let Some(transfer) = transfer {
                if !transfer.value.is_zero() {
                    return Capture::Exit((
                        ExitReason::Error(ExitError::StaticModeViolation),
                        vec![],
                    ));
                }
                if self.balance(transfer.source) < transfer.value {
                    return Capture::Exit((ExitReason::Error(ExitError::OutOfFund), vec![]));
                }
            }
        }

        if let Some(program) =  non_evm_program(&code_address, self.state) {
            let (reason, value) = if program.found_eth_call(&input) {
                match program.eth_call(&input) {
                    Ok(val) => (ExitReason::Succeed(Returned), val),
                    Err(e) =>
                        (ExitReason::Revert(Reverted), revert_msg(e.to_string()))
                }
            } else {
                self.non_evm_invoke(&code_address, &transfer, &input, is_static, &context, program)
            };
            return Capture::Exit((reason, value))
        }

        let call = CallInterrupt {
            code_address,
            transfer,
            input,
            is_static,
            context,
        };

        Capture::Trap(call)
    }

    fn pre_validate(
        &mut self,
        _context: &Context,
        _opcode: Opcode,
        _stack: &Stack,
    ) -> Result<(), ExitError> {
        Ok(())
    }

    fn call_feedback(&mut self, _feedback: Self::CallFeedback) -> Result<(), ExitError> {
        Ok(())
    }

    /// Handle other unknown external opcodes.
    fn other(&mut self, opcode: Opcode, _stack: &mut Machine) -> Result<(), ExitError> {
        Err(ExitError::IncompatibleVersionEVM(opcode.0))
    }

    fn transient_storage(&self, address: H160, index: U256) -> U256 {
        if let Some(value) = self.journal.t_storage_diff(&address, &index) {
            return value;
        }

        U256::zero()
    }

    fn set_transient_storage(
        &mut self,
        address: H160,
        index: U256,
        value: U256,
    ) -> Result<(), ExitError> {
        if !self.mutable {
            return Err(ExitError::StaticModeViolation);
        }

        self.journal
            .get_mut(&address)
            .push(Diff::TStorageChange { key: index, value });
        Ok(())
    }
}

impl<'a, T: Origin + Allocate> JournaledState<'a, T> {
    pub fn non_evm_invoke(
        &mut self,
        _code_address: &H160,
        transfer: &Option<Transfer>,
        input: &[u8],
        is_static: bool,
        context: &Context,
        program: Box<dyn Program + 'a>,
    ) -> (ExitReason, Vec<u8>) {

        if is_static {
            return (Reverted.into(), revert_msg("StaticModeViolation".to_string()))
        }

        // TODO: uncomment, security issue !
        // if context.address != *code_address {
        //     return (Reverted.into(), revert_msg("DelegateCallProhibited".to_string()))
        // }

        if let Some(transfer) = transfer {
            if !transfer.value.is_zero() {
                return (Reverted.into(), revert_msg("TransferProhibited".to_string()))
            }
        }

        let (ix, seed) = match program.ix_from_abi(input, context.caller) {
            Ok(x) => x,
            Err(e) => return (Reverted.into(), revert_msg(e.to_string()))
        };

        let non_evm_state = self.journal.non_evm_state();

        let binds = match non_evm_state.ix_accounts_mut(self.state, &ix) {
            Ok(binds) => binds,
            Err(e) => return (Reverted.into(), revert_msg(e.to_string()))
        };

        let return_value = match program.emulate(&ix, binds) {
            Ok(x) => x,
            Err(e) => return (Reverted.into(), revert_msg(e.to_string()))
        };

        let ixs = self.journal.non_evm_ix.get_or_insert(vec![]);
        ixs.push(Ix::new(ix, seed));

        (ExitReason::Succeed(Returned), return_value)
    }
}
