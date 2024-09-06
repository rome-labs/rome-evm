use {
    super::PrecompileResult, evm::H160, solana_program::alt_bn128::prelude::alt_bn128_pairing,
    solana_program::msg,
};

pub const ADDRESS: H160 = H160([
    0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8,
]);

pub fn contract(input: &[u8]) -> PrecompileResult {
    msg!("ecPairing");
    alt_bn128_pairing(input).unwrap()
}
