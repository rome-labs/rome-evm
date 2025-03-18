use {
    super::iterative_lock::iterative_lock,
    crate::{state::State, LockOverrides},
    rome_evm::{
        context::{
            account_lock::AccountLock,
            iterative::{deserialize_impl, serialize_impl},
            Context,
        },
        error::Result,
        state::{origin::Origin, Allocate},
        tx::{legacy::Legacy, tx::Tx},
        vm::{vm_iterative::MachineIterative, Vm},
        Data, Holder, Iterations, StateHolder, H160, H256,
    },
    solana_program::{
        account_info::{AccountInfo, IntoAccountInfo},
        keccak,
        pubkey::Pubkey,
    },
    std::cell::RefCell,
};

pub struct ContextEstimateGas<'a, 'b> {
    pub state: &'b State<'a>,
    pub holder: u64,
    pub tx_hash: H256,
    pub legacy: Legacy,
    pub lock_overrides: RefCell<Vec<Pubkey>>,
    pub session: u64,
}
impl<'a, 'b> ContextEstimateGas<'a, 'b> {
    pub fn new(state: &'b State<'a>, legacy: Legacy) -> Result<Self> {
        let hash = H256::from(keccak::hash(&[1, 2, 3]).to_bytes());
        let holder = 0;
        let _state_holder = state.info_state_holder(holder, true)?;
        Ok(Self {
            state,
            legacy,
            tx_hash: hash,
            holder,
            lock_overrides: RefCell::new(vec![]),
            session: 1, // must not be equal to default value of the StateHolder.session
        })
    }
}

impl<'a, 'b> Context for ContextEstimateGas<'a, 'b> {
    fn tx(&self) -> Result<Tx> {
        Ok(Tx::from_legacy(self.legacy.clone()))
    }
    fn save_iteration(&self, iteration: Iterations) -> Result<()> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        StateHolder::set_iteration(&info, iteration)?;
        self.state.update(bind);
        Ok(())
    }
    fn restore_iteration(&self) -> Result<Iterations> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        StateHolder::get_iteration(&info)
    }
    fn serialize<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        vm: &Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        serialize_impl(&info, vm, self.state)?;
        self.state.update(bind);
        Ok(())
    }
    fn deserialize<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        vm: &mut Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        deserialize_impl(&info, vm, self.state)
    }
    fn allocate_holder(&self) -> Result<()> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let len = bind.1.data.len() + self.state.alloc_limit();
        self.state.realloc(&mut bind, len)?;
        self.state.update(bind);
        Ok(())
    }

    fn new_session(&self) -> Result<()> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        StateHolder::set_link(&info, self.tx_hash, self.session)?;
        self.state.update(bind);
        Ok(())
    }

    fn exists_session(&self) -> Result<bool> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        StateHolder::is_linked(&info, self.tx_hash, self.session)
    }

    fn tx_hash(&self) -> H256 {
        self.tx_hash
    }

    fn fee_recipient(&self) -> Option<H160> {
        // TODO: take into account the allocation of fee_recipient account
        None
    }

    fn check_nonce(&self) -> bool {
        false
    }

    fn state_holder_len(&self) -> Result<usize> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();
        Ok(Holder::size(&info))
    }
}

impl AccountLock for ContextEstimateGas<'_, '_> {
    fn lock(&self) -> Result<()> {
        let mut lock_overrides = self.lock_overrides.borrow_mut();
        *lock_overrides = iterative_lock(self.state, self.holder)?;
        Ok(())
    }
    fn locked(&self) -> Result<bool> {
        // during transaction emulation accounts are not locked
        Ok(true)
    }
    fn unlock(&self) -> Result<()> {
        // it doesn't make sense for emulation
        Ok(())
    }
    fn lock_new_one(&self, _info: &AccountInfo) -> Result<()> {
        unreachable!()
    }
    fn check_writable(&self, _info: &AccountInfo) -> Result<()> {
        Ok(())
    }
}

impl<'a, 'b> LockOverrides for ContextEstimateGas<'a, 'b> {
    fn lock_overrides(&self) -> Vec<Pubkey> {
        self.lock_overrides.borrow().clone()
    }
}
