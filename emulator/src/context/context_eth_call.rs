use rome_evm::{
    accounts::Iterations,
    context::{account_lock::AccountLock, Context},
    error::Result,
    state::{origin::Origin, Allocate},
    tx::{legacy::Legacy, tx::Tx},
    vm::{vm_iterative::MachineIterative, Vm},
    H160, H256,
};

pub struct ContextEthCall {
    pub legacy: Legacy,
}
impl ContextEthCall {
    pub fn new(legacy: Legacy) -> Self {
        Self { legacy }
    }
}

impl Context for ContextEthCall {
    fn tx(&self) -> Result<Tx> {
        let tx = Tx::from_legacy(self.legacy.clone());
        Ok(tx)
    }
    fn save_iteration(&self, _: Iterations) -> Result<()> {
        unreachable!()
    }
    fn restore_iteration(&self) -> Result<Iterations> {
        unreachable!()
    }
    fn serialize_vm<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        _: &Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        unreachable!()
    }
    fn deserialize_vm<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        _: &mut Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        unreachable!()
    }
    fn allocate_holder(&self) -> Result<()> {
        unreachable!()
    }
    fn bind_tx_to_holder(&self) -> Result<()> {
        unreachable!()
    }
    fn is_tx_binded_to_holder(&self) -> Result<bool> {
        unreachable!()
    }
    fn tx_hash(&self) -> H256 {
        unreachable!()
    }
    fn gas_recipient(&self) -> Result<Option<H160>> {
        Ok(None)
    }
    fn check_nonce(&self) -> bool {
        false
    }
}

impl AccountLock for ContextEthCall {
    fn lock(&self) -> Result<()> {
        unreachable!()
    }
    fn locked(&self) -> Result<bool> {
        unreachable!()
    }
    fn unlock(&self) -> Result<()> {
        unreachable!()
    }
    fn lock_new_one(&self) -> Result<()> {
        unreachable!()
    }
}
