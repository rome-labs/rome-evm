mod snapshot;
#[allow(clippy::module_inception)]
mod vm;
pub mod vm_atomic;
pub mod vm_iterative;

pub use snapshot::*;
pub use vm::*;
pub use vm_atomic::*;
pub use vm_iterative::*;

use crate::error::Result;

pub trait Execute<T> {
    // updating the state of the vm
    fn advance(&mut self) -> Result<()>;

    // using the state of the vm, consume the input and return the result
    // Ok() if the vm successfully consumed the input
    // Err if the vm was in a state to consume the input, but the input was invalid
    fn consume(&mut self, a: T) -> Result<()>;
}

