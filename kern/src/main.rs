#![no_std]
#![no_main]

pub mod exc;
mod tframe;

use log::info;
use riscv::arch::freeze_hart;

/// Checks at compile time that the kern_main conforms to the ABI expected by
/// its caller in `shoo`.
#[allow(dead_code)]
const ASSERT_KERN_MAIN_IS_RIGHT_TYPE: riscv::KernEntry = kern_main;

#[export_name = "_entry"]
#[no_mangle]
pub extern "C" fn kern_main(core_id: usize) -> ! {
    // reinit the serial port ;; this may be bad if we have multiple CPUs; add a barrier
    riscv::print::init();
    info!("Hello world from the kernel on cpu {}!", core_id);
    freeze_hart()
}
