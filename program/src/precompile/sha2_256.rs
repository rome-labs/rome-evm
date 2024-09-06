use {super::PrecompileResult, evm::H160, solana_program::hash::hash, solana_program::msg};

pub const ADDRESS: H160 = H160([
    0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
]);

#[must_use]
pub fn contract(input: &[u8]) -> PrecompileResult {
    msg!("sha2_256");

    PrecompileResult::from(hash(input).as_ref())
}
