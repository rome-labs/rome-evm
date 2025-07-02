use {
    solana_program::{
        instruction::AccountMeta,
        system_instruction::{
            create_account, allocate, assign, transfer,
        },
        pubkey::Pubkey, system_program,  instruction::Instruction,
        rent::Rent, sysvar::Sysvar,
    },
    crate::{
        error::{Result, RomeProgramError::*,}, U256, origin::Origin,
        H160, pda::Seed,
    },
    super::{next, Bind, len_eq, get_account_mut},
    std::{
        convert::TryFrom,
    },
};

pub struct CreateA();
impl CreateA {
    pub const ABI_LEN: usize = 32 + 32 + 32 + 32 ;
    pub fn new_from_abi<T: Origin>(state: &T, abi: &[u8]) -> Result<(Instruction, Seed)> {
        len_eq!(abi, CreateA::ABI_LEN);

        let (left, rest) = abi.split_at(32);
        let owner = Pubkey::try_from(left).unwrap();

        let (left, rest) = rest.split_at(32);
        let len = U256::from_big_endian(left).as_usize();

        let (from, salt) = rest.split_at(32);
        let from = H160::from_slice(&from[12..]);

        let (auth, seed) = state
            .base()
            .pda
            .from_balance_key(&from, salt);

        let rent = Rent::get()?.minimum_balance(len);

        let ix = create_account(&state.signer(), &auth, rent, len as u64, &owner);
        Ok((ix, seed))
    }

    pub fn emulate(meta: &Vec<AccountMeta>, lamports: u64, len: u64, owner: &Pubkey, binds: &mut Vec<Bind>) -> Result<()> {
        let iter  = &mut meta.iter();
        let signer = next(iter)?;
        let new = next(iter)?;

        {
            let signer_ = get_account_mut(&signer, binds)?;
            signer_.lamports = signer_
                .lamports
                .checked_sub(lamports)
                .ok_or(InsufficientLamports(signer, lamports))?;
        }

        let new_ = get_account_mut(&new, binds)?;
        if !(new_.lamports == 0 && new_.data.is_empty() && system_program::ID == new_.owner) {
            return Err(AccountAlreadyExists(new))
        }

        new_.lamports = lamports;
        new_.data.resize(len as usize, 0);
        new_.owner = *owner;

        Ok(())
    }
}
pub struct Allocate();
impl Allocate {
    pub fn new_from_abi(abi: &[u8]) -> Result<Instruction> {
        len_eq!(abi, 32 * 2);

        let (acc, len) = abi.split_at(32);
        let acc = Pubkey::try_from(acc).unwrap();
        let len = U256::from_big_endian(len).as_u64();

        let ix = allocate(&acc, len);
        Ok(ix)
    }

    pub fn emulate(meta: &Vec<AccountMeta>, len: u64, binds: &mut Vec<Bind>) -> Result<()> {
        let iter  = &mut meta.iter();
        let key = next(iter)?;
        let acc = get_account_mut(&key, binds)?;

        if !acc.data.is_empty() || system_program::ID != acc.owner {
            return Err(AccountAlreadyInUse(key))
        }

        acc.data.resize(len as usize, 0);

        Ok(())
    }
}
pub struct Transfer();
impl Transfer {
    pub const ABI_LEN: usize = 32*3;
    pub fn new_from_abi<T: Origin>(
        state: &T, 
        caller: &H160, 
        abi: &[u8]
    ) -> Result<(Instruction, Seed)> {

        len_eq!(abi, Transfer::ABI_LEN);

        let (left, rest) = abi.split_at(32);
        let to = Pubkey::try_from(left).unwrap();

        let (left, salt) = rest.split_at(32);
        let lamports = U256::from_big_endian(left).as_u64();

        let (auth, seed) = state
            .base()
            .pda
            .from_balance_key(caller, salt);

        let ix = transfer(&auth, &to, lamports);

        Ok((ix, seed))
    }
    pub fn emulate(meta: &Vec<AccountMeta>, lamports: u64, binds: &mut Vec<Bind>) -> Result<()> {
        let iter  = &mut meta.iter();
        let from = next(iter)?;
        let to = next(iter)?;
        
        if from == to {
            return Ok(())
        }

        {
            let from_ = get_account_mut(&from, binds)?;
            if !from_.data.is_empty() {
                return Err(TransferFromAccountWithData(from))
            }
            from_.lamports = from_
                .lamports
                .checked_sub(lamports)
                .ok_or(InsufficientLamports(from, lamports))?;
        }

        let to_ = get_account_mut(&to, binds)?;
        to_.lamports = to_
            .lamports
            .checked_add(lamports)
            .ok_or(CalculationOverflow)?;

        Ok(())
    }
}

pub struct Assign();
impl Assign {
    pub fn new_from_abi(abi: &[u8]) -> Result<Instruction> {
        len_eq!(abi, 32 * 2);

        let (acc, owner) = abi.split_at(32);
        let acc = Pubkey::try_from(acc).unwrap();
        let owner = Pubkey::try_from(owner).unwrap();

        Ok(assign(&acc, &owner))
    }

    pub fn emulate(meta: &Vec<AccountMeta>, owner: &Pubkey, binds: &mut Vec<Bind>) -> Result<()> {
        let iter  = &mut meta.iter();
        let key = next(iter)?;
        let acc = get_account_mut(&key, binds)?;

        if system_program::ID != acc.owner {
            return Err(AccountAlreadyInUse(key))
        }

        #[cfg(not(target_os = "solana"))]
        if !is_zeroed(&acc.data) {
            return Err(AccountAlreadyInUse(key))
        }

        acc.owner = *owner;

        Ok(())
    }
}

#[cfg(not(target_os = "solana"))]
fn is_zeroed(buf: &[u8]) -> bool {
    const ZEROS_LEN: usize = 1024;
    const ZEROS: [u8; ZEROS_LEN] = [0; ZEROS_LEN];
    let mut chunks = buf.chunks_exact(ZEROS_LEN);

    #[allow(clippy::indexing_slicing)]
    {
        chunks.all(|chunk| chunk == &ZEROS[..])
            && chunks.remainder() == &ZEROS[..chunks.remainder().len()]
    }
}

#[cfg(feature = "single-state")]
mod single_state_mod {
    use {
        solana_program::pubkey::Pubkey,
        crate::{len_eq, U256, error::RomeProgramError::InvalidNonEvmInstructionData,
            non_evm::{get_pubkey, get_vec_slices, aux::decode_item,},
        },
        std::{
            convert::TryFrom, str::FromStr,
        },
    };

    pub fn find_pda(abi: &[u8]) -> crate::error::Result<Vec<u8>> {

        let program_id = get_pubkey(abi)?;
        let seeds = get_vec_slices(abi, 32)?;

        let (key, bump) = Pubkey::find_program_address(seeds.as_slice(), &program_id);

        #[allow(unused_assignments)]
        let mut val = Vec::with_capacity(32*2);
        val = key.to_bytes().to_vec();
        val.resize(64, 0);
        *val.last_mut().unwrap() = bump;

        Ok(val)
    }

    pub fn bytes32_to_base58(abi: &[u8]) -> crate::error::Result<Vec<u8>> {
        len_eq!(abi, 32);
        let key = Pubkey::try_from(abi).unwrap();
        let b58 = format!("{}", key);

        let offset: U256 = 32.into();
        let len: U256 = b58.len().into();

        let mut vec = vec![0_u8; 64];
        offset.to_big_endian(&mut vec[0..32]);
        len.to_big_endian(&mut vec[32..]);

        let mut a = b58.as_bytes().to_vec();
        vec.append(&mut a);
        Ok(vec)
    }
    pub fn base58_to_bytes32(abi: &[u8]) -> crate::error::Result<Vec<u8>> {
        let b58 = decode_item(abi, 0)?;
        let str = std::str::from_utf8(b58)
            .map_err(|_| InvalidNonEvmInstructionData)?;

        let key = Pubkey::from_str(&str)?;
        Ok(key.to_bytes().to_vec())
    }
}
#[cfg(feature = "single-state")]
pub use single_state_mod::*;




