use {
    rome_evm::{
        accounts::Iterations,
        context::{account_lock::AccountLock, Context},
        error::Result,
        state::{origin::Origin, Allocate},
        tx::tx::Tx,
        vm::{vm_iterative::MachineIterative, Vm},
        H160, H256,
    },
    solana_program::account_info::AccountInfo,
};

pub struct ContextEthCall {
    pub tx: Tx,
}
impl ContextEthCall {
    pub fn new(tx: Tx) -> Self {
        Self { tx }
    }
}

impl Context for ContextEthCall {
    fn tx(&self) -> &Tx {
        &self.tx
    }
    fn save_iteration(&self, _: Iterations) -> Result<()> {
        unreachable!()
    }
    fn restore_iteration(&self) -> Result<Iterations> {
        unreachable!()
    }
    fn serialize<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        _: &Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        unreachable!()
    }
    fn deserialize<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        _: &mut Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        unreachable!()
    }
    fn allocate_holder(&self) -> Result<()> {
        unreachable!()
    }
    fn new_session(&self) -> Result<()> {
        unreachable!()
    }
    fn exists_session(&self) -> Result<bool> {
        unreachable!()
    }
    fn tx_hash(&self) -> H256 {
        unreachable!()
    }
    fn fee_recipient(&self) -> Option<H160> {
        None
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
    fn lock_new_one(&self, _info: &AccountInfo) -> Result<()> {
        unreachable!()
    }
    fn check_writable(&self, _info: &AccountInfo) -> Result<()> {
        unreachable!()
    }
}
