use {
    crate::{
        accounts::{Data, Holder, TxHolder},
        error::{Result, RomeProgramError::*},
        state::State,
    },
    evm::H256,
    solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey},
    std::{cell::RefMut, convert::TryInto, mem::size_of},
};

// holder_idex | offset | hash | tx
pub fn args(data: &[u8]) -> Result<(u64, usize, H256, &[u8])> {
    if data.len() < size_of::<u64>() + size_of::<u64>() + size_of::<H256>() {
        return Err(InvalidInstructionData);
    }
    let (left, right) = data.split_at(size_of::<u64>());
    let index = u64::from_le_bytes(left.try_into().unwrap());

    let (left, right) = right.split_at(size_of::<u64>());
    let offset = u64::from_le_bytes(left.try_into().unwrap()) as usize;

    let (left, right) = right.split_at(32);
    let hash = H256::from_slice(left);

    Ok((index, offset, hash, right))
}

pub fn transmit_tx<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    msg!("Instruction: Transmit tx");

    let (index, offset, hash, tx) = args(data)?;

    let state = State::new(program_id, accounts);
    let info = state.info_tx_holder(index, true)?;

    let required_len = offset + tx.len();

    if TxHolder::from_account(info)?.hash != hash
        || required_len > Holder::from_account(info)?.len()
    {
        state.realloc(info, Holder::offset(info) + required_len)?;
    }

    TxHolder::from_account_mut(info)?.hash = hash;
    let holder = Holder::from_account_mut(info)?;

    let mut location = RefMut::map(holder, |a| &mut a[offset..required_len]);
    location.copy_from_slice(tx);

    Ok(())
}
