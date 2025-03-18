use {
    super::fake,
    crate::stubs::Stubs,
    rome_evm::{
        assert::asserts,
        error::{Result, RomeProgramError::*},
        state::{base::Base, pda::Pda},
        AccountType::{self, *},
        Data, OwnerInfo, H160, U256, state::aux::Account,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{
        account_info::IntoAccountInfo, msg, pubkey::Pubkey, rent::Rent, system_program,
        sysvar::Sysvar, program_stubs::set_syscall_stubs,
    },
    std::{cell::RefCell, collections::BTreeMap, ops::Deref, sync::Arc},
};

#[derive(Clone, Debug)]
pub struct Item {
    pub account: Account,
    pub signer: bool,
    pub address: Option<H160>,
}
pub type Slots = BTreeMap<U256, bool>;
pub type Bind = (Pubkey, Account);

impl<'a> Deref for State<'a> {
    type Target = Base<'a>;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
pub struct State<'a> {
    pub base: Base<'a>,
    pub client: Arc<RpcClient>,
    pub accounts: RefCell<BTreeMap<Pubkey, Item>>,
    pub storage: RefCell<BTreeMap<H160, Slots>>,
    pub signer: Option<Pubkey>,
}

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
            base: Base::new(program_id, chain),
            client,
            accounts: RefCell::new(BTreeMap::new()),
            storage: RefCell::new(BTreeMap::new()),
            signer,
        };

        if let Some(signer) = signer {
            if signer != fake::ID {
                let bind = state.info_sys(&signer).map_err(|_| InvalidSigner)?;
                state.set_signer(&bind.0);
            } else {
                // the fake signer is used to execute an estimate_gas request using the iterative_tx pipeline
                let acc = Account {
                    lamports: u64::MAX,
                    data: vec![],
                    owner: Pubkey::default(),
                    executable: false,
                    rent_epoch: 0,
                    writeable: true,
                };
                state.insert((fake::ID, acc), None);
                state.set_signer(&fake::ID);
            }
        }

        Ok(state)
    }

    pub fn info_addr(&self, address: &H160, or_create: bool) -> Result<Bind> {
        let key = self.pda.balance_key(address).0;
        self.info_pda(&key, Balance, Some(*address), or_create)
    }
    pub fn info_slot(&self, address: &H160, slot: &U256, or_create: bool) -> Result<(Bind, u8)> {
        let (key, _, sub_ix) = self.slot_to_key(address, slot);
        let bind = self.info_pda(&key, Storage, Some(*address), or_create)?;
        self.update_slots(address, slot, or_create);

        Ok((bind, sub_ix))
    }
    pub fn info_tx_holder(&self, index: u64, or_create: bool) -> Result<Bind> {
        let signer = self.signer.expect("signer expected");
        let (key, _) = self.pda.tx_holder_key(&signer, index);
        self.info_pda(&key, TxHolder, None, or_create)
    }
    pub fn info_state_holder(&self, index: u64, or_create: bool) -> Result<Bind> {
        let signer = self.signer.expect("signer expected");
        let (key, _) = self.pda.state_holder_key(&signer, index);
        self.info_pda(&key, StateHolder, None, or_create)
    }
    pub fn info_ro_lock(&self, key: &Pubkey, or_create: bool) -> Result<Bind> {
        let (key, _) = self.pda.ro_lock_key(key);
        self.info_pda(&key, RoLock, None, or_create)
    }
    pub fn info_owner_reg(&self, or_create: bool) -> Result<Bind> {
        let (key, _) = self.pda.owner_info_key();
        self.info_pda(&key, OwnerInfo, None, or_create)
    }
    pub fn info_pda(
        &self,
        key: &Pubkey,
        typ: AccountType,
        addr: Option<H160>,
        or_create: bool,
    ) -> Result<Bind> {
        if let Some(mut bind) = self.load(key, addr, or_create)? {
            let info = bind.into_account_info();
            AccountType::is_ok(&info, typ, self.program_id)?;
            Ok(bind)
        } else if or_create {
            self.create_pda(&typ, *key, addr)?;
            self.info_pda(key, typ, addr, or_create)
        } else {
            Err(PdaAccountNotFound(*key, typ))
        }
    }
    pub fn info_external(
        &self,
        key: &Pubkey,
        writeable: bool,
    ) -> Result<Bind> {

        if let Some(bind) = self.load(key, None, writeable)? {
            Ok(bind)
        } else {
            let new = Account {
                lamports: 0,
                data: vec![],
                owner: system_program::ID,
                executable: false,
                rent_epoch: 0,
                writeable,
            };

            let bind = (*key, new);
            self.insert(bind, None);
            self.info_external(key, writeable)
        }
    }

    pub fn info_program(
        &self,
        key: &Pubkey,
    ) -> Result<Bind> {
        if !self.accounts.borrow_mut().contains_key(&key) {
            let new = Account::new_executable();

            let bind = (*key, new);
            self.insert(bind, None);
        }

        self.info_external(key, false)
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
    pub fn info_sys(&self, key: &Pubkey) -> Result<Bind> {
        self.load(key, None, false)?.ok_or(AccountNotFound(*key))
    }
    pub fn load(
        &self,
        key: &Pubkey,
        address: Option<H160>,
        writeable: bool
    ) -> Result<Option<Bind>> {

        if let Some(item) = self.accounts.borrow_mut().get_mut(key) {
            item.account.writeable |= writeable;
            let bind = (*key, item.account.clone());
            return Ok(Some(bind))
        }

        self
            .client
            .get_account_with_commitment(key, self.client.commitment())?
            .value
            .map_or_else(
                || Ok(None),
                |sdk| {
                    let acc = if sdk.executable {
                        Account::new_executable()
                    } else {
                        Account {
                            lamports: sdk.lamports,
                            data: sdk.data,
                            owner: sdk.owner,
                            executable: sdk.executable,
                            rent_epoch: sdk.rent_epoch,
                            writeable,
                        }
                    };
                    let bind = (*key, acc);
                    self.insert(bind, address);
                    self.load(key, address, writeable)
            })
   }
    pub fn update(&self, bind: Bind) {
        let mut accounts = self.accounts.borrow_mut();
        let item = accounts.get_mut(&bind.0).unwrap();
        item.account = bind.1;
        item.account.writeable = true;
    }
    pub fn set_signer(&self, key: &Pubkey) {
        let mut accounts = self.accounts.borrow_mut();
        let item = accounts.get_mut(key).unwrap();
        item.signer = true;
    }
    pub fn insert(&self, bind: Bind, address: Option<H160>) {
        let item = Item {
            account: bind.1,
            signer: false,
            address,
        };
        let mut accounts = self.accounts.borrow_mut();
        assert!(accounts.insert(bind.0, item).is_none());
    }

    pub fn count_space(
        &self,
        old: usize,
        new: usize,
        typ: &AccountType,
        key: &Pubkey,
    ) -> Result<()> {
        let f = |len: usize, func: &dyn Fn(&Base<'a>, usize) -> Result<()>| -> Result<()> {
            match typ {
                New => Err(Custom(format!("resizing of uninitialized account {}", key))),
                // TODO: remove RoLock from alloc_state
                Balance | Storage | AccountType::RoLock => func(&self.base, len),
                _ => Ok(()),
            }
        };

        if old < new {
            let diff = new.saturating_sub(old);
            self.inc_alloc(diff)?;
            f(diff, &Base::inc_alloc_payed)
        } else {
            let diff = old.saturating_sub(new);
            self.inc_dealloc(diff)?;
            f(diff, &Base::inc_dealloc_payed)
        }
    }
    pub fn realloc(&self, bind: &mut Bind, len: usize) -> Result<()> {
        assert!(!bind.1.data.is_empty());
        assert_eq!(&bind.1.owner, self.program_id);

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

        if rent > acc.lamports {
            self.syscall.inc(); // transfer
        }

        let _sys_acc = self.info_sys(&system_program::ID)?;
        acc.lamports = rent;

        Ok(())
    }
    fn pda_size(typ: &AccountType) -> usize {
        let mut def = def_bind();
        let info = def.into_account_info();
        Pda::empty_size(&info, typ)
    }
    pub fn create_pda(&self, typ: &AccountType, key: Pubkey, addr: Option<H160>) -> Result<()> {
        let len = State::pda_size(typ);
        let epoch = self.client.get_epoch_info()?;

        let rent = Rent::get()?.minimum_balance(len);
        let _sys_acc = self.info_sys(&system_program::ID)?;

        let pda = Account {
            lamports: rent,
            data: vec![0; len],
            owner: *self.program_id,
            executable: false,
            rent_epoch: epoch.epoch,
            writeable: true,
        };
        self.syscall.inc();

        let mut bind = (key, pda);
        {
            let info = bind.into_account_info();
            Pda::init(&info, typ)?;
        }
        self.count_space(0, len, typ, &bind.0)?;
        self.insert(bind, addr);

        Ok(())
    }
}

pub fn def_bind() -> Bind {
    (Pubkey::default(), Account::default())
}
