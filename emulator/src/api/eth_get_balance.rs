use {
    crate::state::State,
    rome_evm::{error::Result, AccountState, Data, H160, U256},
    solana_client::rpc_client::RpcClient,
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey},
    std::sync::Arc,
};

pub fn eth_get_balance<'a>(
    program_id: &'a Pubkey,
    address: &'a H160,
    client: Arc<RpcClient>,
    chain: u64,
) -> Result<U256> {
    msg!("eth_getBalance");
    let state = State::new(program_id, None, client, chain)?;
    let balance = if let Ok(mut bind) = state.info_addr(address, false) {
        let info = bind.into_account_info();
        let balance = AccountState::from_account(&info)?.balance;
        balance
    } else {
        U256::zero()
    };

    Ok(balance)
}
