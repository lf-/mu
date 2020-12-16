#![no_std]
#![no_main]
#![feature(asm, panic_info_message)]
#![feature(const_fn)]

mod arch;
#[macro_use]
mod print;
mod addr;
mod globals;
mod interrupts;
mod isr;
mod task;

use core::fmt::Write;
use core::{ffi::c_void, panic::PanicInfo, sync::atomic::Ordering};

use crate::arch::*;
use crate::globals::*;
use addr::MAX_CPUS;
use riscv_paging::{virt_map, PageTable, PhysAccess, PhysAddr, PteAttrs, VirtAddr};

use bitvec::prelude::*;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    // We implement panicking across cores by having the panicking core
    // send machine software interrupts to all the other cores, which
    // will then, in the handler, detect that PANICKED is true, and halt
    // themselves, incrementing PANIC_CHECKIN

    PANICKED.store(true, Ordering::SeqCst);
    PANIC_CHECKIN.fetch_add(1, Ordering::SeqCst);
    let num_cpus = NUM_CPUS.load(Ordering::SeqCst);
    let my_core_id = core_id();

    for hartid in 0..MAX_CPUS {
        // don't cross-processor interrupt ourselves
        if hartid == my_core_id {
            continue;
        }
        machinecall(MachineCall::InterruptHart, hartid);
    }

    while PANIC_CHECKIN.load(Ordering::SeqCst) != num_cpus {
        core::hint::spin_loop();
    }

    struct PanicSerial(print::Serial);
    impl core::fmt::Write for PanicSerial {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            self.0.transmit(s.as_bytes());
            Ok(())
        }
    }

    // we know all the cores are halted, so we can violate aliasing on the
    // serial driver
    let serial = unsafe {
        let mut serial = print::Serial::new(addr::UART0 as *mut _);
        serial.init(print::Baudrate::B38400);
        serial
    };
    let mut serial = PanicSerial(serial);

    let _ = write!(serial, "!!! Panic !!! At the core {}\n", my_core_id);
    if let Some(msg) = info.message() {
        let _ = write!(serial, ":: {}\n", msg);
    }
    if let Some(loc) = info.location() {
        let _ = write!(serial, "@ {}\n", loc);
    }

    loop {}
}

extern "C" {
    static SUPERVISOR_VECTORS: c_void;
    #[link_name = "stext"]
    static SEC_TEXT: c_void;
    #[link_name = "etext"]
    static SEC_ETEXT: c_void;
    #[link_name = "srodata"]
    static SEC_RODATA: c_void;
    #[link_name = "erodata"]
    static SEC_ERODATA: c_void;
    #[link_name = "srwdata"]
    static SEC_SRWDATA: c_void;
    #[link_name = "end"]
    static SEC_END: c_void;
}

#[no_mangle]
unsafe extern "C" fn startup() {
    // this function will be hit by as many harts as we have, at once
    // thus, we will spinloop the ones we don't have work for yet
    let core_id = m_core_id();
    if core_id != 0 {
        loop {}
    }

    // § 3.1.6 RISC-V privileged ISA
    let mut new_mstatus = get_mstatus();
    // set MPP (previous mode) to supervisor, privilege level 1
    new_mstatus.set_m_prev_pl(ArchPrivilegeLevel::Supervisor);

    set_mstatus(new_mstatus);

    // turn off paging
    set_satp(Satp(0));

    // set the exception return address
    set_mepc(kern_main);

    // set the delegated exceptions and interrupts to be all of the base arch ones
    // ... except env calls from S-mode
    set_medeleg(0xffff & !(1 << 9));
    set_mideleg(0xffff);

    // ensure SEIE, STIE, SSIE are on
    let mut sie = get_sie();
    let view = sie.view_bits_mut::<Lsb0>();
    // we don't take interrupts in kernel mode
    view.set(SIE_SEIE, false);
    view.set(SIE_STIE, false);
    view.set(SIE_SSIE, false);
    set_sie(sie);

    // the 1 enables vectored mode
    set_stvec(&SUPERVISOR_VECTORS as *const c_void as u64 | 1);

    interrupts::init_timers();

    // put our hart id into the thread pointer
    set_core_id(core_id);
    NUM_CPUS.fetch_add(1, Ordering::SeqCst);

    asm!("mret");
    core::hint::unreachable_unchecked();
}

unsafe extern "C" fn kern_main() -> ! {
    let core_id = core_id();

    let endaddr = &SEC_END as *const _ as usize;
    if core_id == 0 {
        crate::print::init();
        for page in (endaddr..addr::PHYSMEM + addr::PHYSMEM_LEN).step_by(4096) {
            //println!("wtf {:x}", page);
            PhysMem::free(PhysAddr::new(page))
        }
    }
    println!("hello world from core {}!", core_id);
    // we will hit this with one core!
    // println!("hello world from risc-v!!");
    get_sstatus();
    get_sip();

    let root_pt = PageTable::<PhysMem>::alloc().expect("root pagetable alloc failed");
    let satp = Satp::new(&root_pt, 0, TranslationMode::Sv39);

    // sets the running task so we can hit exceptions properly
    let task = task::FAULT_TASKS.get(core_id);
    task.hart_id = core_id;
    // crash stack
    task.kernel_sp = EXCEPTION_STACKS.get(core_id).as_mut_ptr() as *mut _;
    task.kernel_satp = satp;
    set_running_task(task);

    let textaddr = &SEC_TEXT as *const _ as usize;
    let etextaddr = &SEC_ETEXT as *const _ as usize;
    virt_map(
        root_pt,
        PhysAddr::new(textaddr),
        VirtAddr(textaddr),
        etextaddr.checked_sub(textaddr).unwrap(),
        PteAttrs::R | PteAttrs::X,
    )
    .unwrap();

    let rodataaddr = &SEC_RODATA as *const _ as usize;
    let erodataaddr = &SEC_ERODATA as *const _ as usize;
    virt_map(
        root_pt,
        PhysAddr::new(rodataaddr),
        VirtAddr(rodataaddr),
        erodataaddr.checked_sub(rodataaddr).unwrap(),
        PteAttrs::R.into(),
    )
    .unwrap();

    let srwdataaddr = &SEC_SRWDATA as *const _ as usize;
    virt_map(
        root_pt,
        PhysAddr::new(srwdataaddr),
        VirtAddr(srwdataaddr),
        endaddr.checked_sub(srwdataaddr).unwrap(),
        PteAttrs::R | PteAttrs::W,
    )
    .unwrap();

    virt_map(
        root_pt,
        PhysAddr::new(0),
        VirtAddr(addr::PHYSMEM_MAP),
        addr::PHYSMEM_LEN + addr::PHYSMEM,
        PteAttrs::R | PteAttrs::W,
    )
    .unwrap();

    virt_map(
        root_pt,
        PhysAddr::new(addr::UART0),
        VirtAddr(addr::UART0),
        addr::UART0LEN,
        PteAttrs::R | PteAttrs::W,
    )
    .unwrap();

    // TODO: this is probably actually not usable from S-mode so we can probably
    // not map it
    virt_map(
        root_pt,
        PhysAddr::new(addr::CLINT),
        VirtAddr(addr::CLINT),
        addr::CLINT_LEN,
        PteAttrs::R | PteAttrs::W,
    )
    .unwrap();

    set_satp(satp);
    (0 as *mut u8).write_volatile(0);

    panic!("test test test!!");

    loop {}
}
