use {
    super::{check_rlp, decode_to, fix, rlp_at, rlp_header, Base},
    crate::error::{Result, RomeProgramError::*},
    evm::{H160, H256, U256},
    rlp::{Rlp, RlpStream},
    solana_program::keccak::hashv,
};

#[derive(Debug, Clone, Default)]
pub struct Legacy {
    pub nonce: u64,
    pub gas_price: U256,
    pub gas_limit: U256,
    pub to: Option<H160>,
    pub value: U256,
    pub data: Option<Vec<u8>>,
    pub v: U256,
    pub r: U256,
    pub s: U256,
    pub chain_id: U256,
    pub from: H160,
}

impl Base for Legacy {
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
    fn hash_unsign(&self, rlp: &Rlp) -> Result<H256> {
        let rlp2 = rlp_at(rlp, 6)?;

        let mut stream = RlpStream::new();
        stream.append(&self.chain_id);
        stream.append(&"");
        stream.append(&"");
        let rlp3 = stream.as_raw();

        let rlp1 = rlp_header(rlp2.len() + rlp3.len());

        Ok(H256::from(hashv(&[&rlp1, rlp2, rlp3]).to_bytes()))
    }
    fn rs(&self) -> (U256, U256) {
        (self.r, self.s)
    }
    fn recovery_id(&self) -> Result<u8> {
        let id = if self.v >= 35.into() {
            ((self.v % 2) == U256::zero()) as u8
        } else if self.v == 27.into() {
            0_u8
        } else if self.v == 28.into() {
            1_u8
        } else {
            return Err(Custom("incorrect tx.v".to_string()));
        };

        Ok(id)
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
    fn access_list(&self) -> Option<&super::eip2930::AccessList> {
        None
    }
}

impl Legacy {
    pub fn rlp_at_chain_id(rlp: &rlp::Rlp) -> Result<U256> {
        let v = fix(rlp, 6)?;

        if v >= 35.into() {
            Ok((v - 1) / 2 - 17)
        } else if v == 27.into() || v == 28.into() {
            Err(IncorrectChainId(None))
        } else {
            Err(Custom("incorrect tx.v".to_string()))
        }
    }

    pub fn from_rlp(rlp: &rlp::Rlp) -> Result<Self> {
        check_rlp(rlp, 9)?;
        let nonce: u64 = rlp.val_at(0)?;
        let gas_price = fix(rlp, 1)?;
        let gas_limit = fix(rlp, 2)?;
        let to = decode_to(rlp, 3)?;
        let value = fix(rlp, 4)?;
        let data = rlp.val_at(5)?;
        let v = fix(rlp, 6)?;
        let r = fix(rlp, 7)?;
        let s = fix(rlp, 8)?;

        let chain_id = Legacy::rlp_at_chain_id(rlp)?;

        Ok(Legacy {
            nonce,
            gas_price,
            gas_limit,
            to,
            value,
            data: Some(data),
            v,
            r,
            s,
            chain_id,
            from: H160::default(),
        })
    }
}
