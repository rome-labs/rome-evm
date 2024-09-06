use {
    super::Emulation,
    crate::state::State,
    rome_evm::{
        accounts::{AccountState, Data},
        api::create_balance::args,
        error::{Result, RomeProgramError::*},
        state::origin::Origin,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{account_info::IntoAccountInfo, msg, pubkey::Pubkey},
    std::sync::Arc,
};

pub fn create_balance<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    let (address, balance) = args(data)?;
    msg!("Instruction: create balance {} {}", address, balance);
    let state = State::new(program_id, Some(*signer), client)?;
    let mut bind = state.info_addr(&address, true)?;

    {
        let info = bind.into_account_info();
        if state.allocated() == 0 {
            return Err(AccountInitialized(*info.key));
        }
        let mut account_state = AccountState::from_account_mut(&info)?;
        account_state.balance = balance;
    }
    state.update(bind)?;

    Emulation::without_vm(&state)
}
