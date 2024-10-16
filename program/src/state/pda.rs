use {
    crate::{
        error::Result, upgrade_authority, AccountState, AccountType, AddressTable, Data, OwnerInfo,
        RoLock, SignerInfo, StateHolder, TxHolder, ACCOUNT_SEED, ADDRESS_TABLE_SIZE, OWNER_INFO,
        RO_LOCK_SEED, SIGNER_INFO, STATE_HOLDER_SEED, TX_HOLDER_SEED,
    },
    evm::{H160, U256},
    solana_program::{account_info::AccountInfo, pubkey::Pubkey},
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

pub struct Pda<'a> {
    chain: Vec<u8>,
    program_id: &'a Pubkey,
}

impl<'a> Pda<'a> {
    pub fn new(program_id: &'a Pubkey, chain: u64) -> Self {
        Self {
            chain: chain.to_le_bytes().to_vec(),
            program_id,
        }
    }

    pub fn balance_key(&self, address: &H160) -> (Pubkey, Seed) {
        let mut seed = Seed {
            items: vec![
                self.chain.clone(),
                ACCOUNT_SEED.to_vec(),
                address.as_bytes().to_vec(),
            ],
        };
        let (key, bump_seed) =
            Pubkey::find_program_address(seed.cast().as_slice(), self.program_id);
        seed.add(bump_seed);
        (key, seed)
    }

    pub fn tx_holder_key(&self, base: &Pubkey, index: u64) -> (Pubkey, Seed) {
        let mut seed = Seed {
            items: vec![
                self.chain.clone(),
                TX_HOLDER_SEED.to_vec(),
                base.as_ref().to_vec(),
                index.to_le_bytes().to_vec(),
            ],
        };
        let (key, bump_seed) =
            Pubkey::find_program_address(seed.cast().as_slice(), self.program_id);
        seed.add(bump_seed);
        (key, seed)
    }

    pub fn state_holder_key(&self, base: &Pubkey, index: u64) -> (Pubkey, Seed) {
        let mut seed = Seed {
            items: vec![
                self.chain.clone(),
                STATE_HOLDER_SEED.to_vec(),
                base.as_ref().to_vec(),
                index.to_le_bytes().to_vec(),
            ],
        };
        let (key, bump_seed) =
            Pubkey::find_program_address(seed.cast().as_slice(), self.program_id);
        seed.add(bump_seed);
        (key, seed)
    }

    pub fn storage_index(slot: &U256) -> ([u8; 32], usize) {
        let (index, sub_index) = slot.div_mod(ADDRESS_TABLE_SIZE.into());
        let mut index_be = [0_u8; 32];
        index.to_big_endian(&mut index_be);

        (index_be, sub_index.as_usize())
    }

    pub fn storage_key(&self, base: &Pubkey, index_be: [u8; 32]) -> (Pubkey, Seed) {
        let mut seed = Seed {
            items: vec![
                self.chain.clone(),
                base.as_ref().to_vec(),
                index_be.to_vec(),
            ],
        };
        let (key, bump_seed) =
            Pubkey::find_program_address(seed.cast().as_slice(), self.program_id);
        seed.add(bump_seed);
        (key, seed)
    }

    pub fn ro_lock_key(&self, key: &Pubkey) -> (Pubkey, Seed) {
        self.pda_from_key(key, RO_LOCK_SEED)
    }

    pub fn signer_info_key(&self, key: &Pubkey) -> (Pubkey, Seed) {
        self.pda_from_key(key, SIGNER_INFO)
    }

    fn pda_from_key(&self, key: &Pubkey, str: &[u8]) -> (Pubkey, Seed) {
        let mut seed = Seed {
            items: vec![self.chain.clone(), str.to_vec(), key.as_ref().to_vec()],
        };
        let (key, bump_seed) =
            Pubkey::find_program_address(seed.cast().as_slice(), self.program_id);
        seed.add(bump_seed);
        (key, seed)
    }

    pub fn owner_info_key(&self) -> (Pubkey, Seed) {
        let mut seed = Seed {
            items: vec![OWNER_INFO.to_vec(), upgrade_authority::ID.as_ref().to_vec()],
        };
        let (key, bump_seed) =
            Pubkey::find_program_address(seed.cast().as_slice(), self.program_id);
        seed.add(bump_seed);
        (key, seed)
    }

    pub fn init(info: &AccountInfo, typ: &AccountType) -> Result<()> {
        match typ {
            AccountType::New => unreachable!(),
            AccountType::Balance => AccountState::init(info),
            AccountType::Storage => AddressTable::init(info),
            AccountType::TxHolder => TxHolder::init(info),
            AccountType::StateHolder => StateHolder::init(info),
            AccountType::RoLock => AccountType::init(info, AccountType::RoLock),
            AccountType::SignerInfo => SignerInfo::init(info),
            AccountType::OwnerInfo => AccountType::init(info, AccountType::OwnerInfo),
        }
    }
    pub fn empty_size(info: &AccountInfo, typ: &AccountType) -> usize {
        match typ {
            AccountType::New => unreachable!(),
            AccountType::Balance => AccountState::offset(info) + AccountState::size(info),
            AccountType::Storage => AddressTable::offset(info) + AddressTable::size(info),
            AccountType::TxHolder => TxHolder::offset(info) + TxHolder::size(info),
            AccountType::StateHolder => StateHolder::offset(info) + StateHolder::size(info),
            AccountType::RoLock => RoLock::offset(info),
            AccountType::SignerInfo => SignerInfo::offset(info) + SignerInfo::size(info),
            AccountType::OwnerInfo => OwnerInfo::offset(info),
        }
    }
}
