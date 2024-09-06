use {
    crate::stubs::Stubs,
    rome_evm::{
        accounts,
        assert::asserts,
        error::{Result, RomeProgramError::*},
        origin::Origin,
        pda_balance, pda_ro_lock, pda_signer_info, pda_state_holder, pda_storage, pda_tx_holder,
        storage_index, AccountState,
        AccountType::{self, *},
        AddressTable, Data, RoLock, StateHolder, TxHolder, H160, U256,
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
pub struct State<'a> {
    pub client: Arc<RpcClient>,
    pub program_id: &'a Pubkey,
    pub accounts: RefCell<BTreeMap<Pubkey, Item>>,
    pub signer: Option<Pubkey>,
    pub alloc: RefCell<usize>,
    pub dealloc: RefCell<usize>,
    pub alloc_state: RefCell<usize>,
    pub dealloc_state: RefCell<usize>,
}

pub type Bind = (Pubkey, Account);

impl<'a> State<'a> {
    pub fn new(
        program_id: &'a Pubkey,
        signer: Option<Pubkey>,
        client: Arc<RpcClient>,
    ) -> Result<Self> {
        asserts();
        let stubs = Stubs::from_chain(Arc::clone(&client))?;
        set_syscall_stubs(stubs);

        let state = Self {
            client,
            program_id,
            accounts: RefCell::new(BTreeMap::new()),
            signer,
            alloc: RefCell::new(0),
            dealloc: RefCell::new(0),
            alloc_state: RefCell::new(0),
            dealloc_state: RefCell::new(0),
        };

        if let Some(signer) = signer {
            let bind = state.load(&signer, None)?.ok_or(InvalidSigner)?;
            let mut accounts = state.accounts.borrow_mut();
            let item = accounts.get_mut(&bind.0).unwrap();
            item.signer = true;
        }

        Ok(state)
    }
    pub fn info_addr(&self, address: &H160, or_create: bool) -> Result<Bind> {
        let key = pda_balance(address, self.program_id).0;
        let mut bind = if let Some(bind) = self.load(&key, Some(*address))? {
            bind
        } else if or_create {
            let len = {
                let mut def = def_bind();
                let info = def.into_account_info();
                AccountState::offset(&info) + AccountState::size(&info)
            };
            let bind = self.create_pda(len, Balance, key)?;
            self.insert(&bind, Some(*address));
            bind
        } else {
            return Err(BalanceAccountNotFound(key, *address));
        };

        let info = bind.into_account_info();
        AccountType::is_ok(&info, Balance, self.program_id)?;
        Ok(bind)
    }
    pub fn info_slot(&self, address: &H160, slot: &U256, or_create: bool) -> Result<(Bind, usize)> {
        let (index_be, subindex) = storage_index(slot);
        let base = self.info_addr(address, or_create)?.0;
        let key = pda_storage(&base, index_be, self.program_id).0;

        let (mut bind, subindex) = if let Some(bind) = self.load(&key, None)? {
            (bind, subindex)
        } else if or_create {
            let len = {
                let mut def = def_bind();
                let info = def.into_account_info();
                AddressTable::offset(&info) + AddressTable::size(&info)
            };
            let bind = self.create_pda(len, Storage, key)?;
            self.insert(&bind, None);
            (bind, subindex)
        } else {
            return Err(StorageAccountNotFound(key, *address, *slot));
        };

        let info = bind.into_account_info();
        AccountType::is_ok(&info, Storage, self.program_id)?;
        Ok((bind, subindex))
    }
    pub fn load(&self, key: &Pubkey, address: Option<H160>) -> Result<Option<Bind>> {
        let mut accounts = self.accounts.borrow_mut();

        let bind = if let Some(item) = accounts.get(key) {
            (*key, item.account.clone())
        } else if let Some(acc) = self
            .client
            .get_account_with_commitment(key, self.client.commitment())?
            .value
        {
            let item = Item {
                account: acc.clone(),
                writable: false,
                signer: false,
                address,
            };
            accounts.insert(*key, item);
            (*key, acc)
        } else {
            return Ok(None);
        };

        Ok(Some(bind))
    }
    pub fn update(&self, bind: Bind) -> Result<()> {
        let mut accounts = self.accounts.borrow_mut();
        let item = accounts.get_mut(&bind.0).unwrap();
        item.account = bind.1;
        item.writable = true;
        Ok(())
    }
    pub fn insert(&self, bind: &Bind, address: Option<H160>) {
        let item = Item {
            account: bind.1.clone(),
            writable: true,
            signer: false,
            address,
        };
        let mut accounts = self.accounts.borrow_mut();
        accounts.insert(bind.0, item);
    }

    pub fn count_space(&self, old: usize, new: usize, typ: AccountType, key: &Pubkey) -> Result<()> {

        let f = |len: usize, func: &dyn Fn(&State<'a>, usize) -> Result<()>| -> Result<()> {
            match typ {
                New => return Err(Custom(format!("resizing of uninitialized account {}", key))),
                Balance | Storage | AccountType::RoLock => func(&self, len),
                _ => Ok(())
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
        assert!(bind.1.data.len() > 0);
        assert_eq!(&bind.1.owner, self.program_id());

        let typ = {
            let info = bind.into_account_info();
            let typ = AccountType::from_account(&info)?;
            typ.clone()
        };
        self.count_space(bind.1.data.len(), len, typ, &bind.0)?;

        let acc = &mut bind.1;
        acc.data.resize(len, 0);
        msg!("resized len: {}", acc.data.len());

        let rent = Rent::get()?.minimum_balance(acc.data.len());

        let _sys_acc = self
            .load(&system_program::ID, None)?
            .ok_or(AccountNotFound(system_program::ID))?;
        acc.lamports = rent;

        Ok(())
    }
    pub fn create_pda(&self, len: usize, typ: AccountType, key: Pubkey) -> Result<Bind> {
        let epoch = self.client.get_epoch_info()?;

        let rent = Rent::get()?.minimum_balance(len);
        let _sys_acc = self
            .load(&system_program::ID, None)?
            .ok_or(AccountNotFound(system_program::ID))?;

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
            match typ {
                New => unreachable!(),
                Balance => AccountState::init(&info)?,
                Storage => AddressTable::init(&info)?,
                TxHolder => TxHolder::init(&info)?,
                StateHolder => StateHolder::init(&info)?,
                AccountType::RoLock => AccountType::init(&info, AccountType::RoLock)?,
                SignerInfo => accounts::SignerInfo::init(&info)?,
            }
        }

        self.count_space(0, len, typ, &bind.0)?;
        Ok(bind)
    }
    pub fn info_tx_holder(&self, index: u64, or_create: bool) -> Result<Bind> {
        let signer = self.signer.expect("signer expected");
        let (key, _) = pda_tx_holder(&signer, index, self.program_id);

        let mut bind = if let Some(bind) = self.load(&key, None)? {
            bind
        } else if or_create {
            let len = {
                let mut def = def_bind();
                let info = def.into_account_info();
                TxHolder::offset(&info) + TxHolder::size(&info)
            };
            let bind = self.create_pda(len, TxHolder, key)?;
            self.insert(&bind, None);
            bind
        } else {
            return Err(TxHolderAccountNotFound(key, index));
        };

        let info = bind.into_account_info();
        AccountType::is_ok(&info, AccountType::TxHolder, self.program_id)?;
        Ok(bind)
    }
    pub fn info_state_holder(&self, index: u64, or_create: bool) -> Result<Bind> {
        let signer = self.signer.expect("signer expected");
        let (key, _) = pda_state_holder(&signer, index, self.program_id);

        let mut bind = if let Some(bind) = self.load(&key, None)? {
            bind
        } else if or_create {
            let len = {
                let mut def = def_bind();
                let info = def.into_account_info();
                StateHolder::offset(&info) + StateHolder::size(&info)
            };
            let bind = self.create_pda(len, StateHolder, key)?;
            self.insert(&bind, None);
            bind
        } else {
            return Err(StateHolderAccountNotFound(key, index));
        };

        let info = bind.into_account_info();
        AccountType::is_ok(&info, AccountType::StateHolder, self.program_id)?;
        Ok(bind)
    }
    pub fn info_ro_lock(&self, key: &Pubkey, or_create: bool) -> Result<Bind> {
        let (key, _) = pda_ro_lock(key, self.program_id);

        let mut bind = if let Some(bind) = self.load(&key, None)? {
            bind
        } else if or_create {
            let len = {
                let mut def = def_bind();
                let info = def.into_account_info();
                RoLock::offset(&info)
            };
            let bind = self.create_pda(len, AccountType::RoLock, key)?;
            self.insert(&bind, None);
            bind
        } else {
            return Err(RoLockAccountNotFound(key));
        };

        let info = bind.into_account_info();
        AccountType::is_ok(&info, AccountType::RoLock, self.program_id)?;
        Ok(bind)
    }
    pub fn info_signer_info(&self, key: &Pubkey, or_create: bool) -> Result<Bind> {
        let (key, _) = pda_signer_info(key, self.program_id);

        let mut bind = if let Some(bind) = self.load(&key, None)? {
            bind
        } else if or_create {
            let len = {
                let mut def = def_bind();
                let info = def.into_account_info();
                accounts::SignerInfo::offset(&info) + accounts::SignerInfo::size(&info)
            };
            let bind = self.create_pda(len, SignerInfo, key)?;
            self.insert(&bind, None);
            bind
        } else {
            return Err(SignerInfoAccountNotFound(key));
        };

        let info = bind.into_account_info();
        AccountType::is_ok(&info, SignerInfo, self.program_id)?;
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
