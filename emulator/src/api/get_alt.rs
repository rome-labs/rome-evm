use {
    crate::state::State,
    rome_evm::{
        api::split_u64,
        error::{Result, RomeProgramError::*}, AltSlots, Data, origin::Origin,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{
        account_info::IntoAccountInfo, msg, pubkey::Pubkey,
        address_lookup_table::instruction::derive_lookup_table_address,
    },
    std::sync::Arc,
};

pub fn args(data: &[u8]) -> Result<(u64, u64)> {
    let (holder, data) = split_u64(data)?;
    let (chain, data) = split_u64(data)?;

    if !data.is_empty() {
        return Err(InvalidInstructionData)
    }

    Ok((holder, chain))
}

pub fn get_alt<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Option<Pubkey>> {
    msg!("get_alt");

    let (holder, chain) = args(data)?;
    let state = State::new(program_id, Some(*signer), client, chain)?;

    let mut bind = state.info_alt_slots(holder, false)?;
    let info = bind.into_account_info();
    let slots = AltSlots::from_account(&info)?
        .iter()
        .map(|x| x.0)
        .collect::<Vec<_>>();

    if let Some(slot) = slots.last() {
        let (key, _) = derive_lookup_table_address(&state.signer(), *slot);
        Ok(Some(key))
    } else {
        Ok(None)
    }
}