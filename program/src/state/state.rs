use crate::{pda_signer_info, SignerInfo};
use {
    crate::{
        accounts::Data, error::RomeProgramError::*, error::*, origin::Origin, pda_balance,
        pda_ro_lock, pda_state_holder, pda_storage, pda_tx_holder, storage_index, AccountState,
        AccountType, AddressTable, RoLock, Seed, StateHolder, TxHolder,
    },
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
}

#[allow(dead_code)]
impl<'a> State<'a> {
    pub fn new(program_id: &'a Pubkey, accounts: &'a [AccountInfo<'a>]) -> Self {
        let keys = accounts.iter().map(|a| *a.key);
        let all = HashMap::from_iter(keys.zip(accounts.iter()));

        Self {
            program_id,
            all,
            balance: RefCell::new(HashMap::new()),
            storage: RefCell::new(HashMap::new()),
            allocated: RefCell::new(0),
            deallocated: RefCell::new(0),
        }
    }

    pub fn info_addr(&self, address: &H160, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let mut balance = self.balance.borrow_mut();
        let info = if let Some(&info) = balance.get(address) {
            info
        } else {
            let (key, seed) = pda_balance(address, self.program_id);
            let info = self
                .all
                .get(&key)
                .cloned()
                .ok_or(BalanceAccountNotFound(key, *address))?;

            if system_program::check_id(info.owner) && or_create {
                let len = AccountState::offset(info) + AccountState::size(info);
                self.create_pda(info, len, &seed, AccountType::Balance)?;
            }

            AccountType::is_ok(info, AccountType::Balance, self.program_id)?;
            balance.insert(*address, info);
            info
        };

        Ok(info)
    }
    pub fn info_slot(
        &self,
        address: &H160,
        slot: &U256,
        or_create: bool,
    ) -> Result<(&'a AccountInfo<'a>, usize)> {
        let mut storage = self.storage.borrow_mut();
        let address_slot = (*address, *slot);
        let (index_be, subindex) = storage_index(slot);

        let info = if let Some(&info) = storage.get(&address_slot) {
            info
        } else {
            let base = self.info_addr(address, or_create)?.key;
            let (key, seed) = pda_storage(base, index_be, self.program_id);
            let info = self
                .all
                .get(&key)
                .cloned()
                .ok_or(StorageAccountNotFound(key, *address, *slot))?;

            if system_program::check_id(info.owner) && or_create {
                let len = AddressTable::offset(info) + AddressTable::size(info);
                self.create_pda(info, len, &seed, AccountType::Storage)?;
            }

            AccountType::is_ok(info, AccountType::Storage, self.program_id)?;
            storage.insert(address_slot, info);
            info
        };

        Ok((info, subindex))
    }
    pub fn info_tx_holder(&self, index: u64, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let signer = self.signer()?;
        let (key, seed) = pda_tx_holder(signer.key, index, self.program_id);

        let info = self
            .all
            .get(&key)
            .cloned()
            .ok_or(TxHolderAccountNotFound(key, index))?;

        if system_program::check_id(info.owner) && or_create {
            msg!("info_tx_holde: info.data_len() = {}", info.data_len());
            let len = TxHolder::offset(info) + TxHolder::size(info);
            self.create_pda(info, len, &seed, AccountType::TxHolder)?;
        }
        AccountType::is_ok(info, AccountType::TxHolder, self.program_id)?;

        Ok(info)
    }
    pub fn info_state_holder(&self, index: u64, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let signer = self.signer()?;
        let (key, seed) = pda_state_holder(signer.key, index, self.program_id);

        let info = self
            .all
            .get(&key)
            .cloned()
            .ok_or(StateHolderAccountNotFound(key, index))?;

        if system_program::check_id(info.owner) && or_create {
            let len = StateHolder::offset(info) + StateHolder::size(info);
            self.create_pda(info, len, &seed, AccountType::StateHolder)?;
        }
        AccountType::is_ok(info, AccountType::StateHolder, self.program_id)?;

        Ok(info)
    }
    pub fn info_ro_lock(&self, key: &Pubkey, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let (key, seed) = pda_ro_lock(key, self.program_id);

        let info = self
            .all
            .get(&key)
            .cloned()
            .ok_or(RoLockAccountNotFound(key))?;

        if system_program::check_id(info.owner) && or_create {
            let len = RoLock::offset(info);
            self.create_pda(info, len, &seed, AccountType::RoLock)?;
        }

        AccountType::is_ok(info, AccountType::RoLock, self.program_id)?;
        Ok(info)
    }
    pub fn info_signer_info(&self, key: &Pubkey, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let (key, seed) = pda_signer_info(key, self.program_id);

        let info = self
            .all
            .get(&key)
            .cloned()
            .ok_or(SignerInfoAccountNotFound(key))?;

        if system_program::check_id(info.owner) && or_create {
            let len = SignerInfo::offset(info) + SignerInfo::size(info);
            self.create_pda(info, len, &seed, AccountType::SignerInfo)?;
        }

        AccountType::is_ok(info, AccountType::SignerInfo, self.program_id)?;
        Ok(info)
    }
    pub fn info_sys(&self, key: &Pubkey) -> Result<&'a AccountInfo<'a>> {
        for id in [&recent_blockhashes::ID, &system_program::ID] {
            if key == id {
                return self.all.get(key).cloned().ok_or(AccountNotFound(*key));
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
    pub fn pda_init(pda: &'a AccountInfo<'a>, typ: AccountType) -> Result<()> {
        match typ {
            AccountType::New => unreachable!(),
            AccountType::Balance => AccountState::init(pda),
            AccountType::Storage => AddressTable::init(pda),
            AccountType::TxHolder => TxHolder::init(pda),
            AccountType::StateHolder => StateHolder::init(pda),
            AccountType::RoLock => AccountType::init(pda, AccountType::RoLock),
            AccountType::SignerInfo => SignerInfo::init(pda),
        }
    }
    pub fn create_pda(
        &self,
        pda: &'a AccountInfo<'a>,
        len: usize,
        seed: &Seed,
        typ: AccountType,
    ) -> Result<()> {
        assert_eq!(pda.lamports(), 0);
        assert_eq!(pda.data_len(), 0);
        assert!(!pda.is_signer);
        assert!(pda.is_writable);
        assert_eq!(*pda.owner, system_program::ID);

        let system = self.info_sys(&system_program::ID)?;
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

        State::pda_init(pda, typ)
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
