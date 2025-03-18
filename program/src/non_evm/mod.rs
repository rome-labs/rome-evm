pub mod spl_token;
pub mod aspl_token;
pub mod system;
pub mod non_evm_state;
pub mod system_ix;
pub mod aspl_token_ix;
pub mod spl_token_ix;
pub mod aux;

pub use {
    spl_token::SplToken,
    aspl_token::ASplToken,
    aspl_token_ix::{Create,},
    system::System,
    system_ix::{CreateA, Allocate, Assign, Transfer, find_pda,},
    non_evm_state::{NonEvmState, Bind,},
    aux::{
        len_ge, len_eq, next, accounts_mut, get_vec_slices, get_pubkey,
    },
};
#[cfg(not(target_os = "solana"))]
pub use aux::dispatcher;

use {
    crate::{error::Result,  state::pda::Seed, H160, },
    solana_program::{instruction::Instruction, },
};

pub trait Program {
    fn ix_from_abi(&self, _input: &[u8], _caller: H160) -> Result<(Instruction, Seed)> {
        unimplemented!()
    }
    fn eth_call(&self, input: &[u8]) -> Result<Vec<u8>>;

    fn emulate(&self, _ix: &Instruction, _accs: Vec<Bind>) -> Result<Vec<u8>> {
        unimplemented!()
    }
    fn found_eth_call(&self, _: &[u8]) -> bool {
        true
    }
}
