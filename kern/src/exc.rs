//! kernel exception entry points. This includes non-boot entries to the kernel
//! from userspace for example
//!
//! This module also includes the exit to userspace.

use riscv::{
    addr::TRAP_DATA,
    arch::{PhysMem, Satp},
    paging::{PageSize, PhysAccess, PteAttrs},
};

use crate::tframe::{TrapFrame, TrapHandler};

const _ASSERT_K_ENTRY_IS_RIGHT_TYPE: TrapHandler = k_entry;

#[no_mangle]
pub unsafe extern "C" fn k_entry(tf: *mut TrapFrame) -> ! {
    panic!("Returned from userspace");
}

extern "C" {
    fn k_enter_userspace(tf: *mut TrapFrame) -> !;
}

pub unsafe fn enter_userspace(tf: TrapFrame) -> ! {
    let pt = Satp::current().as_pagetable().expect("paging is enabled");
    let _ = pt.virt_unmap_one(TRAP_DATA);
    let pa = PhysMem::alloc().expect("OOM");
    pt.virt_map_one(pa, TRAP_DATA, PageSize::Page4k, PteAttrs::R | PteAttrs::W)
        .expect("map in orig");
    let target_pt = tf.new_satp.as_pagetable().expect("invalid pt");
    target_pt
        .virt_map_one(pa, TRAP_DATA, PageSize::Page4k, PteAttrs::R | PteAttrs::W)
        .expect("map in new");

    let ptr = TRAP_DATA.as_mut_ptr::<TrapFrame>();
    ptr.write(tf);

    k_enter_userspace(ptr)
}
