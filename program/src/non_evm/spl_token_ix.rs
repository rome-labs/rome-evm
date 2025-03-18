use {
    solana_program::{
        instruction::{Instruction, AccountMeta}, pubkey::Pubkey, account_info::{
            AccountInfo, IntoAccountInfo,
        }, program_pack::{Pack,},
    },
    crate::{
        U256, error::Result, error::RomeProgramError::*, origin::Origin, non_evm::Bind,
    },
    std::{
        convert::TryFrom, mem::size_of,
    },
    spl_token::{
        instruction::initialize_account3,
        processor::Processor,
    },
    super::{len_eq,},
};

pub struct Transfer();
impl Transfer {
    pub const ABI_LEN: usize = 32 * 3;

    pub fn new_from_abi(abi: &[u8], auth: &Pubkey) -> Result<Instruction> {
        len_eq!(abi, Self::ABI_LEN);

        // TODO: check the case with inconsistency of data length
        let (from, rest) = abi.split_at(32);
        let from = Pubkey::try_from(from).unwrap();

        let (to, rest) = rest.split_at(32);
        let to = Pubkey::try_from(to).unwrap();

        let amount = U256::from_big_endian(rest).as_u64();

        spl_token::instruction::transfer(&spl_token::id(), &from, &to, &auth, &[], amount)
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

fn u64_to_bytes32(x: u64, dst: &mut [u8; 32]) {
    let val: U256 = x.into();
    val.to_big_endian(dst);
}

pub fn account_state<T: Origin>(abi: &[u8], state: &T) -> Result<Vec<u8>> {
    let len = size_of::<SplAccount>();
    let mut  vec = vec![0_u8; len];

    let ptr = vec.as_mut_ptr().cast::<SplAccount>();
    let dst = unsafe { &mut *ptr };

    let key = Pubkey::try_from(abi).unwrap();
    let acc = state.account(&key).unwrap_or_default();

    let spl = spl_token::state::Account::unpack(acc.data.as_slice())?;

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

pub fn info<'b>(meta: &[AccountMeta], binds: Vec<Bind<'b>>) -> Vec<AccountInfo<'b>> {
    binds
        .into_iter()
        .filter(|(&key, _)|
            meta.iter().any(|m| m.pubkey == key)
        )
        .map(|(k,  v)| (k, false, v).into_account_info())
        .collect::<Vec<_>>()
}
