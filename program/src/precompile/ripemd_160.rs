use {
    super::PrecompileResult,
    evm::H160,
    ripemd::{Digest, Ripemd160},
    solana_program::msg,
};

pub const ADDRESS: H160 = H160([
    0_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3,
]);

pub fn contract(input: &[u8]) -> PrecompileResult {
    msg!("ripemd_160");

    let mut hasher = Ripemd160::new();
    hasher.update(input);
    let hash = hasher.finalize();
    let mut result = vec![0_u8; 12];
    result.extend(&hash[..]);

    result
}
