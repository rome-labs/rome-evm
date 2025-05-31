pub mod atomic;
pub mod iterative;
pub mod iterative_lock;

pub use atomic::ContextAt;
pub use iterative::ContextIt;

use {
    crate::{
        accounts::Iterations,
        error::Result,
        state::{origin::Origin, Allocate},
        tx::tx::Tx,
        vm::Vm,
    },
    evm::{H160, H256},
    solana_program::account_info::AccountInfo,
};

pub trait Context {
    fn tx(&self) -> Result<Tx>;
    fn set_iteration(&self, iteration: Iterations) -> Result<()>;
    fn get_iteration(&self) -> Result<Iterations>;
    fn serialize<T: Origin + Allocate>(&self, vm: &Vm<T>) -> Result<()>;
    fn deserialize<T: Origin + Allocate>(&self, vm: &mut Vm<T>) -> Result<()>;
    fn allocate_holder(&self) -> Result<()>;
    fn new_session(&self) -> Result<()>;
    fn has_session(&self) -> Result<bool>;
    fn tx_hash(&self) -> H256;
    fn fee_recipient(&self) -> Option<H160>;
    fn is_gas_estimate(&self) -> bool;
    fn state_holder_len(&self) -> Result<usize>;
    fn collect_fees(&self, lamports_fee: u64, lamports_refund: u64) -> Result<()>;
    fn fees(&self) -> Result<(u64, u64)>;
}

pub trait AccountLock {
    fn lock(&self) -> Result<()>;   // TODO: replace &self to &mut self, remove RefCell from impl
    fn locked(&self) -> Result<bool>;
    fn unlock(&self) -> Result<()>;
    fn lock_new_one(&self, info: &AccountInfo) -> Result<()>;
    fn check_writable(&self, info: &AccountInfo) -> Result<()>;
}
