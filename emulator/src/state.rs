use {
    super::fake,
    crate::stubs::Stubs,
    rome_evm::{
        assert::asserts,
        error::{Result, RomeProgramError::*},
        state::{base::Base, pda::Pda},
        AccountType::{self, *},
        Data, OwnerInfo, H160, U256, state::aux::Account, origin::Origin, pda::Seed,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{
        account_info::IntoAccountInfo, msg, pubkey::Pubkey, rent::Rent, system_program,
        sysvar::Sysvar, program_stubs::set_syscall_stubs, system_instruction,
    },
    std::{
        cell::RefCell, collections::BTreeMap, ops::Deref, sync::Arc, cmp::Ordering::{Greater, Less}},
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
        // 1. needs for transmit_tx,  
        // 2. reduces the number of failures if tx depends on timestamp
        let _ = state.info_sys(&system_program::ID)?; 
            
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
                    writable: true,
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
    pub fn info_alt_slots(&self, index: u64, or_create: bool) -> Result<Bind> {
        let signer = self.signer.expect("signer expected");
        let (key, _) = self.pda.alt_slots_key(&signer, index);
        self.info_pda(&key, AltSlots, None, or_create)
    }
    // TODO: the missing account must be included in the transaction accounts
    pub fn info_pda(
        &self,
        key: &Pubkey,
        typ: AccountType,
        addr: Option<H160>,
        or_create: bool,
    ) -> Result<Bind> {
        if let Some(mut bind) = self.load(key, addr, or_create)? {
            // TODO: associated_spl_token::create_associated_token_account accepts the account owned by EVM.
            // It's state is not required, only pubkey is used. This approach is incorrect.
            // External programs should not accept EVM accounts. It is related to the implementation of
            // emulation of the external programs.
            // It is necessary to replace aspl_token instruction by  system_program::create_account
            // and spl_token::InitializeAccount3 and remove this workaround:
            if bind.1.lamports == 0
                && system_program::check_id(&bind.1.owner)
                && bind.1.data.is_empty()
                && bind.1.writable == false {

                if or_create {
                    self.create_pda(&typ, *key, addr)?;
                    return self.info_pda(key, typ, addr, or_create);
                } else {
                    return Err(PdaAccountNotFound(*key, typ))
                }
            }

            self.update_writable(&bind.0, or_create);
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
        writable: bool,
    ) -> Result<Bind> {
        if let Some(bind) = self.load(key, None, writable)? {
            self.update_writable(&bind.0, writable);
            Ok(bind)
        } else {
            let new = Account {
                lamports: 0,
                data: vec![],
                owner: system_program::ID,
                executable: false,
                rent_epoch: 0,
                writable,
            };

            let bind = (*key, new);
            self.insert(bind, None);
            self.info_external(key, writable)
        }
    }
    pub fn info_sol_wallet(&self, or_create: bool) -> Result<Bind> {
        let (key, _) = self.pda.sol_wallet();
        let bind = self.info_external(&key, true)?;

        if bind.1.lamports == 0 {
            if !or_create {
                return Err(AccountNotFound(key))
            }
            assert_eq!(bind.1.data.len(), 0);
            assert!(bind.1.writable);
            assert_eq!(bind.1.owner, system_program::ID);

            let rent = Rent::get()?.minimum_balance(0);
            let ix = &system_instruction::create_account(
                &self.signer(),
                &key,
                rent,
                0,
                &system_program::ID,
            );

            self.invoke_signed(&ix, &Seed::default(), false)?;
        }

        Ok(bind)
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
        writable: bool
    ) -> Result<Option<Bind>> {

        if let Some(item) = self.accounts.borrow().get(key) {
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
                            writable,
                        }
                    };
                    let bind = (*key, acc);
                    self.insert(bind, address);
                    self.load(key, address, writable)
            })
   }
    pub fn update(&self, bind: Bind) {
        let mut accounts = self.accounts.borrow_mut();
        let item = accounts.get_mut(&bind.0).unwrap();
        item.account = bind.1;
        item.account.writable = true;
    }
    pub fn set_signer(&self, key: &Pubkey) {
        let mut accounts = self.accounts.borrow_mut();
        let item = accounts.get_mut(key).unwrap();
        item.signer = true;
    }
    pub fn update_writable(&self, key: &Pubkey, writeable: bool) {
        let mut accounts = self.accounts.borrow_mut();
        let item = accounts.get_mut(key).unwrap();
        item.account.writable |= writeable;
    }
    pub fn insert(&self, bind: Bind, address: Option<H160>) {
        let item = Item {
            account: bind.1,
            signer: false,
            address,
        };
        let mut accounts = self.accounts.borrow_mut();

        if let Some (item) = accounts.insert(bind.0, item) {
            assert_eq!(item.account.lamports, 0);
            assert!(item.account.data.is_empty());
            assert_eq!(item.account.owner, system_program::ID);
            assert_eq!(item.account.writable, false);
        }
    }

    pub fn set_addr(&self, key: &Pubkey, address: Option<H160>) {
        if let Some(address) = address {
            let mut accounts = self.accounts.borrow_mut();
            let item = accounts.get_mut(key).unwrap();
            item.address = Some(address)
        }
    }

    pub fn inc_space_counter(&self, alloc: usize, dealloc: usize, refund_to_signer: bool) -> Result<()> {
        Base::inc_alloc(&self.base, alloc)?;
        Base::inc_dealloc(&self.base, dealloc)?;

        if refund_to_signer {
            Base::inc_alloc_payed(&self.base, alloc)?;
            Base::inc_dealloc_payed(&self.base, dealloc)?;
        }

        Ok(())
    }

    pub fn realloc(&self, key: &Pubkey, len: usize) -> Result<()> {
        let mut bind = self.info_sys(key)?;

        assert!(!bind.1.data.is_empty());
        assert_eq!(&bind.1.owner, self.program_id);

        let alloc = len.saturating_sub(bind.1.data.len());
        let dealloc = bind.1.data.len().saturating_sub(len);

        let refund_to_signer = {
            let info = bind.into_account_info();
            let typ = AccountType::from_account(&info)?;
            typ.is_paid()
        };
        self.inc_space_counter(alloc, dealloc, refund_to_signer)?;

        bind.1.data.resize(len, 0);
        msg!("resized len: {}", bind.1.data.len());

        let lamports = bind.1.lamports;
        let rent = Rent::get()?.minimum_balance(bind.1.data.len());

        match rent.cmp(&lamports) {
            Greater => {
                self.update(bind);
                let ix = system_instruction::transfer(&self.signer(), key, rent - lamports);
                self.invoke_signed(&ix, &Seed::default(), refund_to_signer)?;
            }
            Less => {
                let refund = lamports - rent;

                bind.1.lamports -= refund;
                self.update(bind);

                let mut signer_bind = self.info_sys(&self.signer())?;
                signer_bind.1.lamports += refund;
                self.update(signer_bind);

                if refund_to_signer {
                    self.add_refund(refund)?;
                }
            }
            _ => {}
        }


        Ok(())
    }
    fn pda_size(typ: &AccountType) -> usize {
        let mut def = def_bind();
        let info = def.into_account_info();
        Pda::empty_size(&info, typ)
    }
    pub fn create_pda(&self, typ: &AccountType, key: Pubkey, addr: Option<H160>) -> Result<()> {
        let len = State::pda_size(typ);
        let rent = Rent::get()?.minimum_balance(len);

        let ix = system_instruction::create_account(
            &self.signer(),
            &key,
            rent,
            len as u64,
            self.program_id,
        );

        self.invoke_signed(&ix, &Seed::default(), typ.is_paid())?;

        self.set_addr(&key, addr);

        let mut accs = self.accounts.borrow_mut();
        let item = accs.get_mut(&key).unwrap();
        let info = (&key, &mut item.account).into_account_info();
        Pda::init(&info, typ)?;

        Ok(())
    }
}

pub fn def_bind() -> Bind {
    (Pubkey::default(), Account::default())
}
