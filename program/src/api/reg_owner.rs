use {
    crate::{
        error::{Result, RomeProgramError::*},
        upgrade_authority, Data, OwnerInfo, State,
    },
    solana_program::{
        account_info::AccountInfo, clock::Clock, msg, pubkey::Pubkey, sysvar::Sysvar,
    },
    std::{
        convert::{TryFrom, TryInto},
        mem::size_of,
    },
};

// pubkey | chain_id
pub fn args(data: &[u8]) -> Result<(Pubkey, u64)> {
    if data.len() != size_of::<Pubkey>() + size_of::<u64>() {
        return Err(InvalidInstructionData);
    }

    let (left, right) = data.split_at(size_of::<Pubkey>());
    let key = Pubkey::try_from(left).unwrap();
    let chain = u64::from_le_bytes(right.try_into().unwrap());

    Ok((key, chain))
}

pub fn check(info: &AccountInfo, signer: &Pubkey, chain: u64) -> Result<()> {
    if *signer != upgrade_authority::ID {
        return Err(Custom(format!(
            "private instruction must be signed by upgrade_authority keypair: {}",
            upgrade_authority::ID
        )));
    }

    if OwnerInfo::is_owned(info, chain)? {
        return Err(Custom(format!("chain {} is already registered", chain)));
    }

    Ok(())
}

pub fn reg(info: &AccountInfo, key: Pubkey, chain: u64) -> Result<()> {
    let mut owner_info = OwnerInfo::from_account_mut(info)?;
    let owner = owner_info.last_mut().unwrap();
    let clock = Clock::get()?;

    owner.key = key;
    owner.chain = chain;
    owner.mint_address = None;
    owner.slot = clock.slot;

    Ok(())
}

// Instruction is used to registry rollup owner.
// After the registration the rollup owner is able to init state of the rollup by using
// the create_balance instruction.
//
// This private instruction must be signed by the upgrade-authority keypair.
pub fn reg_owner<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    let (key, chain) = args(data)?;
    msg!("Instruction: register owner {} of chain {}", key, chain);

    let state = State::new_unchecked(program_id, accounts, chain)?;
    let info = state.info_owner_reg(true)?;

    check(info, state.signer.key, chain)?;
    state.realloc(info, info.data_len() + size_of::<OwnerInfo>())?;
    reg(info, key, chain)
}
