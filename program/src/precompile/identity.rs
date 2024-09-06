use {super::PrecompileResult, evm::H160, solana_program::msg};

pub const ADDRESS: H160 = H160([
    0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4,
]);

#[must_use]
pub fn contract(input: &[u8]) -> PrecompileResult {
    msg!("identity");

    input.to_vec()
}
