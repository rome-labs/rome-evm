use {
    super::{account_lock::AccountLock, Context},
    crate::{
        accounts::Iterations,
        accounts::{Data, Holder, StateHolder},
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
    pub rlp: &'b [u8],
    pub session: u64,
    pub fee_addr: Option<H160>,
}

impl<'a, 'b> ContextIterative<'a, 'b> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: &'b State<'a>,
        accounts: &'a [AccountInfo<'a>],
        holder: u64,
        lock_overrides: &'a [u8],
        rlp: &'b [u8],
        tx_hash: H256,
        session: u64,
        fee_addr: Option<H160>,
    ) -> Result<Self> {
        let state_holder = state.info_state_holder(holder, true)?;

        Ok(Self {
            state,
            origin_accounts: accounts,
            lock_overrides,
            state_holder,
            tx_hash,
            rlp,
            session,
            fee_addr,
        })
    }
}

impl<'a, 'b> Context for ContextIterative<'a, 'b> {
    fn tx(&self) -> Result<Tx> {
        Tx::from_instruction(self.rlp)
    }

    fn save_iteration(&self, iteration: Iterations) -> Result<()> {
        StateHolder::set_iteration(self.state_holder, iteration)
    }

    fn restore_iteration(&self) -> Result<Iterations> {
        StateHolder::get_iteration(self.state_holder)
    }

    fn serialize<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        vm: &Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        serialize_impl(self.state_holder, vm, self.state)
    }

    fn deserialize<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        vm: &mut Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        deserialize_impl(self.state_holder, vm, self.state)
    }

    fn allocate_holder(&self) -> Result<()> {
        let len = self.state_holder.data_len() + self.state.alloc_limit();
        self.state.realloc(self.state_holder, len)
    }

    fn new_session(&self) -> Result<()> {
        StateHolder::set_link(self.state_holder, self.tx_hash, self.session)
    }

    fn exists_session(&self) -> Result<bool> {
        StateHolder::is_linked(self.state_holder, self.tx_hash, self.session)
    }

    fn tx_hash(&self) -> H256 {
        self.tx_hash
    }

    fn fee_recipient(&self) -> Option<H160> {
        self.fee_addr
    }

    fn state_holder_len(&self) -> Result<usize> {
        Ok(Holder::size(self.state_holder))
    }
}

// these functions are used both in the contract and in the emulator
pub fn serialize_impl<T: Origin + Allocate, B: Origin, L: AccountLock + Context>(
    info: &AccountInfo,
    vm: &Vm<T, MachineIterative, L>,
    state: &B,
) -> Result<()> {
    let mut into: &mut [u8] = &mut Holder::from_account_mut(info)?;
    Snapshot::serialize(&vm.snapshot, &mut into)?;
    vm.handler.serialize(&mut into)?;
    vm.return_value.serialize(&mut into)?;
    vm.exit_reason.serialize(&mut into)?;
    state.base().pda.serialize(&mut into)
}

pub fn deserialize_impl<T: Origin + Allocate, B: Origin, L: AccountLock + Context>(
    info: &AccountInfo,
    vm: &mut Vm<T, MachineIterative, L>,
    state: &B,
) -> Result<()> {
    let mut bin: &[u8] = &Holder::from_account(info)?;

    vm.snapshot = Snapshot::deserialize(&mut bin)?;
    vm.handler = JournaledState::deserialize(&mut bin, vm.handler.state)?;
    vm.return_value = BorshDeserialize::deserialize(&mut bin)?;
    vm.exit_reason = BorshDeserialize::deserialize(&mut bin)?;
    state.base().pda.deserialize(&mut bin)
}
