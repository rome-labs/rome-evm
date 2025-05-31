use {
    super::Context,
    crate::{
        accounts::Iterations,
        accounts::{Data, Holder, StateHolder},
        error::Result,
        state::{origin::Origin, Allocate, JournaledState},
        tx::tx::Tx,
        vm::{Snapshot, Vm},
        State, do_tx_holder::transmit_fee,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    evm::{H160, H256},
    solana_program::account_info::AccountInfo,
};

pub struct ContextIt<'a, 'b> {
    pub state: &'b State<'a>,
    pub origin_accounts: &'a [AccountInfo<'a>],
    pub lock_overrides: &'a [u8],
    pub state_holder: &'a AccountInfo<'a>,
    pub tx_hash: H256,
    pub rlp: &'b [u8],
    pub session: u64,
    pub fee_addr: Option<H160>,
    pub tx_holder: Option<&'a AccountInfo<'a>>,
}

impl<'a, 'b> ContextIt<'a, 'b> {
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
        tx_holder: Option<&'a AccountInfo<'a>>,
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
            tx_holder
        })
    }
}

impl<'a, 'b> Context for ContextIt<'a, 'b> {
    fn tx(&self) -> Result<Tx> {
        Tx::from_instruction(self.rlp)
    }

    fn set_iteration(&self, iteration: Iterations) -> Result<()> {
        StateHolder::set_iteration(self.state_holder, iteration)
    }

    fn get_iteration(&self) -> Result<Iterations> {
        StateHolder::get_iteration(self.state_holder)
    }

    fn serialize<T: Origin + Allocate>(&self, vm: &Vm<T>) -> Result<()> {
        serialize_impl(self.state_holder, vm)
    }

    fn deserialize<T: Origin + Allocate>(&self, vm: &mut Vm<T>) -> Result<()> {
        deserialize_impl(self.state_holder, vm)
    }

    fn allocate_holder(&self) -> Result<()> {
        let len = self.state_holder.data_len() + self.state.alloc_limit();
        self.state.realloc(self.state_holder, len)
    }

    fn new_session(&self) -> Result<()> {
        StateHolder::set_session(self.state_holder, self.tx_hash, self.session)?;

        if let Some(info) = self.tx_holder {
            let fee = transmit_fee(info)?;
            self.collect_fees(fee, 0)?;
        }
        
        Ok(())
    }

    fn has_session(&self) -> Result<bool> {
        StateHolder::has_session(self.state_holder, self.tx_hash, self.session)
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

    fn collect_fees(&self, lamports_fee: u64, lamports_refund: u64) -> Result<()> {
        StateHolder::collect_fees(self.state_holder, lamports_fee, lamports_refund)
    }

    fn fees(&self) -> Result<(u64, u64)> {
        StateHolder::fees(self.state_holder)
    }

    fn is_gas_estimate(&self) -> bool {
        false
    }
}

// these functions are used both in the contract and in the emulator
pub fn serialize_impl<T: Origin + Allocate>(info: &AccountInfo, vm: &Vm<T>) -> Result<()> {
    let mut into: &mut [u8] = &mut Holder::from_account_mut(info)?;
    Snapshot::serialize(&vm.snapshot, &mut into)?;
    vm.handler.serialize(&mut into)?;
    vm.return_value.serialize(&mut into)?;
    vm.exit_reason.serialize(&mut into)?;
    vm.handler.state.base().pda.serialize(&mut into)
}

pub fn deserialize_impl<T: Origin + Allocate>(info: &AccountInfo, vm: &mut Vm<T>) -> Result<()> {
    let mut bin: &[u8] = &Holder::from_account(info)?;

    vm.snapshot = Snapshot::deserialize(&mut bin)?;
    vm.handler = JournaledState::deserialize(&mut bin, vm.handler.state)?;
    vm.return_value = BorshDeserialize::deserialize(&mut bin)?;
    vm.exit_reason = BorshDeserialize::deserialize(&mut bin)?;
    vm.handler.state.base().pda.deserialize(&mut bin)
}
