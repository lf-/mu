//! Architecture-specific functions

use core::mem;
use core::ops::RangeInclusive;
use core::ptr;

use fidget_spinner::ArchDetails;
use riscv_paging::PAGE_MASK;
use riscv_paging::{Addr, PageSize, PageTable, PhysAccess, PhysPageMetadata, PAGE_SIZE};

use bitvec::prelude::*;

use crate::addr::PHYSMEM_MAP;

pub type Mutex<T> = fidget_spinner::Mutex<T, Arch>;
pub type Phys<T> = riscv_paging::Phys<T, PhysMem>;
pub type PhysAddr = riscv_paging::PhysAddr<PhysMem>;

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
pub const MSTATUS_SUM: usize = 18;

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
    };
    ($docs:expr, $fn_name:ident, $csr_name:ident, struct $ret_ty:ident) => {
        #[doc=$docs]
        pub unsafe fn $fn_name() -> $ret_ty {
            let ret;
            asm!(
                concat!("csrr {0}, ", stringify!($csr_name)),
                out(reg) ret
            );
            $ret_ty(ret)
        }
    };
    ($docs:expr, $fn_name:ident, $csr_name:ident, enum $ret_ty:ident) => {
        #[doc=$docs]
        pub unsafe fn $fn_name() -> $ret_ty {
            let ret: usize;
            asm!(
                concat!("csrr {0}, ", stringify!($csr_name)),
                out(reg) ret
            );
            $ret_ty::from(ret)
        }
    };
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
    };

    ($docs:expr, $fn_name:ident, $csr_name:ident, struct $working_ty:ty) => {
        #[doc=$docs]
        pub unsafe fn $fn_name($csr_name: $working_ty) {
            asm!(
                concat!("csrw ", stringify!($csr_name), ", {0}"),
                in(reg) $csr_name.0
            );
        }
    };
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
    struct StatusReg
);

#[cfg(target_arch = "riscv64")]
csrw!(
    r#"Sets the RISC-V machine-mode status register

This is restricted to rv64 since it has a different status register format.
"#,
    set_mstatus,
    mstatus,
    struct StatusReg
);

csrw!(
    "Sets the machine exception return address",
    set_mepc,
    mepc,
    *const ()
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
csrw!("Sets the machine mode trap vector", set_mtvec, mtvec, u64);

csrr!("Gets the machine mode interrupt enables", get_mie, mie);
csrw!("Sets the machine mode interrupt enables", set_mie, mie);

// -----Supervisor Instructions-----

csrr!(
    "Gets the supervisor interrupt enable register",
    get_sie,
    sie
);

csrr!(
    "Gets the supervisor interrupt pending register",
    get_sip,
    sip
);

csrw!(
    "Sets the supervisor interrupt enable register",
    set_sie,
    sie
);

csrw!(
    "Sets the supervisor address translation and protection register",
    set_satp,
    satp,
    struct Satp
);

csrr!(
    "Gets the supervisor address translation and protection register",
    get_satp,
    satp,
    struct Satp
);

csrw!(
    "Sets the running task (saves the given task pointer to sscratch)",
    set_running_task,
    sscratch
);

csrw!("Sets the supervisor trap vector", set_stvec, stvec, u64);

csrr!(
    "Gets the supervisor status register",
    get_sstatus,
    sstatus,
    struct StatusReg
);

csrw!(
    "Sets the supervisor status register",
    set_sstatus,
    sstatus,
    struct StatusReg
);

csrr!("Gets the supervisor trap cause", get_scause, scause, enum SCause);

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

/// Freezes a hart unrecoverably.
///
/// This is accomplished by turning off interrupts to S mode and then executing
/// `wfi`s (§ 3.3.3 Privileged) forever.
pub fn freeze_hart() -> ! {
    // safety: this is in s mode so it is ok
    unsafe {
        let mut sreg = get_sstatus();
        sreg.set_s_ints(false);
        set_sstatus(sreg);
    }
    loop {
        unsafe { asm!("wfi") }
    }
}

/// Call into M-mode. Do not change these without also fixing their respective
/// definitions in vectors.s.
#[repr(usize)]
pub enum MachineCall {
    /// Requests that the `mc_arg` hart be interrupted.
    InterruptHart = 0,
    /// Requests that the STIP flag be cleared in mstatus.
    ClearTimerInt = 1,
}

/// Call into machine mode.
pub fn machinecall(mc: MachineCall, mc_arg: usize) {
    let mcnum = mc as usize;
    // a0 and a1 are clobbered in machine mode here
    unsafe {
        asm!("ecall",
            inout("a0") mcnum => _,
            inout("a1") mc_arg => _)
    };
}

#[allow(dead_code)]
#[repr(u8)]
pub enum ArchPrivilegeLevel {
    User = 0,
    Supervisor = 1,
    Reserved = 2,
    Machine = 3,
}

pub enum TranslationMode {
    /// no translation/protection
    Bare,
    Sv39,
    Other(u8),
}

/// `satp` CSR
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Satp(pub u64);

impl Satp {
    pub const DISABLED: Satp = Satp(0);

    /// Makes the `satp` register required for using the given page table
    pub fn new(pt: &PageTable<PhysMem>, asid: u16, mode: TranslationMode) -> Satp {
        let ppn = pt.get_base().get() / (PAGE_SIZE as usize);
        assert!(ppn < 1 << 43, "ppn out of range");
        let mut v = 0u64;
        let bits = v.view_bits_mut::<Lsb0>();
        bits[0..=43].store(ppn);
        bits[44..=59].store(asid);
        let mut satp = Satp(v);
        satp.set_mode(mode);
        satp
    }

    /// Gets the current Satp. Convenience function.
    pub fn current() -> Satp {
        unsafe { get_satp() }
    }

    /// Turns the Satp into a page table. Will fail if the Satp has the wrong
    /// mode.
    pub unsafe fn as_pagetable(&self) -> Option<PageTable<PhysMem>> {
        if matches!(self.mode(), TranslationMode::Sv39) {
            Some(PageTable::from_raw(Phys::new_raw(
                (self.ppn() * PAGE_SIZE) as usize,
            )))
        } else {
            None
        }
    }

    /// Gets the physical page number in the Satp
    pub fn ppn(&self) -> u64 {
        let v = self.0.view_bits::<Lsb0>();
        v[0..=43].load()
    }

    /// gets the current mode
    pub fn mode(&self) -> TranslationMode {
        let mode_raw = self.0.view_bits::<Lsb0>()[60..=63].load::<u8>();
        match mode_raw {
            0 => TranslationMode::Bare,
            8 => TranslationMode::Sv39,
            other => TranslationMode::Other(other),
        }
    }

    /// sets the translation mode
    pub fn set_mode(&mut self, new: TranslationMode) {
        self.0.view_bits_mut::<Lsb0>()[60..=63].store(match new {
            TranslationMode::Bare => 0,
            TranslationMode::Sv39 => 8,
            TranslationMode::Other(o) => o,
        })
    }

    /// returns whether paging is enabled
    pub fn paging_enabled(&self) -> bool {
        !matches!(self.mode(), TranslationMode::Bare)
    }
}

/// status register. some fields may not have valid values depending on CPU mode
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct StatusReg(pub u64);

impl StatusReg {
    /// gets the machine previous privilege level (mstatus.MPP)
    pub fn m_prev_pl(&self) -> ArchPrivilegeLevel {
        unsafe {
            // @#$#$@! rustc please let me just cast this bloody thing
            mem::transmute::<u8, ArchPrivilegeLevel>(
                self.0.view_bits::<Lsb0>()[MSTATUS_MPP].load(), //
            )
        }
    }

    /// Sets Supervisor User Memory (true => S mode accesses to User marked
    /// memory allowed)
    pub fn set_sum(&mut self, new: bool) {
        self.0.view_bits_mut::<Lsb0>().set(MSTATUS_SUM, new);
    }

    /// sets the machine previous privilege level (mstatus.MPP)
    pub fn set_m_prev_pl(&mut self, new: ArchPrivilegeLevel) {
        self.0.view_bits_mut::<Lsb0>()[MSTATUS_MPP].store(new as u8);
    }

    /// get machine interrupts enabled (mstatus.MIE)
    pub fn m_ints(&self) -> bool {
        self.0.view_bits::<Lsb0>()[MSTATUS_MIE]
    }

    /// sets machine interrupts enabled (mstatus.MIE)
    pub fn set_m_ints(&mut self, new: bool) {
        self.0.view_bits_mut::<Lsb0>().set(MSTATUS_MIE, new);
    }

    /// set whether supervisor mode will receive interrupts
    pub fn set_s_ints(&mut self, new: bool) {
        self.0.view_bits_mut::<Lsb0>().set(SSTATUS_SIE, new);
    }
}

/// An implementation of ArchDetails
pub struct Arch;

impl ArchDetails for Arch {
    fn core_id() -> usize {
        core_id()
    }
}

static PHYS_FREELIST: Mutex<Option<Phys<PhysPageMetadata<PhysMem>>>> = Mutex::new(None);

/// A structure implementing physical memory access
#[derive(Clone, Copy)]
pub struct PhysMem;

fn pm_base() -> usize {
    if unsafe { get_satp().paging_enabled() } {
        PHYSMEM_MAP
    } else {
        0
    }
}

impl PhysAccess for PhysMem {
    unsafe fn address<T>(ptr: riscv_paging::PhysAddr<Self>) -> *mut T {
        pm_base().wrapping_add(ptr.get()) as *mut T
    }

    unsafe fn alloc() -> Option<riscv_paging::PhysAddr<Self>> {
        let mut guard = PHYS_FREELIST.lock();
        // grab the pointer to the next thing in the list
        let ret = (*guard)?;
        // get the metadata block at the front of the list
        let mine = *ret.as_ptr();
        // get the next one in the list. If the list is empty in an OOM
        // situation we will still keep the last page in the list. Undecided
        // as to whether this is intentional or not.
        let next = mine.next?;
        assert!(
            next.addr().is_page_aligned(PageSize::Page4k),
            "Loaded corrupt (?) metadata from free list: addr to next page not page aligned"
        );
        *guard = Some(next);
        Some(ret.addr())
    }

    unsafe fn free(addr: riscv_paging::PhysAddr<Self>) {
        assert!(
            addr.is_page_aligned(PageSize::Page4k),
            "Freed page address must be page aligned"
        );
        let mut guard = PHYS_FREELIST.lock();
        let tail = *guard;
        let record = PhysPageMetadata { next: tail };
        // store the record pointing to the existing tail the start of the page
        // we're freeing
        let ptr: Phys<PhysPageMetadata<PhysMem>> = Phys::new(addr);
        ptr.as_ptr().write(record);

        // store the pointer to the block we just made on the free list pointer
        *guard = Some(ptr);
    }
}

// ---------------------------- Faults ----------------------------

// has only the top bit set
const TOP: usize = !0 & !(!0 >> 1);

pub enum SCause {
    Exception(ExceptionType),
    Interrupt(InterruptType),
}

impl From<usize> for SCause {
    fn from(v: usize) -> Self {
        if (v as isize) < 0 {
            SCause::Interrupt(InterruptType::from(!TOP & v))
        } else {
            SCause::Exception(ExceptionType::from(v))
        }
    }
}

typesafe_ints::int_enum!(
#[derive(Debug)]
pub enum ExceptionType(usize) {
    InsnAddressMisaligned = 0,
    InsnAccessFault = 1,
    IllegalInsn = 2,
    Breakpoint = 3,
    LoadAddressMisaligned = 4,
    LoadAccessFault = 5,
    StoreAmoAddressMisaligned = 6,
    StoreAmoAccessFault = 7,
    EnvCallU = 8,
    EnvCallS = 9,
    InsnPageFault = 12,
    LoadPageFault = 13,
    StoreAmoPageFault = 15,
}
);

typesafe_ints::int_enum!(
#[derive(Debug)]
pub enum InterruptType(usize) {
    SSoftware = 1,
    MSoftware = 3,
    STimer    = 5,
    MTimer    = 7,
    SExternal = 9,
    MExternal = 11,
}
);
