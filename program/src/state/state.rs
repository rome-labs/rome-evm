use {
    super::pda::{Pda, Seed},
    crate::{error::RomeProgramError::*, error::*, origin::Origin, AccountType, OwnerInfo,},
    evm::{H160, U256},
    solana_program::{
        account_info::AccountInfo, msg, program::invoke_signed, pubkey::Pubkey, rent::Rent,
        system_instruction, system_program, sysvar::recent_blockhashes, sysvar::Sysvar,
    },
    std::{cell::RefCell, cmp::Ordering::*, collections::HashMap, iter::FromIterator},
};

pub struct State<'a> {
    pub program_id: &'a Pubkey,
    all: HashMap<Pubkey, &'a AccountInfo<'a>>,
    balance: RefCell<HashMap<H160, &'a AccountInfo<'a>>>,
    storage: RefCell<HashMap<(H160, U256), &'a AccountInfo<'a>>>,
    pub allocated: RefCell<usize>,
    pub deallocated: RefCell<usize>,
    pub chain: u64,
    pda: Pda<'a>,
}

#[allow(dead_code)]
impl<'a> State<'a> {
    pub fn new(program_id: &'a Pubkey, accounts: &'a [AccountInfo<'a>], chain: u64) -> Result<Self> {
        let state = Self::new_unchecked(program_id, accounts, chain);
        let info = state.info_owner_reg(false)?;
        OwnerInfo::check_chain(info, chain)?;

        Ok(state)
    }
    pub fn new_unchecked(program_id: &'a Pubkey, accounts: &'a [AccountInfo<'a>], chain: u64) -> Self {
        let keys = accounts.iter().map(|a| *a.key);
        let all = HashMap::from_iter(keys.zip(accounts.iter()));

        Self {
            program_id,
            all,
            balance: RefCell::new(HashMap::new()),
            storage: RefCell::new(HashMap::new()),
            allocated: RefCell::new(0),
            deallocated: RefCell::new(0),
            chain,
            pda: Pda::new(program_id, chain),
        }
    }
    pub fn info_addr(&self, address: &H160, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let mut balance = self.balance.borrow_mut();

        if let Some(&info) = balance.get(address) {
            Ok(info)
        } else {
            let (key, seed) = self.pda.balance_key(address);
            let info = self.info_pda(key, seed, AccountType::Balance, or_create)?;
            balance.insert(*address, info);
            Ok(info)
        }
    }
    pub fn info_slot(
        &self,
        address: &H160,
        slot: &U256,
        or_create: bool,
    ) -> Result<(&'a AccountInfo<'a>, usize)> {
        let mut storage = self.storage.borrow_mut();
        let addr_slot = (*address, *slot);
        let (index_be, subindex) = Pda::storage_index(slot);

        let info = if let Some(&info) = storage.get(&addr_slot) {
            info
        } else {
            let base = self.info_addr(address, or_create)?.key;
            let (key, seed) = self.pda.storage_key(base, index_be);
            let info = self.info_pda(key, seed, AccountType::Storage, or_create)?;

            storage.insert(addr_slot, info);
            info
        };

        Ok((info, subindex))
    }
    pub fn info_tx_holder(&self, index: u64, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let signer = self.signer()?;
        let (key, seed) = self.pda.tx_holder_key(signer.key, index);
        self.info_pda(key, seed, AccountType::TxHolder, or_create)
    }
    pub fn info_state_holder(&self, index: u64, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let signer = self.signer()?;
        let (key, seed) = self.pda.state_holder_key(signer.key, index);
        self.info_pda(key, seed, AccountType::StateHolder, or_create)
    }
    pub fn info_ro_lock(&self, key: &Pubkey, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let (key, seed) = self.pda.ro_lock_key(key);
        self.info_pda(key, seed, AccountType::RoLock, or_create)
    }
    pub fn info_signer_reg(&self, key: &Pubkey, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let (key, seed) = self.pda.signer_info_key(key);
        self.info_pda(key, seed, AccountType::SignerInfo, or_create)
    }
    pub fn info_owner_reg(&self, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let (key, seed) = self.pda.owner_info_key();
        self.info_pda(key, seed, AccountType::OwnerInfo, or_create)
    }

    pub fn info_pda(
        &self,
        key: Pubkey,
        seed: Seed,
        typ: AccountType,
        or_create: bool,
    ) -> Result<&'a AccountInfo<'a>> {
        let info = self
            .all
            .get(&key)
            .cloned()
            .ok_or(AccountNotFound(key, typ.clone()))?;

        if system_program::check_id(info.owner) && or_create {
            self.create_pda(info, &seed, &typ)?;
        }

        AccountType::is_ok(info, typ, self.program_id)?;
        Ok(info)
    }
    pub fn info_sys(&self, key: &Pubkey) -> Result<&'a AccountInfo<'a>> {
        for id in [&recent_blockhashes::ID, &system_program::ID] {
            if key == id {
                return self
                    .all
                    .get(key)
                    .cloned()
                    .ok_or(SystemAccountNotFound(*key));
            }
        }
        panic!("Try to use non-pda account: {}", key)
    }

    pub fn signer(&self) -> Result<&'a AccountInfo<'a>> {
        let mut signer = None;

        for &info in self.all.values() {
            if info.is_signer && info.is_writable {
                if signer.is_some() {
                    return Err(InvalidSigner);
                }
                signer = Some(info)
            }
        }

        signer.ok_or(InvalidSigner)
    }
    pub fn realloc(&self, info: &'a AccountInfo<'a>, len: usize) -> Result<()> {
        if info.data_len() == len {
            return Ok(());
        }
        assert_eq!(info.owner, self.program_id());
        if info.data_len() < len {
            self.inc_allocated(len.saturating_sub(info.data_len()))?
        } else {
            self.inc_deallocated(info.data_len().saturating_sub(len))?
        }
        info.realloc(len, false)?;
        let rent = Rent::get()?.minimum_balance(info.data_len());

        let signer = self.signer()?;
        match rent.cmp(&info.lamports()) {
            Greater => {
                let sys = self.info_sys(&system_program::ID)?;

                invoke_signed(
                    &system_instruction::transfer(signer.key, info.key, rent - info.lamports()),
                    &[signer.clone(), info.clone(), sys.clone()],
                    &[],
                )?;
            }
            Less => {
                let refund = info.lamports() - rent;
                **info.try_borrow_mut_lamports()? -= refund;
                **signer.try_borrow_mut_lamports()? += refund;
            }
            _ => {}
        }

        Ok(())
    }
    pub fn create_pda(
        &self,
        pda: &'a AccountInfo<'a>,
        seed: &Seed,
        typ: &AccountType,
    ) -> Result<()> {
        assert_eq!(pda.lamports(), 0);
        assert_eq!(pda.data_len(), 0);
        assert!(!pda.is_signer);
        assert!(pda.is_writable);
        assert_eq!(*pda.owner, system_program::ID);

        let system = self.info_sys(&system_program::ID)?;
        let len = Pda::empty_size(pda, typ);
        let rent = Rent::get()?.minimum_balance(len);
        let payer = self.signer()?;
        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                pda.key,
                rent,
                len as u64,
                self.program_id,
            ),
            &[payer.clone(), pda.clone(), system.clone()],
            &[seed.cast().as_slice()],
        )?;
        assert_eq!(pda.owner, self.program_id);
        self.inc_allocated(len)?;
        msg!("pda is created {}", pda.key);

        Pda::init(pda, typ)
    }
    pub fn all(&self) -> &HashMap<Pubkey, &'a AccountInfo<'a>> {
        &self.all
    }
    pub fn inc_allocated(&self, len: usize) -> Result<()> {
        if len > 0 {
            if *self.deallocated.borrow() > 0 {
                return Err(AllocationError(
                    "error to allocate account data: deallocation found".to_string(),
                ));
            }
            let mut allocated = self.allocated.borrow_mut();
            *allocated = allocated.saturating_add(len);
        }

        Ok(())
    }
    pub fn inc_deallocated(&self, len: usize) -> Result<()> {
        if len > 0 {
            if *self.allocated.borrow() > 0 {
                return Err(AllocationError(
                    "error to deallocate account data: allocation found".to_string(),
                ));
            }
            let mut deallocated = self.deallocated.borrow_mut();
            *deallocated = deallocated.saturating_add(len);
        }

        Ok(())
    }
}
