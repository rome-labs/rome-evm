use {
    super::{AccountLock, Context},
    crate::{
        accounts::Iterations,
        error::Result,
        state::{origin::Origin, Allocate},
        tx::tx::Tx,
        vm::{vm_iterative::MachineIterative, Vm},
        State,
    },
    evm::{H160, H256},
};

pub struct ContextAtomic<'a, 'b> {
    pub state: &'b State<'a>,
    pub tx: Tx,
    pub fee_addr: Option<H160>,
}
impl<'a, 'b> ContextAtomic<'a, 'b> {
    pub fn new(state: &'b State<'a>, tx: Tx, fee_addr: Option<H160>) -> Self {
        Self {
            state,
            tx,
            fee_addr,
        }
    }
}

impl<'a, 'b> Context for ContextAtomic<'a, 'b> {
    fn tx(&self) -> &Tx {
        &self.tx
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
}
