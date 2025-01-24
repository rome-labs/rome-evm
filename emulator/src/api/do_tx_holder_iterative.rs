use {
    super::{do_tx_iterative::iterative_tx, Emulation},
    crate::{context::ContextIterative, state::State},
    rome_evm::{
        api::{split_fee, split_hash, split_u64},
        error::Result,
        Holder, H160, H256,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey},
    std::sync::Arc,
};

// session | holder_index | tx_hash | chain_id | Option<fee_recipient>
pub fn args(data: &[u8]) -> Result<(u64, u64, H256, u64, Option<H160>)> {
    let (session, data) = split_u64(data)?;
    let (holder, data) = split_u64(data)?;
    let (hash, data) = split_hash(data)?;
    let (chain, data) = split_u64(data)?;
    let (fee_addr, _) = split_fee(data)?;

    Ok((session, holder, hash, chain, fee_addr))
}

pub fn do_tx_holder_iterative<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!("Instruction: Iterative transaction from holder");

    let (session, holder, hash, chain, fee_addr) = args(data)?;
    let state = State::new(program_id, Some(*signer), Arc::clone(&client), chain)?;

    let mut bind = state.info_tx_holder(holder, false)?;
    let info = bind.into_account_info();
    let rlp = Holder::rlp(&info, hash, chain)?;

    let context = ContextIterative::new(&state, holder, hash, session, fee_addr, &rlp)?;
    iterative_tx(&state, context)
}
