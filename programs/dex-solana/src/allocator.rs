#![allow(unused_imports)]
#![allow(dead_code)]

use std::alloc::{GlobalAlloc, Layout};

#[cfg(target_os = "solana")]
struct BumpAllocator;

// Copied from solana_program::entrypoint.
const HEAP_START_ADDRESS: u64 = 0x3_0000_0000;

// Reference implementation from
// https://github.com/blockworks-foundation/mango-v4/pull/801
#[cfg(target_os = "solana")]
unsafe impl GlobalAlloc for BumpAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let heap_start = HEAP_START_ADDRESS as usize;
        let pos_ptr = heap_start as *mut usize;

        let mut pos = *pos_ptr;
        if pos == 0 {
            // The first 8 bytes staring from heap_start stores current heap position.
            pos = heap_start + 8;
        }

        // Makes sure begin is aligned with layout.
        let mask = layout.align().wrapping_sub(1);
        // Find next aligned pos.
        let begin = pos.saturating_add(mask) & (!mask);

        // Update pos.
        let end = begin.saturating_add(layout.size());
        *pos_ptr = end;

        // Write a byte to trigger heap overflow errors early
        let end_ptr = end as *mut u8;
        *end_ptr = 0;

        begin as *mut u8
    }

    #[inline]
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't need to free.
    }
}

#[cfg(all(target_os = "solana", not(feature = "no-entrypoint")))]
#[global_allocator]
static A: BumpAllocator = BumpAllocator;
