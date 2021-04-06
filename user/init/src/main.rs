// init process
#![no_std]

use mu::syscall;

extern crate mu;

fn main() {
    syscall::log("ffffff");
    loop {}
}
