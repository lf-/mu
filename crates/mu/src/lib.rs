//! Base functions for building stuff on top of mu
#![no_std]
#![feature(asm)]
#![feature(lang_items)]
#![feature(panic_info_message)]

pub mod panic;
pub mod print;
pub mod syscall;

extern "C" {
    fn main(argc: isize, argv: *const *const u8) -> isize;
}

#[no_mangle]
extern "C" fn _start() -> ! {
    let argv = [];
    unsafe { main(0, argv.as_ptr()) };
    unreachable!()
}

// convince rustc to not ICE us :D
trait Termination {}

impl Termination for () {}

#[lang = "start"]
fn lang_start<T: Termination>(main: fn() -> T, argc: isize, argv: *const *const u8) -> isize {
    main();
    2
}
