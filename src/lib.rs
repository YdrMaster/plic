//! Provide structs and methods to operate riscv plic device.

#![no_std]
#![deny(warnings, missing_docs)]

use core::{cell::UnsafeCell, mem::size_of, num::NonZeroU32};

/// See §1.
const COUNT_SOURCE: usize = 1024;
/// See §1.
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

/// Trait for enums of external interrupt source.
///
/// See §1.4.
pub trait InterruptSource {
    /// The identifier number of the interrupt source.
    fn id(self) -> NonZeroU32;
}

#[cfg(feature = "primitive-id")]
impl InterruptSource for NonZeroU32 {
    #[inline]
    fn id(self) -> NonZeroU32 {
        self
    }
}

#[cfg(feature = "primitive-id")]
impl InterruptSource for u32 {
    #[inline]
    fn id(self) -> NonZeroU32 {
        NonZeroU32::new(self).expect("interrupt source id can not be zero")
    }
}

/// A hart context is a given privilege mode on a given hart.
///
/// See §1.1.
pub trait HartContext {
    /// See §6.
    ///
    /// > How PLIC organizes interrupts for the contexts (Hart and privilege mode)
    /// > is out of RISC-V PLIC specification scope, however it must be spec-out
    /// > in vendor’s PLIC specification.
    fn index(self) -> usize;
}

#[cfg(feature = "primitive-id")]
impl HartContext for usize {
    #[inline]
    fn index(self) -> usize {
        self
    }
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
    pub fn set_priority<S>(&self, source: S, val: u32)
    where
        S: InterruptSource,
    {
        let ptr = self.priorities.0[source.id().get() as usize].get();
        unsafe { ptr.write_volatile(val) }
    }

    /// See §4.
    #[inline]
    pub fn get_priority<S>(&self, source: S) -> u32
    where
        S: InterruptSource,
    {
        let ptr = self.priorities.0[source.id().get() as usize].get();
        unsafe { ptr.read_volatile() }
    }

    /// See §4.
    #[inline]
    pub fn disable_source<S>(&self, source: S)
    where
        S: InterruptSource,
    {
        self.set_priority(source, 0)
    }

    /// See §4.
    #[inline]
    pub fn probe_source_priority_bits<S>(&self, source: S) -> u32
    where
        S: InterruptSource,
    {
        let ptr = self.priorities.0[source.id().get() as usize].get();
        unsafe {
            ptr.write_volatile(!0);
            ptr.read_volatile()
        }
    }

    /// See §5.
    #[inline]
    pub fn is_source_pending<S>(&self, source: S) -> bool
    where
        S: InterruptSource,
    {
        let source = source.id().get() as usize;
        let group = source / U32_BITS;
        let index = source % U32_BITS;

        let ptr = self.pending_bits.0[group].get();
        (unsafe { ptr.read_volatile() } & (1 << index)) != 0
    }

    /// See §6.
    #[inline]
    pub fn enable_context<S, C>(&self, source: S, context: C)
    where
        S: InterruptSource,
        C: HartContext,
    {
        let source = source.id().get() as usize;
        let context = context.index();
        let pos = source * context;
        let group = pos / U32_BITS;
        let index = pos % U32_BITS;

        let ptr = self.enables.0[group].get();
        unsafe { ptr.write_volatile(ptr.read_volatile() | (1 << index)) }
    }

    /// See §6.
    #[inline]
    pub fn disable_context<S, C>(&self, source: S, context: C)
    where
        S: InterruptSource,
        C: HartContext,
    {
        let source = source.id().get() as usize;
        let context = context.index();
        let pos = source * context;
        let group = pos / U32_BITS;
        let index = pos % U32_BITS;

        let ptr = self.enables.0[group].get();
        unsafe { ptr.write_volatile(ptr.read_volatile() & !(1 << index)) }
    }

    /// See §6.
    #[inline]
    pub fn is_context_enabled<S, C>(&self, source: S, context: C) -> bool
    where
        S: InterruptSource,
        C: HartContext,
    {
        let source = source.id().get() as usize;
        let context = context.index();
        let pos = source * context;
        let group = pos / U32_BITS;
        let index = pos % U32_BITS;

        let ptr = self.enables.0[group].get();
        (unsafe { ptr.read_volatile() } & (1 << index)) != 0
    }

    /// See §7.
    #[inline]
    pub fn get_threshold<C>(&self, context: C) -> u32
    where
        C: HartContext,
    {
        let ptr = self.context_local[context.index()].priority_threshold.get();
        unsafe { ptr.read_volatile() }
    }

    /// See §7.
    #[inline]
    pub fn set_threshold<C>(&self, context: C, val: u32)
    where
        C: HartContext,
    {
        let ptr = self.context_local[context.index()].priority_threshold.get();
        unsafe { ptr.write_volatile(val) }
    }

    /// See §7.
    #[inline]
    pub fn probe_threshold_bits<C>(&self, context: C) -> u32
    where
        C: HartContext,
    {
        let ptr = self.context_local[context.index()].priority_threshold.get();
        unsafe {
            ptr.write_volatile(!0);
            ptr.read_volatile()
        }
    }

    /// See §8.
    #[inline]
    pub fn claim<C>(&self, context: C) -> Option<NonZeroU32>
    where
        C: HartContext,
    {
        let ptr = self.context_local[context.index()]
            .claim_or_completion
            .get();
        NonZeroU32::new(unsafe { ptr.read_volatile() })
    }

    /// See §9.
    #[inline]
    pub fn complete<C, S>(&self, context: C, source: S)
    where
        C: HartContext,
        S: InterruptSource,
    {
        let ptr = self.context_local[context.index()]
            .claim_or_completion
            .get();
        unsafe { ptr.write_volatile(source.id().get()) }
    }
}

#[test]
fn test() {
    assert_eq!(
        size_of::<Plic>(),
        0x20_0000 + COUNT_CONTEXT * size_of::<ContextLocal>()
    )
}
