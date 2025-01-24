use {
    crate::state::State,
    rome_evm::{error::Result, StateHolder, H256},
    solana_client::rpc_client::RpcClient,
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey},
    std::sync::Arc,
};

pub fn confirm_tx_iterative(
    program_id: &Pubkey,
    holder: u64,
    hash: H256,
    signer: &Pubkey,
    client: Arc<RpcClient>,
    chain: u64,
    session: u64,
) -> Result<bool> {
    msg!("confirmation of iterative tx");
    let state = State::new(program_id, Some(*signer), client, chain)?;
    let mut bind = state.info_state_holder(holder, false)?;
    let info = bind.into_account_info();

    if !StateHolder::is_linked(&info, hash, session)? {
        msg!(
            "Iterative tx is not linked to state holder account: {}",
            info.key
        );
        return Ok(false);
    }

    Ok(StateHolder::get_iteration(&info)?.is_complete())
}
