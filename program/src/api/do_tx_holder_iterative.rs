use {
    crate::{
        context::ContextIterative,
        error::Result,
        split_fee, split_hash, split_u64,
        state::State,
        vm::{vm_iterative::MachineIterative::FromStateHolder, Execute, Vm},
        Holder,
    },
    evm::{H160, H256},
    solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey},
};

// unique | session | holder_index | tx_hash | chain_id | Option<fee_recipient> | lock_overrides
#[allow(clippy::type_complexity)]
pub fn args(data: &[u8]) -> Result<(u64, u64, H256, u64, Option<H160>, &[u8])> {
    let (_, data) = split_u64(data)?;
    let (session, data) = split_u64(data)?;
    let (holder, data) = split_u64(data)?;
    let (hash, data) = split_hash(data)?;
    let (chain, data) = split_u64(data)?;
    let (fee_addr, lock_overrides) = split_fee(data)?;

    Ok((session, holder, hash, chain, fee_addr, lock_overrides))
}

pub fn do_tx_holder_iterative<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    msg!("Instruction: Iterative transaction from holder");

    let (session, holder, hash, chain, fee_addr, lock_overrides) = args(data)?;
    let state = State::new(program_id, accounts, chain)?;
    let info = state.info_tx_holder(holder, false)?;
    let rlp = Holder::rlp(info, hash, chain)?;

    let context = ContextIterative::new(
        &state,
        accounts,
        holder,
        lock_overrides,
        &rlp,
        hash,
        session,
        fee_addr,
    )?;
    let mut vm = Vm::new_iterative(&state, &context)?;
    vm.consume(FromStateHolder)
}
