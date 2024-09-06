mod account_state;
mod account_type;
mod address_table;
mod code;
mod holder;
mod lock;
mod ro_lock;
mod signer_info;
mod state_holder;
mod storage;
mod tx_holder;
mod valids;

pub use account_state::*;
pub use account_type::*;
pub use address_table::*;
pub use code::Code;
pub use holder::Holder;
pub use lock::{Lock, LockType};
pub use ro_lock::RoLock;
pub use signer_info::SignerInfo;
pub use state_holder::{Iterations, StateHolder};
pub use storage::Storage;
pub use tx_holder::TxHolder;
pub use valids::Valids;

use {
    crate::error::{Result, RomeProgramError::InvalidDataLength},
    solana_program::account_info::AccountInfo,
    std::{
        cell::{Ref, RefMut},
        mem::{align_of, size_of},
        ptr,
    },
};

pub trait Data {
    type Item<'a>;
    type ItemMut<'a>;
    fn from_account<'a>(info: &'a AccountInfo) -> Result<Self::Item<'a>>;
    fn from_account_mut<'a>(info: &'a AccountInfo) -> Result<Self::ItemMut<'a>>;
    fn size(info: &AccountInfo) -> usize;
    fn offset(info: &AccountInfo) -> usize;
}

fn cast<'a, T>(info: &'a AccountInfo, offset: usize, len: usize) -> Result<Ref<'a, T>> {
    assert_eq!(align_of::<T>(), 1);

    let data = info.data.borrow();

    if data.len() < offset + len {
        return Err(InvalidDataLength(*info.key, data.len(), offset + len));
    }

    let data = Ref::map(data, |a| &a[offset..offset + len]);
    assert_eq!(data.len(), size_of::<T>());

    let state = Ref::map(data, |a| {
        let ptr = a.as_ptr().cast::<T>();
        unsafe { &*ptr }
    });

    Ok(state)
}

fn cast_mut<'a, T>(info: &'a AccountInfo, offset: usize, len: usize) -> Result<RefMut<'a, T>> {
    assert_eq!(align_of::<T>(), 1);

    let data = info.data.borrow_mut();

    if data.len() < offset + len {
        return Err(InvalidDataLength(*info.key, data.len(), offset + len));
    }

    let data = RefMut::map(data, |a| &mut a[offset..offset + len]);
    assert_eq!(data.len(), size_of::<T>());

    let state = RefMut::map(data, |a| {
        let ptr = a.as_mut_ptr().cast::<T>();
        unsafe { &mut *ptr }
    });

    Ok(state)
}

fn cast_slice<'a, T>(info: &'a AccountInfo, offset: usize, count: usize) -> Result<Ref<'a, [T]>> {
    assert_eq!(align_of::<T>(), 1);

    let data = info.data.borrow();

    let len = count * size_of::<T>();
    if data.len() < offset + len {
        return Err(InvalidDataLength(*info.key, data.len(), offset + len));
    }

    let data = Ref::map(data, |a| &a[offset..offset + len]);
    assert_eq!(data.len() % size_of::<T>(), 0);

    let slice = Ref::map(data, |a| {
        let ptr = a.as_ptr().cast::<T>();
        unsafe { &*ptr::slice_from_raw_parts(ptr, count) }
    });

    Ok(slice)
}

fn cast_slice_mut<'a, T>(
    info: &'a AccountInfo,
    offset: usize,
    count: usize,
) -> Result<RefMut<'a, [T]>> {
    assert_eq!(align_of::<T>(), 1);

    let data = info.data.borrow_mut();

    let len = count * size_of::<T>();
    if data.len() < offset + len {
        return Err(InvalidDataLength(*info.key, data.len(), offset + len));
    }

    let data = RefMut::map(data, |a| &mut a[offset..offset + len]);
    assert_eq!(data.len() % size_of::<T>(), 0);

    let slice = RefMut::map(data, |a| {
        let ptr = a.as_mut_ptr().cast::<T>();
        unsafe { &mut *ptr::slice_from_raw_parts_mut(ptr, count) }
    });

    Ok(slice)
}