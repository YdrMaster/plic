//! Provide structs and methods to operate riscv plic device.

#![no_std]
#![deny(warnings, missing_docs)]

use core::{cell::UnsafeCell, mem::size_of, num::NonZeroU32};

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

/// The PLIC memory mapping.
///
/// See §3.
#[repr(C, align(4096))]
pub struct Plic {
    priorities: Priorities,
    pending_bits: PendingBits,
    _reserved0: [u8; 4096 - size_of::<PendingBits>()],
    enables: Enables,
    _reserved1: [u8; 0xe000],
    context_local: [ContextLocal; COUNT_CONTEXT],
}

impl Plic {
    /// See §4.
    #[inline]
    pub fn write_source_priorities(&self, source: usize, val: u32) {
        unsafe { self.priorities.0[source].get().write_volatile(val) }
    }

    /// See §4.
    #[inline]
    pub fn read_source_priorities(&self, source: usize) -> u32 {
        unsafe { self.priorities.0[source].get().read_volatile() }
    }

    /// See §4.
    #[inline]
    pub fn disable_source(&self, source: usize) {
        self.write_source_priorities(source, 0)
    }

    /// See §4.
    #[inline]
    pub fn probe_source_priorities_bits(&self, source: usize) -> u32 {
        let ptr = self.priorities.0[source].get();
        unsafe {
            ptr.write_volatile(!0);
            ptr.read_volatile()
        }
    }

    /// See §5.
    #[inline]
    pub fn read_source_pending(&self, source: usize) -> bool {
        let group = source / U32_BITS;
        let index = source % U32_BITS;

        let ptr = self.pending_bits.0[group].get();
        (unsafe { ptr.read_volatile() } & (1 << index)) != 0
    }

    /// See §6.
    #[inline]
    pub fn enable_context(&self, source: usize, context: usize) {
        let pos = source * context;
        let group = pos / U32_BITS;
        let index = pos % U32_BITS;

        let ptr = self.enables.0[group].get();
        unsafe { ptr.write_volatile(ptr.read_volatile() | (1 << index)) }
    }

    /// See §6.
    #[inline]
    pub fn disable_context(&self, source: usize, context: usize) {
        let pos = source * context;
        let group = pos / U32_BITS;
        let index = pos % U32_BITS;

        let ptr = self.enables.0[group].get();
        unsafe { ptr.write_volatile(ptr.read_volatile() & !(1 << index)) }
    }

    /// See §6.
    #[inline]
    pub fn read_context_enable(&self, source: usize, context: usize) -> bool {
        let pos = source * context;
        let group = pos / U32_BITS;
        let index = pos % U32_BITS;

        let ptr = self.enables.0[group].get();
        (unsafe { ptr.read_volatile() } & (1 << index)) != 0
    }

    /// See §7.
    #[inline]
    pub fn read_context_priority_threshold(&self, context: usize) -> u32 {
        let ptr = self.context_local[context].priority_threshold.get();
        unsafe { ptr.read_volatile() }
    }

    /// See §7.
    #[inline]
    pub fn write_context_priority_threshold(&self, context: usize, val: u32) {
        let ptr = self.context_local[context].priority_threshold.get();
        unsafe { ptr.write_volatile(val) }
    }

    /// See §7.
    #[inline]
    pub fn probe_context_priority_threshold_bits(&self, context: usize) -> u32 {
        let ptr = self.context_local[context].priority_threshold.get();
        unsafe {
            ptr.write_volatile(!0);
            ptr.read_volatile()
        }
    }

    /// See §8.
    #[inline]
    pub fn claim(&self, context: usize) -> Option<NonZeroU32> {
        let ptr = self.context_local[context].claim_or_completion.get();
        NonZeroU32::new(unsafe { ptr.read_volatile() })
    }

    /// See §9.
    #[inline]
    pub fn complete(&self, context: usize, id: NonZeroU32) {
        let ptr = self.context_local[context].claim_or_completion.get();
        unsafe { ptr.write_volatile(id.get()) }
    }
}

#[test]
fn test() {
    assert_eq!(
        size_of::<Plic>(),
        0x20_0000 + COUNT_CONTEXT * size_of::<ContextLocal>()
    )
}
