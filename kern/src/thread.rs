use riscv::{addr::MAX_THREADS, arch::Mutex};

use crate::tframe::TrapFrame;

static THREADS: Mutex<[Option<Thread>; MAX_THREADS]> =
    Mutex::new([const { Option::<Thread>::None }; MAX_THREADS]);

unsafe impl Send for Thread {}

struct Thread {
    /// Trap frame to reenter this thread
    tframe: TrapFrame,
}
