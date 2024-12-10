use {
    crate::{
        context::ContextAtomic,
        error::Result,
        split_fee, split_hash, split_u64,
        state::State,
        vm::{vm_atomic::MachineAtomic, Execute, Vm},
        Holder,
    },
    evm::{H160, H256},
    solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey},
};

// holder_index | tx_hash | chain_id | Option<fee_recipient>
pub fn args(data: &[u8]) -> Result<(u64, H256, u64, Option<H160>)> {
    let (holder, data) = split_u64(data)?;
    let (hash, data) = split_hash(data)?;
    let (chain, data) = split_u64(data)?;
    let (fee_addr, _) = split_fee(data)?;

    Ok((holder, hash, chain, fee_addr))
}

pub fn do_tx_holder<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    msg!("Instruction: Atomic transaction from holder");

    let (holder, hash, chain, fee_addr) = args(data)?;
    let state = State::new(program_id, accounts, chain)?;

    let info = state.info_tx_holder(holder, false)?;
    let rlp = Holder::rlp(info, hash, chain)?;
    let context = ContextAtomic::new(&state, &rlp, fee_addr);

    let mut vm = Vm::new_atomic(&state, &context)?;
    vm.consume(MachineAtomic::Lock)
}
