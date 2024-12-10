use {
    super::{Base, Legacy},
    crate::{
        error::{Result, RomeProgramError::*},
        tx::{eip1559::Eip1559, eip2930::Eip2930},
    },
    evm::H160,
    rlp::Rlp,
    solana_program::{keccak::hash, msg, secp256k1_recover::secp256k1_recover},
    std::ops::{Deref, DerefMut},
};

enum TxType<'a> {
    Legacy(Rlp<'a>),
    Eip2930(Rlp<'a>),
    Eip1559(Rlp<'a>),
}

pub struct Tx {
    tx: Box<dyn Base>,
}

impl Deref for Tx {
    type Target = Box<dyn Base>;
    fn deref(&self) -> &Self::Target {
        &self.tx
    }
}
impl DerefMut for Tx {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tx
    }
}

impl Tx {
    #[cfg(not(target_os = "solana"))]
    pub fn from_legacy(tx: Legacy) -> Self {
        Self { tx: Box::new(tx) }
    }

    fn tx_type(data: &[u8]) -> Result<TxType> {
        let rlp = Rlp::new(data);

        if rlp.is_list() {
            Ok(TxType::Legacy(rlp))
        } else {
            // if it is not wrapped then we need to use rlp.as_raw instead of rlp.data
            let first_byte = *rlp
                .as_raw()
                .first()
                .ok_or(Custom("RLP: empty slice".to_string()))?;

            let (first, data) = if first_byte <= 0x7f {
                (first_byte, rlp.as_raw())
            } else {
                let data = rlp.data()?;
                let first = *data.first().ok_or(Custom("RLP: empty slice".to_string()))?;
                (first, data)
            };

            let bytes = data.get(1..).ok_or(Custom("RLP: no tx body".to_string()))?;
            let rlp = Rlp::new(bytes);
            match first {
                0x01 => Ok(TxType::Eip2930(rlp)),
                0x02 => Ok(TxType::Eip1559(rlp)),
                _ => Err(Custom(format!("RLP: invalid tx type {first}"))),
            }
        }
    }

    pub fn from_instruction(data: &[u8]) -> Result<Self> {
        let tx_type = Tx::tx_type(data)?;

        let (mut tx, rlp): (Box<dyn Base>, Rlp) = match tx_type {
            TxType::Legacy(rlp) => {
                let legacy = Legacy::from_rlp(&rlp)?;
                (Box::new(legacy), rlp)
            }
            TxType::Eip2930(rlp) => {
                let eip2930 = Eip2930::from_rlp(&rlp)?;
                (Box::new(eip2930), rlp)
            }
            TxType::Eip1559(rlp) => {
                let eip1559 = Eip1559::from_rlp(&rlp)?;
                (Box::new(eip1559), rlp)
            }
        };

        let from = Tx::recovery_from(&*tx, &rlp)?;
        tx.set_from(from);

        Ok(Self { tx })
    }

    pub fn chain_id_from_rlp(data: &[u8]) -> Result<u64> {
        let tx_type = Tx::tx_type(data)?;

        let chain = match tx_type {
            TxType::Legacy(rlp) => Legacy::rlp_at_chain_id(&rlp)?,
            TxType::Eip2930(rlp) => Eip2930::rlp_at_chain_id(&rlp)?,
            TxType::Eip1559(rlp) => Eip1559::rlp_at_chain_id(&rlp)?,
        };

        Ok(chain.as_u64())
    }

    fn recovery_from(tx: &dyn Base, rlp: &Rlp) -> Result<H160> {
        let mut rs = [0_u8; 64];

        let (r, s) = tx.rs();
        r.to_big_endian(&mut rs[0..32]);
        s.to_big_endian(&mut rs[32..64]);

        let recovery_id = tx.recovery_id()?;
        let hash = tx.hash_unsign(rlp)?;

        let pub_key = Tx::syscall(hash.as_bytes(), recovery_id, &rs)?;
        let from = H160::from_slice(&pub_key[12..]);

        Ok(from)
    }
    pub fn syscall(hash_to_sign: &[u8], recovery: u8, rs: &[u8]) -> Result<[u8; 32]> {
        let pub_key = secp256k1_recover(hash_to_sign, recovery, rs).map_err(|e| {
            msg!("{:?}", e);
            InvalidEthereumSignature(e.to_string())
        })?;
        let pub_key = hash(&pub_key.to_bytes()).to_bytes();

        Ok(pub_key)
    }

    #[cfg(test)]
    fn access_list(&self) -> Option<&super::eip2930::AccessList> {
        self.tx.access_list()
    }
    #[cfg(test)]
    fn vrs(&self) -> (u8, evm::U256, evm::U256) {
        let recovery_id = self.tx.recovery_id().unwrap();
        let (r, s) = self.tx.rs();
        (recovery_id, r, s)
    }
}

#[cfg(test)]
mod tests {
    use {
        crate::tx::{
            eip2930::{AccessList, AccessListItem},
            tx::Tx,
        },
        evm::{H160, U256},
    };

    #[test]
    fn eip2930_without_access_list() {
        let raw_tx = hex::decode("01f901ef018209068508d8f9fc0083124f8094f5b4f13bdbe12709bd3ea280ebf4b936e99b20f280b90184c5d404940000000000000000000000000000000000000000000000000c4d67a76e15d8190000000000000000000000000000000000000000000000000029d9d8fb7440000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001200000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000020000000000000000000000007b73644935b8e68019ac6356c40661e1bc315860000000000000000000000000761d38e5ddf6ccf6cf7c55759d5210750b5d60f30000000000000000000000000000000000000000000000000000000000000000000000000000000000000000381fe4eb128db1621647ca00965da3f9e09f4fac000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2000000000000000000000000000000000000000000000000000000000000000ac001a0881e7f5298290794bcaa0294986db5c375cbf135dd3c21456b159c470568b687a061fc5f52abab723053fbedf29e1c60b89006416d6c86e1c54ef85a3e84f2dc6e").unwrap();
        let mut tx = Tx::from_instruction(&raw_tx).unwrap();
        let from = hex::decode("82a33964706683db62b85a59128ce2fc07c91658").unwrap();
        let to = hex::decode("f5b4f13bdbe12709bd3ea280ebf4b936e99b20f2").unwrap();
        let data = hex::decode("c5d404940000000000000000000000000000000000000000000000000c4d67a76e15d8190000000000000000000000000000000000000000000000000029d9d8fb7440000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001200000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000020000000000000000000000007b73644935b8e68019ac6356c40661e1bc315860000000000000000000000000761d38e5ddf6ccf6cf7c55759d5210750b5d60f30000000000000000000000000000000000000000000000000000000000000000000000000000000000000000381fe4eb128db1621647ca00965da3f9e09f4fac000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2000000000000000000000000000000000000000000000000000000000000000a")
            .unwrap();
        let r = U256::from_big_endian(
            &hex::decode("881e7f5298290794bcaa0294986db5c375cbf135dd3c21456b159c470568b687")
                .unwrap(),
        );
        let s = U256::from_big_endian(
            &hex::decode("61fc5f52abab723053fbedf29e1c60b89006416d6c86e1c54ef85a3e84f2dc6e")
                .unwrap(),
        );

        assert_eq!(tx.from(), H160::from_slice(&from));
        assert_eq!(tx.to(), Some(H160::from_slice(&to)));
        assert_eq!(tx.chain_id(), 1_u64);
        assert_eq!(tx.nonce(), 2310_u64);
        assert_eq!(tx.gas_limit(), 1_200_000_u64.into());
        assert_eq!(tx.gas_price(), 38_000_000_000_u64.into());
        assert_eq!(tx.value(), 0_u64.into());
        assert_eq!(tx.data().as_ref().unwrap(), &data);
        assert_eq!(tx.access_list().unwrap().0.len(), 0);
        assert_eq!(tx.vrs(), (1_u8, r, s));
    }

    #[test]
    fn eip2930_with_access_list() {
        let raw_tx = hex::decode("01f90126018223ff850a02ffee00830f4240940000000000a8fb09af944ab3baf7a9b3e1ab29d880b876200200001525000000000b69ffb300000000557b933a7c2c45672b610f8954a3deb39a51a8cae53ec727dbdeb9e2d5456c3be40cff031ab40a55724d5c9c618a2152e99a45649a3b8cf198321f46720b722f4ec38f99ba3bb1303258d2e816e6a95b25647e01bd0967c1b9599fa3521939871d1d0888f845d694724d5c9c618a2152e99a45649a3b8cf198321f46c0d694720b722f4ec38f99ba3bb1303258d2e816e6a95bc0d69425647e01bd0967c1b9599fa3521939871d1d0888c001a08323efae7b9993bd31a58da7924359d24b5504aa2b33194fcc5ae206e65d2e62a054ce201e3b4b5cd38eb17c56ee2f9111b2e164efcd57b3e70fa308a0a51f7014").unwrap();
        let mut tx = Tx::from_instruction(&raw_tx).unwrap();
        let from = hex::decode("e9c790e8fde820ded558a4771b72eec916c04763").unwrap();
        let to = hex::decode("0000000000a8fb09af944ab3baf7a9b3e1ab29d8").unwrap();
        let data = hex::decode("200200001525000000000b69ffb300000000557b933a7c2c45672b610f8954a3deb39a51a8cae53ec727dbdeb9e2d5456c3be40cff031ab40a55724d5c9c618a2152e99a45649a3b8cf198321f46720b722f4ec38f99ba3bb1303258d2e816e6a95b25647e01bd0967c1b9599fa3521939871d1d0888")
            .unwrap();
        let r = U256::from_big_endian(
            &hex::decode("8323efae7b9993bd31a58da7924359d24b5504aa2b33194fcc5ae206e65d2e62")
                .unwrap(),
        );
        let s = U256::from_big_endian(
            &hex::decode("54ce201e3b4b5cd38eb17c56ee2f9111b2e164efcd57b3e70fa308a0a51f7014")
                .unwrap(),
        );
        let address1 = hex::decode("724d5c9c618a2152e99a45649a3b8cf198321f46").unwrap();
        let address2 = hex::decode("720b722f4ec38f99ba3bb1303258d2e816e6a95b").unwrap();
        let address3 = hex::decode("25647e01bd0967c1b9599fa3521939871d1d0888").unwrap();
        let access_list = AccessList(vec![
            AccessListItem {
                address: H160::from_slice(&address1),
                storage_keys: vec![],
            },
            AccessListItem {
                address: H160::from_slice(&address2),
                storage_keys: vec![],
            },
            AccessListItem {
                address: H160::from_slice(&address3),
                storage_keys: vec![],
            },
        ]);

        assert_eq!(tx.from(), H160::from_slice(&from));
        assert_eq!(tx.to(), Some(H160::from_slice(&to)));
        assert_eq!(tx.chain_id(), 1_u64);
        assert_eq!(tx.nonce(), 9215_u64);
        assert_eq!(tx.gas_limit(), 1_000_000_u64.into());
        assert_eq!(tx.gas_price(), 43_000_000_000_u64.into());
        assert_eq!(tx.value(), 0_u64.into());
        assert_eq!(tx.data().as_ref().unwrap(), &data);
        assert_eq!(tx.access_list().unwrap(), &access_list);
        assert_eq!(tx.vrs(), (1_u8, r, s));
    }

    #[test]
    fn lecacy() {
        let raw_tx = hex::decode("f9015482078b8505d21dba0083022ef1947a250d5630b4cf539739df2c5dacb4c659f2488d880c46549a521b13d8b8e47ff36ab50000000000000000000000000000000000000000000066ab5a608bd00a23f2fe000000000000000000000000000000000000000000000000000000000000008000000000000000000000000048c04ed5691981c42154c6167398f95e8f38a7ff00000000000000000000000000000000000000000000000000000000632ceac70000000000000000000000000000000000000000000000000000000000000002000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20000000000000000000000006c6ee5e31d828de241282b9606c8e98ea48526e225a0c9077369501641a92ef7399ff81c21639ed4fd8fc69cb793cfa1dbfab342e10aa0615facb2f1bcf3274a354cfe384a38d0cc008a11c2dd23a69111bc6930ba27a8").unwrap();
        let tx = Tx::from_instruction(&raw_tx).unwrap();
        let from = hex::decode("a12e1462d0ced572f396f58b6e2d03894cd7c8a4").unwrap();

        assert_eq!(tx.from(), H160::from_slice(&from));
    }

    #[test]
    fn test_recovery_sender() {
        // https://github.com/ethereum/go-ethereum/blob/master/core/types/transaction_signing_test.go
        // Tests that the rlp decoding properly extracts the from address
        let rlp_tx_hex =
            ["f864808504a817c800825208943535353535353535353535353535353535353535808025a0044852b2a670ade5407e78fb2863c51de9fcb96542a07186fe3aeda6bb8a116da0044852b2a670ade5407e78fb2863c51de9fcb96542a07186fe3aeda6bb8a116d",
                "f864018504a817c80182a410943535353535353535353535353535353535353535018025a0489efdaa54c0f20c7adf612882df0950f5a951637e0307cdcb4c672f298b8bcaa0489efdaa54c0f20c7adf612882df0950f5a951637e0307cdcb4c672f298b8bc6",
                "f864028504a817c80282f618943535353535353535353535353535353535353535088025a02d7c5bef027816a800da1736444fb58a807ef4c9603b7848673f7e3a68eb14a5a02d7c5bef027816a800da1736444fb58a807ef4c9603b7848673f7e3a68eb14a5",
                "f865038504a817c803830148209435353535353535353535353535353535353535351b8025a02a80e1ef1d7842f27f2e6be0972bb708b9a135c38860dbe73c27c3486c34f4e0a02a80e1ef1d7842f27f2e6be0972bb708b9a135c38860dbe73c27c3486c34f4de",
                "f865048504a817c80483019a28943535353535353535353535353535353535353535408025a013600b294191fc92924bb3ce4b969c1e7e2bab8f4c93c3fc6d0a51733df3c063a013600b294191fc92924bb3ce4b969c1e7e2bab8f4c93c3fc6d0a51733df3c060",
                "f865058504a817c8058301ec309435353535353535353535353535353535353535357d8025a04eebf77a833b30520287ddd9478ff51abbdffa30aa90a8d655dba0e8a79ce0c1a04eebf77a833b30520287ddd9478ff51abbdffa30aa90a8d655dba0e8a79ce0c1",
                "f866068504a817c80683023e3894353535353535353535353535353535353535353581d88025a06455bf8ea6e7463a1046a0b52804526e119b4bf5136279614e0b1e8e296a4e2fa06455bf8ea6e7463a1046a0b52804526e119b4bf5136279614e0b1e8e296a4e2d",
                "f867078504a817c807830290409435353535353535353535353535353535353535358201578025a052f1a9b320cab38e5da8a8f97989383aab0a49165fc91c737310e4f7e9821021a052f1a9b320cab38e5da8a8f97989383aab0a49165fc91c737310e4f7e9821021",
                "f867088504a817c8088302e2489435353535353535353535353535353535353535358202008025a064b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c12a064b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c10",
                "f867098504a817c809830334509435353535353535353535353535353535353535358202d98025a052f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afba052f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afb"];
        let rlp_tx = rlp_tx_hex
            .iter()
            .map(|rlp_str| hex::decode(rlp_str).unwrap())
            .collect::<Vec<Vec<u8>>>();

        let expected_hex = [
            "f0f6f18bca1b28cd68e4357452947e021241e9ce",
            "23ef145a395ea3fa3deb533b8a9e1b4c6c25d112",
            "2e485e0c23b4c3c542628a5f672eeab0ad4888be",
            "82a88539669a3fd524d669e858935de5e5410cf0",
            "f9358f2538fd5ccfeb848b64a96b743fcc930554",
            "a8f7aba377317440bc5b26198a363ad22af1f3a4",
            "f1f571dc362a0e5b2696b8e775f8491d3e50de35",
            "d37922162ab7cea97c97a87551ed02c9a38b7332",
            "9bddad43f934d313c2b79ca28a432dd2b7281029",
            "3c24d7329e92f84f08556ceb6df1cdb0104ca49f",
        ];

        let expected = expected_hex.iter().map(|addr| {
            let bin = hex::decode(addr).unwrap();
            H160::from_slice(&bin)
        });

        // decoding will do sender recovery and we don't expect any of these to error, so we should
        // check that the address matches for each decoded transaction
        let tx_senders = rlp_tx.iter().map(|rlp| {
            let tx = Tx::from_instruction(rlp).unwrap();
            tx.from()
        });

        for (sender, expected) in tx_senders.zip(expected) {
            assert_eq!(sender, expected);
        }
    }

    #[test]
    fn eip1559() {
        let raw_tx = hex::decode("02f86f0102843b9aca0085029e7822d68298f094d9e1459a7a482635700cbc20bbaf52d495ab9c9680841b55ba3ac080a0c199674fcb29f353693dd779c017823b954b3c69dffa3cd6b2a6ff7888798039a028ca912de909e7e6cdef9cdcaf24c54dd8c1032946dfa1d85c206b32a9064fe8").unwrap();
        let tx = Tx::from_instruction(&raw_tx).unwrap();
        let from = hex::decode("001e2b7dE757bA469a57bF6b23d982458a07eFcE").unwrap();
        let to = hex::decode("D9e1459A7A482635700cBc20BBAF52D495Ab9C96").unwrap();

        assert_eq!(tx.from(), H160::from_slice(&from));
        assert_eq!(tx.to(), Some(H160::from_slice(&to)));
    }

    #[test]
    #[should_panic]
    fn unknown_tx_type() {
        let raw_tx = hex::decode("0x03f9011d83aa36a7820fa28477359400852e90edd0008252089411e9ca82a3a762b4b5bd264d4173a242e7a770648080c08504a817c800f8a5a0012ec3d6f66766bedb002a190126b3549fce0047de0d4c25cffce0dc1c57921aa00152d8e24762ff22b1cfd9f8c0683786a7ca63ba49973818b3d1e9512cd2cec4a0013b98c6c83e066d5b14af2b85199e3d4fc7d1e778dd53130d180f5077e2d1c7a001148b495d6e859114e670ca54fb6e2657f0cbae5b08063605093a4b3dc9f8f1a0011ac212f13c5dff2b2c6b600a79635103d6f580a4221079951181b25c7e654901a0c8de4cced43169f9aa3d36506363b2d2c44f6c49fc1fd91ea114c86f3757077ea01e11fdd0d1934eda0492606ee0bb80a7bf8f35cc5f86ec60fe5031ba48bfd544").unwrap();
        let _ = Tx::from_instruction(&raw_tx).unwrap();
    }
}
