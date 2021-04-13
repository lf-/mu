#![no_std]
//! Shared constants and functions between userspace and kernel

pub type KernResult<T> = Result<T, KernErr>;

typesafe_ints::int_enum_only! (
/// System call numbers
#[derive(Debug)]
pub enum SyscallNum(usize) {
    /// `LogMessage(len: usize, message: *const u8)`
    LogMessage = 0,
}
);

typesafe_ints::int_enum_only! (
/// System call error returns
#[derive(Debug)]
pub enum KernErr(usize) {
    BadUtf8 = 0,
}
);

impl From<core::str::Utf8Error> for KernErr {
    fn from(_: core::str::Utf8Error) -> Self {
        Self::BadUtf8
    }
}
