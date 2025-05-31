use {
    crate::{
        H160, U256, error::Result, error::RomeProgramError::*, origin::Origin,
        non_evm::{
            Bind, NonEvmState, spl_pda,
        },
    },
    spl_token::{instruction::initialize_account3, processor::Processor,},
    solana_program::{
        instruction::{Instruction, AccountMeta}, pubkey::Pubkey, account_info::{
            AccountInfo, IntoAccountInfo,
        },
        program_pack::Pack,
    },
    super::{len_eq,},
    std::{
        mem::size_of, convert::TryFrom,
    },
};

#[repr(C, packed)]
#[derive(Default)]
pub struct SplAccount {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: [u8; 32],
    pub delegate: Pubkey,
    pub state: [u8; 32],
    pub is_native: [u8; 32],
    pub native_value: [u8; 32],
    pub delegated_amount: [u8; 32],
    pub close_authority: Pubkey,
}

pub struct Transfer();
impl Transfer {
    pub const ABI_LEN: usize = 32 * 3;

    pub fn new_from_abi(abi: &[u8], auth: &Pubkey) -> Result<Instruction> {
        len_eq!(abi, Self::ABI_LEN);

        // TODO: check the case with inconsistency of data length
        let (to, rest) = abi.split_at(32);
        let to = Pubkey::try_from(to).unwrap();

        let (mint, rest) = rest.split_at(32);
        let mint= Pubkey::try_from(mint).unwrap();
        let tokens = U256::from_big_endian(rest);
        
        let (from, _) = spl_pda(&auth, &mint, &spl_token::ID);
        
        if tokens > u64::MAX.into() {
            return Err(InvalidNonEvmInstructionData)
        }

        let tokens = tokens.as_u64();

        spl_token::instruction::transfer(&spl_token::id(), &from, &to, &auth, &[], tokens)
            .map_err(|e| e.into())
    }
    pub fn emulate(ix: &Instruction, binds: Vec<Bind>, amount: u64) -> Result<()> {
        let auth = ix.accounts.get(2).unwrap().pubkey;

        let mut info = info(&ix.accounts, binds);
        let auth_info = info
            .get_mut(2)
            .ok_or(InvalidNonEvmInstructionData)?;

        if *auth_info.key != auth {
            return Err(InvalidAuthority(auth))
        }

        auth_info.is_signer = true;
        let _ = Processor::process_transfer(&spl_token::ID, &info, amount, None)?;
        Ok(())
    }
}

pub struct InitAccount();
impl InitAccount {
    pub fn new_from_abi(abi: &[u8]) -> Result<Instruction> {
        len_eq!(abi, 32 * 3);

        let (new, rest) = abi.split_at(32);
        let (mint, owner) = rest.split_at(32);

        let new = Pubkey::try_from(new).unwrap();
        let mint = Pubkey::try_from(mint).unwrap();
        let owner = Pubkey::try_from(owner).unwrap();

        let ix = initialize_account3(&spl_token::ID, &new, &mint, &owner)?;
        Ok(ix)
    }
    pub fn emulate(ix: &Instruction, binds: Vec<Bind>, owner: &Pubkey) -> Result<()> {
        let  info = info(&ix.accounts, binds);
        let _ = Processor::process_initialize_account3(&spl_token::ID, &info, owner.clone())?;
        Ok(())
    }
}

pub fn info<'b>(meta: &[AccountMeta], binds: Vec<Bind<'b>>) -> Vec<AccountInfo<'b>> {
    binds
        .into_iter()
        .filter(|(&key, _)|
            meta.iter().any(|m| m.pubkey == key)
        )
        .map(|(k,  v)| (k, false, v).into_account_info())
        .collect::<Vec<_>>()
}

fn u64_to_bytes32(x: u64, dst: &mut [u8; 32]) {
    let val: U256 = x.into();
    val.to_big_endian(dst);
}

pub fn spl_account_state<T: Origin>(
    abi: &[u8],
    state: &T,
    non_evm_state: &NonEvmState
) -> Result<spl_token::state::Account> {

    len_eq!(abi, 32);
    let key = Pubkey::try_from(abi).unwrap();

    let acc = if let Some(acc ) = non_evm_state.get(&key) {
        acc
    } else {
        state.account(&key)?
    };

    let spl = spl_token::state::Account::unpack(acc.data.as_slice())?;

    Ok(spl)
}

pub fn account_raw_state(spl: spl_token::state::Account) -> Result<Vec<u8>> {
    let len = size_of::<SplAccount>();
    let mut  vec = vec![0_u8; len];
    let ptr = vec.as_mut_ptr().cast::<SplAccount>();
    let dst = unsafe { &mut *ptr };

    dst.mint = spl.mint;
    dst.owner = spl.owner;
    u64_to_bytes32(spl.amount, &mut dst.amount);
    dst.delegate = spl.delegate.unwrap_or_default();
    *dst.state.last_mut().unwrap() = spl.state as u8;
    *dst.is_native.last_mut().unwrap() = spl.is_native.is_some().into();
    u64_to_bytes32(spl.is_native.unwrap_or_default(), &mut dst.native_value);
    u64_to_bytes32(spl.delegated_amount, &mut dst.delegated_amount);
    dst.close_authority = spl.close_authority.unwrap_or_default();

    Ok(vec)
}

pub fn balance_ge<T:Origin >(
    abi: &[u8],
    state: &T,
    non_evm_state: &NonEvmState
)-> Result<Vec<u8>> {
    len_eq!(abi, 32 + 32 + 32);

    let (left, right) = abi.split_at(32);
    let caller = H160::from_slice(&left[12..]);

    let (left, right) = right.split_at(32);
    let mint = Pubkey::try_from(left).unwrap();
    let balance = U256::from_big_endian(right);

    let (key, _) = state.base().pda.balance_key(&caller);
    let (spl_key, _) = spl_pda(&key, &mint, &spl_token::ID);

    let abi = spl_key.to_bytes();
    let spl_acc = spl_account_state(abi.as_slice(), state, non_evm_state)?;
    let spl_balance: U256 = spl_acc.amount.into();

    if spl_balance < balance {
        let mes = format!("SPL balance is less than expected {} {}", spl_acc.amount, balance);
        return Err(Custom(mes))
    }

    Ok(vec![])
}