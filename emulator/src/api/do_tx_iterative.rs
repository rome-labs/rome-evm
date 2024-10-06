use {
    super::Emulation,
    crate::{state::State, ContextIterative, Instruction::DoTxIterative},
    rome_evm::{
        context::{account_lock::AccountLock, Context},
        error::{Result, RomeProgramError::*},
        origin::Origin,
        vm::{self, vm_iterative::MachineIterative, Execute},
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{msg, pubkey::Pubkey},
    std::{mem::size_of, sync::Arc},
};

// holder_index | tx
pub fn args(data: &[u8]) -> Result<(u64, &[u8])> {
    if data.len() <= size_of::<u64>() {
        return Err(InvalidInstructionData);
    }
    let (left, tx) = data.split_at(size_of::<u64>());
    let holder = u64::from_le_bytes(left.try_into().unwrap());

    Ok((holder, tx))
}

pub fn iterative_tx<L: AccountLock + Context>(state: &State, context: L) -> Result<Emulation> {
    let mut steps = 0;
    let mut iteration = 0;
    let mut alloc = 0;
    let mut dealloc = 0;
    let mut alloc_state = 0;
    let mut dealloc_state = 0;

    loop {
        msg!("  iteration {}", iteration);

        let mut vm = vm::Vm::new_iterative(state, &context)?;
        iteration += 1;

        match vm.consume(MachineIterative::FromStateHolder) {
            Err(UnnecessaryIteration(_)) => {
                msg!("Lock after emulation");
                vm.context.lock()?;
                // restore vm state
                vm.context.deserialize_vm(&mut vm)?;

                return Emulation::with_vm(
                    state,
                    vm.exit_reason,
                    vm.return_value,
                    steps,
                    iteration,
                    alloc,
                    dealloc,
                    alloc_state,
                    dealloc_state,
                );
            }
            Err(e) => return Err(e),
            _ => {},
        }
        steps += vm.steps_executed;
        alloc += state.allocated();
        dealloc += state.deallocated();
        alloc_state += *state.alloc_state.borrow();
        dealloc_state += *state.dealloc_state.borrow();
        state.reset_counters();
    }
}

pub fn do_tx_iterative<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!("Instruction: Iterative transaction");
    let state = State::new(program_id, Some(*signer), Arc::clone(&client))?;
    let context = ContextIterative::new(&state, data, DoTxIterative)?;

    iterative_tx(&state, context)
}
