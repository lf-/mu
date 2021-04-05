//! panic handlers

// for now we will just infloop after printing, because process termination is
// as yet undecided
#[panic_handler]
fn do_panic(info: &core::panic::PanicInfo) -> ! {
    crate::println!("PANIC");
    if let Some(content) = info.message() {
        crate::println!(":: {}", content);
    }
    if let Some(loc) = info.location() {
        crate::println!("at {}", loc);
    }
    loop {}
}
