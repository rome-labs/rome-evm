use anyhow::Context;
use rome_evm::{H160, U256};
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

pub struct InitParams {
    /// Solana RPC URL
    pub rpc_url: String,
    /// Rome EVM owner keypair
    pub owner_keypair: PathBuf,
    /// Rome EVM keypair
    pub evm_keypair: PathBuf,
    /// Mint to address
    pub mint_to: H160,
    /// Mint amount
    pub mint_amount: U256,
}

impl InitParams {
    /// Get the init params from the environment
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            rpc_url: env::var("SOLANA_RPC").context("SOLANA_RPC not specified")?,
            owner_keypair: env::var("CONTRACT_OWNER_KEYPAIR")
                .context("CONTRACT_OWNER_KEYPAIR not specified")?
                .parse()
                .context("Failed to parse CONTRACT_OWNER_KEYPAIR path")?,
            evm_keypair: env::var("ROME_EVM_KEYPAIR")
                .context("ROME_EVM_KEYPAIR not specified")?
                .parse()
                .context("Failed to parse ROME_EVM_KEYPAIR path")?,
            mint_to: env::var("MINT_TO")
                .context("MINT_TO not specified")
                .map(|s| {
                    H160::from_str(&s).map_err(|e| {
                        anyhow::anyhow!("Failed to parse H160 from MINT_TO {s:#?} {:?}", e)
                    })
                })??,
            mint_amount: env::var("MINT_AMOUNT")
                .context("MINT_AMOUNT not specified")
                .map(|s| {
                    U256::from_dec_str(&s).map_err(|e| {
                        anyhow::anyhow!("Failed to parse U256 from MINT_AMOUNT {s:#?} {:?}", e)
                    })
                })??,
        })
    }
}
