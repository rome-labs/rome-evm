use {
    super::PrecompileResult, evm::H160,
    solana_bn254::prelude::alt_bn128_multiplication, solana_program::msg,
};

pub const ADDRESS: H160 = H160([
    0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7,
]);

pub fn contract(input: &[u8]) -> PrecompileResult {
    msg!("ecMul");
    alt_bn128_multiplication(input).unwrap()
}
