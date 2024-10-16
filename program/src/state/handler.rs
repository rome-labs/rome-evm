use {
    super::{precompiled_contract, JournaledState},
    crate::{origin::Origin, precompile::built_in_contract, state::Allocate, state::Diff},
    evm::{
        Capture, Context, CreateScheme, ExitError, ExitReason, Handler, Machine, Opcode, Stack,
        Transfer, H160, H256, U256,
    },
    solana_program::keccak::hash,
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
        U256::from(self.state.chain_id())
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
        self.journal.get_mut(&address).push(Diff::Suicide);
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

        self.journal.get_mut(&caller).push(Diff::NonceChange);

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

        if let Some(f) = built_in_contract(&code_address) {
            let return_value = f(&input);
            let ok = ExitReason::Succeed(evm::ExitSucceed::Returned);
            return Capture::Exit((ok, return_value));
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

    fn call_feedback(
        &mut self,
        _feedback: Self::CallFeedback,
    ) -> std::result::Result<(), ExitError> {
        Ok(())
    }

    /// Handle other unknown external opcodes.
    fn other(
        &mut self,
        _opcode: Opcode,
        _stack: &mut Machine,
    ) -> std::result::Result<(), ExitError> {
        Err(ExitError::IncompatibleVersionEVM)
    }
}
