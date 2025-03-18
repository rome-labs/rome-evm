use {evm::H160, solana_program::msg, super::impl_contract};

impl_contract!(Identity, [0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4,]);

#[must_use]
fn contract(input: &[u8]) -> Vec<u8> {
    msg!("identity");

    input.to_vec()
}
