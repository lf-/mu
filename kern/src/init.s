// kernel entry point
// largely nicked from https://github.com/mit-pdos/xv6-riscv/blob/riscv/kernel/entry.S
.data
.globl STACKS
.text
.section .text.first
.globl start
.globl _entry
_entry:
    // we arrive here, in machine mode, once qemu jumps to the start of memory

    // set up a stack
    la sp, STACKS
    li a0, 16384     // use 16k stacks
    csrr a1, mhartid // get the hart (core) id

    // we want to get the pointer to the top of the (descending) stack
    // thus we want 8k * (hartid + 1)
    addi a1, a1, 1
    mul a0, a0, a1
    add sp, sp, a0

    call startup
spin:
    j spin // startup() will not return
.text
