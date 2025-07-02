use {
    crate::{
        error::{Result, RomeProgramError::*,},
        split_u64,
        state::{State, origin::Origin,},
        accounts::{AltId, AltSlots, Data,},
        pda::Seed,
    },
    solana_program::{
        address_lookup_table::{
            instruction::*,
            state::{AddressLookupTable, LookupTableMeta},
        },
        account_info::AccountInfo, msg, pubkey::{Pubkey, PUBKEY_BYTES},
    },
    std::{convert::TryFrom, mem::size_of},
};

pub fn alt_alloc<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    msg!("Instruction: alt_alloc");

    let (holder, chain, session, recent_slot, keys) = args(data)?;
    let state = State::new(program_id, accounts, chain)?;
    let info = state.info_alt_slots(holder, true)?;

    let actions = get_alloc_actions(
        info,
        session,
        recent_slot,
        keys,
    )?;

    actions
        .iter()
        .map(|x| track_slots(&state, info, x, session))
        .collect::<Result<Vec<_>>>()?;
    // CPI deallocations must be performed after local deallocations
    actions
        .into_iter()
        .map(|x| x.apply(&state))
        .collect::<Result<Vec<_>>>()?;

    Ok(())
}

pub fn args(data: &[u8]) -> Result<(u64, u64, u64, u64, Vec<Pubkey>)> {
    let (holder, data) = split_u64(data)?;
    let (chain, data) = split_u64(data)?;
    let (session, data) = split_u64(data)?;
    let (slot, data) = split_u64(data)?;

    let keys = data
        .chunks(PUBKEY_BYTES)
        .into_iter()
        .map(|x| Pubkey::try_from(x))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| {
            InvalidInstructionData
        })?;

    Ok((holder, chain, session, slot, keys))
}

#[derive(Clone)]
pub enum Action {
    Create { slot: u64 },
    Extend { slot: u64, keys: Vec<Pubkey>},
    Deactivate {key: Pubkey },
    Close { key: Pubkey, slot: u64 }
}
use Action::*;
impl Action {
    pub fn apply<T: Origin>(self, state: &T) -> Result<()> {
        let auth = state.signer();

        match self {
            Create{ slot } => {
                let (ix, _) = create_lookup_table(auth, auth, slot);
                state.invoke_signed(&ix, &Seed::default(), false)
            },
            Extend{ slot, keys} => {
                let (key, _) = derive_lookup_table_address(&auth, slot);
                let ix = extend_lookup_table(key, auth, Some(auth), keys);
                state.invoke_signed(&ix, &Seed::default(), false)
            },
            Deactivate { key} => {
                let ix = deactivate_lookup_table(key, auth);
                state.invoke_signed(&ix, &Seed::default(), false)
            },
            Close { key, slot: _ }=> {
                let ix = close_lookup_table(key, auth, auth);
                state.invoke_signed(&ix, &Seed::default(), false)
            },
        }
    }
}

pub fn track_slots<'a, 'b>(state: &'b State<'a>,  info: &'a AccountInfo<'a>, action: &Action, session: u64) -> Result<()> {
    match action {
        Create{ slot } => {
            let size = info.data_len() + size_of::<u64>();
            state.realloc(info, size)?;

            AltSlots::push(info, *slot)?;
            AltId::set_session(info, session)?;
        },
        Close { key: _, slot }=> {
            AltSlots::remove(info, *slot)?;
            let len = info.data_len() - size_of::<u64>();
            state.realloc(info, len)?;
            // test bounds
            let _ = AltSlots::size(info);
        },
        _ => {},
    }
    Ok(())
}

pub fn get_alloc_actions(
    info: &AccountInfo,
    session: u64,
    recent_slot: u64,
    keys: Vec<Pubkey>,
) -> Result<Vec<Action>> {
    let mut vec = vec![];

    let mut slots = AltSlots::from_account(info)?
        .iter()
        .map(|x| x.0)
        .collect::<Vec<_>>();

    if let Some((latest, _)) = slots.split_last_mut() {
        if AltId::has_session(info, session)? {
            vec.push(Extend{ slot: *latest, keys });
        } else {
            vec.push(Create{ slot: recent_slot });
            vec.push(Extend{ slot: recent_slot, keys });
        }
    } else {
        vec.push(Create{ slot: recent_slot });
        vec.push(Extend{ slot: recent_slot, keys });
    }

    Ok(vec)
}

pub trait Alt {
    fn alt_meta(&self, key: Pubkey) -> Result<LookupTableMeta>;
}
impl<'a> Alt for State<'a> {
    fn alt_meta(&self, key: Pubkey) -> Result<LookupTableMeta> {
        let data = self.data(&key)?;
        let alt =  AddressLookupTable::deserialize(&data)?;

        Ok(alt.meta)
    }
}