use {
    super::Emulation,
    crate::{state::State, ContextAt},
    rome_evm::{
        api::split_fee,
        error::Result,
        tx::tx::Tx,
        vm::{vm_atomic::{VmAt, MachineAt}, Execute},
        H160,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{msg, pubkey::Pubkey},
    std::sync::Arc,
};

pub fn do_tx<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!("Instruction: Atomic transaction");
    let (fee_addr, rlp) = split_fee(data)?;
    let chain = Tx::chain_id_from_rlp(rlp)?;
    let state = State::new(program_id, Some(*signer), client, chain)?;
    atomic_transaction(state, rlp, fee_addr)
}

pub fn atomic_transaction(state: State, rlp: &[u8], fee_addr: Option<H160>) -> Result<Emulation> {
    let context = ContextAt::new(&state);
    let mut vm = VmAt::new(&state, rlp, fee_addr, &context)?;
    vm.consume(MachineAt::Lock)?;

    let (fee, refund) = state.get_fees();
    let report = Emulation::with_vm(
        &state,
        vm.vm.exit_reason,
        vm.vm.return_value,
        vm.vm.steps_executed,
        1,
        state.alloc(),
        state.dealloc(),
        state.alloc_payed(),
        state.dealloc_payed(),
        vec![],
        state.syscall.count(),
        fee,
        refund,
        false,
        None,
    );

    report
}
