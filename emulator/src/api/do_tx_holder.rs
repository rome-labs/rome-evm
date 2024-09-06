use {
    super::{do_tx::atomic_transaction, Emulation},
    crate::Instruction::DoTxHolder,
    rome_evm::error::Result,
    solana_client::rpc_client::RpcClient,
    solana_program::{msg, pubkey::Pubkey},
    std::sync::Arc,
};

pub fn do_tx_holder<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!("Instruction: Atomic transaction from holder");
    atomic_transaction(program_id, data, signer, client, DoTxHolder)
}
