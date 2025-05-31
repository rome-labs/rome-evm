use {
    crate::{
        context::ContextAt,
        error::{Result, RomeProgramError::*},
        split_fee, split_hash, split_u64,
        state::State,
        vm::{vm_atomic::MachineAt, Execute, VmAt},
        Holder, origin::Origin, TxHolder, SIG_VERIFY_COST, Data,
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

pub fn transmit_fee (info: &AccountInfo) -> Result<u64> {
    let iter_cnt = TxHolder::from_account(info)?.iter_cnt as u64;
    SIG_VERIFY_COST.checked_mul(iter_cnt).ok_or(CalculationOverflow)
}
pub fn add_transmit_fee<T: Origin>(state: &T, info: &AccountInfo) -> Result<()> {
    let lamports = transmit_fee(info)?;
    state.base().add_fee(lamports)
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
    add_transmit_fee(&state, info)?;
    
    let rlp = Holder::rlp(info, hash, chain)?;

    let context = ContextAt::new(&state);
    let mut vm = VmAt::new(&state, &rlp, fee_addr, &context)?;

    vm.consume(MachineAt::Lock)
}
