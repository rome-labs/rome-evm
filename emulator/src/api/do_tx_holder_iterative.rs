use {
    super::{do_tx_iterative::iterative_tx, Emulation},
    crate::{context::ContextIterative, state::State, Instruction::DoTxHolderIterative},
    rome_evm::{
        error::{Result, RomeProgramError::*},
        H256,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{msg, pubkey::Pubkey},
    std::{mem::size_of, sync::Arc},
};

// holder_index | tx_hash
pub fn args(data: &[u8]) -> Result<(u64, H256)> {
    if data.len() != size_of::<u64>() + size_of::<H256>() {
        return Err(InvalidInstructionData);
    }

    let (left, right) = data.split_at(size_of::<u64>());
    let holder = u64::from_le_bytes(left.try_into().unwrap());
    let hash = H256::from_slice(right);

    Ok((holder, hash))
}

pub fn do_tx_holder_iterative<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!("Instruction: Iterative transaction from holder");

    let state = State::new(program_id, Some(*signer), Arc::clone(&client))?;
    let context = ContextIterative::new(&state, data, DoTxHolderIterative)?;

    iterative_tx(&state, context)
}
