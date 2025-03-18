use {
    crate::{
        accounts::{AccountState, Data, OwnerInfo},
        error::{Result, RomeProgramError::*},
        origin::Origin,
        State,
    },
    evm::{H160, U256},
    solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey},
    std::{cell::RefMut, convert::TryInto, mem::size_of},
};

// Instruction is used to synchronize the initial state of contract with the state of L1.
// This private instruction is available only for rollup owner.
// It is not possible to overwrite account state.
pub fn create_balance<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    let (address, balance, chain) = args(data)?;
    msg!(
        "Instruction: create balance {} {} chain_id {}",
        address,
        balance,
        chain
    );

    let state = State::new(program_id, accounts, chain)?;
    let owner_info = state.info_owner_reg(false)?;
    let owner = get_onwer_mut(owner_info, state.signer.key, chain)?;
    let info = state.info_addr(&address, true)?;
    mint_balance(&state, info, owner, balance, address, chain)
}

// address | balance | chain_id
pub fn args(data: &[u8]) -> Result<(H160, U256, u64)> {
    if data.len() != size_of::<H160>() + size_of::<U256>() + size_of::<u64>() {
        return Err(InvalidInstructionData);
    }

    let (left, right) = data.split_at(size_of::<H160>());
    let address = H160::from_slice(left);
    let (left, right) = right.split_at(size_of::<U256>());
    let balance = U256::from_big_endian(left);
    let chain = u64::from_le_bytes(right.try_into().unwrap());

    Ok((address, balance, chain))
}

pub fn get_onwer_mut<'a>(
    info: &'a AccountInfo<'a>,
    signer: &Pubkey,
    chain: u64,
) -> Result<RefMut<'a, OwnerInfo>> {
    let owner = OwnerInfo::get_mut(info, signer, chain)?;
    owner.ok_or(Custom(format!(
        "signer {} is not registered as the owner of chain {}",
        &signer, chain
    )))
}

pub fn mint_balance<T: Origin>(
    state: &T,
    info: &AccountInfo,
    mut owner: RefMut<OwnerInfo>,
    balance: U256,
    addr: H160,
    chain: u64,
) -> Result<()> {
    if owner.mint_address.is_some() {
        return Err(Custom(format!(
            "chain {} has been already initialized",
            chain
        )));
    }

    if state.base().alloc() == 0 {
        return Err(AccountInitialized(*info.key));
    }
    let mut account_state = AccountState::from_account_mut(info)?;
    let current = account_state.balance;
    assert!(current.is_zero());
    account_state.balance = balance;
    owner.mint_address = Some(addr);

    Ok(())
}
