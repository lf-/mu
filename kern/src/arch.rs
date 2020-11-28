//! Architecture-specific functions

use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

/// Gets the identifier of the core running this function
pub fn core_id() -> usize {
    let ret: usize;
    unsafe {
        asm!(
            "csrr {0}, mhartid",
            out(reg) ret
        )
    }
    ret
}

/// A non-reentrant (!!!) mutex
///
/// You can totally deadlock yourself. Try not to.
///
/// This is almost certainly unsound if panic is not abort. Fortunately, we are a
/// kernel
// TODO: implement deadlock detection and owner finding
pub struct Mutex<T> {
    inner: UnsafeCell<T>,
    locked: AtomicBool,
}

unsafe impl<T> Sync for Mutex<T> {}

#[must_use = "you need to use this to use the mutex"]
pub struct LockGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<T> Mutex<T> {
    pub const fn new(inner: T) -> Mutex<T> {
        Mutex {
            inner: UnsafeCell::new(inner),
            locked: AtomicBool::new(false),
        }
    }

    pub fn lock(&self) -> LockGuard<T> {
        // if we are not locked, this will set the mutex to locked
        while self.locked.compare_and_swap(false, true, Ordering::SeqCst) {
            // spin
        }

        LockGuard { mutex: self }
    }
}

impl<T> Deref for LockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // safety: you have the lock guard which can only be created by locking
        // the object
        unsafe { &*self.mutex.inner.get() }
    }
}

impl<T> DerefMut for LockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // safety: you have the lock guard which can only be created by locking
        // the object
        unsafe { &mut *self.mutex.inner.get() }
    }
}

impl<T> Drop for LockGuard<'_, T> {
    fn drop(&mut self) {
        // safety: i am so tired idk
        self.mutex.locked.store(false, Ordering::SeqCst);
    }
}
