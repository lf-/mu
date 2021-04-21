.section trampolines
// supervisor mode vectors for running in user mode
.align 8 // 2^8 = 256; 4 * XLEN

// jump to rust
.globl k_return_from_userspace
k_return_from_userspace:
    // get a pointer to the trap frame
    csrrw x1, sscratch, x1
    // save whatever regs we had on entry there
    sd x2,  8*1(x1)
    sd x3,  8*2(x1)
    sd x4,  8*3(x1)
    sd x5,  8*4(x1)
    sd x6,  8*5(x1)
    sd x7,  8*6(x1)
    sd x8,  8*7(x1)
    sd x9,  8*8(x1)
    sd x10, 8*9(x1)
    sd x11, 8*10(x1)
    sd x12, 8*11(x1)
    sd x13, 8*12(x1)
    sd x14, 8*13(x1)
    sd x15, 8*14(x1)
    sd x16, 8*15(x1)
    sd x17, 8*16(x1)
    sd x18, 8*17(x1)
    sd x19, 8*18(x1)
    sd x20, 8*19(x1)
    sd x21, 8*20(x1)
    sd x22, 8*21(x1)
    sd x23, 8*22(x1)
    sd x24, 8*23(x1)
    sd x25, 8*24(x1)
    sd x26, 8*25(x1)
    sd x27, 8*26(x1)
    sd x28, 8*27(x1)
    sd x29, 8*28(x1)
    sd x30, 8*29(x1)
    sd x31, 8*30(x1)

    // copy the tf before swapping it back
    mv x2, x1
    csrrw x1, sscratch, x1
    sd x1, 8*0(x2)
    // first arg to entry is the pointer to our CPU's TrapFrame with regs
    mv a0, x2

    // .user_pc
    csrr t0, sepc
    sd t0, 8*31+4*8(a0)

    // .target_fn
    ld t0, 8*31(a0)

    // now try to get a coherent set of regs for kernel entry
    ld sp, 8*31+8*2(a0)
    ld tp, 8*31+8*1(a0)

    // we should have good kernel regs now
    jr t0

.globl k_enter_userspace
// unsafe extern "C" fn k_enter_userspace(*mut TrapFrame) -> !
k_enter_userspace:
    csrw sscratch, a0

    // mask off spp to get spp=0 => enter user mode
    li t0, (1 << 8)
    csrc sstatus, t0

    ld t0, 8*31+4*8(a0)
    csrw sepc, t0

    // trap frame must still be mapped here
    ld t0, 8*31+3*8(a0)
    csrw satp, t0
    sfence.vma

    ld x1,  8*0 (a0)
    ld x2,  8*1 (a0)
    ld x3,  8*2 (a0)
    ld x4,  8*3 (a0)
    ld x5,  8*4 (a0)
    ld x6,  8*5 (a0)
    ld x7,  8*6 (a0)
    ld x8,  8*7 (a0)
    ld x9,  8*8 (a0)
    // a0 here
    ld x11, 8*10(a0)
    ld x12, 8*11(a0)
    ld x13, 8*12(a0)
    ld x14, 8*13(a0)
    ld x15, 8*14(a0)
    ld x16, 8*15(a0)
    ld x17, 8*16(a0)
    ld x18, 8*17(a0)
    ld x19, 8*18(a0)
    ld x20, 8*19(a0)
    ld x21, 8*20(a0)
    ld x22, 8*21(a0)
    ld x23, 8*22(a0)
    ld x24, 8*23(a0)
    ld x25, 8*24(a0)
    ld x26, 8*25(a0)
    ld x27, 8*26(a0)
    ld x28, 8*27(a0)
    ld x29, 8*28(a0)
    ld x30, 8*29(a0)
    ld x31, 8*30(a0)
    ld x10, 8*9 (a0)
    // regs are all back to normal

    sret