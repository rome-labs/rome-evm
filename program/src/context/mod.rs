pub mod account_lock;
pub mod atomic;
pub mod iterative;

pub use atomic::ContextAtomic;
pub use iterative::ContextIterative;

use {
    crate::{
        accounts::Iterations,
        accounts::{Data, Holder, SignerInfo, TxHolder},
        error::Result,
        state::{origin::Origin, Allocate, State},
        tx::tx::Tx,
        vm::{vm_iterative::MachineIterative, Vm},
    },
    account_lock::AccountLock,
    evm::{H160, H256},
    solana_program::account_info::AccountInfo,
};

pub trait Context {
    fn tx(&self) -> Result<Tx>;
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
}

pub fn tx_from_holder(info: &AccountInfo, hash: H256) -> Result<Tx> {
    let holder = TxHolder::from_account(info)?;
    holder.check_hash(info, hash)?;
    let rlp = Holder::from_account(info)?;
    Tx::from_instruction(&rlp)
}

fn gas_recipient(state: &State) -> Result<Option<H160>> {
    let signer = state.signer()?;
    if let Ok(info) = state.info_signer_info(signer.key, false) {
        let signer_info = SignerInfo::from_account(info)?;
        Ok(Some(signer_info.address))
    } else {
        Ok(None)
    }
}
