//! Provide structs and methods to operate riscv plic device.

#![no_std]
// #![deny(warnings, missing_docs)]

use core::{cell::UnsafeCell, mem::size_of};

const COUNT_SOURCE: usize = 1024;
const COUNT_CONTEXT: usize = 15872;
const U32_BITS: usize = u32::BITS as _;

#[repr(transparent)]
struct Priorities([UnsafeCell<u32>; COUNT_SOURCE]);

#[repr(transparent)]
struct PendingBits([UnsafeCell<u32>; COUNT_SOURCE / U32_BITS]);

#[repr(transparent)]
struct Enables([UnsafeCell<u32>; COUNT_SOURCE * COUNT_CONTEXT / U32_BITS]);

#[repr(C, align(4096))]
struct ContextLocal {
    priority_threshold: UnsafeCell<u32>,
    claim_or_completion: UnsafeCell<u32>,
    _reserved: [u8; 4096 - 2 * size_of::<u32>()],
}

#[repr(C, align(4096))]
pub struct Plic {
    priorities: Priorities,
    pending_bits: PendingBits,
    _reserved0: [u8; 4096 - size_of::<PendingBits>()],
    enables: Enables,
    _reserved1: [u8; 0xe000],
    context_local: ContextLocal,
}

#[test]
fn test() {
    assert_eq!(size_of::<Plic>(), 0x20_0000 + size_of::<ContextLocal>())
}
