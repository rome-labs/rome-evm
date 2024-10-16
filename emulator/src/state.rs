use {
    super::fake,
    crate::stubs::Stubs,
    rome_evm::{
        assert::asserts,
        error::{Result, RomeProgramError::*},
        origin::Origin,
        state::pda::Pda,
        AccountType::{self, *},
        Data, H160, U256, OwnerInfo,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{
        account_info::IntoAccountInfo, msg, pubkey::Pubkey, rent::Rent, system_program,
        sysvar::Sysvar,
    },
    solana_sdk::{account::Account, program_stubs::set_syscall_stubs},
    std::{cell::RefCell, collections::BTreeMap, sync::Arc},
};

#[derive(Clone, Debug)]
pub struct Item {
    pub account: Account,
    pub writable: bool,
    pub signer: bool,
    pub address: Option<H160>,
}
pub type Slots = BTreeMap<U256, bool>;
pub struct State<'a> {
    pub client: Arc<RpcClient>,
    pub program_id: &'a Pubkey,
    pub accounts: RefCell<BTreeMap<Pubkey, Item>>,
    pub storage: RefCell<BTreeMap<H160, Slots>>,
    pub signer: Option<Pubkey>,
    pub alloc: RefCell<usize>,
    pub dealloc: RefCell<usize>,
    pub alloc_state: RefCell<usize>,
    pub dealloc_state: RefCell<usize>,
    pub pda: Pda<'a>,
    pub chain: u64,
}

pub type Bind = (Pubkey, Account);

impl<'a> State<'a> {
    pub fn new(
        program_id: &'a Pubkey,
        signer: Option<Pubkey>,
        client: Arc<RpcClient>,
        chain: u64,
    ) -> Result<Self> {
        let state = Self::new_unchecked(program_id, signer, client, chain)?;
        let mut bind = state.info_owner_reg(false)?;
        let info = bind.into_account_info();
        OwnerInfo::check_chain(&info, chain)?;

        Ok(state)
    }

    pub fn new_unchecked(
        program_id: &'a Pubkey,
        signer: Option<Pubkey>,
        client: Arc<RpcClient>,
        chain: u64,
    ) -> Result<Self> {
        asserts();
        let stubs = Stubs::from_chain(Arc::clone(&client))?;
        set_syscall_stubs(stubs);

        let state = Self {
            client,
            program_id,
            accounts: RefCell::new(BTreeMap::new()),
            storage: RefCell::new(BTreeMap::new()),
            signer,
            alloc: RefCell::new(0),
            dealloc: RefCell::new(0),
            alloc_state: RefCell::new(0),
            dealloc_state: RefCell::new(0),
            pda: Pda::new(program_id, chain),
            chain,
        };

        if let Some(signer) = signer {
            if signer != fake::ID {
                let bind = state.load(&signer, None)?.ok_or(InvalidSigner)?;
                state.set_signer(&bind.0);
            } else {
                // the fake signer is used to execute an estimate_gas request using the iterative_tx pipeline
                let acc = Account {
                    lamports: 0,
                    data: vec![],
                    owner: Pubkey::default(),
                    executable: false,
                    rent_epoch: 0,
                };
                state.insert(&(fake::ID, acc), None, false);
                state.set_signer(&fake::ID);
            }
        }

        Ok(state)
    }

    pub fn info_addr(&self, address: &H160, or_create: bool) -> Result<Bind> {
        let key = self.pda.balance_key(address).0;
        self.info_pda(key, Balance, Some(*address), or_create)
    }
    pub fn info_slot(&self, address: &H160, slot: &U256, or_create: bool) -> Result<(Bind, usize)> {
        let (index_be, subindex) = Pda::storage_index(slot);
        let base = self.info_addr(address, or_create)?.0;
        let key = self.pda.storage_key(&base, index_be).0;
        let bind = self.info_pda(key, Storage, Some(*address), or_create)?;
        self.update_slots(address, slot, or_create);

        Ok((bind, subindex))
    }
    pub fn info_tx_holder(&self, index: u64, or_create: bool) -> Result<Bind> {
        let signer = self.signer.expect("signer expected");
        let (key, _) = self.pda.tx_holder_key(&signer, index);
        self.info_pda(key, TxHolder, None, or_create)
    }
    pub fn info_state_holder(&self, index: u64, or_create: bool) -> Result<Bind> {
        let signer = self.signer.expect("signer expected");
        let (key, _) = self.pda.state_holder_key(&signer, index);
        self.info_pda(key, StateHolder, None, or_create)
    }
    pub fn info_ro_lock(&self, key: &Pubkey, or_create: bool) -> Result<Bind> {
        let (key, _) = self.pda.ro_lock_key(key);
        self.info_pda(key, RoLock, None, or_create)
    }
    pub fn info_signer_info(&self, key: &Pubkey, or_create: bool) -> Result<Bind> {
        let (key, _) = self.pda.signer_info_key(key);
        self.info_pda(key, SignerInfo, None, or_create)
    }
    pub fn info_owner_reg(&self, or_create: bool) -> Result<Bind> {
        let (key, _) = self.pda.owner_info_key();
        self.info_pda(key, OwnerInfo, None, or_create)
    }
    pub fn info_pda(
        &self,
        key: Pubkey,
        typ: AccountType,
        addr: Option<H160>,
        or_create: bool,
    ) -> Result<Bind> {
        let mut bind = if let Some(bind) = self.load(&key, addr)? {
            bind
        } else if or_create {
            self.create_pda(&typ, key, addr)?
        } else {
            return Err(AccountNotFound(key, typ));
        };

        let info = bind.into_account_info();
        AccountType::is_ok(&info, typ, self.program_id)?;
        Ok(bind)
    }
    fn update_slots(&self, address: &H160, slot: &U256, writable: bool) {
        let mut storage = self.storage.borrow_mut();

        storage
            .entry(*address)
            .and_modify(|slots| {
                slots
                    .entry(*slot)
                    .and_modify(|rw| *rw |= writable)
                    .or_insert(writable);
            })
            .or_insert(BTreeMap::from([(*slot, writable)]));
    }
    pub fn load(&self, key: &Pubkey, address: Option<H160>) -> Result<Option<Bind>> {
        let loaded = { self.accounts.borrow().get(key).cloned() };

        let opt = if let Some(item) = loaded {
            let bind = (*key, item.account);
            Some(bind)
        } else if let Some(acc) = self
            .client
            .get_account_with_commitment(key, self.client.commitment())?
            .value
        {
            let bind = (*key, acc);
            self.insert(&bind, address, false);
            Some(bind)
        } else {
            None
        };

        Ok(opt)
    }
    pub fn update(&self, bind: Bind) -> Result<()> {
        let mut accounts = self.accounts.borrow_mut();
        let item = accounts.get_mut(&bind.0).unwrap();
        item.account = bind.1;
        item.writable = true;
        Ok(())
    }
    pub fn set_signer(&self, key: &Pubkey) {
        let mut accounts = self.accounts.borrow_mut();
        let item = accounts.get_mut(key).unwrap();
        item.signer = true;
    }
    pub fn insert(&self, bind: &Bind, address: Option<H160>, writable: bool) {
        let item = Item {
            account: bind.1.clone(),
            writable,
            signer: false,
            address,
        };
        let mut accounts = self.accounts.borrow_mut();
        accounts.insert(bind.0, item);
    }

    pub fn count_space(
        &self,
        old: usize,
        new: usize,
        typ: &AccountType,
        key: &Pubkey,
    ) -> Result<()> {
        let f = |len: usize, func: &dyn Fn(&State<'a>, usize) -> Result<()>| -> Result<()> {
            match typ {
                New => Err(Custom(format!("resizing of uninitialized account {}", key))),
                Balance | Storage | AccountType::RoLock => func(self, len),
                _ => Ok(()),
            }
        };

        if old < new {
            let diff = new.saturating_sub(old);
            self.inc_alloc(diff)?;
            f(diff, &State::inc_alloc_state)
        } else {
            let diff = old.saturating_sub(new);
            self.inc_dealloc(diff)?;
            f(diff, &State::inc_dealloc_state)
        }
    }
    pub fn realloc(&self, bind: &mut Bind, len: usize) -> Result<()> {
        assert!(!bind.1.data.is_empty());
        assert_eq!(&bind.1.owner, self.program_id());

        let typ = {
            let info = bind.into_account_info();
            let typ = AccountType::from_account(&info)?;
            typ.clone()
        };
        self.count_space(bind.1.data.len(), len, &typ, &bind.0)?;

        let acc = &mut bind.1;
        acc.data.resize(len, 0);
        msg!("resized len: {}", acc.data.len());

        let rent = Rent::get()?.minimum_balance(acc.data.len());

        let _sys_acc = self
            .load(&system_program::ID, None)?
            .ok_or(SystemAccountNotFound(system_program::ID))?;
        acc.lamports = rent;

        Ok(())
    }
    fn pda_size(typ: &AccountType) -> usize {
        let mut def = def_bind();
        let info = def.into_account_info();
        Pda::empty_size(&info, typ)
    }
    pub fn create_pda(&self, typ: &AccountType, key: Pubkey, addr: Option<H160>) -> Result<Bind> {
        let len = State::pda_size(typ);
        let epoch = self.client.get_epoch_info()?;

        let rent = Rent::get()?.minimum_balance(len);
        let _sys_acc = self
            .load(&system_program::ID, None)?
            .ok_or(SystemAccountNotFound(system_program::ID))?;

        let pda = Account {
            lamports: rent,
            data: vec![0; len],
            owner: *self.program_id,
            executable: false,
            rent_epoch: epoch.epoch,
        };

        let mut bind = (key, pda);
        {
            let info = bind.into_account_info();
            Pda::init(&info, typ)?;
        }
        self.count_space(0, len, typ, &bind.0)?;
        self.insert(&bind, addr, true);

        Ok(bind)
    }
    pub fn inc_alloc(&self, len: usize) -> Result<()> {
        if len > 0 {
            if *self.dealloc.borrow() > 0 {
                return Err(AllocationError(
                    "error to allocate account data: deallocation found".to_string(),
                ));
            }
            let mut alloc = self.alloc.borrow_mut();
            *alloc = alloc.saturating_add(len);
        }

        Ok(())
    }
    pub fn inc_dealloc(&self, len: usize) -> Result<()> {
        if len > 0 {
            if *self.alloc.borrow() > 0 {
                return Err(AllocationError(
                    "error to deallocate account data: allocation found".to_string(),
                ));
            }
            let mut dealloc = self.dealloc.borrow_mut();
            *dealloc = dealloc.saturating_add(len);
        }

        Ok(())
    }
    pub fn inc_alloc_state(&self, len: usize) -> Result<()> {
        if len > 0 {
            if *self.dealloc_state.borrow() > 0 {
                return Err(AllocationError(
                    "error to allocate account data: deallocation state found".to_string(),
                ));
            }
            let mut alloc_state = self.alloc_state.borrow_mut();
            *alloc_state = alloc_state.saturating_add(len);
        }

        Ok(())
    }
    pub fn inc_dealloc_state(&self, len: usize) -> Result<()> {
        if len > 0 {
            if *self.alloc_state.borrow() > 0 {
                return Err(AllocationError(
                    "error to deallocate account data: allocation state found".to_string(),
                ));
            }
            let mut dealloc_state = self.dealloc_state.borrow_mut();
            *dealloc_state = dealloc_state.saturating_add(len);
        }

        Ok(())
    }
    pub fn reset_counters(&self) {
        *self.alloc.borrow_mut() = 0;
        *self.dealloc.borrow_mut() = 0;
        *self.alloc_state.borrow_mut() = 0;
        *self.dealloc_state.borrow_mut() = 0;
    }
}

pub fn def_bind() -> Bind {
    let empty = Account {
        lamports: 0,
        data: vec![],
        owner: Pubkey::default(),
        executable: false,
        rent_epoch: 0,
    };
    (Pubkey::default(), empty.clone())
}
