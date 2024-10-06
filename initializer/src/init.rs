use crate::params::InitParams;
use anyhow::Context;
use rome_evm::{state::pda_balance, H160, U256};
use solana_client::rpc_client::RpcClient;
use solana_program::instruction::AccountMeta;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Keypair,
    signer::{keypair::read_keypair_file, Signer},
    transaction::Transaction,
};

pub struct Intializer {
    rpc_client: RpcClient,
    owner: Keypair,
    program_id: Pubkey,
    mint_address: H160,
    mint_amount: U256,
}

impl Intializer {
    /// Convert the [InitParams] to the [Intializer]
    pub fn new(params: InitParams) -> anyhow::Result<Intializer> {
        let rpc_client =
            RpcClient::new_with_commitment(params.rpc_url, CommitmentConfig::confirmed());

        let owner = read_keypair_file(&params.owner_keypair).expect("read owner keypair error");
        let program_id = read_keypair_file(&params.evm_keypair)
            .expect("read program_id keypair error")
            .pubkey();
        let mint_address = params.mint_to;
        let mint_amount = params.mint_amount;

        Ok(Intializer {
            rpc_client,
            owner,
            program_id,
            mint_address,
            mint_amount,
        })
    }
    /// Create mint [Instruction]
    pub fn create_mint_ix(&self) -> Instruction {
        // Get the solana mint address
        let (solana_mint_address, _) = pda_balance(&self.mint_address, &self.program_id);

        // prepare the data for the instruction
        let mut data = vec![0x01];
        data.extend(self.mint_address.as_bytes());

        // Convert the mint amount to big endian
        let mut index_be = [0_u8; 32];
        self.mint_amount.to_big_endian(&mut index_be);

        // Append the amount to the data
        data.extend_from_slice(&index_be);

        // Create the instruction
        Instruction::new_with_bytes(
            self.program_id,
            &data,
            vec![
                AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
                AccountMeta::new(solana_mint_address, false),
                AccountMeta::new(self.owner.pubkey(), true),
            ],
        )
    }

    /// Mint the native tokens
    pub fn init(&self) -> anyhow::Result<()> {
        println!(
            "Minting {:?} native tokens to address {:?} ...",
            self.mint_amount, self.mint_address
        );

        // Create the mint instruction
        let ix = self.create_mint_ix();

        // Get the latest blockhash
        let blockhash = self
            .rpc_client
            .get_latest_blockhash()
            .context("Unable to get latest blockhash")?;

        // Create the transaction
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.owner.pubkey()),
            &[&self.owner],
            blockhash,
        );

        // Send the transaction
        let signature = self
            .rpc_client
            .send_and_confirm_transaction(&tx)
            .map_err(|e| anyhow::anyhow!("Unable to send transaction: {:?}", e))?;

        // Print the transaction signature
        println!("Signature: {:?}", signature);
        println!(
            "https://explorer.solana.com/tx/{}?cluster=custom&customUrl={}",
            signature,
            self.rpc_client.url()
        );

        Ok(())
    }
}
