//! kernel exception entry points. This includes non-boot entries to the kernel
//! from userspace for example

use crate::tframe::TrapFrame;

extern "C" fn k_entry(tf: *mut TrapFrame) {
    panic!("Returned from userspace");
}
