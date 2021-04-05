use crate::syscall;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        let _ = core::fmt::Write::write_fmt(
            &mut $crate::print::Writer,
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
        let writer = &mut $crate::print::Writer;
        let _ = core::fmt::Write::write_fmt(
            writer,
            format_args!($($arg)*)
        );
        let _ = core::fmt::Write::write_str(writer, "\n");
    }}
}

/// writes characters to the system log device
pub struct Writer;

impl log::Log for Writer {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if let Some(mp) = record.module_path() {
            println!("{} [{}]: {}", record.level(), mp, record.args());
        } else {
            println!("{}: {}", record.level(), record.args());
        }
    }

    fn flush(&self) {
        // no op
    }
}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        syscall::log(s);
        Ok(())
    }
}
