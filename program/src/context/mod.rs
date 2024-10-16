pub mod account_lock;
pub mod atomic;
pub mod iterative;

pub use atomic::ContextAtomic;
pub use iterative::ContextIterative;

use {
    crate::{
        accounts::Iterations,
        accounts::{Data, SignerInfo},
        error::Result,
        state::{origin::Origin, Allocate, State},
        tx::tx::Tx,
        vm::{vm_iterative::MachineIterative, Vm},
    },
    account_lock::AccountLock,
    evm::{H160, H256},
};

pub trait Context {
    fn tx(&self) -> &Tx;
    fn save_iteration(&self, iteration: Iterations) -> Result<()>;
    fn restore_iteration(&self) -> Result<Iterations>;
    fn serialize_vm<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        vm: &Vm<T, MachineIterative, L>,
    ) -> Result<()>;
    fn deserialize_vm<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        vm: &mut Vm<T, MachineIterative, L>,
    ) -> Result<()>;
    fn allocate_holder(&self) -> Result<()>;
    fn bind_tx_to_holder(&self) -> Result<()>;
    fn is_tx_binded_to_holder(&self) -> Result<bool>;
    fn tx_hash(&self) -> H256;
    fn gas_recipient(&self) -> Result<Option<H160>>;
    fn check_nonce(&self) -> bool {
        true
    }
}

fn gas_recipient(state: &State) -> Result<Option<H160>> {
    let signer = state.signer()?;
    if let Ok(info) = state.info_signer_reg(signer.key, false) {
        let signer_info = SignerInfo::from_account(info)?;
        Ok(Some(signer_info.address))
    } else {
        Ok(None)
    }
}
