mod blake2f;
mod ecadd;
mod ecmul;
mod ecpairing;
mod ecrecover;
mod identity;
// mod modexp;
mod ripemd_160;
mod sha2_256;

use {
    blake2f::*, ecadd::*, ecmul::*, ecpairing::*, ecrecover::*, identity::*, ripemd_160::*, sha2_256::*,
    evm::H160,
    crate::{
        non_evm::{Program, SplToken, ASplToken, System, Withdraw,},
        origin::Origin,
    },
};

pub fn non_evm_program<'a, T:Origin>(address: &H160, state: &'a T) -> Option<Box<dyn Program + 'a>>  {
    // TODO: check on-chain and enable
    assert_ne!(
        *address,
        H160([0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5]),
        "big_mod_exp call is disabled"
    );

    match *address {
        _ if *address == Ecrecover::ADDRESS  => Some(Box::new(Ecrecover())),
        _ if *address == Sha2::ADDRESS => Some(Box::new(Sha2())),
        _ if *address == Ripemd::ADDRESS => Some(Box::new(Ripemd())),
        _ if *address == Identity::ADDRESS => Some(Box::new(Identity())),
        // _ if *address == Modexp::ADDRESS => Some(Box::new(Modexp()),
        _ if *address == Ecadd::ADDRESS => Some(Box::new(Ecadd())),
        _ if *address == Ecmul::ADDRESS => Some(Box::new(Ecmul())),
        _ if *address == Ecpairing::ADDRESS => Some(Box::new(Ecpairing())),
        _ if *address == Blake2f::ADDRESS => Some(Box::new(Blake2f())),

        _ if *address == SplToken::<'a, T>::ADDRESS => Some(Box::new(SplToken::new(state))),
        _ if *address == ASplToken::<'a, T>::ADDRESS => Some(Box::new(ASplToken::new(state))),
        _ if *address == System::<'a, T>::ADDRESS => Some(Box::new(System::new(state))),

        _ if *address == Withdraw::<'a, T>::ADDRESS => Some(Box::new(Withdraw::new(state))),
        _ => None
    }
}

macro_rules! impl_contract {
    ($name:ident, $address:expr) => {
        use {
            crate::{
                non_evm::{Program, NonEvmState, Bind, EvmDiff,}, state::pda::Seed, 
                error::{Result, RomeProgramError::Unimplemented},
            },
            solana_program::instruction::Instruction,
        };

        pub struct $name();

        impl $name {
            pub const ADDRESS: H160 = H160($address);
        }

        impl Program for $name {
            fn eth_call(&self, input: &[u8], _: &NonEvmState) -> Result<Vec<u8>> {
                Ok(contract(input))
            }
            fn found_eth_call(&self, _: &[u8]) -> bool {
                true
            }
            fn ix_from_abi(&self, _: &[u8], _: &evm::Context) -> Result<(Instruction, Seed, Vec<EvmDiff>)> {
                Err(Unimplemented("method is not supported by precompile contract".to_string()))
            }
            fn emulate(&self, _: &Instruction, _: &mut Vec<Bind>) -> Result<()> {
                unimplemented!()
            }
            fn transfer_allowed(&self) -> bool {
                false
            }
        }
    };
}

pub(crate) use  impl_contract;
