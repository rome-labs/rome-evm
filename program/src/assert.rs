use {
    crate::{accounts::*, STORAGE_LEN},
    std::mem::{align_of, size_of},
};

pub fn asserts() {
    assert_eq!(align_of::<AccountState>(), 1);
    assert_eq!(align_of::<AccountType>(), 1);
    assert_eq!(align_of::<Slot>(), 1);
    assert_eq!(align_of::<Storage>(), 1);
    assert_eq!(align_of::<Code>(), 1);
    assert_eq!(align_of::<Valids>(), 1);
    assert_eq!(align_of::<Holder>(), 1);
    assert_eq!(align_of::<TxHolder>(), 1);
    assert_eq!(align_of::<StateHolder>(), 1);
    assert_eq!(align_of::<Lock>(), 1);
    assert_eq!(align_of::<RoLock>(), 1);
    assert_eq!(align_of::<OwnerInfo>(), 1);
    assert_eq!(size_of::<TxHolder>(), size_of::<StateHolder>());
    assert!(STORAGE_LEN <= u8::MAX as usize + 1);
    assert_eq!(align_of::<Ver>(), 1);
    assert_eq!(align_of::<AccountType>(), 1);
    assert_eq!(size_of::<Lock>(), 41);
}
