use {
    super::Emulation,
    crate::state::State,
    rome_evm::{
        accounts::OwnerInfo,
        api::reg_owner::{args, check, reg},
        error::Result,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey},
    std::{mem::size_of, sync::Arc},
};
pub fn reg_owner<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    let chain = args(data)?;
    msg!("Instruction: chain_id registration {}", chain);

    let state = State::new_unchecked(program_id, Some(*signer), client, chain)?;

    let (len, key) = {
        let mut bind = state.info_owner_reg(true)?;
        let info = bind.into_account_info();
        check(&info, signer, chain)?;
        let len = bind.1.data.len() + size_of::<OwnerInfo>();
        (len, bind.0)
    };

    state.realloc(&key, len)?;

    let mut bind = state.info_owner_reg(false)?;
    let info = bind.into_account_info();
    reg(&info, chain)?;
    state.update(bind);

    // create rome-evm SOL wallet
    let _ = state.info_sol_wallet(true)?;

    Emulation::without_vm(&state)
}
