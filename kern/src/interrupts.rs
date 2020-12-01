//! Hardware abstraction code for the interrupt controllers

use core::ffi::c_void;
use core::ptr;

use bitvec::prelude::*;

use crate::addr;
use crate::arch;
use crate::isr;

const TIMER_ISR_EMPTY: TimerIsrData = TimerIsrData {
    regs: [0; 2],
    my_mtimecmp: ptr::null_mut(),
    my_interval: 0,
};

pub static mut TIMER_ISR_DATA: [TimerIsrData; addr::MAX_CPUS] = [TIMER_ISR_EMPTY; addr::MAX_CPUS];
pub static CLINT: Clint = Clint {
    base: addr::CLINT as *mut _,
};

/// Data to be used by our timer ISRs in `vectors.s`. Do not change this
/// structure without checking those first!
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TimerIsrData {
    pub regs: [u64; 2],
    pub my_mtimecmp: *mut u64,
    pub my_interval: u64,
}

/// Some dude that gives you interrupts
/// jk it's the SiFive CLINT interrupt controller
pub struct Clint {
    base: *mut (),
}

/// safety: the accesses don't step on each other
unsafe impl Sync for Clint {}

impl Clint {
    /// Creates a new Clint interface.
    ///
    /// Safety: given pointer must actually point to a Clint
    pub const unsafe fn new(base: *mut ()) -> Clint {
        Clint { base }
    }

    /// Send a software interrupt to a hart
    pub unsafe fn interrupt_hart(&self, hart_id: u8) {
        // get the address of the msip
        let addr = (self.base as *mut u32).offset(hart_id as isize);
        // interrupt it
        addr.write_volatile(1);
    }

    unsafe fn my_mtimecmp(&self, hart_id: u8) -> *mut u64 {
        let mtimecmp_base = self.base as usize + 0x4000;
        let addr = (mtimecmp_base as *mut u64).offset(hart_id as isize);
        addr
    }

    /// Sets the next mtimecmp interrupt time in ~cycles from now
    pub unsafe fn schedule_interrupt(&self, hart_id: u8, int_time: u64) {
        // documented in SiFive U74MC manual, section 9.5
        // available at https://sifive.cdn.prismic.io/sifive/aee0dd4c-d156-496e-a6c4-db0cf54bbe68_sifive_U74MC_rtl_full_20G1.03.00_manual.pdf
        let mtime_base = self.base as usize + 0xbff8;
        let addr = self.my_mtimecmp(hart_id);

        let now = (mtime_base as *const u64).read_volatile();
        let next_time = now.wrapping_add(int_time);
        addr.write_volatile(next_time);
    }
}

extern "C" {
    static MACHINE_VECTORS: c_void;
}

pub unsafe fn init_timers() {
    let hart = arch::m_core_id();

    // stolen from xv6, apparently 1/10s in qemu
    let interval = 1000000;
    CLINT.schedule_interrupt(hart as u8, interval);

    // TODO:
    // this is probably a bad idea since this stuff is really probably volatile
    // also we should fix that static mut. yikes.
    let my_isr_data = &mut TIMER_ISR_DATA[hart];

    *my_isr_data = TimerIsrData {
        my_mtimecmp: CLINT.my_mtimecmp(hart as u8),
        my_interval: interval,
        ..TIMER_ISR_DATA[hart]
    };

    // we are using non-vectored interrupt mode; set the machine trap vector
    arch::set_mtvec(&MACHINE_VECTORS as *const c_void as u64);
    arch::set_mscratch(&mut TIMER_ISR_DATA[hart] as *mut TimerIsrData as usize);

    // turn on machine interrupts
    let mut status = arch::get_mstatus();
    let view = status.view_bits_mut::<Lsb0>();
    view.set(arch::MSTATUS_MIE, true);
    arch::set_mstatus(status);

    // enable machine timer interrupts
    let mut mie = arch::get_mie();
    let view = mie.view_bits_mut::<Lsb0>();
    view.set(arch::MIE_MTIE, true);
    arch::set_mie(mie);
}
