use {
    evm::H160, solana_bn254::prelude::alt_bn128_multiplication, solana_program::msg,
    super::impl_contract,
};

impl_contract!(Ecmul, [0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7,]);

fn contract(input: &[u8]) -> Vec<u8> {
    msg!("ecMul");
    alt_bn128_multiplication(input).unwrap()
}
