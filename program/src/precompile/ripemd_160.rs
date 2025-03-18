use {
    evm::H160,
    ripemd::{Digest, Ripemd160},
    solana_program::msg,
    super::impl_contract,
};

impl_contract!(Ripemd, [0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3,]);

fn contract(input: &[u8]) -> Vec<u8> {
    msg!("ripemd_160");

    let mut hasher = Ripemd160::new();
    hasher.update(input);
    let hash = hasher.finalize();
    let mut result = vec![0_u8; 12];
    result.extend(&hash[..]);

    result
}
