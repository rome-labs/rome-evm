use {
    crate::state::{State,},
    rome_evm::{
        context::account_lock::AccountLock,
        error::{Result, RomeProgramError::*},
        info::Info,
        origin::Origin,
        non_evm::dispatcher,
        Base, Code, Data, Account, H160, H256, U256, pda::Seed, non_evm::NonEvmState,
    },
    solana_program::{
        account_info::IntoAccountInfo, clock::Slot, instruction::Instruction, pubkey::Pubkey,
        sysvar::recent_blockhashes,
    },
    std::{
        cmp::Ordering::{Greater, Less},
    },
};

impl Info for State<'_> {}

impl Origin for State<'_> {
    fn nonce(&self, address: &H160) -> Result<u64> {
        let mut bind = self.info_addr(address, false)?;
        let info = bind.into_account_info();
        Info::nonce(self, &info)
    }
    fn balance(&self, address: &H160) -> Result<U256> {
        let mut bind = self.info_addr(address, false)?;
        let info = bind.into_account_info();
        Info::balance(self, &info)
    }
    fn code(&self, address: &H160) -> Result<Vec<u8>> {
        let mut bind = self.info_addr(address, false)?;
        let info = bind.into_account_info();
        Info::code(self, &info)
    }
    fn valids(&self, address: &H160) -> Result<Vec<u8>> {
        let mut bind = self.info_addr(address, false)?;
        let info = bind.into_account_info();
        Info::valids(self, &info)
    }
    fn storage(&self, address: &H160, slot: &U256) -> Result<Option<U256>> {
        let (mut bind, sub_ix) = self.info_slot(address, slot, false)?;
        let info = bind.into_account_info();
        Info::storage(self, &info, sub_ix)
    }
    fn inc_nonce<L: AccountLock>(&self, address: &H160, context: &L) -> Result<()> {
        let mut bind = self.info_addr(address, true)?;
        let info = bind.into_account_info();
        Info::inc_nonce(self, &info, context)?;
        self.update(bind);
        Ok(())
    }
    fn add_balance<L: AccountLock>(
        &self,
        address: &H160,
        balance: &U256,
        context: &L,
    ) -> Result<()> {
        let mut bind = self.info_addr(address, true)?;
        let info = bind.into_account_info();
        Info::add_balance(self, &info, balance, context)?;
        self.update(bind);
        Ok(())
    }
    fn sub_balance<L: AccountLock>(
        &self,
        address: &H160,
        balance: &U256,
        context: &L,
    ) -> Result<()> {
        let mut bind = self.info_addr(address, true)?;
        let info = bind.into_account_info();
        Info::sub_balance(self, &info, balance, address, context)?;
        self.update(bind);
        Ok(())
    }
    fn set_code<L: AccountLock>(
        &self,
        address: &H160,
        code: &[u8],
        valids: &[u8],
        context: &L,
    ) -> Result<()> {
        let mut bind = self.info_addr(address, true)?;

        let len = bind.1.data.len();
        let offset = Code::offset(&bind.into_account_info());
        let required = offset + code.len() + valids.len();

        match len.cmp(&required) {
            Less => {
                self.realloc(&mut bind, required)?;
            }
            Greater => {
                // TODO: implement deallocation of the unused contract space
                return Err(Unimplemented(
                    format!(
                        "the contract deployment space must be deallocated according to size of the contract \
                        pubkey: {}, available: {}, required: {}",
                        bind.0,
                        len,
                        required
                    ))
                );
            }
            _ => {}
        }

        let info = bind.into_account_info();
        Info::set_code(self, &info, code, valids, address, context)?;
        self.update(bind);
        Ok(())
    }
    fn set_storage<L: AccountLock>(
        &self,
        address: &H160,
        slot: &U256,
        value: &U256,
        context: &L,
    ) -> Result<()> {
        let (mut bind, sub_ix) = self.info_slot(address, slot, true)?;
        let info = bind.into_account_info();
        Info::set_storage(self, &info, sub_ix, value, context)?;
        self.update(bind);
        Ok(())
    }
    fn block_hash(&self, block: U256, slot: Slot) -> Result<H256> {
        let mut bind = self.info_sys(&recent_blockhashes::ID)?;
        let info = bind.into_account_info();
        Info::block_hash(self, &info, block, slot)
    }
    fn base(&self) -> &Base {
        &self.base
    }

    fn account(&self, key: &Pubkey) -> Result<Account> {
        let bind = self.info_external(key, false)?;

        Ok(bind.1)
    }

    fn invoke_signed(&self, ix: &Instruction, _: Seed) -> Result<()> {

        let f_len = |vec: &Vec<(&Pubkey, &mut Account)>| -> Vec<usize>{
            vec
                .iter()
                .map(|(_, a)| a.data.len())
                .collect::<Vec<usize>>()
        };

        let f_len_dif = |a: &Vec<usize>, b: &Vec<usize>| -> usize {
            a
                .iter()
                .zip(b)
                .map(|(&x, &y)| x.saturating_sub(y))
                .collect::<Vec<usize>>()
                .iter()
                .sum()
        };

        let _ = self.info_program(&ix.program_id)?;

        for meta in ix.accounts.iter() {
            let _ = self.info_external(&meta.pubkey, meta.is_writable)?;
        }

        let mut accs = self.accounts.borrow_mut();

        let iter = accs
            .iter_mut()
            .map(|(a, b )| (a, &mut b.account));
        let binds = NonEvmState::filter_accounts(iter, ix)?;

        let old = f_len(&binds);
        let program = dispatcher(ix, self);

        program.emulate(ix, binds)?;

        // TODO remove the code duplication
        let iter = accs
            .iter_mut()
            .map(|(a, b )| (a, &mut b.account));
        let binds = NonEvmState::filter_accounts(iter, ix)?;

        let new = f_len(&binds);
        let alloc = f_len_dif(&new, &old);
        let dealloc = f_len_dif(&old, &new);

        Base::inc_alloc(&self.base, alloc)?;
        Base::inc_alloc_payed(&self.base, alloc)?;
        Base::inc_dealloc(&self.base, dealloc)?;
        Base::inc_dealloc_payed(&self.base, dealloc)?;

        self.syscall.inc();
        Ok(())
    }

    fn signer(&self) -> Pubkey {
        self.signer.unwrap()
    }
}
