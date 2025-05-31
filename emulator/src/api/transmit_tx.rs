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

    let _sys_acc = state.info_sys(&system_program::ID)?;
    // TODO: the client side should implement the holder filling with taking into account holder header allocation
    let len = from + tx.len();

    let (filled_len, header, reset, key) = {
        let mut bind = state.info_tx_holder(holder, true)?;
        let info = bind.into_account_info();
        let reset = TxHolder::from_account(&info)?.hash != ix_hash;

        if reset {
            TxHolder::reset(&info, ix_hash)?;
        }
        TxHolder::inc_iteration(&info)?;

        let filled_len = Holder::from_account(&info)?.len();
        let header = Holder::offset(&info);

        (filled_len, header, reset, bind.0)
    };

    if reset || len > filled_len {
        state.realloc(&key, header + len)?;
    }

    let mut bind = state.info_tx_holder(holder, false)?;
    let info = bind.into_account_info();
    Holder::fill(&info, from, len, tx)?;
    state.update(bind);

    Emulation::without_vm(&state)
}
