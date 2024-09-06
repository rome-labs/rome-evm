use {
    crate::{
        context::ContextIterative,
        error::{Result, RomeProgramError::InvalidInstructionData},
        state::State,
        vm::{vm_iterative::MachineIterative::FromStateHolder, Execute, Vm},
        Instruction::DoTxIterative,
    },
    solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey},
    std::{convert::TryInto, mem::size_of},
};

// unique | holder_index | overrides_len | overrides | tx
pub fn args(data: &[u8]) -> Result<(u64, &[u8], &[u8])> {
    if data.len() <= size_of::<u64>() * 3 {
        return Err(InvalidInstructionData);
    }

    let (_, right) = data.split_at(size_of::<u64>());
    let (left, right) = right.split_at(size_of::<u64>());
    let holder = u64::from_le_bytes(left.try_into().unwrap());
    let (left, right) = right.split_at(size_of::<u64>());
    let len = u64::from_le_bytes(left.try_into().unwrap()) as usize;

    if right.len() <= len {
        return Err(InvalidInstructionData);
    }
    let (lock_overrides, tx) = right.split_at(len);

    Ok((holder, lock_overrides, tx))
}

pub fn do_tx_iterative<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    msg!("Instruction: Iterative transaction");
    let state = State::new(program_id, accounts);
    let context = ContextIterative::new(&state, accounts, data, DoTxIterative)?;
    let mut vm = Vm::new_iterative(&state, &context)?;
    vm.consume(FromStateHolder)
}
