use {
    super::{do_tx_holder_iterative, Emulation},
    crate::{
        state::State,
        ContextIterative,
        Instruction::{self, DoTxHolderIterative, DoTxIterative},
    },
    rome_evm::{
        accounts::{Data, Iterations, StateHolder},
        context::{account_lock::AccountLock, Context},
        error::{Result, RomeProgramError::*},
        origin::Origin,
        vm::{self, vm_iterative::MachineIterative, Execute},
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey},
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

pub fn iterative_transaction<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
    instruction: Instruction,
) -> Result<Emulation> {
    let mut steps = 0;
    let mut iteration = 0;
    let mut alloc = 0;
    let mut dealloc = 0;
    let mut alloc_state = 0;
    let mut dealloc_state = 0;
    let state = State::new(program_id, Some(*signer), Arc::clone(&client))?;

    // TODO remove and use unique tx_id in data
    let holder = match instruction {
        DoTxIterative => args(data)?.0,
        DoTxHolderIterative => do_tx_holder_iterative::args(data)?.0,
        _ => unreachable!(),
    };
    if let Ok(mut bind) = state.info_state_holder(holder, false) {
        let info = bind.into_account_info();
        let mut state_holder = StateHolder::from_account_mut(&info)?;
        state_holder.iteration = Iterations::Lock;
    }

    loop {
        msg!("  iteration {}", iteration);
        state.reset_counters();
        let context = ContextIterative::new(&state, data, instruction.clone())?;

        let mut vm = vm::Vm::new_iterative(&state, &context)?;
        iteration += 1;

        match vm.consume(MachineIterative::FromStateHolder) {
            Err(UnnecessaryIteration(_)) => {
                msg!("Lock after emulation");
                vm.context.lock()?;
                // restore vm state
                vm.context.deserialize_vm(&mut vm)?;

                return Emulation::with_vm(
                    &state,
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
            Ok(()) => {
                steps += vm.steps_executed;
                alloc += state.allocated();
                dealloc += state.deallocated();
                alloc_state += *state.alloc_state.borrow();
                dealloc_state += *state.dealloc_state.borrow();
            }
        }
    }
}

pub fn do_tx_iterative<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!("Instruction: Iterative transaction");
    iterative_transaction(program_id, data, signer, client, DoTxIterative)
}
