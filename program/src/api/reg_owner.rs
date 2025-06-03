use {
    crate::{
        error::{Result, RomeProgramError::*},
        registration_key, Data, OwnerInfo, State,
    },
    solana_program::{
        account_info::AccountInfo, clock::Clock, msg, pubkey::Pubkey, sysvar::Sysvar,
    },
    std::{
        convert::{TryInto},
        mem::size_of,
    },
};

// chain_id
pub fn args(data: &[u8]) -> Result<u64> {
    if data.len() != size_of::<u64>() {
        return Err(InvalidInstructionData);
    }

    let chain = u64::from_le_bytes(data.try_into().unwrap());

    Ok(chain)
}

pub fn check(info: &AccountInfo, signer: &Pubkey, chain: u64) -> Result<()> {
    if *signer != registration_key::ID {
        return Err(Custom(format!(
            "private instruction must be signed by registration keypair: {}",
            registration_key::ID
        )));
    }

    if OwnerInfo::is_owned(info, chain)? {
        return Err(Custom(format!("chain {} is already registered", chain)));
    }

    Ok(())
}

pub fn reg(info: &AccountInfo, chain: u64) -> Result<()> {
    let mut owner_info = OwnerInfo::from_account_mut(info)?;
    let owner = owner_info.last_mut().unwrap();
    let clock = Clock::get()?;

    owner._key = Pubkey::default();
    owner.chain = chain;
    owner._mint_address = None;
    owner.slot = clock.slot;

    Ok(())
}

// Instruction is used to registry rollup owner.
// This private instruction must be signed by the upgrade-authority keypair.
pub fn reg_owner<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    let chain = args(data)?;
    msg!("Instruction: chain_id registration {}", chain);

    let state = State::new_unchecked(program_id, accounts, chain)?;
    let info = state.info_owner_reg(true)?;
    check(info, state.signer.key, chain)?;
    state.realloc(info, info.data_len() + size_of::<OwnerInfo>())?;
    reg(info, chain)?;
    let _ = state.info_sol_wallet(true);

    Ok(())
}
