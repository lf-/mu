.globl SUPERVISOR_VECTORS
.align 4
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
.align 4
.option push
.option norvc
MACHINE_VECTORS:
    // we use direct mode for machine exceptions because there are fewer
    // we seem to have to manage mtime/mtimecmp in machine mode
    m_exc_vec: j m_exc
.option pop

.equ MTIME_ADDR, 0x2000000 + 0xbff8;


m_exc:
    // atomic swap mscratch with a0
    csrrw a0, mscratch, a0

    // a0 now points to a TimerIsrData structure
    sd a1, 0(a0)
    sd a2, 8(a0)
    csrr a1, mcause
    // if it is an exception go to spin9000
    bgtz a1, spin9000
    // eat the exc bit
    slli a1, a1, 1
    srli a1, a1, 1

    li a2, 7 // machine timer interrupt
    // if it's not a timer interrupt, jump to a spin loop (unexpected)
    // i.e. in case of exceptions as well
    bne a1, a2, spin9000

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
    // lol supervisor timer interrupts are unsupported by qemu so we have to use
    // software interrupts instead.
    li a1, 1 << 1 // supervisor software interrupt
    csrs sip, a1

    ld a1, 0(a0)
    ld a2, 8(a0)
    csrrw a0, mscratch, a0
    mret
    spin9000: j spin9000