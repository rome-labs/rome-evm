use {
    super::Emulation,
    crate::state::State,
    rome_evm::{
        accounts::{Data, SignerInfo},
        api::reg_signer::args,
        error::Result,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey},
    std::sync::Arc,
};

pub fn reg_signer<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    let address = args(data)?;
    msg!("Instruction: set signer info {}", address);

    let state = State::new(program_id, Some(*signer), client)?;
    let mut bind = state.info_signer_info(signer, true)?;

    {
        let info = bind.into_account_info();
        let mut signer_info = SignerInfo::from_account_mut(&info)?;
        signer_info.address = address;
    }

    state.update(bind)?;
    Emulation::without_vm(&state)
}
