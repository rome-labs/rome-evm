use {
    crate::{
        error::{Result, RomeProgramError::*}, context::AccountLock,
        tx::tx::{Tx, TxType,},
        State, tx::deposit::Deposit, context::ContextAt, origin::Origin, pda::Seed, RSOL_DECIMALS,
    },
    evm::{U256},
    solana_program::{
        account_info::AccountInfo, msg, pubkey::Pubkey,
        system_instruction::transfer,
    },
    std::{convert::TryInto, mem::size_of},
};

pub fn deposit<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    msg!("Instruction: deposit");

    let (chain, rlp) = args(data)?;
    let state = State::new(program_id, accounts, chain)?;
    let context = ContextAt::new(&state);

    let tx = from_rlp(rlp)?;
    mint(&tx, &state, &context)?;

    let wallet = state.info_sol_wallet(false)?.key;
    spl_transfer(tx.mint, &state, wallet)?;

    Ok(())
}

//  chain_id | rlp
pub fn args(data: &[u8]) -> Result<(u64, &[u8])> {
    if data.len() <=  size_of::<u64>() {
        return Err(InvalidInstructionData);
    }

    let (left, rlp) = data.split_at(size_of::<u64>());
    let chain = u64::from_le_bytes(left.try_into().unwrap());

    Ok((chain, rlp))
}

pub fn from_rlp(rlp: &[u8]) -> Result<Deposit> {
    let tx = match Tx::tx_type(rlp)? {
        TxType::Deposit(rlp) => Deposit::from_rlp(&rlp)?,
        _ => return Err(IncorrectRlpType)
    };

    if tx.mint != tx.value || tx.from != tx.to || tx.data.is_some() {
        return Err(InvalidDepositInstruction)
    }
    Ok(tx)
}

pub fn mint<T:Origin, L: AccountLock>(
    tx: &Deposit,
    state: &T,
    context: &L,
) -> Result<()> {

    context.lock()?;

    state.add_balance(&tx.from, &tx.mint, context)?;
    state.inc_nonce(&tx.from, context)?;
    Ok(())
}

pub fn spl_transfer<T:Origin>(rsol: U256, state: &T, wallet: &Pubkey) -> Result<()> {
    let (lamports, remainder) = rsol.div_mod(U256::exp10(RSOL_DECIMALS - 9));

    if !remainder.is_zero() {
        return Err(TxValueNotMultipleOf10_9)
    }

    if lamports > U256::from(u64::MAX) {
        return Err(TxValueExceedsU64)
    }

    let ix = transfer(&state.signer(), wallet, lamports.as_u64());

    state.invoke_signed(&ix, &Seed::default(), false)
}