use {
    super::LockOverrides,
    crate::state::State,
    rome_evm::{
        context::{
            account_lock::AccountLock,
            iterative::{deserialize_impl, serialize_impl},
            Context,
        },
        error::Result,
        state::{origin::Origin, Allocate},
        tx::tx::Tx,
        vm::{vm_iterative::MachineIterative, Vm},
        Data, Holder, Iterations, StateHolder, H160, H256,
    },
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey},
    std::cell::RefCell,
};

pub struct ContextIterative<'a, 'b> {
    pub state: &'b State<'a>,
    pub holder: u64,
    pub tx_hash: H256,
    pub lock_overrides: RefCell<Vec<Pubkey>>,
    pub session: u64,
    pub fee_addr: Option<H160>,
    pub rlp: &'b [u8],
}

impl<'a, 'b> ContextIterative<'a, 'b> {
    pub fn new(
        state: &'b State<'a>,
        holder: u64,
        tx_hash: H256,
        session: u64,
        fee_addr: Option<H160>,
        rlp: &'b [u8],
    ) -> Result<Self> {
        // allocation affects the vm behaviour.
        // it is important to allocate state_holder before the starting the vm
        let state_holder = state.info_state_holder(holder, true)?;
        msg!("state_holder data length: {}", state_holder.1.data.len());

        Ok(Self {
            state,
            holder,
            tx_hash,
            lock_overrides: RefCell::new(vec![]),
            session,
            fee_addr,
            rlp,
        })
    }
}

impl<'a, 'b> Context for ContextIterative<'a, 'b> {
    fn tx(&self) -> Result<Tx> {
        Tx::from_instruction(self.rlp)
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
        self.fee_addr
    }

    fn state_holder_len(&self) -> Result<usize> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();
        Ok(Holder::size(&info))
    }
}

impl<'a, 'b> LockOverrides for ContextIterative<'a, 'b> {
    fn lock_overrides(&self) -> Vec<Pubkey> {
        self.lock_overrides.borrow().clone()
    }
}
