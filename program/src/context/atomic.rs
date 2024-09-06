use {
    super::{gas_recipient, tx_from_holder, AccountLock, Context},
    crate::{
        accounts::Iterations,
        api::do_tx_holder,
        error::Result,
        state::{origin::Origin, Allocate},
        tx::tx::Tx,
        vm::{vm_iterative::MachineIterative, Vm},
        Instruction, State,
    },
    evm::{H160, H256},
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
                let (index, hash) = do_tx_holder::args(self.data)?;
                let info = self.state.info_tx_holder(index, false)?;
                tx_from_holder(info, hash)
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
