use {
    evm::H160, solana_bn254::prelude::alt_bn128_pairing, solana_program::msg,
    super::impl_contract,
};

impl_contract!(Ecpairing, [0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8,]);

fn contract(input: &[u8]) -> Vec<u8> {
    msg!("ecPairing");
    alt_bn128_pairing(input).unwrap()
}
