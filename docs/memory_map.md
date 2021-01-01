# Memory Map

## Boot process

Boot code is initially identity mapped at the start of memory. It then needs
to load the kernel at the start of the kernel half of address space.

`shoo` loads the device tree passed from the machine mode initialization code
and finds the initrd. It then loads the kernel from that initrd and passes
control with a reference to the initrd so the kernel can subsequently load
`mu`, the privileged init process.

## Virtual memory map

- `0x0000_0000_0000_0000` start of memory, this entire section belongs to userspace
- `0x0000_003f_ffff_ffff` last userspace address

--------------------

- `0xffff_ffc0_0000_0000` first kernel address
- `0xffff_ffc0_0001_0000` first used kernel address
- `0xffff_ffe0_0000_0000` top of kernel stack
- `0xffff_ffe0_0000_0000` start of identity map of physical memory
- `0xffff_ffff_ffff_ffff` last kernel address