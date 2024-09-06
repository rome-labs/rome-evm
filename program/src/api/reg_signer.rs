use {
    crate::{
        accounts::{Data, SignerInfo},
        error::{Result, RomeProgramError::*},
        State,
    },
    evm::H160,
    solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey},
    std::mem::size_of,
};

pub fn args(data: &[u8]) -> Result<H160> {
    if data.len() != size_of::<H160>() {
        return Err(InvalidInstructionData);
    }

    Ok(H160::from_slice(data))
}

pub fn reg_signer<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    let address = args(data)?;
    msg!("Instruction: set signer info {}", address);

    let state = State::new(program_id, accounts);
    let signer = state.signer()?;

    let info = state.info_signer_info(signer.key, true)?;
    let mut signer_info = SignerInfo::from_account_mut(info)?;
    signer_info.address = address;

    Ok(())
}
