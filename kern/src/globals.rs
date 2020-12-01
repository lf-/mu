use core::sync::atomic::*;

use crate::addr;

#[no_mangle]
// do not change this size without also changing it in init.s
pub static STACKS: [usize; 8192 * addr::MAX_CPUS] = [0usize; 8192 * addr::MAX_CPUS];

pub static PANICKED: AtomicBool = AtomicBool::new(false);
pub static PANIC_CHECKIN: AtomicUsize = AtomicUsize::new(0);
pub static NUM_CPUS: AtomicUsize = AtomicUsize::new(0);
