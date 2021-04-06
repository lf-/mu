#![no_std]
#![no_main]

pub mod exc;
mod tframe;

use arch::Satp;
use exc::enter_userspace;
use log::info;
use riscv::arch;
use riscv::paging::Addr;
use riscv::{arch::freeze_hart, KernelEntryParams};
use tframe::TrapFrame;

/// Checks at compile time that the kern_main conforms to the ABI expected by
/// its caller in `shoo`.
#[allow(dead_code)]
const ASSERT_KERN_MAIN_IS_RIGHT_TYPE: riscv::KernEntry = kern_main;

unsafe extern "C" fn entry(_: *mut TrapFrame) -> ! {
    panic!("fff");
}

#[export_name = "_entry"]
#[no_mangle]
pub extern "C" fn kern_main(params: &KernelEntryParams) -> ! {
    // reinit the serial port ;; this may be bad if we have multiple CPUs; add a barrier
    riscv::print::init();
    info!("Hello world from the kernel on cpu {}!", params.core_id);

    let tf = TrapFrame {
        hart_id: arch::core_id(),
        kernel_sp: params.stack_pointer.get() as *mut _,
        new_satp: Satp::current(),
        regs: [
            /* ra */ 0,
            /* sp */ params.init_sp.get(),
            /* gp */ 0,
            /* tp */ 0,
            /* t0 */ 0,
            /* t1 */ 0,
            /* t2 */ 0,
            /* s0 */ 0,
            /* s1 */ 0,
            /* a0 */ 0,
            /* a1 */ 0,
            /* a2 */ 0,
            /* a3 */ 0,
            /* a4 */ 0,
            /* a5 */ 0,
            /* a6 */ 0,
            /* a7 */ 0,
            /* s2 */ 0,
            /* s3 */ 0,
            /* s4 */ 0,
            /* s5 */ 0,
            /* s6 */ 0,
            /* s7 */ 0,
            /* s8 */ 0,
            /* s9 */ 0,
            /* s10 */ 0,
            /* s11 */ 0,
            /* t3 */ 0,
            /* t4 */ 0,
            /* t5 */ 0,
            /* t6 */ 0,
        ],
        user_pc: params.init_entrypoint,
        target_fn: entry,
    };
    unsafe { enter_userspace(tf) };
    freeze_hart()
}
