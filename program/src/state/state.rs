use {
    super::{
        base::Base,
        pda::{Pda, Seed},
    },
    crate::{error::RomeProgramError::*, error::*, AccountType, OwnerInfo, Data},
    evm::{H160, U256},
    solana_program::{
        account_info::AccountInfo, pubkey::Pubkey, rent::Rent,
        system_instruction, system_program, sysvar::recent_blockhashes, sysvar::Sysvar,
    },
    std::{cmp::Ordering::*, collections::HashMap, iter::FromIterator, ops::Deref},
};
use crate::origin::Origin;

pub struct State<'a> {
    all: HashMap<Pubkey, &'a AccountInfo<'a>>,
    pub base: Base<'a>,
    pub signer: &'a AccountInfo<'a>,
}

impl<'a> Deref for State<'a> {
    type Target = Base<'a>;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

#[allow(dead_code)]
impl<'a> State<'a> {
    pub fn new(
        program_id: &'a Pubkey,
        accounts: &'a [AccountInfo<'a>],
        chain: u64,
    ) -> Result<Self> {
        let state = Self::new_unchecked(program_id, accounts, chain)?;
        let info = state.info_owner_reg(false)?;
        OwnerInfo::check_chain(info, chain)?;

        Ok(state)
    }
    pub fn new_unchecked(
        program_id: &'a Pubkey,
        accounts: &'a [AccountInfo<'a>],
        chain: u64,
    ) -> Result<Self> {
        let keys = accounts.iter().map(|a| *a.key);
        let all = HashMap::from_iter(keys.zip(accounts.iter()));
        let signer = Self::signer(&all)?;

        Ok(Self {
            all,
            base: Base::new(program_id, chain),
            signer,
        })
    }
    pub fn info_addr(&self, address: &H160, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let (key, seed) = self.pda.balance_key(address);
        self.info_pda(&key, &seed, AccountType::Balance, or_create)
    }
    pub fn info_slot(
        &self,
        address: &H160,
        slot: &U256,
        or_create: bool,
    ) -> Result<(&'a AccountInfo<'a>, u8)> {
        let (key, seed, sub_ix) = self.slot_to_key(address, slot);
        let info = self.info_pda(&key, &seed, AccountType::Storage, or_create)?;

        Ok((info, sub_ix))
    }
    pub fn info_tx_holder(&self, index: u64, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let (key, seed) = self.pda.tx_holder_key(self.signer.key, index);
        self.info_pda(&key, &seed, AccountType::TxHolder, or_create)
    }
    pub fn info_state_holder(&self, index: u64, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let (key, seed) = self.pda.state_holder_key(self.signer.key, index);
        self.info_pda(&key, &seed, AccountType::StateHolder, or_create)
    }
    pub fn info_ro_lock(&self, base: &Pubkey, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let (key, seed) = self.pda.ro_lock_key(base);
        self.info_pda(&key, &seed, AccountType::RoLock, or_create)
    }
    pub fn info_owner_reg(&self, or_create: bool) -> Result<&'a AccountInfo<'a>> {
        let (key, seed) = self.pda.owner_info_key();
        self.info_pda(&key, &seed, AccountType::OwnerInfo, or_create)
    }

    pub fn info_pda(
        &self,
        key: &Pubkey,
        seed: &Seed,
        typ: AccountType,
        or_create: bool,
    ) -> Result<&'a AccountInfo<'a>> {
        let info = self
            .all
            .get(key)
            .cloned()
            .ok_or(PdaAccountNotFound(*key, typ.clone()))?;

        if system_program::check_id(info.owner) && or_create {
            self.create_pda(info, seed, &typ)?;
        }

        AccountType::is_ok(info, typ, self.program_id)?;
        Ok(info)
    }

    pub fn info_sol_wallet(&self, or_create: bool) -> Result<&'a AccountInfo<'a>>{
        let (key, seed) = self.pda.sol_wallet();
        let info = self
            .all
            .get(&key)
            .cloned()
            .ok_or(AccountNotFound(key))?;

        if info.lamports() == 0  {
            if !or_create {
                return Err(AccountNotFound(*info.key))
            }

            assert_eq!(info.data_len(), 0);
            assert!(info.is_writable);
            assert_eq!(*info.owner, system_program::ID);

            let rent = Rent::get()?.minimum_balance(0);
            let ix = &system_instruction::create_account(
                self.signer.key,
                info.key,
                rent,
                0,
                &system_program::ID,
            );
            self.invoke_signed(&ix, &seed, false)?;
            assert!(info.lamports() > 0);
        }

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

    fn signer(map: &HashMap<Pubkey, &'a AccountInfo<'a>>) -> Result<&'a AccountInfo<'a>> {
        let mut signer = None;

        for &info in map.values() {
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
        assert_eq!(info.owner, self.program_id);
        if info.data_len() < len {
            self.inc_alloc(len.saturating_sub(info.data_len()))?
        } else {
            self.inc_dealloc(info.data_len().saturating_sub(len))?
        }

        info.realloc(len, false)?;

        let rent = Rent::get()?.minimum_balance(info.data_len());
        let is_paid = AccountType::from_account(info)?.is_paid();

        match rent.cmp(&info.lamports()) {
            Greater => {
                let ix = system_instruction::transfer(self.signer.key, info.key, rent - info.lamports());
                self.invoke_signed(&ix, &Seed::default(), is_paid)?;
            }
            Less => {
                let refund = info.lamports() - rent;
                **info.try_borrow_mut_lamports()? -= refund;
                **self.signer.try_borrow_mut_lamports()? += refund;
                if is_paid {
                    self.add_refund(refund)?;
                }
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

        let len = Pda::empty_size(pda, typ);
        let rent = Rent::get()?.minimum_balance(len);

        let ix = system_instruction::create_account(
            self.signer.key,
            pda.key,
            rent,
            len as u64,
            self.program_id,
        );

        self.invoke_signed(&ix, seed, typ.is_paid())?;
        assert_eq!(pda.owner, self.program_id);
        self.inc_alloc(len)?;
        Pda::init(pda, typ)
    }
    pub fn all(&self) -> &HashMap<Pubkey, &'a AccountInfo<'a>> {
        &self.all
    }
}
