pub mod spl_token;
pub mod aspl_token;
pub mod system;
pub mod non_evm_state;
pub mod system_ix;
pub mod aspl_token_ix;
pub mod spl_token_ix;
pub mod aux;
mod withdraw;

pub use {
    spl_token::SplToken,
    aspl_token::{ASplToken, spl_pda},
    aspl_token_ix::{Create,},
    system::System,
    system_ix::{CreateA, Allocate, Assign, Transfer,},
    withdraw::Withdraw,
    non_evm_state::{NonEvmState, Bind,},
    aux::{
        len_ge, len_eq, next, get_vec_slices, get_pubkey, get_account_mut,
    },
    evm::Context,
};

use {
    crate::{error::Result,  state::pda::Seed, Diff, H160},
    solana_program::{instruction::Instruction, },
};

pub type EvmDiff = (H160, Diff);

pub trait Program {
    fn ix_from_abi(&self, _input: &[u8], _context: &Context) -> Result<(Instruction, Seed, Vec<EvmDiff>)>;
    fn eth_call(&self, _: &[u8], _: &NonEvmState) -> Result<Vec<u8>>;
    fn emulate(&self, _ix: &Instruction, _: &mut Vec<Bind>) -> Result<()>;
    fn found_eth_call(&self, _: &[u8]) -> bool;
    fn transfer_allowed(&self) -> bool;
}
