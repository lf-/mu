use core::{cell::UnsafeCell, sync::atomic::*};

use crate::addr;

/// Structure is only accessible from asm, where its sync guarantees are upheld
#[repr(transparent)]
pub struct AsmOnly<T>(UnsafeCell<T>);
unsafe impl<T> Sync for AsmOnly<T> {}

#[no_mangle]
// do not change this size without also changing it in init.s
pub static STACKS: AsmOnly<[u8; 16384 * addr::MAX_CPUS]> =
    AsmOnly(UnsafeCell::new([0u8; 16384 * addr::MAX_CPUS]));

impl HasEmpty for [u8; 8192] {
    const EMPTY: UnsafeCell<Self> = UnsafeCell::new([0u8; 8192]);
}

// TODO: how do I get these into assembly in my fault handler? I could put a pointer
// into the Task structure I suppose?? idk what the fuck im doing
pub static EXCEPTION_STACKS: PerHartMut<[u8; 8192]> = PerHartMut::new();

pub static PANICKED: AtomicBool = AtomicBool::new(false);
pub static PANIC_CHECKIN: AtomicUsize = AtomicUsize::new(0);
pub static NUM_CPUS: AtomicUsize = AtomicUsize::new(0);

/// Thing that has an empty state
pub trait HasEmpty {
    const EMPTY: UnsafeCell<Self>;
}

/// [`core::marker::Sync`] trait for per-hart mutable state
#[repr(transparent)]
pub struct PerHartMut<T: HasEmpty>([UnsafeCell<T>; addr::MAX_CPUS]);

/// safety: the structure itself is Sync as it is unsafe to access a given item
unsafe impl<T: HasEmpty> Sync for PerHartMut<T> {}

impl<T: HasEmpty> PerHartMut<T> {
    pub const fn new() -> PerHartMut<T> {
        PerHartMut([T::EMPTY; addr::MAX_CPUS])
    }
    /// Gets a reference to the exception task for the given hart.
    ///
    /// safety: you must only call this once, only from the hart with your own
    /// `hart` number
    pub unsafe fn get(&self, hart: usize) -> &'static mut T {
        &mut *self.0[hart].get()
    }
}
