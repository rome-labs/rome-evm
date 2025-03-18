use {
    crate::{error::{Result, RomeProgramError::*},  Account, U256,},
    solana_program::{
        instruction::AccountMeta, pubkey::Pubkey, program_error::ProgramError, msg,
    },
    super:: Bind,
    std::{
        collections::HashMap, convert::TryFrom,
    }
};
#[cfg(not(target_os = "solana"))]
use {
    solana_program::instruction::Instruction,
    super::{SplToken, ASplToken, System, Program},
};

#[macro_export]
macro_rules! len_eq {
    ($abi:ident, $len:expr) => {
        if $abi.len() != $len {
            return Err(InvalidNonEvmInstructionData)
        }
    }
}
#[macro_export]
macro_rules! len_ge {
    ($abi:ident, $exp:expr) => {
        if $abi.len() < $exp || $exp == 0 {
            return Err(InvalidNonEvmInstructionData)
        }
    }
}

#[macro_export]
macro_rules! val_eq {
    ($var:ident, $val:expr) => {
        if $var != $val {
            return Err(InvalidNonEvmInstructionData)
        }
    }
}
pub use len_eq;
pub use len_ge;

#[cfg(not(target_os = "solana"))]
pub fn dispatcher<'a, T: crate::origin::Origin>(ix: &Instruction, state: &'a T) -> Box<dyn Program + 'a> {
    use solana_program::system_program;

    match ix.program_id {
        ::spl_token::ID => Box::new(SplToken::new(state)),
        spl_associated_token_account::ID => Box::new(ASplToken::new(state)),
        system_program::ID => Box::new(System::new(state)),
        _ => unimplemented!()
    }
}

pub fn next<'a, I: Iterator<Item = Bind<'a>>>(iter: &mut I) -> Result<I::Item> {
    iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys.into())
}

pub fn accounts_mut<'b>(meta: &[AccountMeta], map: &'b mut HashMap<Pubkey, Account>) -> Vec<(&'b Pubkey, &'b mut Account)> {
    map
        .iter_mut()
        .filter(|(&key, _)|
            meta.iter().any(|m| m.pubkey == key)
        )
        .collect::<Vec<_>>()
}


pub fn get_pubkey(src: &[u8]) -> Result<Pubkey> {
    len_ge!(src, 32);
    let (left, _) = src.split_at(32);
    let key = Pubkey::try_from(left).unwrap();

    Ok(key)
}
pub fn get_usize(src: &[u8], offset: usize) -> Result<usize> {
    len_ge!(src, offset + 32);
    let val = U256::from_big_endian(&src[offset..offset+32]).as_usize();
    Ok(val)
}


// 0000000000000000000000000000000000000000000000000000000000000040     offset
// 0000000000000000000000000000000000000000000000000000000000000003     len

// 0000000000000000000000000000000000000000000000000000000000000060     offset_item_1 from this
// 00000000000000000000000000000000000000000000000000000000000000c0     offset_item_2
// 0000000000000000000000000000000000000000000000000000000000000120     offset_item_2

pub fn split_to_items(abi: &[u8], offset_pos: usize) -> Result<Vec<usize>> {
    let mut offset = get_usize(abi, offset_pos)?;
    let len = get_usize(abi, offset)?;

    offset += 32;
    let ref_point = offset;

    msg!("size {}", len );
    let mut vec = vec![];
    for _ in 0..len {
        vec.push(ref_point + get_usize(abi, offset)?);
        offset += 32;
    }

    Ok(vec)
}
// 0000000000000000000000000000000000000000000000000000000000000020
// 0000000000000000000000000000000000000000000000000000000000000004
// e903000000000000000000000000000000000000000000000000000000000000
pub fn decode_item(abi: &[u8], offset_pos: usize) -> Result<&[u8]> {
    let mut offset = get_usize(abi, offset_pos)?;
    offset += offset_pos;

    let len = get_usize(abi, offset)?;
    offset += 32;

    len_ge!(abi, offset+len);
    let item = &abi[offset..offset+len];

    Ok(item)
}
pub fn get_vec_slices(abi: &[u8], start: usize) -> Result<Vec<&[u8]>> {
    let items = split_to_items(abi, start)?;

    items
        .iter()
        .map(|item| decode_item(abi, *item))
        .collect::<Result<Vec<&[u8]>>>()
}


pub fn slice_to_abi(msg: &[u8]) -> Vec<u8> {
    let len = 32 + 32 + msg.len();
    let mut abi = vec![0; len];

    let (left, right) = abi.split_at_mut(32);
    let x: U256 = 32.into();
    x.to_big_endian(left);

    let (left, right) = right.split_at_mut(32);
    let x: U256 = msg.len().into();
    x.to_big_endian(left);

    right.copy_from_slice(msg);

    abi
}