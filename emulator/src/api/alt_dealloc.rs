use {
    super::Emulation,
    crate::state::State,
    rome_evm::{
        api::{
            alt_dealloc::{args, get_dealloc_actions, }
        },
        error::Result,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{
        account_info::IntoAccountInfo, msg, pubkey::Pubkey,
    },
    std::sync::Arc,
    super::alt_alloc::track_slots,
};

pub fn alt_dealloc<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!("Instruction: alt_dealloc");

    let (holder, chain, session) = args(data)?;
    let state = State::new(program_id, Some(*signer), client, chain)?;

    let mut bind = state.info_alt_slots(holder, true)?;
    let info = bind.into_account_info();

    let actions = get_dealloc_actions(&state, &info, session)?;

    actions
        .iter()
        .map(|x| track_slots(&state, &bind.0, x, session))
        .collect::<Result<Vec<_>>>()?;
    actions
        .into_iter()
        .map(|x| x.apply(&state))
        .collect::<Result<Vec<_>>>()?;

    Emulation::without_vm(&state)
}


