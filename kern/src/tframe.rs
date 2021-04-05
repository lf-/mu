//! Implementation of a trap frame

use core::fmt::Display;
use core::iter;
use core::{cell::UnsafeCell, ffi::c_void, ptr};

use riscv::{
    arch::Satp,
    globals::{HasEmpty, PerHartMut},
};

pub type TrapHandler = unsafe extern "C" fn(*mut TrapFrame) -> !;

static TRAP_FRAMES: PerHartMut<TrapFrame> = PerHartMut::new();

/// The contents of a trap frame. This is used when entering the kernel from an
/// exception or other reason.
#[derive(Clone, Debug)]
#[repr(C)]
pub struct TrapFrame {
    pub regs: [usize; 31],
    pub target_fn: TrapHandler,
    pub hart_id: usize,
    pub kernel_sp: *mut c_void,
    pub new_satp: Satp,
}

impl HasEmpty for TrapFrame {
    const EMPTY: core::cell::UnsafeCell<Self> = UnsafeCell::new(TrapFrame {
        regs: [0; 31],
        target_fn: unreachable_default_fn,
        hart_id: !0,
        kernel_sp: ptr::null_mut(),
        new_satp: Satp::DISABLED,
    });
}

unsafe extern "C" fn unreachable_default_fn(_: *mut TrapFrame) -> ! {
    panic!("should not have jumped to a default trap frame target function");
}

/// Helper struct that `impl`s [Display] for the registers of a TrapFrame
pub struct FormatRegs<'a>(&'a TrapFrame);

fn fmt_reg(reg: usize, v: usize, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match reg {
        0 => write!(f, "x0 "),
        1 => write!(f, "ra "),
        2 => write!(f, "sp "),
        3 => write!(f, "gp "),
        4 => write!(f, "tp "),
        5 => write!(f, "t0 "),
        6 => write!(f, "t1 "),
        7 => write!(f, "t2 "),
        8 => write!(f, "s0 "),
        9 => write!(f, "s1 "),
        10 => write!(f, "a0 "),
        11 => write!(f, "a1 "),
        12 => write!(f, "a2 "),
        13 => write!(f, "a3 "),
        14 => write!(f, "a4 "),
        15 => write!(f, "a5 "),
        16 => write!(f, "a6 "),
        17 => write!(f, "a7 "),
        18 => write!(f, "s2 "),
        19 => write!(f, "s3 "),
        20 => write!(f, "s4 "),
        21 => write!(f, "s5 "),
        22 => write!(f, "s6 "),
        23 => write!(f, "s7 "),
        24 => write!(f, "s8 "),
        25 => write!(f, "s9 "),
        26 => write!(f, "s10"),
        27 => write!(f, "s11"),
        28 => write!(f, "t3 "),
        29 => write!(f, "t4 "),
        30 => write!(f, "t5 "),
        31 => write!(f, "t6 "),
        _ => unreachable!(),
    }?;
    write!(f, ": {:016x}\t", v)
}

impl Display for FormatRegs<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for (idx, reg) in iter::once(0usize)
            .chain(self.0.regs.iter().copied())
            .enumerate()
        {
            fmt_reg(idx, reg, f)?;
            if (idx + 1) % 4 == 0 && idx != 0 {
                write!(f, "\n")?;
            }
        }
        Ok(())
    }
}

impl TrapFrame {
    pub fn display_regs(&self) -> FormatRegs {
        FormatRegs(&self)
    }
}
