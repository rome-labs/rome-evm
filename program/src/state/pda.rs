use {
    crate::{
        error::Result, state::base::Syscall, upgrade_authority, AccountState, AccountType, Data,
        OwnerInfo, RoLock, StateHolder, Storage, TxHolder, ACCOUNT_SEED, OWNER_INFO, RO_LOCK_SEED,
        STATE_HOLDER_SEED, STORAGE_LEN, TX_HOLDER_SEED,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    evm::{H160, U256},
    solana_program::{account_info::AccountInfo, pubkey::Pubkey},
    std::{cell::RefCell, collections::HashMap, rc::Rc},
};

#[derive(BorshSerialize, BorshDeserialize, Default, Clone)]
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
    pub fn from_vec(slice: Vec<&[u8]>) -> Self {
        let items = slice
            .iter()
            .map(|&a| a.to_vec())
            .collect::<Vec<_>>();

        Self {
            items
        }
    }
}

type BaseIndex = (Pubkey, [u8; 32]);

pub struct Pda<'a> {
    chain: Vec<u8>,
    program_id: &'a Pubkey,
    pub balance: RefCell<HashMap<H160, (Pubkey, Seed)>>,
    pub storage: RefCell<HashMap<BaseIndex, (Pubkey, Seed)>>,
    pub ro_lock: RefCell<HashMap<Pubkey, (Pubkey, Seed)>>,
    pub syscall: Rc<Syscall>,
}

impl<'a> Pda<'a> {
    pub fn new(program_id: &'a Pubkey, chain: u64, syscall: Rc<Syscall>) -> Self {
        Self {
            chain: chain.to_le_bytes().to_vec(),
            program_id,
            balance: RefCell::new(HashMap::new()),
            storage: RefCell::new(HashMap::new()),
            ro_lock: RefCell::new(HashMap::new()),
            syscall,
        }
    }

    pub fn balance_key(&self, address: &H160) -> (Pubkey, Seed) {
        let mut balance = self.balance.borrow_mut();
        if let Some(cache) = balance.get(address) {
            return cache.clone();
        }

        let mut seed = Seed {
            items: vec![
                self.chain.clone(),
                ACCOUNT_SEED.to_vec(),
                address.as_bytes().to_vec(),
            ],
        };
        // TODO: move seed to find_pda
        let (key, bump_seed) = self.find_pda(&seed);
        seed.add(bump_seed);
        balance.insert(*address, (key, seed.clone()));

        (key, seed)
    }
    pub fn from_balance_key(&self, address: &H160, salt: &[u8]) -> (Pubkey, Seed) {
        let (key, _) = self.balance_key(address);

        let vec = vec![key.as_ref(), salt];
        let mut seed = Seed::from_vec(vec);

        let (key, bump) = self.find_pda(&seed);
        seed.add(bump);

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
        let (key, bump_seed) = self.find_pda(&seed);
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
        let (key, bump_seed) = self.find_pda(&seed);
        seed.add(bump_seed);
        (key, seed)
    }

    pub fn storage_index(slot: &U256) -> ([u8; 32], u8) {
        let (index, sub_ix) = slot.div_mod(STORAGE_LEN.into());
        let mut index_be = [0_u8; 32];
        index.to_big_endian(&mut index_be);

        assert!(sub_ix <= u8::MAX.into());
        (index_be, sub_ix.as_usize() as u8)
    }

    pub fn storage_key(&self, base: &Pubkey, index_be: [u8; 32]) -> (Pubkey, Seed) {
        let mut storage = self.storage.borrow_mut();
        if let Some(cache) = storage.get(&(*base, index_be)) {
            return cache.clone();
        }

        let mut seed = Seed {
            items: vec![
                self.chain.clone(),
                base.as_ref().to_vec(),
                index_be.to_vec(),
            ],
        };
        let (key, bump_seed) = self.find_pda(&seed);
        seed.add(bump_seed);
        storage.insert((*base, index_be), (key, seed.clone()));

        (key, seed)
    }

    pub fn ro_lock_key(&self, key: &Pubkey) -> (Pubkey, Seed) {
        let mut ro_lock = self.ro_lock.borrow_mut();
        if let Some(cache) = ro_lock.get(key) {
            return cache.clone();
        }
        let bind = self.pda_from_key(key, RO_LOCK_SEED);
        ro_lock.insert(*key, bind.clone());

        bind
    }

    fn pda_from_key(&self, key: &Pubkey, str: &[u8]) -> (Pubkey, Seed) {
        let mut seed = Seed {
            items: vec![self.chain.clone(), str.to_vec(), key.as_ref().to_vec()],
        };
        let (key, bump_seed) = self.find_pda(&seed);
        seed.add(bump_seed);
        (key, seed)
    }

    pub fn owner_info_key(&self) -> (Pubkey, Seed) {
        let mut seed = Seed {
            items: vec![OWNER_INFO.to_vec(), upgrade_authority::ID.as_ref().to_vec()],
        };
        let (key, bump_seed) = self.find_pda(&seed);
        seed.add(bump_seed);
        (key, seed)
    }

    pub fn init(info: &AccountInfo, typ: &AccountType) -> Result<()> {
        match typ {
            AccountType::New => unreachable!(),
            AccountType::Balance => AccountState::init(info),
            AccountType::Storage => Storage::init(info),
            AccountType::TxHolder => TxHolder::init(info),
            AccountType::StateHolder => StateHolder::init(info),
            AccountType::RoLock => RoLock::init(info),
            AccountType::OwnerInfo => OwnerInfo::init(info),
        }
    }
    pub fn empty_size(info: &AccountInfo, typ: &AccountType) -> usize {
        match typ {
            AccountType::New => unreachable!(),
            AccountType::Balance => AccountState::offset(info) + AccountState::size(info),
            AccountType::Storage => Storage::offset(info) + Storage::size(info),
            AccountType::TxHolder => TxHolder::offset(info) + TxHolder::size(info),
            AccountType::StateHolder => StateHolder::offset(info) + StateHolder::size(info),
            AccountType::RoLock => RoLock::offset(info),
            AccountType::OwnerInfo => OwnerInfo::offset(info),
        }
    }
    pub fn serialize(&self, into: &mut &mut [u8]) -> Result<()> {
        let balance = self.balance.borrow();
        let storage = self.storage.borrow();
        let ro_lock = self.ro_lock.borrow();

        balance.serialize(into)?;
        storage.serialize(into)?;
        ro_lock.serialize(into)?;

        Ok(())
    }
    pub fn deserialize(&self, from: &mut &[u8]) -> Result<()> {
        *self.balance.borrow_mut() = BorshDeserialize::deserialize(from)?;
        *self.storage.borrow_mut() = BorshDeserialize::deserialize(from)?;
        *self.ro_lock.borrow_mut() = BorshDeserialize::deserialize(from)?;

        Ok(())
    }

    fn find_pda(&self, seed: &Seed) -> (Pubkey, u8) {
        self.syscall.inc();
        Pubkey::find_program_address(seed.cast().as_slice(), self.program_id)
    }

    #[cfg(not(target_os = "solana"))]
    pub fn reset(&self) {
        self.balance.borrow_mut().clear();
        self.storage.borrow_mut().clear();
        self.ro_lock.borrow_mut().clear();
    }
}
