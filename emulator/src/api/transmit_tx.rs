use {
    super::Emulation,
    crate::state::State,
    rome_evm::{api::transmit_tx::args, error::Result, Data, Holder, TxHolder},
    solana_client::rpc_client::RpcClient,
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey, system_program},
    std::sync::Arc,
};

pub fn transmit_tx<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!("Instruction: Transmit tx");

    let (holder, from, ix_hash, chain, tx) = args(data)?;
    let state = State::new(program_id, Some(*signer), client, chain)?;
    let mut bind = state.info_tx_holder(holder, true)?;

    let (filled_len, header, hash) = {
        let info = bind.into_account_info();
        let len = Holder::from_account(&info)?.len();
        let header = Holder::offset(&info);
        let hash = TxHolder::from_account(&info)?.hash;
        (len, header, hash)
    };

    let _sys_acc = state.info_sys(&system_program::ID)?;
    // TODO: the client side should implement the holder filling with taking into account holder header allocation
    let to = from + tx.len();
    if hash != ix_hash || to > filled_len {
        state.realloc(&mut bind, header + to)?;
    }
    {
        let info = bind.into_account_info();
        Holder::fill(&info, hash, from, to, tx)?;
    }
    state.update(bind);

    Emulation::without_vm(&state)
}
