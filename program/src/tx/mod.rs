mod eip1559;
mod eip2930;
pub mod legacy;
#[allow(clippy::module_inception)]
pub mod tx;

use {
    crate::error::{Result, RomeProgramError::*},
    evm::{H160, U256},
    legacy::Legacy,
    rlp::{DecoderError, Rlp},
};

pub trait Base {
    fn nonce(&self) -> u64;
    fn to(&self) -> Option<H160>;
    fn value(&self) -> U256;
    fn data(&self) -> &Vec<u8>;
    fn gas_limit(&self) -> U256;
    fn gas_price(&self) -> U256;
    fn to_rlp(&self) -> Vec<u8>;
    fn rs(&self) -> (U256, U256);
    fn recovery_id(&self) -> Result<u8>;
    fn chain_id(&self) -> Option<u64>;
    fn from(&self) -> H160;
    fn set_from(&mut self, from: H160);
    #[cfg(test)]
    fn access_list(&self) -> Option<&eip2930::AccessList>;
}

fn fix(view: &Rlp, index: usize) -> std::result::Result<U256, DecoderError> {
    let f = |a: &[u8]| {
        if !a.is_empty() && a[0] == 0 {
            return Err(DecoderError::RlpInvalidIndirection);
        }
        if a.len() <= 32 {
            let mut buf = [0_u8; 32];
            buf[(32 - a.len())..].copy_from_slice(a);
            let res = U256::from_big_endian(&buf);

            return Ok(res);
        }
        Err(DecoderError::RlpIsTooBig)
    };
    let value = &view.at(index)?;

    value.decoder().decode_value(f)
}

fn decode_to(rlp: &Rlp, offset: usize) -> Result<Option<H160>> {
    let to = {
        let to = rlp.at(offset)?;
        if to.is_empty() {
            if to.is_data() {
                None
            } else {
                return Err(Custom("RLP: contract code expected".to_string()));
            }
        } else {
            Some(to.as_val()?)
        }
    };

    Ok(to)
}

fn check_rlp(rlp: &Rlp, count: usize) -> Result<()> {
    if rlp.at(count).is_ok() {
        return Err(Custom(format!("RlpIncorrectListLen: {count}")));
    }

    let payload = rlp.payload_info()?;
    let len = payload.header_len + payload.value_len;

    if rlp.as_raw().len() != len {
        return Err(Custom(format!(
            "RlpInconsistentLengthAndData: {} {}",
            rlp.as_raw().len(),
            count
        )));
    }

    Ok(())
}
