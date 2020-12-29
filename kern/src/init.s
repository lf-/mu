// kernel entry point
// largely nicked from https://github.com/mit-pdos/xv6-riscv/blob/riscv/kernel/entry.S
.data
.globl STACKS
.text
.section .text.first
.globl start
.globl _entry
// _entry(mhartid: usize, dtb: *const u8)
_entry:
    // we arrive here, in machine mode, once qemu jumps to the start of memory

    // set up a stack
    la sp, STACKS
    li t0, 16384     // use 16k stacks

    // we want to get the pointer to the top of the (descending) stack
    // thus we want 16k * (hartid + 1)
    addi t1, a0, 1
    mul t0, t0, t1
    add sp, sp, t0

    call startup
spin:
    j spin // startup() will not return
.text
