use {
    crate::state::State,
    rome_evm::{
        context::{
            iterative::{deserialize_impl, serialize_impl},
            Context,
        },
        error::{Result, RomeProgramError::*},
        state::{origin::Origin, Allocate},
        tx::{
            tx::Tx, legacy::Legacy,
        },
        vm::Vm,
        Data, Holder, Iterations, StateHolder, H160, H256,
        api::do_tx_holder::transmit_fee, SIG_VERIFY_COST,
    },
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey, keccak,},
    std::cell::RefCell,
    super::TRANSMIT_TX_SIZE,
};

pub enum Request<'a> {
    GasEstimate(Legacy),
    Rlp(&'a[u8]),
}

pub struct ContextIt<'a, 'b> {
    pub state: &'b State<'a>,
    pub holder: u64,
    pub tx_hash: H256,
    pub lock_overrides: RefCell<Vec<Pubkey>>,
    pub session: u64,
    pub fee_addr: Option<H160>,
    // pub rlp: &'b [u8],
    pub request: Request<'a>,
    pub with_tx_holder: bool,
}

impl<'a, 'b> ContextIt<'a, 'b> {
    pub fn new(
        state: &'b State<'a>,
        holder: u64,
        tx_hash: H256,
        session: u64,
        fee_addr: Option<H160>,
        rlp: &'a [u8],
        with_tx_holder: bool,
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
            request: Request::Rlp(rlp),
            with_tx_holder,
        })
    }

    pub fn new_gas_estimate(state: &'b State<'a>, legacy: Legacy) -> Result<Self> {
        let hash = H256::from(keccak::hash(&[1, 2, 3]).to_bytes());
        let holder = 0;
        let _state_holder = state.info_state_holder(holder, true)?;
        Ok(Self {
            state,
            holder,
            tx_hash: hash,
            lock_overrides: RefCell::new(vec![]),
            session: 1, // must not be equal to default value of the StateHolder.session
            fee_addr: None,
            request: Request::GasEstimate(legacy),
            with_tx_holder: true,
        })
    }
}

impl<'a, 'b> Context for ContextIt<'a, 'b> {
    fn tx(&self) -> Result<Tx> {
        match &self.request {
            Request::Rlp(rlp) => Tx::from_instruction(rlp),
            Request::GasEstimate(legacy) => Ok(Tx::from_legacy(legacy.clone()))
        }
    }
    fn set_iteration(&self, iteration: Iterations) -> Result<()> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        StateHolder::set_iteration(&info, iteration)?;
        self.state.update(bind);
        Ok(())
    }
    fn get_iteration(&self) -> Result<Iterations> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        StateHolder::get_iteration(&info)
    }
    fn serialize<T: Origin + Allocate>(&self, vm: &Vm<T>) -> Result<()> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        serialize_impl(&info, vm)?;
        self.state.update(bind);
        Ok(())
    }
    fn deserialize<T: Origin + Allocate>(&self, vm: &mut Vm<T>) -> Result<()> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        deserialize_impl(&info, vm)
    }
    fn allocate_holder(&self) -> Result<()> {
        let bind = self.state.info_state_holder(self.holder, false)?;
        let len = bind.1.data.len() + self.state.alloc_limit();
        self.state.realloc(&bind.0, len)?;
        Ok(())
    }

    fn new_session(&self) -> Result<()> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        StateHolder::set_session(&info, self.tx_hash, self.session)?;
        self.state.update(bind);

        if self.with_tx_holder {
            let fee = match &self.request {
                Request::GasEstimate(legacy) => {    // gas_estimate request
                    if let Some(data) = legacy.data.as_ref() {
                        let cnt = data.len() as u64 / TRANSMIT_TX_SIZE + 1;
                        SIG_VERIFY_COST.checked_mul(cnt).ok_or(CalculationOverflow)?
                    } else {
                        0
                    }
                },
                Request::Rlp(_) => {
                    let mut bind = self.state.info_tx_holder(self.holder, false)?;
                    let info = bind.into_account_info();
                    transmit_fee(&info)?
                }
            };
            
            self.collect_fees(fee, 0)?;
        }

        Ok(())
    }

    fn has_session(&self) -> Result<bool> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();
        StateHolder::has_session(&info, self.tx_hash, self.session)
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

    fn fees(&self) -> Result<(u64, u64)> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();
        StateHolder::fees(&info)
    }

    fn collect_fees(&self, lmp_fee: u64, lmp_refund: u64) -> Result<()> {
        let mut bind = self.state.info_state_holder(self.holder, false)?;
        let info = bind.into_account_info();

        StateHolder::collect_fees(&info, lmp_fee, lmp_refund)?;
        self.state.update(bind);
        Ok(())
    }
    fn is_gas_estimate(&self) -> bool {
        match self.request {
            Request::Rlp(_) => false,
            _ => true
        }
    }

}
