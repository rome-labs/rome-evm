use {
    super::Emulation,
    crate::state::State,
    rome_evm::{
        api::create_balance::{args, get_onwer_mut, mint_balance},
        error::Result,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey},
    std::sync::Arc,
};

pub fn create_balance<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    let (address, balance, chain) = args(data)?;
    msg!(
        "Instruction: create balance {} {} chain_id {}",
        address,
        balance,
        chain
    );

    let state = State::new(program_id, Some(*signer), client, chain)?;
    let mut owners = state.info_owner_reg(false)?;
    let owners_info = owners.into_account_info();
    let owner = get_onwer_mut(&owners_info, signer, chain)?;

    let mut bind = state.info_addr(&address, true)?;
    let info = bind.into_account_info();
    mint_balance(&state, &info, owner, balance, address, chain)?;

    state.update(bind);
    state.update(owners);

    Emulation::without_vm(&state)
}
