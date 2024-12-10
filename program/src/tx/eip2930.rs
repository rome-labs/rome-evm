use crate::tx::{rlp_at, rlp_header};
use rlp::Rlp;
use solana_program::keccak::hashv;
use {
    super::{check_rlp, decode_to, fix, Base},
    evm::{H160, H256, U256},
};

#[derive(Debug, Clone, rlp::RlpEncodable, rlp::RlpDecodable, PartialEq)]
pub struct AccessListItem {
    pub address: H160,
    pub storage_keys: Vec<H256>,
}

#[derive(Debug, Clone, rlp::RlpEncodableWrapper, rlp::RlpDecodableWrapper, PartialEq)]
pub struct AccessList(pub Vec<AccessListItem>);

#[derive(Debug, Clone)]
pub struct Eip2930 {
    pub chain_id: U256,
    pub nonce: u64,
    pub gas_price: U256,
    pub gas_limit: U256,
    pub to: Option<H160>,
    pub value: U256,
    pub data: Option<Vec<u8>>,
    #[allow(dead_code)]
    pub access_list: AccessList,
    pub recovery_id: u8,
    pub r: U256,
    pub s: U256,
    pub from: H160,
}

impl Base for Eip2930 {
    fn nonce(&self) -> u64 {
        self.nonce
    }
    fn to(&self) -> Option<H160> {
        self.to
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
    fn gas_price(&self) -> U256 {
        self.gas_price
    }
    fn hash_unsign(&self, rlp: &Rlp) -> crate::error::Result<H256> {
        let rlp2 = rlp_at(rlp, 8)?;
        let rlp1 = rlp_header(rlp2.len());

        Ok(H256::from(hashv(&[&[1], &rlp1, rlp2]).to_bytes()))
    }
    fn rs(&self) -> (U256, U256) {
        (self.r, self.s)
    }
    fn recovery_id(&self) -> crate::error::Result<u8> {
        Ok(self.recovery_id)
    }
    fn chain_id(&self) -> u64 {
        self.chain_id.as_u64()
    }
    fn from(&self) -> H160 {
        self.from
    }
    fn set_from(&mut self, from: H160) {
        self.from = from;
    }
    #[cfg(test)]
    fn access_list(&self) -> Option<&AccessList> {
        Some(&self.access_list)
    }
}
impl Eip2930 {
    pub fn rlp_at_chain_id(rlp: &rlp::Rlp) -> crate::error::Result<U256> {
        let chain = fix(rlp, 0)?;
        Ok(chain)
    }

    pub fn from_rlp(rlp: &rlp::Rlp) -> crate::error::Result<Self> {
        check_rlp(rlp, 11)?;
        let chain_id = Eip2930::rlp_at_chain_id(rlp)?;
        let nonce: u64 = rlp.val_at(1)?;
        let gas_price = fix(rlp, 2)?;
        let gas_limit = fix(rlp, 3)?;
        let to = decode_to(rlp, 4)?;
        let value = fix(rlp, 5)?;
        let data = rlp.val_at(6)?;
        let access_list = rlp.val_at(7)?;
        let recovery_id: u8 = rlp.at(8)?.as_val()?;
        let r = fix(rlp, 9)?;
        let s = fix(rlp, 10)?;

        Ok(Eip2930 {
            chain_id,
            nonce,
            gas_price,
            gas_limit,
            to,
            value,
            data: Some(data),
            access_list,
            recovery_id,
            r,
            s,
            from: H160::default(),
        })
    }
}
