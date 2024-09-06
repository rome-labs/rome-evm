use {
    super::Emulation,
    crate::{
        state::State,
        ContextAtomic,
        Instruction::{self, DoTx},
    },
    rome_evm::{
        error::Result,
        origin::Origin,
        vm::{self, vm_atomic::MachineAtomic, Execute},
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{msg, pubkey::Pubkey},
    std::sync::Arc,
};

pub fn atomic_transaction<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
    instruction: Instruction,
) -> Result<Emulation> {
    let state = State::new(program_id, Some(*signer), client)?;
    let context = ContextAtomic::new(&state, data, instruction);
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

pub fn do_tx<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!("Instruction: Atomic transaction");
    atomic_transaction(program_id, data, signer, client, DoTx)
}
