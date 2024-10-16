use {
    super::{do_tx_iterative::iterative_tx, Emulation},
    crate::{context::ContextIterative, state::State},
    rome_evm::{
        error::{Result, RomeProgramError::*},
        Holder, H256,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey},
    std::{mem::size_of, sync::Arc},
};

// holder_index | tx_hash | chain_id
pub fn args(data: &[u8]) -> Result<(u64, H256, u64)> {
    if data.len() != size_of::<u64>() + size_of::<H256>() + size_of::<u64>() {
        return Err(InvalidInstructionData);
    }

    let (left, right) = data.split_at(size_of::<u64>());
    let holder = u64::from_le_bytes(left.try_into().unwrap());

    let (left, right) = right.split_at(size_of::<H256>());
    let hash = H256::from_slice(left);

    let chain = u64::from_le_bytes(right.try_into().unwrap());

    Ok((holder, hash, chain))
}

pub fn do_tx_holder_iterative<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!("Instruction: Iterative transaction from holder");

    let (holder, hash, chain) = args(data)?;
    let state = State::new(program_id, Some(*signer), Arc::clone(&client), chain)?;

    let mut bind = state.info_tx_holder(holder, false)?;
    let info = bind.into_account_info();
    let tx = Holder::tx(&info, hash, chain)?;

    let context = ContextIterative::new(&state, holder, &tx, hash)?;
    iterative_tx(&state, context)
}
