use {
    super::gas_recipient,
    crate::{state::State, Instruction},
    rome_evm::{
        accounts::{Data, Lock},
        context::{account_lock::AccountLock, tx_from_holder, Context},
        error::{Result, RomeProgramError::*},
        state::{origin::Origin, Allocate},
        tx::tx::Tx,
        vm::{vm_iterative::MachineIterative, Vm},
        Iterations, H160, H256,
    },
    solana_program::account_info::IntoAccountInfo,
};

pub struct ContextAtomic<'a, 'b> {
    pub state: &'b State<'a>,
    pub data: &'a [u8],
    pub instr: Instruction,
}
impl<'a, 'b> ContextAtomic<'a, 'b> {
    pub fn new(state: &'b State<'a>, data: &'a [u8], instr: Instruction) -> Self {
        Self { state, data, instr }
    }
}

impl<'a, 'b> Context for ContextAtomic<'a, 'b> {
    fn tx(&self) -> Result<Tx> {
        match self.instr {
            Instruction::DoTx => Tx::from_instruction(self.data),
            Instruction::DoTxHolder => {
                let (index, hash) = rome_evm::api::do_tx_holder::args(self.data)?;
                let mut bind = self.state.info_tx_holder(index, false)?;
                let info = bind.into_account_info();
                tx_from_holder(&info, hash)
            }
            _ => unreachable!(),
        }
    }
    fn save_iteration(&self, _: Iterations) -> Result<()> {
        unreachable!()
    }
    fn restore_iteration(&self) -> Result<Iterations> {
        unreachable!()
    }
    fn serialize_vm<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        _: &Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        unreachable!()
    }
    fn deserialize_vm<T: Origin + Allocate, L: AccountLock + Context>(
        &self,
        _: &mut Vm<T, MachineIterative, L>,
    ) -> Result<()> {
        unreachable!()
    }
    fn allocate_holder(&self) -> Result<()> {
        unreachable!()
    }
    fn bind_tx_to_holder(&self) -> Result<()> {
        unreachable!()
    }
    fn is_tx_binded_to_holder(&self) -> Result<bool> {
        unreachable!()
    }
    fn tx_hash(&self) -> H256 {
        unreachable!()
    }
    fn gas_recipient(&self) -> Result<Option<H160>> {
        gas_recipient(self.state)
    }
}

impl AccountLock for ContextAtomic<'_, '_> {
    fn lock(&self) -> Result<()> {
        let accounts = self.state.accounts.borrow();
        for (key, item) in accounts.iter() {
            let mut bind = (*key, item.account.clone());
            let mut info = bind.into_account_info();
            info.is_writable = item.writable;

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
    fn lock_new_one(&self) -> Result<()> {
        unreachable!()
    }
}
