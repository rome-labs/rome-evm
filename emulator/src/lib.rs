// pub mod emulator;
mod allocate;
#[allow(clippy::let_and_return)]
pub mod api;
mod context;
pub mod entrypoint;
mod origin;
mod state;
mod stubs;
mod vm_eth_call;

pub use api::*;
pub use context::*;
pub use state::{Bind, Item};
pub use vm_eth_call::*;

entrypoint! {
    DoTx => do_tx,
    Deposit => deposit,
    TransmitTx => transmit_tx,
    DoTxHolder => do_tx_holder,
    DoTxIterative => do_tx_iterative,
    DoTxHolderIterative => do_tx_holder_iterative,
    RegOwner => reg_owner,
}
