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

// don't handle exceptions
s_exc:
    // load the cause into a0
    // note: this is literally just for debugging. FIXME
    csrr a0, scause
    spin: j spin

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
    e_ill_insn: j e_ill_insn
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
    // 16+ are unimplemented
m_sw_tab:
    j m_InterruptHart
    j m_ClearTimerInt
.option pop

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