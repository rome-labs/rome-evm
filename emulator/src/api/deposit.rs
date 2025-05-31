use {
    super::Emulation,
    crate::{state::State, context::ContextAt,},
    rome_evm::{
        api::deposit::{args, mint, from_rlp, spl_transfer},
        error::Result,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{msg, pubkey::Pubkey},
    std::sync::Arc,
};

pub fn deposit<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!("Instruction: deposit");

    let (chain, rlp) = args(data)?;
    let state = State::new(program_id, Some(*signer), client, chain)?;
    let context = ContextAt::new(&state);

    let tx = from_rlp(rlp)?;
    mint(&tx, &state, &context)?;

    let wallet = state.info_sol_wallet(false)?;
    spl_transfer(tx.mint, &state, &wallet.0)?;

    Emulation::without_vm(&state)
}
