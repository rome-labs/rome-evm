use {
    crate::{
        context::ContextAtomic,
        error::Result,
        state::State,
        tx::tx::Tx,
        vm::{vm_atomic::MachineAtomic, Execute, Vm},
    },
    solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey},
};

pub fn do_tx<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    msg!("Instruction: Atomic transaction");

    let tx = Tx::from_instruction(data)?;
    let state = State::new(program_id, accounts, tx.chain_id())?;
    let context = ContextAtomic::new(&state, tx);
    let mut vm = Vm::new_atomic(&state, &context)?;
    vm.consume(MachineAtomic::Lock)
}
