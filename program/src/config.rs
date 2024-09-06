use std::env;

const fn parse_u64(s: &str) -> u64 {
    let mut bytes = s.as_bytes();
    let mut val: u64 = 0;
    while let [byte, rest @ ..] = bytes {
        assert!(b'0' <= *byte && *byte <= b'9', "invalid digit");
        val = val * 10 + (*byte - b'0') as u64;
        bytes = rest;
    }

    val
}

/// Values defined during compilation
pub const CHAIN_ID: u64 = parse_u64(env!("CHAIN_ID"));
pub const CONTRACT_OWNER: &str = env!("CONTRACT_OWNER");

/// Unchangeable values
pub const ACCOUNT_SEED: &[u8] = b"ACCOUN_SEED";
pub const EVENT_LOG: &[u8] = b"EVENT_LOG";
pub const EXIT_REASON: &[u8] = b"EXIT_REASON";
pub const REVERT_PANIC: &[u8] = &[0x4e, 0x48, 0x7b, 0x71];
pub const REVERT_ERROR: &[u8] = &[0x08, 0xc3, 0x79, 0xa0]; // Signature for "Error(string)"
pub const LOCK_DURATION: i64 = 2; // each iteration blocks accounts for this number of blocks
pub const RO_LOCK_SEED: &[u8] = b"RO_ACCOUNT_LOCK";
pub const TX_HOLDER_SEED: &[u8] = b"TX_HOLDER_SEED";
pub const STATE_HOLDER_SEED: &[u8] = b"STATE_HOLDER_SEED";
pub const NUMBER_OPCODES_PER_TX: u64 = 500;
pub const SIG_VERIFY_COST: u64 = 5000;
pub const SIGNER_INFO: &[u8] = b"SIGNER_INFO";
pub const GAS_VALUE: &[u8] = b"GAS_VALUE";
pub const GAS_RECIPIENT: &[u8] = b"GAS_RECIPIENT";
