pub mod account_lock;
pub mod atomic;
pub mod iterative;

pub use atomic::ContextAtomic;
pub use iterative::ContextIterative;

use {
    crate::{
        accounts::Iterations,
        error::Result,
        state::{origin::Origin, Allocate},
        tx::tx::Tx,
        vm::{vm_iterative::MachineIterative, Vm},
    },
    account_lock::AccountLock,
    evm::{H160, H256},
};

pub trait Context {
    fn tx(&self) -> Result<Tx>;
    fn save_iteration(&self, iteration: Iterations) -> Result<()>;
    fn restore_iteration(&self) -> Result<Iterations>;
    fn serialize<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        vm: &Vm<T, MachineIterative, L>,
    ) -> Result<()>;
    fn deserialize<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        vm: &mut Vm<T, MachineIterative, L>,
    ) -> Result<()>;
    fn allocate_holder(&self) -> Result<()>;
    fn new_session(&self) -> Result<()>;
    fn exists_session(&self) -> Result<bool>;
    fn tx_hash(&self) -> H256;
    fn fee_recipient(&self) -> Option<H160>;
    fn check_nonce(&self) -> bool {
        true
    }
    fn state_holder_len(&self) -> Result<usize>;
}
