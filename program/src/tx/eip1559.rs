use crate::tx::{check_rlp, decode_to};
use {
    super::{eip2930::AccessList, fix, Base},
    crate::error::Result,
    evm::{H160, U256},
};

#[derive(Debug, Clone)]
pub struct Eip1559 {
    pub chain_id: U256,
    pub nonce: u64,
    pub max_priority_fee_per_gas: U256,
    pub max_fee_per_gas: U256,
    pub gas_limit: U256,
    pub to: Option<H160>,
    pub value: U256,
    pub data: Vec<u8>,
    pub access_list: AccessList,
    pub recovery_id: u8,
    pub r: U256,
    pub s: U256,
    pub from: H160,
}

impl Base for Eip1559 {
    fn nonce(&self) -> u64 {
        self.nonce
    }
    fn to(&self) -> Option<H160> {
        self.to
    }
    fn value(&self) -> U256 {
        self.value
    }
    fn data(&self) -> &Vec<u8> {
        &self.data
    }
    fn gas_limit(&self) -> U256 {
        self.gas_limit
    }
    fn gas_price(&self) -> U256 {
        self.max_priority_fee_per_gas // TODO: figure out it
    }
    fn to_rlp(&self) -> Vec<u8> {
        let rlp = rlp::encode(self);

        let mut prefixed = vec![];
        prefixed.extend_from_slice(&[2]);
        prefixed.extend_from_slice(rlp.as_ref());

        prefixed
    }
    fn rs(&self) -> (U256, U256) {
        (self.r, self.s)
    }
    fn recovery_id(&self) -> Result<u8> {
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
impl Eip1559 {
    pub fn from_rlp(rlp: &rlp::Rlp) -> Result<Self> {
        check_rlp(rlp, 12)?;
        let chain_id = fix(rlp, 0)?;
        let nonce: u64 = rlp.val_at(1)?;
        let max_priority_fee_per_gas = fix(rlp, 2)?;
        let max_fee_per_gas = fix(rlp, 3)?;
        let gas_limit = fix(rlp, 4)?;
        let to = decode_to(rlp, 5)?;
        let value = fix(rlp, 6)?;
        let data = rlp.val_at(7)?;
        let access_list = rlp.val_at(8)?;
        let recovery_id: u8 = rlp.at(9)?.as_val()?;
        let r = fix(rlp, 10)?;
        let s = fix(rlp, 11)?;

        Ok(Eip1559 {
            chain_id,
            nonce,
            max_priority_fee_per_gas,
            max_fee_per_gas,
            gas_limit,
            to,
            value,
            data,
            access_list,
            recovery_id,
            r,
            s,
            from: H160::default(),
        })
    }
}

impl rlp::Encodable for Eip1559 {
    fn rlp_append(&self, stream: &mut rlp::RlpStream) {
        stream.begin_list(9);

        stream.append(&self.chain_id);
        stream.append(&self.nonce);
        stream.append(&self.max_priority_fee_per_gas);
        stream.append(&self.max_fee_per_gas);
        stream.append(&self.gas_limit);

        match self.to.as_ref() {
            Some(to) => stream.append(to),
            None => stream.append(&""),
        };

        stream.append(&self.value);
        stream.append(&self.data);
        stream.append(&self.access_list);
    }
}
