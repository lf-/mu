//! Machine-specific constants

#![allow(dead_code)]

use riscv_paging::VirtAddr;
// https://github.com/qemu/qemu/blob/master/hw/riscv/virt.c
// static const struct MemmapEntry {
//     hwaddr base;
//     hwaddr size;
// } virt_memmap[] = {
//     [VIRT_DEBUG] =       {        0x0,         0x100 },
//     [VIRT_MROM] =        {     0x1000,        0xf000 },
//     [VIRT_TEST] =        {   0x100000,        0x1000 },
//     [VIRT_RTC] =         {   0x101000,        0x1000 },
//     [VIRT_CLINT] =       {  0x2000000,       0x10000 },
//     [VIRT_PCIE_PIO] =    {  0x3000000,       0x10000 },
//     [VIRT_PLIC] =        {  0xc000000, VIRT_PLIC_SIZE(VIRT_CPUS_MAX * 2) },
//     [VIRT_UART0] =       { 0x10000000,         0x100 },
//     [VIRT_VIRTIO] =      { 0x10001000,        0x1000 },
//     [VIRT_FLASH] =       { 0x20000000,     0x4000000 },
//     [VIRT_PCIE_ECAM] =   { 0x30000000,    0x10000000 },
//     [VIRT_PCIE_MMIO] =   { 0x40000000,    0x40000000 },
//     [VIRT_DRAM] =        { 0x80000000,           0x0 },
// };

// note that this needs to be manually synced with vectors.s values
pub const MAX_CPUS: usize = 8;
pub const MAX_THREADS: usize = 64;

pub const UART0: usize = 0x1000_0000;
pub const UART0LEN: usize = 0x1000;
pub const CLINT: usize = 0x200_0000;
pub const CLINT_LEN: usize = 0x10000;
pub const PHYSMEM: usize = 0x8000_0000;
// 128 MiB
pub const PHYSMEM_LEN: usize = 128 * 1024 * 1024;

pub const MAX_VIRT: usize = 0xffff_ffff_ffff_ffff; // sx(0x80_0000_0000)
pub const PHYSMEM_MAP: usize = 0xffff_ffe0_0000_0000; // sx(0x60_0000_0000)

pub const TRAP_DATA: VirtAddr = VirtAddr(0xffff_ffc0_0000_1000);
pub const USERSPACE_STACK_TOP: VirtAddr = VirtAddr(0x0000_0040_0000_0000);
