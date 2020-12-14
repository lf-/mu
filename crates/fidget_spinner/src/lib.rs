//! A spinlock
#![no_std]
// lol i guess we need this
#![feature(const_fn)]

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::panic::Location;
use core::ptr;
use core::sync::atomic::*;

/// Architecture details
pub trait ArchDetails {
    /// Gets the core ID
    fn core_id() -> usize;
}

/// A non-reentrant (!!!) mutex
///
/// You can totally deadlock yourself. We will panic if you try.
///
/// This is almost certainly unsound if panic is not abort. Fortunately, we are a
/// kernel
pub struct Mutex<T, A: ArchDetails> {
    inner: UnsafeCell<T>,
    ticket: AtomicUsize,
    next_ticket: AtomicUsize,
    owner: AtomicUsize,
    owner_location: AtomicPtr<Location<'static>>,
    _arch_details: PhantomData<A>,
}

unsafe impl<T, A: ArchDetails> Sync for Mutex<T, A> {}
unsafe impl<T, A: ArchDetails> Send for Mutex<T, A> {}

#[must_use = "you need to use this to use the mutex"]
pub struct LockGuard<'a, T, A: ArchDetails> {
    mutex: &'a Mutex<T, A>,
}

impl<T, A: ArchDetails> Mutex<T, A> {
    /// Makes a Mutex
    pub const fn new(inner: T) -> Mutex<T, A> {
        Mutex {
            inner: UnsafeCell::new(inner),
            ticket: AtomicUsize::new(0),
            next_ticket: AtomicUsize::new(0),
            owner: AtomicUsize::new(!0),
            owner_location: AtomicPtr::new(ptr::null_mut()),
            _arch_details: PhantomData,
        }
    }

    /// Defeats the lock. This is *wildly* unsafe. Use with caution.
    ///
    /// This pretty much requires you halted all the other cores with S-mode
    /// interrupts disabled to be safe.
    pub unsafe fn defeat(&self) -> *mut T {
        self.inner.get()
    }

    /// Locks the Mutex and returns a LockGuard that can be used to access the
    /// resource
    #[track_caller]
    pub fn lock(&self) -> LockGuard<T, A> {
        // take a unique ticket
        let ticket = self.next_ticket.fetch_add(1, Ordering::SeqCst);
        let core = A::core_id();

        // wait for our ticket to come up
        while ticket != self.ticket.load(Ordering::SeqCst) {
            if self.owner.load(Ordering::SeqCst) == core {
                // we're trying to unlock a lock that belongs to our core
                // this is a deadlock
                // safety: this is a static thing of some kind idk
                let loc = unsafe { *self.owner_location.load(Ordering::SeqCst) };
                panic!(
                    "tried to lock a lock owned by own core!! deadlock. Check {:?}",
                    loc
                );
            }
            // spin
            core::hint::spin_loop();
        }

        self.owner.store(core, Ordering::SeqCst);
        self.owner_location
            .store(Location::caller() as *const _ as *mut _, Ordering::SeqCst);

        LockGuard { mutex: self }
    }
}

impl<T, A: ArchDetails> Deref for LockGuard<'_, T, A> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // safety: you have the lock guard which can only be created by locking
        // the object
        unsafe { &*self.mutex.inner.get() }
    }
}

impl<T, A: ArchDetails> DerefMut for LockGuard<'_, T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // safety: you have the lock guard which can only be created by locking
        // the object
        unsafe { &mut *self.mutex.inner.get() }
    }
}

impl<T, A: ArchDetails> Drop for LockGuard<'_, T, A> {
    fn drop(&mut self) {
        self.mutex.owner.store(!0, Ordering::SeqCst);
        self.mutex
            .owner_location
            .store(ptr::null_mut(), Ordering::SeqCst);
        // increment the ticket by one to let the next user get it
        self.mutex.ticket.fetch_add(1, Ordering::SeqCst);
    }
}
