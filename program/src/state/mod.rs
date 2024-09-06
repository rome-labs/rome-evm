pub mod allocate;
pub mod handler;
pub mod info;
mod journal;
mod journaled_state;
pub mod origin;
#[allow(clippy::module_inception)]
mod state;

pub use allocate::*;
pub use journal::*;
pub use journaled_state::*;
pub use state::*;

use {
    crate::{
        ACCOUNT_SEED, ADDRESS_TABLE_SIZE, RO_LOCK_SEED, SIGNER_INFO, STATE_HOLDER_SEED,
        TX_HOLDER_SEED,
    },
    evm::{H160, U256},
    solana_program::pubkey::Pubkey,
};

#[derive(Default)]
pub struct Seed {
    pub items: Vec<Vec<u8>>,
}
impl Seed {
    pub fn cast(&self) -> Vec<&[u8]> {
        self.items
            .iter()
            .map(|a| a.as_slice())
            .collect::<Vec<&[u8]>>()
    }
    pub fn add(&mut self, bump_seed: u8) {
        self.items.push(vec![bump_seed]);
    }
}

pub fn pda_balance(address: &H160, program_id: &Pubkey) -> (Pubkey, Seed) {
    let mut seed = Seed {
        items: vec![ACCOUNT_SEED.to_vec(), address.as_bytes().to_vec()],
    };
    let (key, bump_seed) = Pubkey::find_program_address(seed.cast().as_slice(), program_id);
    seed.add(bump_seed);
    (key, seed)
}

pub fn pda_tx_holder(base: &Pubkey, index: u64, program_id: &Pubkey) -> (Pubkey, Seed) {
    let mut seed = Seed {
        items: vec![
            TX_HOLDER_SEED.to_vec(),
            base.as_ref().to_vec(),
            index.to_le_bytes().to_vec(),
        ],
    };
    let (key, bump_seed) = Pubkey::find_program_address(seed.cast().as_slice(), program_id);
    seed.add(bump_seed);
    (key, seed)
}

pub fn pda_state_holder(base: &Pubkey, index: u64, program_id: &Pubkey) -> (Pubkey, Seed) {
    let mut seed = Seed {
        items: vec![
            STATE_HOLDER_SEED.to_vec(),
            base.as_ref().to_vec(),
            index.to_le_bytes().to_vec(),
        ],
    };
    let (key, bump_seed) = Pubkey::find_program_address(seed.cast().as_slice(), program_id);
    seed.add(bump_seed);
    (key, seed)
}

pub fn storage_index(slot: &U256) -> ([u8; 32], usize) {
    let (index, sub_index) = slot.div_mod(ADDRESS_TABLE_SIZE.into());
    let mut index_be = [0_u8; 32];
    index.to_big_endian(&mut index_be);

    (index_be, sub_index.as_usize())
}

pub fn pda_storage(base: &Pubkey, index_be: [u8; 32], program_id: &Pubkey) -> (Pubkey, Seed) {
    let mut seed = Seed {
        items: vec![base.as_ref().to_vec(), index_be.to_vec()],
    };
    let (key, bump_seed) = Pubkey::find_program_address(seed.cast().as_slice(), program_id);
    seed.add(bump_seed);
    (key, seed)
}

pub fn pda_ro_lock(key: &Pubkey, program_id: &Pubkey) -> (Pubkey, Seed) {
    pda_from_key(key, program_id, RO_LOCK_SEED)
}

pub fn pda_signer_info(key: &Pubkey, program_id: &Pubkey) -> (Pubkey, Seed) {
    pda_from_key(key, program_id, SIGNER_INFO)
}

fn pda_from_key(key: &Pubkey, program_id: &Pubkey, seed_: &[u8]) -> (Pubkey, Seed) {
    let mut seed = Seed {
        items: vec![seed_.to_vec(), key.as_ref().to_vec()],
    };
    let (key, bump_seed) = Pubkey::find_program_address(seed.cast().as_slice(), program_id);
    seed.add(bump_seed);
    (key, seed)
}

pub fn precompiled_contract(address: evm::H160) -> bool {
    let address = address.0;

    for i in &address[1..] {
        if *i != 0 {
            return false;
        }
    }
    if address[0] >= 0x01 && address[0] <= 0x09 {
        return true;
    }

    false
}
