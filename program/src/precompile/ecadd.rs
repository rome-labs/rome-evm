use {
    super::PrecompileResult, evm::H160, solana_bn254::prelude::alt_bn128_addition,
    solana_program::msg,
};

pub const ADDRESS: H160 = H160([
    0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6,
]);

pub fn contract(input: &[u8]) -> PrecompileResult {
    msg!("ecAdd");
    alt_bn128_addition(input).unwrap()
}
