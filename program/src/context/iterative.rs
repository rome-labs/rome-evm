use {
    super::{account_lock::AccountLock, Context},
    crate::{
        accounts::Iterations,
        accounts::{Data, Holder, StateHolder},
        context::gas_recipient,
        error::Result,
        state::{origin::Origin, Allocate, JournaledState},
        tx::tx::Tx,
        vm::{vm_iterative::MachineIterative, Snapshot, Vm},
        State,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    evm::{H160, H256},
    solana_program::account_info::AccountInfo,
};

pub struct ContextIterative<'a, 'b> {
    pub state: &'b State<'a>,
    pub origin_accounts: &'a [AccountInfo<'a>],
    pub lock_overrides: &'a [u8],
    pub state_holder: &'a AccountInfo<'a>,
    pub tx_hash: H256,
    pub tx: Tx,
}

impl<'a, 'b> ContextIterative<'a, 'b> {
    pub fn new(
        state: &'b State<'a>,
        accounts: &'a [AccountInfo<'a>],
        holder: u64,
        lock_overrides: &'a [u8],
        tx: Tx,
        tx_hash: H256,
    ) -> Result<Self> {
        let state_holder = state.info_state_holder(holder, true)?;

        Ok(Self {
            state,
            origin_accounts: accounts,
            lock_overrides,
            state_holder,
            tx_hash,
            tx,
        })
    }
}

impl<'a, 'b> Context for ContextIterative<'a, 'b> {
    fn tx(&self) -> &Tx {
        &self.tx
    }
    fn save_iteration(&self, iteration: Iterations) -> Result<()> {
        save_iteration_impl(self.state_holder, iteration)
    }
    fn restore_iteration(&self) -> Result<Iterations> {
        restore_iteration_impl(self.state_holder)
    }
    fn serialize_vm<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        vm: &Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        serialize_vm_impl(self.state_holder, vm)
    }
    fn deserialize_vm<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        vm: &mut Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        deserialize_vm_impl(self.state_holder, vm)
    }
    fn allocate_holder(&self) -> Result<()> {
        let len = self.state_holder.data_len() + self.state.available_for_allocation();
        self.state.realloc(self.state_holder, len)
    }

    fn bind_tx_to_holder(&self) -> Result<()> {
        bind_tx_to_holder_impl(self.state_holder, self.tx_hash)
    }

    fn is_tx_binded_to_holder(&self) -> Result<bool> {
        is_tx_binded_to_holder_impl(self.state_holder, self.tx_hash)
    }

    fn tx_hash(&self) -> H256 {
        self.tx_hash
    }

    fn gas_recipient(&self) -> Result<Option<H160>> {
        gas_recipient(self.state)
    }
}

// these functions are used both in the contract and in the emulator
pub fn save_iteration_impl(info: &AccountInfo, iteration: Iterations) -> Result<()> {
    let mut state_holder = StateHolder::from_account_mut(info)?;
    state_holder.iteration = iteration;
    Ok(())
}

pub fn restore_iteration_impl(info: &AccountInfo) -> Result<Iterations> {
    let state_holder = StateHolder::from_account(info)?;
    Ok(state_holder.iteration.clone())
}

pub fn serialize_vm_impl<T: Origin + Allocate, L: AccountLock + Context>(
    info: &AccountInfo,
    vm: &Vm<T, MachineIterative, L>,
) -> Result<()> {
    let mut into: &mut [u8] = &mut Holder::from_account_mut(info)?;
    Snapshot::serialize(&vm.snapshot, &mut into)?;
    vm.handler.serialize(&mut into)?;
    vm.return_value.serialize(&mut into)?;
    vm.exit_reason.serialize(&mut into)?;

    Ok(())
}

pub fn deserialize_vm_impl<T: Origin + Allocate, L: AccountLock + Context>(
    info: &AccountInfo,
    vm: &mut Vm<T, MachineIterative, L>,
) -> Result<()> {
    let mut bin: &[u8] = &Holder::from_account(info)?;

    vm.snapshot = Snapshot::deserialize(&mut bin)?;
    vm.handler = JournaledState::deserialize(&mut bin, vm.handler.state)?;
    vm.return_value = BorshDeserialize::deserialize(&mut bin)?;
    vm.exit_reason = BorshDeserialize::deserialize(&mut bin)?;
    Ok(())
}
pub fn bind_tx_to_holder_impl(info: &AccountInfo, tx_hash: H256) -> Result<()> {
    let mut state_holder = StateHolder::from_account_mut(info)?;
    state_holder.hash = tx_hash;
    Ok(())
}

pub fn is_tx_binded_to_holder_impl(info: &AccountInfo, tx_hash: H256) -> Result<bool> {
    let state_holder = StateHolder::from_account(info)?;
    Ok(state_holder.hash == tx_hash)
}
