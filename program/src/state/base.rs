use {
    super::pda::{Pda, Seed},
    crate::{error::RomeProgramError::*, error::*},
    evm::{H160, U256},
    solana_program::{account_info::MAX_PERMITTED_DATA_INCREASE, pubkey::Pubkey},
    std::{cell::RefCell, rc::Rc},
};

macro_rules! impl_alloc_fn {
    ($alloc:ident, $dealloc:ident, $func:ident) => {
        pub fn $alloc(&self) -> usize {
            *self.$alloc.borrow()
        }

        pub fn $func(&self, len: usize) -> Result<()> {
            if len > 0 {
                if self.$dealloc() > 0 {
                    return Err(AllocationError(
                        "found allocations and deallocations".to_string(),
                    ));
                }
                let mut val = self.$alloc.borrow_mut();
                *val = val.saturating_add(len);
            }

            Ok(())
        }
    };
}

pub struct Base<'a> {
    pub program_id: &'a Pubkey,
    pub chain: u64,
    alloc: RefCell<usize>,
    dealloc: RefCell<usize>,
    alloc_payed: RefCell<usize>,
    dealloc_payed: RefCell<usize>,
    pub pda: Pda<'a>,
    pub syscall: Rc<Syscall>,
}

impl<'a> Base<'a> {
    pub fn new(program_id: &'a Pubkey, chain: u64) -> Self {
        let syscall = Rc::new(Syscall::new());

        Self {
            program_id,
            chain,
            alloc: RefCell::new(0),
            dealloc: RefCell::new(0),
            alloc_payed: RefCell::new(0),
            dealloc_payed: RefCell::new(0),
            pda: Pda::new(program_id, chain, Rc::clone(&syscall)),
            syscall,
        }
    }
    pub fn alloc_limit(&self) -> usize {
        MAX_PERMITTED_DATA_INCREASE.saturating_sub(self.alloc())
    }

    impl_alloc_fn!(alloc, dealloc, inc_alloc);
    impl_alloc_fn!(dealloc, alloc, inc_dealloc);
    impl_alloc_fn!(alloc_payed, dealloc_payed, inc_alloc_payed);
    impl_alloc_fn!(dealloc_payed, alloc_payed, inc_dealloc_payed);

    pub fn slot_to_key(&self, address: &H160, slot: &U256) -> (Pubkey, Seed, u8) {
        let (index_be, sub_ix) = Pda::storage_index(slot);
        let (base, _) = self.pda.balance_key(address);
        let (key, seed) = self.pda.storage_key(&base, index_be);

        (key, seed, sub_ix)
    }

    #[cfg(not(target_os = "solana"))]
    pub fn reset(&self) {
        *self.alloc.borrow_mut() = 0;
        *self.dealloc.borrow_mut() = 0;
        *self.alloc_payed.borrow_mut() = 0;
        *self.dealloc_payed.borrow_mut() = 0;
        self.syscall.reset();
        self.pda.reset();
    }
}

#[derive(Clone)]
pub struct Syscall {
    cnt: RefCell<u64>,
}

impl Syscall {
    pub fn inc(&self) {
        *self.cnt.borrow_mut() += 1;
    }
    pub fn count(&self) -> u64 {
        *self.cnt.borrow()
    }
    pub fn new() -> Self {
        Self {
            cnt: RefCell::new(0),
        }
    }
    #[cfg(not(target_os = "solana"))]
    pub fn reset(&self) {
        *self.cnt.borrow_mut() = 0;
    }
}

impl Default for Syscall {
    fn default() -> Self {
        Syscall::new()
    }
}
