use {
    super::Emulation,
    crate::{
        context::ContextIt,
        state::State,
    },
    rome_evm::{
        api::{split_fee, split_u64},
        context::{AccountLock, Context},
        error::{Result, RomeProgramError::*},
        tx::tx::Tx,
        vm::{self, vm_iterative::MachineIt, Execute},
        H160, H256,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{keccak, msg, pubkey::Pubkey},
    std::sync::Arc,
};

// session | holder_index | Option<fee_recipient> | tx
pub fn args(data: &[u8]) -> Result<(u64, u64, Option<H160>, &[u8])> {
    let (session, data) = split_u64(data)?;
    let (holder, data) = split_u64(data)?;
    let (fee_addr, tx) = split_fee(data)?;

    Ok((session, holder, fee_addr, tx))
}

pub fn iterative_tx(
    state: &State,
    context: ContextIt,
    is_gas_estimate: bool,
) -> Result<Emulation> {
    let mut steps = 0;
    let mut iteration = 0;
    let mut alloc = 0;
    let mut dealloc = 0;
    let mut alloc_payed = 0;
    let mut dealloc_payed = 0;
    let mut syscalls = 0;

    // TODO remove and use unique tx_id in data
    loop {
        msg!("  iteration {}", iteration);

        let mut vm = vm::VmIt::new(state, &context)?;
        iteration += 1;

        match vm.consume(MachineIt::FromStateHolder) {
            Err(UnnecessaryIteration(_)) => {
                msg!("Lock after emulation");
                vm.context.lock()?;
                // restore vm state
                vm.context.deserialize(&mut vm.vm)?;
                let (lmp_fee, lmp_refund) = vm.context.fees()?;

                return Emulation::with_vm(
                    state,
                    vm.vm.exit_reason,
                    vm.vm.return_value,
                    steps,
                    iteration - 1, // do not take into account the unnecessary iteration
                    alloc,
                    dealloc,
                    alloc_payed,
                    dealloc_payed,
                    vm.context.lock_overrides.borrow().clone(),
                    syscalls,
                    lmp_fee,
                    lmp_refund,
                    is_gas_estimate,
                    Some(&context)
                );
            }
            Err(e) => return Err(e),
            _ => {}
        }
        steps += vm.vm.steps_executed;
        alloc += state.alloc();
        dealloc += state.dealloc();
        alloc_payed += state.alloc_payed();
        dealloc_payed += state.dealloc_payed();
        syscalls += state.syscall.count();

        state.reset();
    }
}

pub fn do_tx_iterative<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!("Instruction: Iterative transaction");
    let (session, holder, fee_addr, rlp) = args(data)?;
    let hash = H256::from(keccak::hash(rlp).to_bytes());
    let chain = Tx::chain_id_from_rlp(rlp)?;

    let state = State::new(program_id, Some(*signer), Arc::clone(&client), chain)?;
    let context = ContextIt::new(&state, holder, hash, session, fee_addr, rlp, false)?;
    iterative_tx(&state, context, false)
}
