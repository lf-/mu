//! Architecture-specific functions

use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut, RangeInclusive},
    panic::Location,
    ptr,
    sync::atomic::AtomicPtr,
    sync::atomic::AtomicUsize,
    sync::atomic::{AtomicBool, Ordering},
};

/// Disables interrupts being delivered to the current core
pub unsafe fn disable_interrupts() {
    // we will be invoked from supervisor mode so we need to change the sstatus
    let clear_mask = 1 << SSTATUS_SIE;
    asm!(
        "csrc sstatus, {0}",
        in(reg) clear_mask
    )
}

/// Disables interrupts being delivered to the current core
pub unsafe fn enable_interrupts() {
    // we will be invoked from supervisor mode so we need to change the sstatus
    let set_mask = 1 << SSTATUS_SIE;
    asm!(
        "csrs sstatus, {0}",
        in(reg) set_mask
    )
}

pub const MSTATUS_MPP: RangeInclusive<usize> = 11..=12;
pub const MSTATUS_MIE: usize = 3;

pub const MIE_MTIE: usize = 7;

pub const SSTATUS_SIE: usize = 1;

pub const SIE_SEIE: usize = 9;
pub const SIE_STIE: usize = 5;
pub const SIE_SSIE: usize = 1;

macro_rules! csrr {
    ($docs:expr, $fn_name:ident, $csr_name:ident) => {
        csrr!($docs, $fn_name, $csr_name, usize);
    };

    ($docs:expr, $fn_name:ident, $csr_name:ident, $ret_ty:ty) => {
        #[doc=$docs]
        pub unsafe fn $fn_name() -> $ret_ty {
            let ret;
            asm!(
                concat!("csrr {0}, ", stringify!($csr_name)),
                out(reg) ret
            );
            ret
        }
    }
}

macro_rules! csrw {
    ($docs:expr, $fn_name:ident, $csr_name:ident) => {
        csrw!($docs, $fn_name, $csr_name, usize);
    };

    ($docs:expr, $fn_name:ident, $csr_name:ident, $working_ty:ty) => {
        #[doc=$docs]
        pub unsafe fn $fn_name($csr_name: $working_ty) {
            asm!(
                concat!("csrw ", stringify!($csr_name), ", {0}"),
                in(reg) $csr_name
            );
        }
    }
}

/// Gets the identifier of the core running this function
pub fn m_core_id() -> usize {
    let ret: usize;
    unsafe {
        asm!(
            "csrr {0}, mhartid",
            out(reg) ret
        )
    }
    ret
}

#[cfg(target_arch = "riscv64")]
csrr!(
    "Gets the RISC-V machine-mode mstatus register",
    get_mstatus,
    mstatus,
    u64
);

#[cfg(target_arch = "riscv64")]
csrw!(
    r#"Sets the RISC-V machine-mode status register

This is restricted to rv64 since it has a different status register format.
"#,
    set_mstatus,
    mstatus,
    u64
);

csrw!(
    "Sets the machine exception return address",
    set_mepc,
    mepc,
    unsafe extern "C" fn()
);

csrw!(
    "Sets the bitfield of which machine exceptions are delegated to supervisor mode",
    set_medeleg,
    medeleg
);

csrw!(
    "Sets the bitfield of which machine interrupts are delegated to supervisor mode",
    set_mideleg,
    mideleg
);

csrw!("Sets the machine scratch register", set_mscratch, mscratch);
csrw!(
    "Sets the machine mode trap vector",
    set_mtvec,
    mtvec,
    unsafe extern "C" fn()
);

csrr!("Gets the machine mode interrupt enables", get_mie, mie);
csrw!("Sets the machine mode interrupt enables", set_mie, mie);

// -----Supervisor Instructions-----

csrr!(
    "Gets the supervisor interrupt enable register",
    get_sie,
    sie
);

csrw!(
    "Sets the supervisor interrupt enable register",
    set_sie,
    sie
);

csrw!(
    "Sets the supervisor address translation and protection register",
    set_satp,
    satp
);

// ------------- Unprivileged Instructions ---------------

pub fn set_core_id(new: usize) {
    unsafe {
        asm!(
            "mv tp, {0}",
            in(reg) new,
            options(nomem, nostack)
        )
    }
}

pub fn core_id() -> usize {
    unsafe {
        let tp;
        asm!(
            "mv {0}, tp",
            out(reg) tp,
            options(nomem, nostack)
        );
        tp
    }
}

/// A non-reentrant (!!!) mutex
///
/// You can totally deadlock yourself. We will panic if you try.
///
/// This is almost certainly unsound if panic is not abort. Fortunately, we are a
/// kernel
// TODO: implement deadlock detection and owner finding
pub struct Mutex<T> {
    inner: UnsafeCell<T>,
    ticket: AtomicUsize,
    next_ticket: AtomicUsize,
    owner: AtomicUsize,
    owner_location: AtomicPtr<Location<'static>>,
    mask_interrupts: bool,
}

unsafe impl<T> Sync for Mutex<T> {}
unsafe impl<T> Send for Mutex<T> {}

#[must_use = "you need to use this to use the mutex"]
pub struct LockGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<T> Mutex<T> {
    /// Makes a Mutex
    pub const fn new(inner: T) -> Mutex<T> {
        Mutex {
            inner: UnsafeCell::new(inner),
            ticket: AtomicUsize::new(0),
            next_ticket: AtomicUsize::new(0),
            owner: AtomicUsize::new(!0),
            owner_location: AtomicPtr::new(ptr::null_mut()),
            mask_interrupts: false,
        }
    }

    /// Makes a Mutex that disables interrupts
    pub const fn new_nopreempt(inner: T) -> Mutex<T> {
        Mutex {
            inner: UnsafeCell::new(inner),
            ticket: AtomicUsize::new(0),
            next_ticket: AtomicUsize::new(0),
            owner: AtomicUsize::new(!0),
            owner_location: AtomicPtr::new(ptr::null_mut()),
            mask_interrupts: true,
        }
    }

    #[track_caller]
    pub fn lock(&self) -> LockGuard<T> {
        // take a unique ticket
        let ticket = self.next_ticket.fetch_add(1, Ordering::SeqCst);
        let core = core_id();

        if self.mask_interrupts {
            unsafe { disable_interrupts() };
        }

        // wait for our ticket to come up
        while ticket != self.ticket.load(Ordering::SeqCst) {
            if self.owner.load(Ordering::SeqCst) == core {
                // we're trying to unlock a lock that belongs to our core
                // this is a deadlock
                // safety: this is a static thing of some kind idk
                let loc = unsafe { &*(self.owner.load(Ordering::SeqCst) as *const Location) };
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
        if self.mutex.mask_interrupts {
            // TODO: this is straight up buggy if we are called from within an
            // ISR because we should not enable interrupts if they were off the
            // whole time!!
            unsafe { enable_interrupts() }
        }
        // increment the ticket by one to let the next user get it
        self.mutex.ticket.fetch_add(1, Ordering::SeqCst);
    }
}
