//! Architecture-specific functions

use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut, RangeInclusive},
    sync::atomic::{AtomicBool, Ordering},
};

/// Disables interrupts being delivered to the current core
pub unsafe fn disable_interrupts() {}

pub const MSTATUS_MPP: RangeInclusive<usize> = 11..=12;
pub const MSTATUS_MIE: usize = 3;

pub const MIE_MTIE: usize = 7;

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

pub fn set_tp(new: usize) {
    unsafe {
        asm!(
            "mv tp, {0}",
            in(reg) new,
            options(nomem, nostack)
        )
    }
}

pub fn get_tp() -> usize {
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
