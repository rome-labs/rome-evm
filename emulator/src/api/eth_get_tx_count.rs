use {
    crate::state::State,
    rome_evm::{error::Result, AccountState, Data, H160},
    solana_client::rpc_client::RpcClient,
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey},
    std::sync::Arc,
};

pub fn eth_get_tx_count<'a>(
    program_id: &'a Pubkey,
    address: &'a H160,
    client: Arc<RpcClient>,
    chain: u64,
) -> Result<u64> {
    msg!("eth_getTransactionCount");
    let state = State::new(program_id, None, client, chain)?;
    let nonce = if let Ok(mut bind) = state.info_addr(address, false) {
        let info = bind.into_account_info();
        let nonce = AccountState::from_account(&info)?.nonce;
        nonce
    } else {
        0
    };

    Ok(nonce)
}
