pub const ACCOUNT_SEED: &[u8] = b"ACCOUN_SEED";
pub const EVENT_LOG: &[u8] = b"EVENT_LOG";
pub const EXIT_REASON: &[u8] = b"EXIT_REASON";
pub const REVERT_PANIC: &[u8] = &[0x4e, 0x48, 0x7b, 0x71];
pub const REVERT_ERROR: &[u8] = &[0x08, 0xc3, 0x79, 0xa0]; // Signature for "Error(string)"
pub const LOCK_DURATION: i64 = 3; // each iteration blocks accounts for this number of blocks
pub const RO_LOCK_SEED: &[u8] = b"RO_ACCOUNT_LOCK";
pub const TX_HOLDER_SEED: &[u8] = b"TX_HOLDER_SEED";
pub const STATE_HOLDER_SEED: &[u8] = b"STATE_HOLDER_SEED";
pub const NUMBER_OPCODES_PER_TX: u64 = 500;
pub const SIG_VERIFY_COST: u64 = 5000;
pub const GAS_VALUE: &[u8] = b"GAS_VALUE";
pub const GAS_PRICE: &[u8] = b"GAS_PRICE";
pub const GAS_RECIPIENT: &[u8] = b"GAS_RECIPIENT";
pub const OWNER_INFO: &[u8] = b"OWNER_INFO";
pub const NUMBER_ALLOC_DIFF_PER_TX: u64 = 10; // mut be <= 64  (max_instruction_trace_length)
pub const STORAGE_LEN: usize = 256; // must be <= u8::MAX+1
pub const CONTRACT_SOL_WALLET: &[u8] = b"CONTRACT_SOL_WALLET";
pub const RSOL_DECIMALS: usize = 18;

#[cfg(feature = "ci")]
pub mod registration_key {
    solana_program::declare_id!("9u1wj9K3o9KiDFEppSshGdNhopPBKMbzHgtJr5BMhVjD");
}
#[cfg(feature = "mainnet")]
pub mod registration_key {
    solana_program::declare_id!("RRAM82VtsXY2BTAgQU9v8JqgrcoVMAbCPBqRRH5sV2a");
}
#[cfg(feature = "testnet")]
pub mod registration_key {
    solana_program::declare_id!("RRATMVCuWw4XpjyvGdELvCTcXdMMJQWzL1heAGXEpxq");
}
