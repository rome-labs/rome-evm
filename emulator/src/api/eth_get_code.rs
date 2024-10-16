use {
    crate::state::State,
    rome_evm::{error::Result, Code, Data, H160},
    solana_client::rpc_client::RpcClient,
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey},
    std::sync::Arc,
};

pub fn eth_get_code<'a>(
    program_id: &'a Pubkey,
    address: &'a H160,
    client: Arc<RpcClient>,
    chain: u64,
) -> Result<Vec<u8>> {
    msg!("eth_getCode");
    let state = State::new(program_id, None, client, chain)?;
    let code = if let Ok(mut bind) = state.info_addr(address, false) {
        let info = bind.into_account_info();
        let code = Code::from_account(&info)?.to_vec();
        code
    } else {
        vec![]
    };

    Ok(code)
}
