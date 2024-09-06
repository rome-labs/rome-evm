use {
    crate::{
        context::ContextAtomic,
        error::Result,
        state::State,
        vm::{vm_atomic::MachineAtomic, Execute, Vm},
        Instruction::DoTx,
    },
    solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey},
};

pub fn do_tx<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    msg!("Instruction: Atomic transaction");
    let state = State::new(program_id, accounts);
    let context = ContextAtomic::new(&state, data, DoTx);
    let mut vm = Vm::new_atomic(&state, &context)?;
    vm.consume(MachineAtomic::Lock)
}
