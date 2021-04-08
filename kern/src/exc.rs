//! kernel exception entry points. This includes non-boot entries to the kernel
//! from userspace for example
//!
//! This module also includes the exit to userspace.

use core::mem;

use riscv::{
    addr::TRAP_DATA,
    arch::{set_stvec, PhysMem, Satp},
    paging::{PageSize, PhysAccess, PteAttrs},
};

use crate::tframe::{TrapFrame, TrapHandler, TRAP_FRAMES};

const _ASSERT_K_ENTRY_IS_RIGHT_TYPE: TrapHandler = k_entry;

#[no_mangle]
pub unsafe extern "C" fn k_entry(tf: *mut TrapFrame) -> ! {
    panic!("Returned from userspace");
}

extern "C" {
    fn k_enter_userspace(tf: *mut TrapFrame) -> !;
    // we don't need to model anything about this function other than it should
    // never be called from rust
    fn k_return_from_userspace();
}

pub unsafe fn enter_userspace(tf: TrapFrame) -> ! {
    let global_tf = TRAP_FRAMES.get();
    *global_tf = tf;
    set_stvec(k_return_from_userspace as _);

    k_enter_userspace(global_tf)
}
