use {
    crate::{
        context::ContextAtomic,
        error::{Result, RomeProgramError::*},
        state::State,
        vm::{vm_atomic::MachineAtomic, Execute, Vm},
        Instruction::DoTxHolder,
    },
    evm::H256,
    solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey},
    std::{convert::TryInto, mem::size_of},
};

// holder_index | tx_hash
pub fn args(data: &[u8]) -> Result<(u64, H256)> {
    if data.len() != size_of::<u64>() + size_of::<H256>() {
        return Err(InvalidInstructionData);
    }

    let (left, right) = data.split_at(size_of::<u64>());
    let index = u64::from_le_bytes(left.try_into().unwrap());
    let hash = H256::from_slice(right);

    Ok((index, hash))
}

pub fn do_tx_holder<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    msg!("Instruction: Atomic transaction from holder");
    let state = State::new(program_id, accounts);
    let context = ContextAtomic::new(&state, data, DoTxHolder);
    let mut vm = Vm::new_atomic(&state, &context)?;
    vm.consume(MachineAtomic::Lock)
}
