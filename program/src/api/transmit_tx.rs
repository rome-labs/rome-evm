use {
    crate::{
        accounts::{Data, Holder, TxHolder},
        error::{Result, RomeProgramError::*},
        state::State,
    },
    evm::H256,
    solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey},
    std::{convert::TryInto, mem::size_of},
};

// holder_idex | offset | hash | chain_id | tx
pub fn args(data: &[u8]) -> Result<(u64, usize, H256, u64, &[u8])> {
    if data.len() < size_of::<u64>() + size_of::<u64>() + size_of::<H256>() + size_of::<u64>() {
        return Err(InvalidInstructionData);
    }
    let (left, right) = data.split_at(size_of::<u64>());
    let index = u64::from_le_bytes(left.try_into().unwrap());

    let (left, right) = right.split_at(size_of::<u64>());
    let offset = u64::from_le_bytes(left.try_into().unwrap()) as usize;

    let (left, right) = right.split_at(size_of::<H256>());
    let hash = H256::from_slice(left);

    let (left, right) = right.split_at(size_of::<u64>());
    let chain = u64::from_le_bytes(left.try_into().unwrap());

    Ok((index, offset, hash, chain, right))
}

pub fn transmit_tx<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    msg!("Instruction: Transmit tx");

    let (holder, offset, hash, chain, tx) = args(data)?;
    let state = State::new(program_id, accounts, chain)?;
    let info = state.info_tx_holder(holder, true)?;
    let to = offset + tx.len();

    if TxHolder::from_account(info)?.hash != hash || to > Holder::from_account(info)?.len() {
        state.realloc(info, Holder::offset(info) + to)?;
    }
    Holder::fill(info, hash, offset, to, tx)
}
