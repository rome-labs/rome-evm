use {
    crate::{
        accounts::{Data, SignerInfo},
        error::{Result, RomeProgramError::*},
        State,
    },
    evm::H160,
    solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey},
    std::{convert::TryInto, mem::size_of},
};

// address | chain_id
pub fn args(data: &[u8]) -> Result<(H160, u64)> {
    if data.len() != size_of::<H160>() + size_of::<u64>() {
        return Err(InvalidInstructionData);
    }
    let (left, right) = data.split_at(size_of::<H160>());

    let addr = H160::from_slice(left);
    let chain = u64::from_le_bytes(right.try_into().unwrap());
    Ok((addr, chain))
}

pub fn reg_signer<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    let (address, chain) = args(data)?;
    msg!("Instruction: set signer info {} chain {}", address, chain);

    let state = State::new(program_id, accounts, chain)?;
    let signer = state.signer()?;

    let info = state.info_signer_reg(signer.key, true)?;
    let mut signer_info = SignerInfo::from_account_mut(info)?;
    signer_info.address = address;

    Ok(())
}
