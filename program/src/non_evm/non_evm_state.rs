use {
    crate::{
        error::{Result, RomeProgramError::InconsistentAccountList},
        origin::Origin,
        state::Account,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        instruction::{Instruction},
        program_pack::{IsInitialized, Pack},
        pubkey::Pubkey,
    },
    std::collections::HashMap,
};
#[cfg(target_os = "solana")]
use crate::error::RomeProgramError::ModifyReadOnlyAccount;

pub type Bind<'a> = (&'a Pubkey,  &'a mut Account);

#[derive(Clone, BorshSerialize, BorshDeserialize, Default)]
pub struct NonEvmState {
    accs: HashMap<Pubkey, Account>,     // TODO: add writeable
}

impl NonEvmState {
    #[allow(unused_variables)]
    fn load<T: Origin>(&mut self, state: &T, key: &Pubkey, writeable: bool) -> Result<()> {
        if !self.accs.contains_key(key) {
            let acc = state.account(key)?;

            #[cfg(target_os = "solana")]
            if writeable && !acc.writeable {
                return Err(ModifyReadOnlyAccount(*key))
            }

            self.accs.insert(*key, acc);
        };
        Ok(())
    }

    fn update<T: Origin>(&mut self, state: &T, ix: &Instruction) -> Result<()> {
        ix
            .accounts
            .iter()
            .map(|m| self.load(state, &m.pubkey, m.is_writable))
            .collect::<Result<Vec<_>>>()?;

        Ok(())
    }
    pub fn account_state<T, P>(&mut self, state: &T, key: &Pubkey) -> Result<P>
    where
        P: Pack + IsInitialized,
        T: Origin,
    {
        self.load(state, key, false)?;

        let acc = self.accs.get(key).unwrap();
        P::unpack(&acc.data).map_err(|e| e.into())
    }

    pub fn filter_accounts<'a, I: Iterator<Item = Bind<'a>>>(iter_mut: I, ix: &Instruction) -> Result<Vec<Bind<'a>>> {
        let mut vec = iter_mut
            .filter(|(&key, _)|
                ix.accounts.iter().any(|m| m.pubkey == key)
            )
            .collect::<Vec<_>>();

        // TODO fix it
        if vec.len() != ix.accounts.len() {
            return Err(InconsistentAccountList)
        }

        // sort
        for (i, meta) in ix.accounts.iter().enumerate() {
            let pos = vec.iter().position(|(&key, _)|  key == meta.pubkey ).unwrap();
            // TODO: it panics if instruction has duplicated accounts
            vec.swap(i, pos);
        }
        Ok(vec)

    }

    pub fn ix_accounts_mut<'b, T: Origin>(
        &'b mut self,
        state: &T,
        ix: &Instruction
    ) -> Result<Vec<Bind<'b>>> {

        self.update(state, ix)?;

        let iter_mut = self.accs.iter_mut();
        Self::filter_accounts(iter_mut, ix)
    }
}
