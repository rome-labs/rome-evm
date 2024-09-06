mod snapshot;
#[allow(clippy::module_inception)]
mod vm;
pub mod vm_atomic;
#[cfg(not(target_os = "solana"))]
pub mod vm_eth_call;
pub mod vm_iterative;

pub use snapshot::*;
pub use vm::*;

use crate::error::Result;

pub trait Execute<T> {
    // updating the state of the vm
    fn advance(&mut self) -> Result<()>;

    // using the state of the vm, consume the input and return the result
    // Ok() if the vm successfully consumed the input
    // Err if the vm was in a state to consume the input, but the input was invalid
    fn consume(&mut self, a: T) -> Result<()>;
}

pub enum MachineEthCall {
    Init,
    Execute,
    Exit,
}
