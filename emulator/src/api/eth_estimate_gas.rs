use {
    super::{do_tx_iterative::iterative_tx, fake},
    crate::{context::ContextIt, state::State, Emulation},
    rome_evm::{
        error::Result, tx::legacy::Legacy,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{msg, pubkey::Pubkey},
    std::sync::Arc,
};

pub fn eth_estimate_gas(
    program_id: &Pubkey,
    legacy: Legacy,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!(">> eth_estimateGas emulator started ..");
    let state = State::new(
        program_id,
        Some(fake::ID),
        Arc::clone(&client),
        legacy.chain_id.as_u64(),
    )?;
    
    let context = ContextIt::new_gas_estimate(&state, legacy)?;
    let emulation = iterative_tx(&state, context, true);
    msg!(">> eth_estimateGas emulator finished");

    emulation
}
