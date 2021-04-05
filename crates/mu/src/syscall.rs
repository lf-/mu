//! system calls

use mu_shared::SyscallNum;

unsafe fn syscall2(num: SyscallNum, a1: usize, a2: usize) -> usize {
    let ret: usize;
    asm!("ecall",
        inout("a0") num as usize => ret,
        in("a1") a1,
        in("a2") a2);
    ret
}

pub fn log(msg: &str) {
    unsafe { syscall2(SyscallNum::LogMessage, msg.len(), msg.as_ptr() as usize) };
}
