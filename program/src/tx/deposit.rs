use {
    super::{fix, Base},
    crate::{
        error::Result,
        tx::{check_rlp,},
    },
    evm::{H160, H256, U256},
    rlp::Rlp,
};
#[cfg(test)]
use crate::tx::eip2930::AccessList;

#[derive(Debug, Clone)]
pub struct Deposit {
    pub source_hash: H256,
    pub from: H160,
    pub to: H160,
    pub mint: U256,
    pub value: U256,
    pub gas_limit: U256,
    pub is_system_tx:bool,
    pub data: Option<Vec<u8>>,
}

impl Base for Deposit {
    fn nonce(&self) -> u64 {
        unreachable!()
    }
    fn to(&self) -> Option<H160> {
        Some(self.to)
    }
    fn value(&self) -> U256 {
        self.value
    }
    fn data(&mut self) -> Option<Vec<u8>> {
        self.data.take()
    }
    fn gas_limit(&self) -> U256 {
        self.gas_limit
    }
    fn gas_price(&self) -> U256 { unreachable!() }
    fn hash_unsign(&self, _: &Rlp) -> Result<H256> { unreachable!()}
    fn rs(&self) -> (U256, U256) {
        unreachable!()
    }
    fn recovery_id(&self) -> Result<u8> {
        unreachable!()
    }
    fn chain_id(&self) -> u64 {
        unreachable!()
    }
    fn from(&self) -> H160 {
        self.from
    }
    fn set_from(&mut self, _: H160) {
        unreachable!()
    }
    #[cfg(test)]
    fn access_list(&self) -> Option<&AccessList> {
        unreachable!()
    }
    fn mint(&self) -> U256 {
        self.mint
    }
}
impl Deposit {
    pub fn from_rlp(rlp: &Rlp) -> Result<Self> {
        check_rlp(rlp, 8)?;

        let source_hash: H256 = rlp.val_at(0)?;
        let from: H160 = rlp.at(1)?.as_val()?;
        let to = rlp.at(2)?.as_val()?;
        let mint = fix(rlp, 3)?;
        let value = fix(rlp, 4)?;
        let gas_limit = fix(rlp, 5)?;
        let is_system_tx: bool = rlp.at(6)?.as_val()?;
        let data_ = Rlp::new(rlp.at(7)?.as_raw()).data()?;
        let data = match data_.len() {
            0 => None,
            _ => Some(data_.to_vec()),
        };

        Ok(Deposit {
            source_hash,
            from,
            to,
            mint,
            value,
            gas_limit,
            is_system_tx,
            data,
        })
    }
}
