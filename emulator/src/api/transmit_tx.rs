use {
    super::Emulation,
    crate::state::State,
    rome_evm::{api::transmit_tx::args, error::Result, Data, Holder, TxHolder},
    solana_client::rpc_client::RpcClient,
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey, system_program},
    std::{cell::RefMut, sync::Arc},
};

pub fn transmit_tx<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!("Instruction: Transmit tx");
    let (index, offset, ix_hash, tx) = args(data)?;

    let state = State::new(program_id, Some(*signer), client)?;
    let mut bind = state.info_tx_holder(index, true)?;

    let (available, holder_offset, hash) = {
        let info = bind.into_account_info();
        let len = Holder::from_account(&info)?.len();
        let holder_offset = Holder::offset(&info);
        let hash = TxHolder::from_account(&info)?.hash;
        (len, holder_offset, hash)
    };

    let _sys_acc = state.load(&system_program::ID, None)?;

    let required = offset + tx.len();
    if hash != ix_hash || required > available {
        state.realloc(&mut bind, holder_offset + required)?;
    }

    {
        let info = bind.into_account_info();
        TxHolder::from_account_mut(&info)?.hash = hash;
        let holder = Holder::from_account_mut(&info)?;
        let mut location = RefMut::map(holder, |a| &mut a[offset..required]);
        location.copy_from_slice(tx);
    }
    state.update(bind)?;

    Emulation::without_vm(&state)
}
