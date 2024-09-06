use {
    rome_evm::{state::pda_balance, H160, U256},
    solana_client::rpc_client::RpcClient,
    solana_program::instruction::AccountMeta,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        instruction::Instruction,
        signer::{keypair::read_keypair_file, Signer},
        system_program::ID as SystemID,
        transaction::Transaction,
    },
    std::{env, path::Path, str::FromStr, sync::Arc},
};

fn main() {
    let rpc_url = env::var("SOLANA_RPC").expect("SOLANA_RPC not specified");
    let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let owner_keypair_path =
        env::var("CONTRACT_OWNER_KEYPAIR").expect("CONTRACT_OWNER_KEYPAIR not specified");
    let owner = Arc::new(
        read_keypair_file(Path::new(&owner_keypair_path)).expect("read owner keypair error"),
    );

    let program_id_path = env::var("ROME_EVM_KEYPAIR").expect("ROME_EVM_KEYPAIR not specified");
    let program_id = read_keypair_file(Path::new(&program_id_path))
        .expect("read program_id keypair error")
        .pubkey();

    let mint_address = H160::from_str(&env::var("MINT_TO").expect("MINT_TO not specified"))
        .expect("Failed to parse H160 from MINT_TO");
    let (solana_mint_address, _) = pda_balance(&mint_address, &program_id);

    let mint_amount =
        U256::from_dec_str(&env::var("MINT_AMOUNT").expect("MINT_AMOUNT not specified"))
            .expect("Failed to parse U256 from MINT_AMOUNT");

    print!(
        "Minting {:?} native tokens to address {:?} ...",
        mint_amount, mint_address
    );

    let mut data = vec![0x01];
    data.extend(mint_address.as_bytes());
    let mut index_be = [0_u8; 32];
    mint_amount.to_big_endian(&mut index_be);
    data.extend_from_slice(&index_be);

    let instr = Instruction::new_with_bytes(
        program_id,
        &data,
        vec![
            AccountMeta {
                pubkey: SystemID,
                is_signer: false,
                is_writable: false,
            },
            AccountMeta {
                pubkey: solana_mint_address,
                is_signer: false,
                is_writable: true,
            },
            AccountMeta {
                pubkey: owner.pubkey(),
                is_signer: true,
                is_writable: true,
            },
        ],
    );
    let blockhash = rpc_client.get_latest_blockhash().unwrap();

    let tx =
        Transaction::new_signed_with_payer(&[instr], Some(&owner.pubkey()), &[&owner], blockhash);

    let signature = rpc_client
        .send_and_confirm_transaction(&tx)
        .expect("Unable to send CreateBalance instruction");
    print!("CreateBalance sent: {:?}", signature);
}
