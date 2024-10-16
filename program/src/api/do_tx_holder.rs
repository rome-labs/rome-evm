use {
    crate::{
        context::ContextAtomic,
        error::{Result, RomeProgramError::*},
        state::State,
        vm::{vm_atomic::MachineAtomic, Execute, Vm},
        Holder,
    },
    evm::H256,
    solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey},
    std::{convert::TryInto, mem::size_of},
};

// holder_index | tx_hash | chain_id
pub fn args(data: &[u8]) -> Result<(u64, H256, u64)> {
    if data.len() != size_of::<u64>() + size_of::<H256>() + size_of::<u64>() {
        return Err(InvalidInstructionData);
    }

    let (left, right) = data.split_at(size_of::<u64>());
    let index = u64::from_le_bytes(left.try_into().unwrap());

    let (left, right) = right.split_at(size_of::<H256>());
    let hash = H256::from_slice(left);

    let chain = u64::from_le_bytes(right.try_into().unwrap());

    Ok((index, hash, chain))
}

pub fn do_tx_holder<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    msg!("Instruction: Atomic transaction from holder");

    let (holder, hash, chain) = args(data)?;
    let state = State::new(program_id, accounts, chain)?;

    let info = state.info_tx_holder(holder, false)?;
    let tx = Holder::tx(info, hash, chain)?;
    let context = ContextAtomic::new(&state, tx);

    let mut vm = Vm::new_atomic(&state, &context)?;
    vm.consume(MachineAtomic::Lock)
}
