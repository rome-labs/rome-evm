use {
    evm::H160, solana_bn254::prelude::alt_bn128_addition, solana_program::msg,
    super::impl_contract,
};

impl_contract!(Ecadd, [0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6,]);

fn contract(input: &[u8]) -> Vec<u8> {
    msg!("ecAdd");
    alt_bn128_addition(input).unwrap()
}
