//! entry points into the kernel

use crate::arch::*;
use crate::task::Task;

use core::convert::TryFrom;

/// requests a panic
#[no_mangle]
pub unsafe extern "C" fn entry_panic(
    faulting_task: *const Task,
    scause: usize,
    sepc: usize,
    stval: usize,
) -> ! {
    let typ = ExceptionType::try_from(scause).map_err(|_| scause);
    panic!(
        "Unhandled exception in kernel mode!!\n\
        type={typ:?}\n\
        pc={sepc:x}\nfault addr={stval:x}\n\
        Regs:\n{regs}\n \
        ",
        regs = (*faulting_task).display_regs(),
        typ = typ,
        sepc = sepc,
        stval = stval,
    );
}
