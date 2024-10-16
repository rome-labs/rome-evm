use {
    crate::{
        context::ContextIterative,
        error::{Result, RomeProgramError::InvalidInstructionData},
        state::State,
        vm::{vm_iterative::MachineIterative::FromStateHolder, Execute, Vm},
        Holder,
    },
    evm::H256,
    solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey},
    std::{convert::TryInto, mem::size_of},
};

// unique | holder_index | tx_hash | chain_id | lock_overrides
pub fn args(data: &[u8]) -> Result<(u64, H256, u64, &[u8])> {
    if data.len() < size_of::<u64>() + size_of::<u64>() + size_of::<H256>() + size_of::<u64>() {
        return Err(InvalidInstructionData);
    }
    let (_, right) = data.split_at(size_of::<u64>());
    let (left, right) = right.split_at(size_of::<u64>());
    let holder = u64::from_le_bytes(left.try_into().unwrap());

    let (left, right) = right.split_at(size_of::<H256>());
    let hash = H256::from_slice(left);

    let (left, lock_overrides) = right.split_at(size_of::<u64>());
    let chain = u64::from_le_bytes(left.try_into().unwrap());

    Ok((holder, hash, chain, lock_overrides))
}

pub fn do_tx_holder_iterative<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    msg!("Instruction: Iterative transaction from holder");

    let (holder, hash, chain, lock_overrides) = args(data)?;
    let state = State::new(program_id, accounts, chain)?;
    let info = state.info_tx_holder(holder, false)?;
    let tx = Holder::tx(info, hash, chain)?;

    let context = ContextIterative::new(&state, accounts, holder, lock_overrides, tx, hash)?;
    let mut vm = Vm::new_iterative(&state, &context)?;
    vm.consume(FromStateHolder)
}
