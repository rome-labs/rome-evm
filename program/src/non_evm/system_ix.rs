use {
    solana_program::{
        system_instruction::{
            create_account, allocate, assign, transfer,
        },
        pubkey::Pubkey, system_program,  instruction::Instruction,
        rent::Rent, sysvar::Sysvar,
    },
    crate::{
        error::{Result, RomeProgramError::*,}, U256, non_evm::{
            get_vec_slices, aux::decode_item,
        }, origin::Origin,
        H160, pda::Seed,
    },
    super::{next, Bind, len_eq, get_pubkey,},
    std::{
        convert::TryFrom, str::FromStr,
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

    pub fn emulate(lamports: u64, len: u64, owner: &Pubkey, binds: Vec<Bind>) -> Result<Vec<u8>> {
        let iter  = &mut binds.into_iter();

        let (&signer_key, signer) = next(iter)?;
        let (&new_key, new) = next(iter)?;

        signer.lamports = signer
            .lamports
            .checked_sub(lamports)
            .ok_or(InsufficientSOLs(signer_key))?;

        if !(new.lamports == 0 && new.data.is_empty() && system_program::ID == new.owner) {
            return Err(AccountAlreadyExists(new_key))
        }

        new.lamports = lamports;
        new.data.resize(len as usize, 0);
        new.owner = *owner;

        Ok(new_key.to_bytes().to_vec())
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

    pub fn emulate(len: u64, binds: Vec<Bind>) -> Result<()> {
        let iter  = &mut binds.into_iter();
        let (key, acc) = next(iter)?;

        if !acc.data.is_empty() || system_program::ID != acc.owner {
            return Err(AccountAlreadyInUse(*key))
        }

        acc.data.resize(len as usize, 0);

        Ok(())
    }
}
pub struct Transfer();
impl Transfer {
    pub const ABI_LEN: usize = 32*3;
    pub fn new_from_abi<T: Origin>(state: &T, caller: &H160, abi: &[u8]
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

        solana_program::msg!("AUTH +++++  {}  {}", auth, hex::encode(salt));
        let ix = transfer(&auth, &to, lamports);

        Ok((ix, seed))
    }
    pub fn emulate(ix: &Instruction, lamports: u64, binds: Vec<Bind>) -> Result<()> {
        let auth = ix.accounts.get(0).unwrap().pubkey;
        let to = ix.accounts.get(1).unwrap().pubkey;

        if auth == to {
            return Ok(())
        }

        // TODO
        if binds.len() != ix.accounts.len() {
            return Err(InvalidNonEvmInstructionData)
        }

        let iter  = &mut binds.into_iter();
        let from = next(iter)?;
        let to = next(iter)?;

        if !from.1.data.is_empty() {
            return Err(TransferFromAccountWithData(*from.0))
        }

        from.1.lamports = from
            .1
            .lamports
            .checked_sub(lamports)
            .ok_or(InsufficientSOLs(*from.0))?;

        to.1.lamports = to
            .1
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

    pub fn emulate(owner: &Pubkey, binds: Vec<Bind>) -> Result<()> {
        let iter  = &mut binds.into_iter();
        let (key, acc) = next(iter)?;

        if system_program::ID != acc.owner {
            return Err(AccountAlreadyInUse(*key))
        }

        #[cfg(not(target_os = "solana"))]
        if !is_zeroed(&acc.data) {
            return Err(AccountAlreadyInUse(*key))
        }

        acc.owner = *owner;

        Ok(())
    }
}

pub fn find_pda(abi: &[u8]) -> Result<Vec<u8>> {

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

pub fn bytes32_to_base58(abi: &[u8]) -> Result<Vec<u8>> {
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

pub fn base58_to_bytes32(abi: &[u8]) -> Result<Vec<u8>> {
    let b58 = decode_item(abi, 0)?;
    let str = std::str::from_utf8(b58)
        .map_err(|_| InvalidNonEvmInstructionData)?;

    let key = Pubkey::from_str(&str)?;
    Ok(key.to_bytes().to_vec())
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

