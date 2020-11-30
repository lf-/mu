/// Machine traps. Mostly timers.
pub unsafe extern "C" fn machine_trap() {
    // on timers

    asm!("mret");
    core::hint::unreachable_unchecked();
}
