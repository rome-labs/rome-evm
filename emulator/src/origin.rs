use {
    crate::{ state::{State,}, Item},
    rome_evm::{
        context::AccountLock,
        error::{Result, RomeProgramError::*},
        info::Info,
        origin::Origin,
        Base, Code, Data, Account, H160, H256, U256, pda::Seed,
        non_evm::{ASplToken, Program, SplToken, System, Bind as Bind_,
                  non_evm_state::filter_accounts},
    },
    solana_program::{
        account_info::IntoAccountInfo, clock::Slot, instruction::Instruction, pubkey::Pubkey,
        sysvar::recent_blockhashes,
    },
    std::{
        cmp::Ordering::{Greater, Less}, cell::RefMut, collections::BTreeMap
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
                self.realloc(&bind.0, required)?;
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

        let mut bind = self.info_addr(address, false)?;
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

    fn invoke_signed(&self, ix: &Instruction, _: &Seed, refund_to_signer: bool) -> Result<()> {
        let _ = self.info_program(&ix.program_id)?;
        let signer = &self.signer.unwrap();

        for meta in ix.accounts.iter() {
            let _ = self.info_external(&meta.pubkey, meta.is_writable)?;
        }

        let mut accs = self.accounts.borrow_mut();

        let lmp_old = lamports(&mut accs, &signer)?;
        let binds = filter(&mut accs, ix)?;
        let len_old = data_len(&binds);

        let program = non_evm_program(ix, self);

        if refund_to_signer {
            program.emulate(ix, binds)?;

            let lmp_new = lamports(&mut accs, &signer)?;

            if lmp_old > lmp_new {
                self.add_fee(lmp_old - lmp_new)?;
            } else {
                self.add_refund(lmp_new - lmp_old)?;
            }
        } else {
            program.emulate(ix, binds)?;
        }

        let binds = filter(&mut accs, ix)?;
        let len_new = data_len(&binds);
        let alloc = data_len_diff(&len_new, &len_old);
        let dealloc = data_len_diff(&len_old, &len_new);

        self.inc_space_counter(alloc, dealloc, refund_to_signer)?;
        self.syscall.inc();
        Ok(())
    }

    fn signer(&self) -> Pubkey {
        self.signer.unwrap()
    }
}


fn non_evm_program<'a, T: Origin>(ix: &Instruction, state: &'a T) -> Box<dyn Program + 'a> {
    use solana_program::system_program;

    match ix.program_id {
        ::spl_token::ID => Box::new(SplToken::new(state)),
        spl_associated_token_account::ID => Box::new(ASplToken::new(state)),
        system_program::ID => Box::new(System::new(state)),
        _ => unimplemented!()
    }
}

fn data_len(vec: &Vec<Bind_>) -> Vec<usize>{
    vec
        .iter()
        .map(|(_, a)| a.data.len())
        .collect::<Vec<usize>>()
}

fn data_len_diff(a: &Vec<usize>, b: &Vec<usize>) -> usize{
    a
        .iter()
        .zip(b)
        .map(|(&x, &y)| x.saturating_sub(y))
        .collect::<Vec<usize>>()
        .iter()
        .sum()
}

fn filter<'a>(accs: &'a mut RefMut<BTreeMap<Pubkey, Item>>, ix: &Instruction) -> Result<Vec<Bind_<'a>>>{
    let iter = accs
        .iter_mut()
        .map(|(a, b )| (a, &mut b.account));

    filter_accounts(iter, ix)
}
fn lamports<'a>(accs: &'a mut RefMut<BTreeMap<Pubkey, Item>>, key: &Pubkey) -> Result<u64>{
    let lamports = accs
        .get(&key)
        .ok_or(AccountNotFound(*key))?
        .account
        .lamports;
    Ok(lamports)
}
