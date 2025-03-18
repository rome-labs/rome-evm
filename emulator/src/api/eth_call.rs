use {
    super::Emulation,
    crate::{context::ContextEthCall, state::State},
    rome_evm::{
        error::Result,
        tx::legacy::Legacy,
        vm::{Execute, MachineEthCall, Vm},
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{msg, pubkey::Pubkey},
    std::sync::Arc,
};

pub fn eth_call(program_id: &Pubkey, legacy: Legacy, client: Arc<RpcClient>) -> Result<Emulation> {
    msg!("eth_call");
    let state = State::new(program_id, None, client, legacy.chain_id.as_u64())?;
    let context = ContextEthCall::new(legacy);
    let mut vm = Vm::new_eth_call(&state, &context)?;
    vm.consume(MachineEthCall::Init)?;

    let report = Emulation::with_vm(
        &state,
        vm.exit_reason,
        vm.return_value,
        vm.steps_executed,
        1,
        state.alloc(),
        state.dealloc(),
        state.alloc_payed(),
        state.dealloc_payed(),
        vec![],
        state.syscall.count(),
    );

    report
}
