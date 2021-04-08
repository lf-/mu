#![no_std]
#![feature(asm, panic_info_message)]
#![feature(const_fn)]

pub mod addr;
pub mod arch;
pub mod globals;
pub mod print;

use paging::VirtAddr;
pub use riscv_paging as paging;

use core::fmt::Write;
use core::panic::PanicInfo;
use core::sync::atomic::*;

use crate::addr::*;
use crate::arch::*;

pub static PANICKED: AtomicBool = AtomicBool::new(false);
pub static PANIC_CHECKIN: AtomicUsize = AtomicUsize::new(0);
pub static NUM_CPUS: AtomicUsize = AtomicUsize::new(0);

pub type KernEntry = extern "C" fn(core_id: &KernelEntryParams) -> !;

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

    freeze_hart()
}

#[repr(C)]
pub struct KernelEntryParams {
    pub core_id: usize,
    pub init_sp: VirtAddr,
    pub init_entrypoint: VirtAddr,
    pub stack_pointer: VirtAddr,
    /// number of cpus in the system.
    // TODO(smp): this probably needs to be redesigned along with the boot
    // process, to enable SMP (one core brings up the system then brings up the
    // other cores from halt, bypassing shoo??). but we need it for now for
    // panic handlers
    pub num_cpus: usize,
}
