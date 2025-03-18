use {
    crate::state::State,
    rome_evm::{
        accounts::{Data, Lock},
        context::{account_lock::AccountLock, Context},
        error::{Result, RomeProgramError::*},
        state::{origin::Origin, Allocate},
        tx::tx::Tx,
        vm::{vm_iterative::MachineIterative, Vm},
        Iterations, H160, H256,
    },
    solana_program::account_info::{AccountInfo, IntoAccountInfo},
};

pub struct ContextAtomic<'a, 'b> {
    pub state: &'b State<'a>,
    pub rlp: &'b [u8],
    pub fee_addr: Option<H160>,
}
impl<'a, 'b> ContextAtomic<'a, 'b> {
    pub fn new(state: &'b State<'a>, rlp: &'b [u8], fee_addr: Option<H160>) -> Self {
        Self {
            state,
            rlp,
            fee_addr,
        }
    }
}

impl<'a, 'b> Context for ContextAtomic<'a, 'b> {
    fn tx(&self) -> Result<Tx> {
        Tx::from_instruction(self.rlp)
    }
    fn save_iteration(&self, _: Iterations) -> Result<()> {
        unreachable!()
    }
    fn restore_iteration(&self) -> Result<Iterations> {
        unreachable!()
    }
    fn serialize<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        _: &Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        unreachable!()
    }
    fn deserialize<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        _: &mut Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        unreachable!()
    }
    fn allocate_holder(&self) -> Result<()> {
        unreachable!()
    }
    fn new_session(&self) -> Result<()> {
        unreachable!()
    }
    fn exists_session(&self) -> Result<bool> {
        unreachable!()
    }
    fn tx_hash(&self) -> H256 {
        unreachable!()
    }
    fn fee_recipient(&self) -> Option<H160> {
        self.fee_addr
    }
    fn state_holder_len(&self) -> Result<usize> {
        unreachable!()
    }
}

impl AccountLock for ContextAtomic<'_, '_> {
    fn lock(&self) -> Result<()> {
        let accounts = self.state.accounts.borrow();
        for (key, item) in accounts.iter() {
            let mut bind = (*key, item.account.clone());
            let mut info = bind.into_account_info();
            info.is_writable = item.account.writeable;

            if Lock::is_managed(&info, self.state.program_id)? && info.is_writable {
                let lock = Lock::from_account_mut(&info)?;
                if lock.get()?.is_some() {
                    return Err(AccountLocked(*info.key, lock.lock));
                }
            }
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
