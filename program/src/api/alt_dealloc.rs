
#![allow(deprecated)] // TODO: remove and replace by PodSlotHashes
use solana_program::sysvar::slot_hashes::SlotHashesSysvar;
use {
    crate::{
        api::split_u64,
        error::{Result, RomeProgramError::*},
        state::{State, origin::Origin,},
        accounts::{AltId, AltSlots, Data,},
        config::ALT_OUTDATED_SLOTS_TRACK,
    },
    solana_program::{
        address_lookup_table::{
            instruction::*,
        },
        account_info::AccountInfo, msg, pubkey::Pubkey, clock::Clock,
        sysvar::Sysvar,
    },
    super::alt_alloc::{Action::{self, *}, track_slots, Alt}
};

pub fn alt_dealloc<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &'a [u8],
) -> Result<()> {
    msg!("Instruction: alt_dealloc");

    let (holder, chain, session) = args(data)?;
    let state = State::new(program_id, accounts, chain)?;
    let info = state.info_alt_slots(holder, true)?;

    let actions = get_dealloc_actions(&state, info, session)?;


    actions.clone()
        .into_iter()
        .map(|x| x.apply(&state))
        .collect::< Result<Vec<_>>>()?;
    actions
        .iter()
        .map(|x| track_slots(&state, info, x, session))
        .collect::<Result<Vec<_>>>()?;

    Ok(())
}

pub fn args(data: &[u8]) -> Result<(u64, u64, u64)> {
    if data.len() != std::mem::size_of::<u64>() * 3 {
        return Err(InvalidInstructionData)
    }

    let (holder, data) = split_u64(data)?;
    let (chain, data) = split_u64(data)?;
    let (session, _) = split_u64(data)?;

    Ok((holder, chain, session))
}

pub fn get_dealloc_actions<T: Origin + Alt>(
    state: &T,
    info: &AccountInfo,
    session: u64,
) -> Result<Vec<Action>> {
    let mut vec = vec![];

    let mut f = |a: &[u64]| -> Result<()>{
        let mut b = outdated_slot_actions(state, a)?;
        vec.append(&mut b);
        Ok(())
    };

    let mut slots = AltSlots::from_account(info)?
        .iter()
        .map(|x| x.0)
        .collect::<Vec<_>>();

    if let Some((slot, rest)) = slots.split_last_mut() {
        if !AltId::has_session(info, session)? {
            f(&vec![*slot])?;
        }

        rest.reverse();
        let (to_track, _) = rest.split_at(ALT_OUTDATED_SLOTS_TRACK.min(rest.len()));

        f(to_track)?;
    }

    Ok(vec)
}

pub fn outdated_slot_actions<T: Origin + Alt>(state: &T, slots: &[u64],) -> Result<Vec<Action>> {
    let mut vec = vec![];
    let clock = Clock::get()?;

    for slot in slots {
        let (key, _) = derive_lookup_table_address(&state.signer(), *slot);
        let meta = state.alt_meta(key)?;

        if meta.deactivation_slot == u64::MAX {
            vec.push(Deactivate{ key });
        } else {
            if meta.deactivation_slot != clock.slot {
                if SlotHashesSysvar::position(&meta.deactivation_slot)?.is_none() {
                    vec.push(Close{ key, slot: *slot });
                }
            }
        }
    }

    Ok(vec)
}
