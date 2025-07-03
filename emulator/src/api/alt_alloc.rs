use solana_program::address_lookup_table::state::LOOKUP_TABLE_MAX_ADDRESSES;
use {
    super::Emulation,
    crate::state::State,
    rome_evm::{
        api::alt_alloc::{args, get_alloc_actions, Alt, Action::{self, *}, },
        error::Result, AltId, AltSlots, Data,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{
        address_lookup_table::state::{AddressLookupTable, LookupTableMeta,},
        account_info::IntoAccountInfo, msg, pubkey::Pubkey, 
    },
    std::sync::Arc,
};

pub fn alt_alloc<'a>(
    program_id: &'a Pubkey,
    data: &'a [u8],
    signer: &'a Pubkey,
    client: Arc<RpcClient>,
) -> Result<Emulation> {
    msg!("Instruction: alt_alloc");

    let (holder, chain, session, recent_slot, total, keys) = args(data)?;
    let state = State::new(program_id, Some(*signer), client, chain)?;

    let mut bind = state.info_alt_slots(holder, true)?;
    let info = bind.into_account_info();

    let actions = get_alloc_actions(
            &info,
            session,
            recent_slot,
            total,
            keys,
            &state
        )?;

    actions
        .iter()
        .map(|x| track_slots(&state, &bind.0, x, session))
        .collect::<Result<Vec<_>>>()?;
    actions
        .into_iter()
        .map(|x| x.apply(&state))
        .collect::<Result<Vec<_>>>()?;

    Emulation::without_vm(&state)
}

pub fn track_slots(state: &State, key: &Pubkey, action: &Action, session: u64) -> Result<()> {
    match action {
        Create{ slot } => {
            let bind = state.info_sys(key)?;
            let size = bind.1.data.len() + size_of::<u64>();
            state.realloc(&bind.0, size)?;

            let mut bind = state.info_sys(&bind.0)?;
            let info = bind.into_account_info();

            AltSlots::push(&info, *slot)?;
            state.update(bind);
        },
        Extend {slot: _, keys:_ } => {
            let mut bind = state.info_sys(key)?;
            let info = bind.into_account_info();
            AltId::set_session(&info, session)?;
            state.update(bind);
        }
        Close { key: _, slot }=> {
            let mut bind = state.info_sys(key)?;
            let info = bind.into_account_info();
            
            AltSlots::remove(&info, *slot)?;
            let len = bind.1.data.len() - size_of::<u64>();

            state.update(bind);
            state.realloc(&key, len)?;

            // test bounds
            let mut bind = state.info_sys(&key)?;
            let info = bind.into_account_info();
            let _ = AltSlots::size(&info);
        },
        _ => {},
    }
    Ok(())
}

impl<'a> Alt for State<'a> {
    fn alt_meta(&self, key: Pubkey) -> Result<LookupTableMeta> {
        let bind = self.info_sys(&key)?;
        let alt =  AddressLookupTable::deserialize(&bind.1.data)?;
        Ok(alt.meta)
    }
    fn alt_available(&self, key: Pubkey) -> Result<usize> {
        let bind = self.info_sys(&key)?;
        let alt =  AddressLookupTable::deserialize(&bind.1.data)?;

        let available = LOOKUP_TABLE_MAX_ADDRESSES
            .checked_sub(alt.addresses.len())
            .expect("unexpected state of address lookup table"); // unreachable

        Ok(available)
    }
}

