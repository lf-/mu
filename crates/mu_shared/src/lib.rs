#![no_std]
//! Shared constants and functions between userspace and kernel

/// System call numbers
#[derive(Debug)]
#[repr(usize)]
pub enum SyscallNum {
    /// `LogMessage(len: usize, message: *const u8)`
    LogMessage,
}
