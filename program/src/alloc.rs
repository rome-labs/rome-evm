#[cfg(all(target_os = "solana", not(feature = "no-entrypoint")))]
mod alloc {
    use solana_program::entrypoint::HEAP_START_ADDRESS;

    #[global_allocator]
    static ALLOC: BumpAllocator = BumpAllocator;

    /// The bump allocator with opportunistic freeing which works with increased
    /// heap size.
    struct BumpAllocator;

    unsafe impl core::alloc::GlobalAlloc for BumpAllocator {
        unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
            let addr = self.free_start().unwrap_or(self.heap_start());

            // Align up
            let mask = layout.align() - 1;
            let addr = match addr.checked_add(mask) {
                None => return core::ptr::null_mut(),
                Some(addr) => addr & !mask,
            };
            let end = match addr.checked_add(layout.size()) {
                None => return core::ptr::null_mut(),
                Some(end) => end,
            };

            // Check if we have enough free space left on heap.
            // SAFETY: This is unsound but it will only execute on Solana
            // where accessing memory beyond heap results in segfault which
            // is what we want.
            let _ = unsafe { ((end - 1) as *mut u8).read_volatile() };

            self.set_free_start(end);
            addr as *mut u8
        }

        unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
            // Left as excercise to the read.
            // Or just leave empty.
        }
    }

    impl BumpAllocator {
        /// Returns address of the end of the heap excluding region at the start
        /// reserved for the allocator.
        const fn heap_start(&self) -> usize {
            // We’re storing end pointer at the beginning of heap so the actual
            // start of the heap is HEAP_START_ADDRESS + sizeof(end).
            HEAP_START_ADDRESS as usize + core::mem::size_of::<usize>()
        }

        /// Returns address of the start of the free memory range or None if we
        /// haven’t been initialised yet.
        fn free_start(&self) -> Option<usize> {
            // SAFETY: On Solana location at address HEAP_START_ADDRESS is
            // guaranteed to be zero-initialised and aligned to 4 GiB.
            let addr = unsafe { *(HEAP_START_ADDRESS as *const usize) };
            (addr != 0).then_some(addr)
        }

        /// Sets address of the end of the free memory range.
        fn set_free_start(&self, addr: usize) {
            // SAFETY: On Solana location at address HEAP_START_ADDRESS is
            // guaranteed to be zero-initialised, aligned to 4 GiB and writable.
            unsafe { *(HEAP_START_ADDRESS as *mut usize) = addr }
        }
    }
}
