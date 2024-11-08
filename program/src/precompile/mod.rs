mod blake2f;
mod ecadd;
mod ecmul;
mod ecpairing;
mod ecrecover;
mod identity;
// mod modexp;
mod ripemd_160;
mod sha2_256;

use evm::H160;

pub type PrecompileResult = Vec<u8>;
pub type PrecompileFn = fn(&[u8]) -> PrecompileResult;

pub fn built_in_contract(address: &H160) -> Option<PrecompileFn> {
    let f = if *address == ecrecover::ADDRESS {
        ecrecover::contract
    } else if *address == sha2_256::ADDRESS {
        sha2_256::contract
    } else if *address == ripemd_160::ADDRESS {
        ripemd_160::contract
    } else if *address == identity::ADDRESS {
        identity::contract
    // } else if *address == modexp::ADDRESS {
    //     modexp::contract
    } else if *address == ecadd::ADDRESS {
        ecadd::contract
    } else if *address == ecmul::ADDRESS {
        ecmul::contract
    } else if *address == ecpairing::ADDRESS {
        ecpairing::contract
    } else if *address == blake2f::ADDRESS {
        blake2f::contract
    } else {
        return None;
    };

    Some(f)
}
