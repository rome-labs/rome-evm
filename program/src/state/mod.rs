pub mod allocate;
pub mod aux;
pub mod base;
pub mod handler;
pub mod info;
mod journal;
mod journaled_state;
pub mod origin;
pub mod pda;
#[allow(clippy::module_inception)]
mod state;

pub use allocate::*;
pub use aux::Account;
pub use base::*;
pub use journal::*;
pub use journaled_state::*;
pub use state::*;

pub fn precompiled_contract(address: evm::H160) -> bool {
    let address = address.0;

    for i in &address[1..] {
        if *i != 0 {
            return false;
        }
    }
    if address[0] >= 0x01 && address[0] <= 0x09 {
        return true;
    }

    false
}
