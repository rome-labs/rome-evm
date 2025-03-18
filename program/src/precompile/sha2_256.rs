use {evm::H160, solana_program::hash::hash, solana_program::msg, super::impl_contract,};

impl_contract!(Sha2, [0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,]);

#[must_use]
fn contract(input: &[u8]) -> Vec<u8> {
    msg!("sha2_256");

    Vec::from(hash(input).as_ref())
}
