use {
    crate::state::State,
    rome_evm::{
        context::{AccountLock, atomic::lock_impl,},
        error::Result,
    },
    solana_program::account_info::{AccountInfo, IntoAccountInfo},
};

pub struct ContextAt<'a, 'b> {
    pub state: &'b State<'a>,
}
impl<'a, 'b> ContextAt<'a, 'b> {
    pub fn new(state: &'b State<'a>) -> Self {
        Self {
            state,
        }
    }
}

impl AccountLock for ContextAt<'_, '_> {
    fn lock(&self) -> Result<()> {
        let accounts = self.state.accounts.borrow();
        for (key, item) in accounts.iter() {
            let mut bind = (*key, item.account.clone());
            let mut info = bind.into_account_info();
            info.is_writable = item.account.writable;

            lock_impl(&info, self.state.program_id)?;
        }

        Ok(())
    }
    fn locked(&self) -> Result<bool> {
        unreachable!()
    }
    fn unlock(&self) -> Result<()> {
        unreachable!()
    }
    fn lock_new_one(&self, _info: &AccountInfo) -> Result<()> {
        unreachable!()
    }
    fn check_writable(&self, _info: &AccountInfo) -> Result<()> {
        Ok(())
    }
}
