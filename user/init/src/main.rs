// init process
#![no_std]
#![feature(bench_black_box)]

use mu::syscall;

extern crate mu;

fn main() {
    syscall::log("hello from init");
    syscall::log("hello from init 2");
    loop {}
}
