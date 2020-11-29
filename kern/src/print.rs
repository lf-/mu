#![allow(dead_code)]
//! Support for the National Semiconductor NS16550A serial port
//!
//! Good documentation for it is available at
//! <https://archive.org/details/bitsavers_nationaldamunicationsElementsDataBook_19316911/page/n155/mode/2up>

use core::ptr;
use core::slice;

use bitvec::prelude::*;

use crate::arch::Mutex;

static SERIAL_PORT: Mutex<Option<Serial>> = Mutex::new_nopreempt(None);
pub static PRINT_LOCK: Mutex<()> = Mutex::new_nopreempt(());

/// Receiver Buffer Register
const REG_RBR: isize = 0x00;
/// Transmitter Holding Register
const REG_THR: isize = 0x00;
/// Interrupts ENable Register
const REG_IER: isize = 0x01;
/// Interrupt Information Register
const REG_IIR: isize = 0x01;
/// FIFO Control Register
const REG_FCR: isize = 0x02;
/// Line Control Register
const REG_LCR: isize = 0x03;
/// Modem Control Register
const REG_MCR: isize = 0x04;
/// Line Status Register
const REG_LSR: isize = 0x05;
/// Modem Status Register
const REG_MSR: isize = 0x05;

/// A 16550A serial port
pub struct Serial {
    base: *mut u8,
}

// Baud rates available for the serial port
#[repr(u16)]
pub enum Baudrate {
    B9600 = 12,
    B38400 = 3,
}

impl Serial {
    /// Creates a new serial interface
    ///
    /// Assumes the pointer you gave is both not already managed by a serial port
    /// (or if it is, it is never used for that again) and actually points to a
    /// serial port. It is safe to construct and throw away a serial port if it
    /// is not used.
    pub unsafe fn new(base: *mut ()) -> Serial {
        Serial {
            base: base as *mut u8,
        }
    }

    /// Initializes the serial port, setting the baud rate
    pub fn init(&mut self, baudrate: Baudrate) {
        unsafe {
            // first get the lcr address
            let lcr = self.base.offset(REG_LCR);

            // we want to configure to baudrate at 8n1
            let mut new_lcr = bitarr![Lsb0, u8; 0; 8];
            new_lcr[0..=1].store(0b11u8); // 8 bits per char
            new_lcr.set(2, false); // 0 => one stop bit
            new_lcr[3..=5].store(0b00u8); // no parity
            new_lcr.set(6, false); // no break control
            new_lcr.set(7, true); // enable divisor latch access

            ptr::write_volatile(lcr, new_lcr.load::<u8>());
            // we now have divisor latches enabled.
            let divisor = self.base;
            slice::from_raw_parts_mut(divisor, 2).copy_from_slice(&(baudrate as u16).to_le_bytes());

            new_lcr.set(7, false); // disable divisor latch access to be able to write to the UART
            ptr::write_volatile(lcr, new_lcr.load::<u8>());

            // configure the interrupts
            let ier = self.base.offset(REG_IER);
            // turn them all off. TODO: maybe use a buffering mechanism for serial?
            ptr::write_volatile(ier, 0);

            // configure the FIFOs
            let fcr = self.base.offset(REG_FCR);
            let mut new_fcr = bitarr![Lsb0, u8; 0; 8];
            new_fcr.set(0, true); // -> FIFO enable
            new_fcr.set(1, true); // -> receive FIFO reset
            new_fcr.set(2, true); // -> transmit FIFO reset
            new_fcr.set(3, true); // -> DMA mode 1 (continuous transfer)
            ptr::write_volatile(fcr, new_fcr.load::<u8>());
        }
    }

    /// Transmits a bunch of bytes synchronously
    // TODO: do this faster with interrupts
    pub fn transmit(&self, c: &[u8]) {
        unsafe {
            let lsr = self.base.offset(REG_LSR);
            let mut lsr_val = bitarr![Lsb0, u8; 0; 8];
            // it apparently has a 16 byte FIFO
            for chunk in c.chunks(16) {
                // wait for Transmitter Holding Register Empty (bit 5 of LSR)
                while {
                    lsr_val.store(ptr::read_volatile(lsr));
                    !lsr_val[5]
                } {}
                // write out the buffer's worth to THR
                for &ch in chunk {
                    self.base.write_volatile(ch);
                }
            }
        }
    }
}

/// Initialize the serial port
pub fn init() {
    // TODO: there is a bug here: we need to disable interrupts while we have this lock held
    // it will work fine until we enable them........
    let mut guard = SERIAL_PORT.lock();
    let mut serial = unsafe { Serial::new(crate::addr::UART0 as *mut _) };
    serial.init(Baudrate::B38400);
    *guard = Some(serial);
}

pub struct SerialWriter;
impl core::fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let mut guard = SERIAL_PORT.lock();
        if let Some(port) = &mut *guard {
            port.transmit(s.as_bytes());
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        let _guard = $crate::print::PRINT_LOCK.lock();
        let _ = core::fmt::Write::write_fmt(
            &mut $crate::print::SerialWriter,
            format_args!($($arg)*)
        );
    }};
}

#[macro_export]
macro_rules! println {
    () => {{
        print!("\n");
    }};
    ($($arg:tt)*) => {{
        let _guard = $crate::print::PRINT_LOCK.lock();
        let writer = &mut $crate::print::SerialWriter;
        let _ = core::fmt::Write::write_fmt(
            writer,
            format_args!($($arg)*)
        );
        let _ = core::fmt::Write::write_str(writer, "\n");
    }}
}
