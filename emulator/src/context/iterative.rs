use {
    super::gas_recipient,
    crate::state::State,
    rome_evm::{
        context::{
            account_lock::AccountLock,
            iterative::{
                bind_tx_to_holder_impl, deserialize_vm_impl, is_tx_binded_to_holder_impl,
                restore_iteration_impl, save_iteration_impl, serialize_vm_impl,
            },
            Context,
        },
        error::Result,
        state::{origin::Origin, Allocate},
        tx::tx::Tx,
        vm::{vm_iterative::MachineIterative, Vm},
        Iterations, H160, H256,
    },
    solana_program::{account_info::IntoAccountInfo, msg},
};

pub struct ContextIterative<'a, 'b> {
    pub state: &'b State<'a>,
    pub holder: u64,
    pub tx: &'b Tx,
    pub tx_hash: H256,
}

impl<'a, 'b> ContextIterative<'a, 'b> {
    pub fn new(state: &'b State<'a>, holder: u64, tx: &'b Tx, tx_hash: H256) -> Result<Self> {
        // allocation affects the vm behaviour.
        // it is important to allocate state_holder before the starting the vm
        let state_holder = state.info_state_holder(holder, true)?;
        msg!("state_holder data length: {}", state_holder.1.data.len());

        Ok(Self {
            state,
            holder,
            tx,
            tx_hash,
        })
    }
}

impl<'a, 'b> Context for ContextIterative<'a, 'b> {
    fn tx(&self) -> &Tx {
        self.tx
    }
    fn save_iteration(&self, iteration: Iterations) -> Result<()> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        save_iteration_impl(&info, iteration)?;
        self.state.update(bind)
    }
    fn restore_iteration(&self) -> Result<Iterations> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        restore_iteration_impl(&info)
    }
    fn serialize_vm<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        vm: &Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        serialize_vm_impl(&info, vm)?;
        self.state.update(bind)
    }
    fn deserialize_vm<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        vm: &mut Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        deserialize_vm_impl(&info, vm)
    }
    fn allocate_holder(&self) -> Result<()> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let len = bind.1.data.len() + self.state.available_for_allocation();
        self.state.realloc(&mut bind, len)?;
        self.state.update(bind)
    }

    fn bind_tx_to_holder(&self) -> Result<()> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        bind_tx_to_holder_impl(&info, self.tx_hash)?;
        self.state.update(bind)
    }

    fn is_tx_binded_to_holder(&self) -> Result<bool> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        is_tx_binded_to_holder_impl(&info, self.tx_hash)
    }

    fn tx_hash(&self) -> H256 {
        self.tx_hash
    }

    fn gas_recipient(&self) -> Result<Option<H160>> {
        gas_recipient(self.state)
    }
}
