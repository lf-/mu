#![allow(non_snake_case)]
//! kernel exception entry points. This includes non-boot entries to the kernel
//! from userspace for example
//!
//! This module also includes the exit to userspace.

use core::convert::TryInto;
use mu_shared::{KernResult, SyscallNum};
use riscv::arch::{get_scause, get_sstatus, set_sstatus, set_stvec, ExceptionType};
use riscv::paging::Addr;

#[allow(dead_code)]
mod Reg {
    pub const A0: usize = 9; // x10
    pub const A1: usize = 10; // x11
    pub const A2: usize = 11; // x12
    pub const A3: usize = 12; // x13
    pub const A4: usize = 13; // x14
    pub const A5: usize = 14; // x15
    pub const A6: usize = 15; // x16
    pub const A7: usize = 16; // x17
}

use crate::tframe::{TrapFrame, TrapHandler, TRAP_FRAMES};

const _ASSERT_K_ENTRY_IS_RIGHT_TYPE: TrapHandler = k_entry;

/// Copies some bytes from userspace
///
/// Safety:
/// The slice `into` can't alias `v_user`, which is illegal anyway...
unsafe fn copy_from_user(into: &mut [u8], v_user: *const u8, len: usize) -> usize {
    let sstatus_orig = get_sstatus();
    let mut sstatus = sstatus_orig;
    sstatus.set_sum(true);
    set_sstatus(sstatus);

    let written = into.len().min(len);
    v_user.copy_to_nonoverlapping(into.as_mut_ptr(), written);
    set_sstatus(sstatus_orig);
    written
}

/// `LogMessage(len: usize, message: *const u8)`
unsafe fn sc_LogMessage(len: usize, message: *const u8) -> KernResult<()> {
    let mut buf = [0; 255];
    let written = copy_from_user(&mut buf, message, len);
    let s = core::str::from_utf8(&buf[..written])?;
    log::info!("[u] {}", s);
    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn k_entry(tf: *mut TrapFrame) -> ! {
    let tf = &mut *tf;
    match get_scause() {
        ExceptionType::EnvCallU => {}
        e => panic!("exceptiowo in userspace {:?}", e),
    }

    log::info!("user pc: {:?}", tf.user_pc);
    tf.user_pc = tf.user_pc.offset(4);

    let arg0 = tf.regs[Reg::A1];
    let arg1 = tf.regs[Reg::A2];

    let res = match tf.regs[Reg::A0].try_into() {
        Ok(SyscallNum::LogMessage) => sc_LogMessage(arg0, arg1 as *const _),
        Err(v) => panic!("unknown syscall {}", v),
    };

    tf.regs[Reg::A0] = matches!(res, Ok(_)) as usize;
    tf.regs[Reg::A1] = match res {
        Ok(_) => 0,
        Err(v) => v as usize,
    };

    enter_userspace(tf);
}

extern "C" {
    fn k_enter_userspace(tf: *mut TrapFrame) -> !;
    // we don't need to model anything about this function other than it should
    // never be called from rust
    fn k_return_from_userspace();
}

pub unsafe fn enter_userspace(tf: &TrapFrame) -> ! {
    let global_tf = TRAP_FRAMES.get();
    *global_tf = tf.clone();
    set_stvec(k_return_from_userspace as _);

    k_enter_userspace(global_tf)
}
