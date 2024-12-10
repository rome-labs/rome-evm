use {
    super::{do_tx::atomic_transaction, Emulation},
    crate::state::State,
    rome_evm::{api::do_tx_holder::args, error::Result, Holder},
    solana_client::rpc_client::RpcClient,
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey},
    std::sync::Arc,
};

pub fn do_tx_holder<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!("Instruction: Atomic transaction from holder");

    let (holder, hash, chain, fee_addr) = args(data)?;
    let state = State::new(program_id, Some(*signer), client.clone(), chain)?;

    let mut bind = state.info_tx_holder(holder, false)?;
    let info = bind.into_account_info();
    let rlp = Holder::rlp(&info, hash, chain)?;

    atomic_transaction(state, &rlp, fee_addr)
}
