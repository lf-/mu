.globl SUPERVISOR_VECTORS
.align 8 // 4 * XLEN
// turn off compact instructions to get our vectors to all be 4 bytes
.option push
.option norvc
SUPERVISOR_VECTORS:
    // exception
    s_exc_vec: j s_exc
    // supervisor mode sw interrupt
    s_isr_sw_vec: j s_isr_sw
    reserved2: j reserved2
    reserved3: j reserved3
    reserved4: j reserved4
    // supervisor mode timer interrupt
    s_isr_timer_vec: j s_isr_timer
    reserved6: j reserved6
    reserved7: j reserved7
    reserved8: j reserved8
    // supervisor mode external interrupt
    s_isr_ext_vec: j s_isr_ext
    reserved10: j reserved10
    reserved11: j reserved11
    reserved12: j reserved12
    reserved13: j reserved13
    reserved14: j reserved14
    reserved15: j reserved15
    reserved16: j reserved16
.option pop

// jump to rust and panic
// this happens only in case of kernel crashes
s_panic:
    // get a pointer to a Task struct of some kind in x1
    csrrw x1, sscratch, x1
    // turn off paging, lose our page table but eh
    csrw satp, x0
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

    // copy it before swapping it back
    mv x2, x1
    csrrw x1, sscratch, x1
    sd x1, 8*0(x2)
    // first arg to entry is the pointer to our CPU's Task with regs
    mv a0, x2

    // now try to get a coherent set of regs for kernel entry
    ld sp, 8*31+8*2(a0)
    ld tp, 8*31+8*1(a0)

    csrr a1, scause
    csrr a2, sepc
    csrr a3, stval

    // we should have good kernel regs now
    call entry_panic

    // this should be noreturn
    infloop2: j infloop2

    csrr x1, sscratch
    ld x2,  8*1(x1)
    ld x3,  8*2(x1)
    ld x4,  8*3(x1)
    ld x5,  8*4(x1)
    ld x6,  8*5(x1)
    ld x7,  8*6(x1)
    ld x8,  8*7(x1)
    ld x9,  8*8(x1)
    ld x10, 8*9(x1)
    ld x11, 8*10(x1)
    ld x12, 8*11(x1)
    ld x13, 8*12(x1)
    ld x14, 8*13(x1)
    ld x15, 8*14(x1)
    ld x16, 8*15(x1)
    ld x17, 8*16(x1)
    ld x18, 8*17(x1)
    ld x19, 8*18(x1)
    ld x20, 8*19(x1)
    ld x21, 8*20(x1)
    ld x22, 8*21(x1)
    ld x23, 8*22(x1)
    ld x24, 8*23(x1)
    ld x25, 8*24(x1)
    ld x26, 8*25(x1)
    ld x27, 8*26(x1)
    ld x28, 8*27(x1)
    ld x29, 8*28(x1)
    ld x30, 8*29(x1)
    ld x31, 8*30(x1)
    ld x1, 8*0(x1)
    // regs are all back to normal

    sret

// don't handle exceptions
s_exc:
    // load the cause into a0
    // note: this is literally just for debugging. FIXME
    j s_panic

// external interrupt
s_isr_ext:
    // load the cause into a0
    // note: this is literally just for debugging. FIXME
    csrr a0, scause
    spin1: j spin1

// software interrupt
s_isr_sw:
    // load the cause into a0
    // note: this is literally just for debugging. FIXME
    csrr a0, scause
    spin2: j spin2

// timer interrupt
s_isr_timer:
    // load the cause into a0
    // note: this is literally just for debugging. FIXME
    csrr a0, scause
    spin3: j spin3

.globl MACHINE_VECTORS
.align 8 // 2^8 = 256; 4 * XLEN
.option push
.option norvc
MACHINE_VECTORS:
    // exception
    m_exc_vec: j m_exc
    // should not get supervisor sw interrupts in machine mode!
    m_s_isr_sw_vec: j m_s_isr_sw_vec
    m_reserved2: j m_reserved2
    // we should never get machine software interrupts as they are unused
    m_isr_sw_vec: j m_isr_sw_vec
    m_reserved4: j m_reserved4
    // should not get supervisor timer interrupts in machine mode
    m_s_isr_timer_vec: j m_s_isr_timer_vec
    m_reserved6: j m_reserved6
    m_isr_timer_vec: j m_isr_timer
    m_reserved8: j m_reserved8
    // should not get supervisor timer interrupts in machine mode
    m_s_isr_ext_vec: j m_s_isr_ext_vec
    m_reserved10: j m_reserved10
    // not yet implemented
    m_isr_ext_vec: j m_isr_ext_vec
    m_reserved12: j m_reserved12
    m_reserved13: j m_reserved13
    m_reserved14: j m_reserved14
    m_reserved15: j m_reserved15
    m_reserved16: j m_reserved16
.option pop

.equ CLINT_BASE, 0x2000000 
.equ MTIME_ADDR, CLINT_BASE + 0xbff8;
.equ MSIP_BASE, CLINT_BASE
.equ MAX_CPUS, 8


.option push
.option norvc
m_exc_tab:
    e_insn_addr_misalign: j e_insn_addr_misalign
    e_insn_access_fault: j e_insn_access_fault
    e_ill_insn: j m_ill_insn
    e_breakpoint: j e_breakpoint
    e_load_misalign: j e_load_misalign
    e_load_fault: j e_load_fault
    e_atomic_misalign: j e_atomic_misalign
    e_atomic_fault: j e_atomic_fault
    // supposed to be delegated
    e_envcall_u: j e_envcall_u
    e_envcall_s: j m_machinecall
    e_reserved10: j e_reserved10
    // we can hit machinecalls from m-mode as well if we're running in rust
    e_envcall_m: j m_machinecall
    e_insn_pagefault: j e_insn_pagefault
    e_load_pagefault: j e_load_pagefault
    e_reserved14: j e_reserved14
    e_atomic_pagefault: j e_atomic_pagefault
    // XXX: check your data sheet for other possible exceptions above 16. if
    // those exist, spicy UB will happen here as it will fall onto some other
    // code below.
m_sw_tab:
    j m_InterruptHart
    j m_ClearTimerInt
.option pop

// for convenience in gdb these get stuffed into regs. this is not actually
// required, the full architectural register state can be dumped in qemu
// trivially with `info registers` in the monitor.
m_ill_insn:
    csrr a0, mcause
    csrr a1, mepc
    1: j 1b

m_exc:
    csrrw a2, mscratch, a2
    sd a3, 0(a2)
    sd a4, 8(a2)
    // a3 and a4 are scratch
    li a3, 15
    csrr a4, mcause
    bgtu a4, a3, m_unimp_exc

    // get pointer into jump table
    la a3, m_exc_tab
    slli a4, a4, 2
    add a4, a4, a3
    jr a4

m_unimp_exc: j m_unimp_exc

m_InterruptHart:
    li a3, MAX_CPUS
    // if requested msip > max cpus then we will infloop
    bgtu a1, a3, bad_machinecall
    // construct a pointer to MSIP_BASE[hartid]
    li a3, MSIP_BASE
    slli a1, a1, 2
    add a1, a1, a3

    // write 1 to the MSIP_BASE[hartid]
    li a4, 1
    sw a4, 0(a3)
    j m_machinecall_leave

m_ClearTimerInt:
    // STIP
    li a3, 1 << 5
    // clear STIP
    csrc mip, a3
    j m_machinecall_leave

// software exception i.e. machinecall
// we use the ABI of call num in a0, arg in a1
m_machinecall:
    // max machinecall number
    li a3, 1
    bgtu a0, a3, bad_machinecall

    // load the jump table address into a0
    la a3, m_sw_tab
    // a0 *= 4
    slli a0, a0, 2
    add a3, a0, a3
    // jump to that address
    jr a3
    // code must jump back itself (no link addr)

m_machinecall_leave:
    // if it's a machinecall we will have to add 4 to mepc to resume at the right
    // address
    csrr a3, mepc
    addi a3, a3, 4
    csrw mepc, a3

    // deliberately refuse to return anything
    li a0, 0
    li a1, 0
m_exc_leave:
    ld a3, 0(a2)
    ld a4, 8(a2)
    csrrw a2, mscratch, a2
    mret

bad_machinecall: j bad_machinecall

m_isr_timer:
    // atomic swap mscratch with a0
    csrrw a0, mscratch, a0

    // a0 now points to a TimerIsrData structure
    sd a1, 0(a0)
    sd a2, 8(a0)

    // get the current time
    li a1, MTIME_ADDR
    ld a1, 0(a1)

    // get the interval
    ld a2, 24(a0)

    // get the next interrupt time
    add a2, a1, a2

    // pointer to mtimecmp
    ld a1, 16(a0)
    // store the next interrupt time
    sd a2, 0(a1)

    // next, tell software about it by setting the bit in sip
    li a1, 1 << 5 // supervisor timer interrupt
    csrs mip, a1

    ld a1, 0(a0)
    ld a2, 8(a0)
    csrrw a0, mscratch, a0
    mret