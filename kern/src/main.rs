#![no_std]
#![no_main]
#![feature(asm)]

mod arch;
#[macro_use]
mod print;
mod addr;
mod interrupts;
mod isr;

use crate::arch::*;

use core::panic::PanicInfo;

use bitvec::prelude::*;

#[no_mangle]
// do not change this size without also changing it in init.s
pub static STACKS: [usize; 8192 * addr::MAX_CPUS] = [0usize; 8192 * addr::MAX_CPUS];

#[panic_handler]
fn panic_handler(_info: &PanicInfo) -> ! {
    loop {}
}

#[allow(dead_code)]
#[repr(u8)]
enum ArchPrivilegeLevel {
    User = 0,
    Supervisor = 1,
    Reserved = 2,
    Machine = 3,
}

#[no_mangle]
unsafe extern "C" fn startup() {
    // this function will be hit by as many harts as we have, at once
    // thus, we will spinloop the ones we don't have work for yet
    let core_id = m_core_id();
    if core_id != 0 {
        loop {}
    }

    // § 3.1.6 RISC-V privileged ISA
    let mut new_mstatus = get_mstatus();
    let view = new_mstatus.view_bits_mut::<Lsb0>();
    // set MPP (previous mode) to supervisor, privilege level 1
    view[MSTATUS_MPP].store(ArchPrivilegeLevel::Supervisor as u64);
    set_mstatus(new_mstatus);

    // turn off paging
    set_satp(0);

    // set the exception return address
    set_mepc(kern_main);

    // set the delegated exceptions and interrupts to be all of the base arch ones
    set_medeleg(0xffff);
    set_mideleg(0xffff);

    // ensure SEIE, STIE, SSIE are on
    let mut sie = get_sie();
    let view = sie.view_bits_mut::<Lsb0>();
    view.set(SIE_SEIE, true);
    view.set(SIE_STIE, true);
    view.set(SIE_SSIE, true);
    set_sie(sie);

    interrupts::init_timers();

    // put our hart id into the thread pointer
    set_tp(core_id);

    asm!("mret");
    core::hint::unreachable_unchecked();
}

unsafe extern "C" fn kern_main() {
    let core_id = get_tp();
    if core_id == 0 {
        crate::print::init();
    }
    println!("hello world from core {}!", core_id);
    // we will hit this with one core!
    // println!("hello world from risc-v!!");
}
