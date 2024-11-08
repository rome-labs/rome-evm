mod account_state;
mod account_type;
mod code;
mod holder;
mod lock;
mod owner_info;
mod ro_lock;
mod slot;
mod state_holder;
mod storage;
mod tx_holder;
mod valids;
mod ver;

pub use account_state::*;
pub use account_type::*;
pub use code::Code;
pub use holder::Holder;
pub use lock::{Lock, LockType};
pub use owner_info::OwnerInfo;
pub use ro_lock::RoLock;
pub use slot::Slot;
pub use state_holder::{Iterations, StateHolder};
pub use storage::Storage;
pub use tx_holder::TxHolder;
pub use valids::Valids;
pub use ver::Ver;

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

fn slise_len<T: Data>(info: &AccountInfo) -> usize {
    let offset = T::offset(info);
    let mut len = info.data.borrow().len();
    assert!(len >= offset);
    len -= offset;
    assert!(len % size_of::<T>() == 0);
    len / size_of::<T>()
}
