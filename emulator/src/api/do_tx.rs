use {
    super::Emulation,
    crate::{state::State, ContextAtomic},
    rome_evm::{
        error::Result,
        origin::Origin,
        tx::tx::Tx,
        vm::{self, vm_atomic::MachineAtomic, Execute},
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
    let tx = Tx::from_instruction(data)?;
    let state = State::new(program_id, Some(*signer), client, tx.chain_id())?;
    atomic_transaction(state, tx)
}

pub fn atomic_transaction(state: State, tx: Tx) -> Result<Emulation> {
    let context = ContextAtomic::new(&state, tx);
    let mut vm = vm::Vm::new_atomic(&state, &context)?;
    vm.consume(MachineAtomic::Lock)?;

    let report = Emulation::with_vm(
        &state,
        vm.exit_reason,
        vm.return_value,
        vm.steps_executed,
        1,
        state.allocated(),
        state.deallocated(),
        *state.alloc_state.borrow(),
        *state.dealloc_state.borrow(),
    );

    report
}
