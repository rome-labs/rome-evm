use {
    super::{cast_slice, cast_slice_mut, slice_len, Data, AltId},
    crate::error::{Result, RomeProgramError::AltSlotAlreadyInUse,},
    solana_program::account_info::AccountInfo,
    std::cell::{Ref, RefMut,},
};

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct AltSlots(pub u64);

impl AltSlots {
    pub fn push(info: &AccountInfo, slot: u64) -> Result<()> {
        let mut slots = AltSlots::from_account_mut(info)?;
        
        let (last, outdated) = slots.split_last_mut()
            .expect("expected alt slot");

        if outdated.iter().position(|a| a.0 == slot).is_some() {
            return Err(AltSlotAlreadyInUse(slot))
        }

        last.0 = slot;
        Ok(())
    }

    pub fn remove(info: &AccountInfo, slot: u64) -> Result<()> {
        let mut slots = AltSlots::from_account_mut(info)?;
        assert!(!slots.is_empty());

        let pos = slots.iter().position(|a| a.0 == slot)
            .expect("expected slot to remove");
        let (_, right) = slots.split_at_mut(pos);
        right.copy_within(1.., 0);

        Ok(())
    }
}
impl Data for AltSlots {
    type Item<'a> = Ref<'a, [Self]>;
    type ItemMut<'a> = RefMut<'a, [Self]>;

    fn from_account<'a>(info: &'a AccountInfo) -> Result<Self::Item<'a>> {
        cast_slice(info, Self::offset(info), Self::size(info))
    }
    fn from_account_mut<'a>(info: &'a AccountInfo) -> Result<Self::ItemMut<'a>> {
        cast_slice_mut(info, Self::offset(info), Self::size(info))
    }
    fn size(info: &AccountInfo) -> usize {
        slice_len::<Self>(info)
    }
    fn offset(info: &AccountInfo) -> usize {
        AltId::offset(info) + AltId::size(info)
    }
}