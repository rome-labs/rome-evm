use {
    crate::state::State,
    rome_evm::{
        error::{Result, RomeProgramError::StorageAccountNotFound},
        origin::Origin,
        H160, U256,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{msg, pubkey::Pubkey},
    std::sync::Arc,
};

pub fn eth_get_storage_at<'a>(
    program_id: &'a Pubkey,
    address: &'a H160,
    slot: &'a U256,
    client: Arc<RpcClient>,
) -> Result<U256> {
    msg!("eth_getStorage_at");
    let state = State::new(program_id, None, client)?;

    let value = match state.storage(address, slot) {
        Ok(x) => x.unwrap_or(U256::zero()),
        Err(StorageAccountNotFound(_, _, _)) => U256::zero(),
        Err(e) => return Err(e),
    };

    Ok(value)
}