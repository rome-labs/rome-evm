// pub mod emulator;
mod allocate;
#[allow(clippy::let_and_return)]
pub mod api;
mod context;
pub mod entrypoint;
mod origin;
mod state;
mod stubs;

pub use api::*;
pub use context::*;
pub use state::{Bind, Item};

entrypoint! {
    DoTx => do_tx,
    CreateBalance => create_balance,
    TransmitTx => transmit_tx,
    DoTxHolder => do_tx_holder,
    DoTxIterative => do_tx_iterative,
    DoTxHolderIterative => do_tx_holder_iterative,
    RegOwner => reg_owner,
}
