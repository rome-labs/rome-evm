use {
    crate::{
        accounts::{AccountState, Data},
        error::{Result, RomeProgramError::*},
        origin::Origin,
        State, CONTRACT_OWNER,
    },
    evm::{H160, U256},
    solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey},
    std::{mem::size_of, str::FromStr},
};

// address | balance
pub fn args(data: &[u8]) -> Result<(H160, U256)> {
    if data.len() != size_of::<H160>() + size_of::<U256>() {
        return Err(InvalidInstructionData);
    }

    let (left, right) = data.split_at(size_of::<H160>());
    let address = H160::from_slice(left);
    let balance = U256::from_big_endian(right);

    Ok((address, balance))
}

// Instruction is used to synchronize the initial state of contract with the state of L1.
// This private instruction is available only for contract owner.
// It is not possible to overwrite account state.
pub fn create_balance<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    let (address, balance) = args(data)?;
    msg!("Instruction: create balance {} {}", address, balance);
    let state = State::new(program_id, accounts);
    let info = state.info_addr(&address, true)?;

    if state.allocated() == 0 {
        return Err(AccountInitialized(*info.key));
    }

    let signer = state.signer()?;
    let owner = Pubkey::from_str(CONTRACT_OWNER)?;
    if *signer.key != owner {
        return Err(Custom(
            "private instruction is available only for contract owner".to_string(),
        ));
    }

    let mut account_state = AccountState::from_account_mut(info)?;
    account_state.balance = balance;

    Ok(())
}
