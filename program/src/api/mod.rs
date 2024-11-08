pub mod create_balance;
mod do_tx;
pub mod do_tx_holder;
pub mod do_tx_holder_iterative;
pub mod do_tx_iterative;
pub mod reg_owner;
pub mod transmit_tx;

pub use create_balance::create_balance;
pub use do_tx::do_tx;
pub use do_tx_holder::do_tx_holder;
pub use do_tx_holder_iterative::do_tx_holder_iterative;
pub use do_tx_iterative::do_tx_iterative;
pub use reg_owner::reg_owner;
pub use transmit_tx::transmit_tx;

use {
    crate::{
        error::{Result, RomeProgramError::InvalidInstructionData},
        H160, H256,
    },
    std::convert::TryInto,
    std::mem::size_of,
};

pub fn split_u64(data: &[u8]) -> Result<(u64, &[u8])> {
    if data.len() < size_of::<u64>() {
        return Err(InvalidInstructionData);
    }

    let (left, right) = data.split_at(size_of::<u64>());
    let value = u64::from_le_bytes(left.try_into().unwrap());

    Ok((value, right))
}
pub fn split_hash(data: &[u8]) -> Result<(H256, &[u8])> {
    if data.len() < size_of::<H256>() {
        return Err(InvalidInstructionData);
    }

    let (left, right) = data.split_at(size_of::<H256>());
    let hash = H256::from_slice(left);

    Ok((hash, right))
}
// Option<fee_recipient> | ...
pub fn split_fee(data: &[u8]) -> Result<(Option<H160>, &[u8])> {
    if data.len() < size_of::<u8>() {
        return Err(InvalidInstructionData);
    }
    let (pay_fee, right) = data.split_at(size_of::<u8>());

    if pay_fee[0] == 0 {
        return Ok((None, right));
    }

    if right.len() < size_of::<H160>() {
        return Err(InvalidInstructionData);
    }

    let (left, right) = right.split_at(size_of::<H160>());
    let fee_addr = Some(H160::from_slice(left));

    Ok((fee_addr, right))
}
