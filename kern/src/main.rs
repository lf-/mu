#![no_std]
#![no_main]
#![feature(asm)]

mod arch;
#[macro_use]
mod print;

use core::panic::PanicInfo;

const MAX_CPUS: usize = 8;

#[no_mangle]
// do not change this number without also changing it in init.s
pub static STACKS: [usize; 8192 * MAX_CPUS] = [0usize; 8192 * MAX_CPUS];

#[panic_handler]
fn panic_handler(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
extern "C" fn startup() {
    // this function will be hit by as many harts as we have, at once
    // thus, we will spinloop the ones we don't have work for yet
    if arch::core_id() != 0 {
        loop {}
    }

    crate::print::init();
    println!("hello world!");
    // we will hit this with one core!
    // println!("hello world from risc-v!!");
}
