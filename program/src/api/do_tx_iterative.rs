use {
    crate::{
        context::ContextIterative,
        error::{Result, RomeProgramError::InvalidInstructionData},
        split_fee, split_u64,
        state::State,
        tx::tx::Tx,
        vm::{vm_iterative::MachineIterative::FromStateHolder, Execute, Vm},
        H160, H256,
    },
    solana_program::{account_info::AccountInfo, keccak, msg, pubkey::Pubkey},
};

// unique | session | holder_index | Option<fee_recipient> | overrides_len | overrides | tx
#[allow(clippy::type_complexity)]
pub fn args(data: &[u8]) -> Result<(u64, u64, Option<H160>, &[u8], &[u8])> {
    let (_, data) = split_u64(data)?;
    let (session, data) = split_u64(data)?;
    let (holder, data) = split_u64(data)?;
    let (fee_addr, data) = split_fee(data)?;
    let (len, data) = split_u64(data)?;

    if data.len() < len as usize {
        return Err(InvalidInstructionData);
    }
    let (lock_overrides, tx) = data.split_at(len as usize);

    Ok((session, holder, fee_addr, lock_overrides, tx))
}

pub fn do_tx_iterative<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    msg!("Instruction: Iterative transaction");

    let (session, holder, fee_addr, lock_overrides, rlp) = args(data)?;
    let hash = H256::from(keccak::hash(rlp).to_bytes());
    let chain_id = Tx::chain_id_from_rlp(rlp)?;

    let state = State::new(program_id, accounts, chain_id)?;
    let context = ContextIterative::new(
        &state,
        accounts,
        holder,
        lock_overrides,
        rlp,
        hash,
        session,
        fee_addr,
    )?;
    let mut vm = Vm::new_iterative(&state, &context)?;
    vm.consume(FromStateHolder)
}
