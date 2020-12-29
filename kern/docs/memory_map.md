# Memory Map

## Boot process

Boot code is initially identity mapped at the start of memory. It then needs
to load the kernel at the start of the kernel half of address space.

`prr` loads the device tree passed from the machine mode initialization code and
finds the initrd. It then loads the kernel from that initrd and passes control
with a reference to the initrd so the kernel can subsequently load `mu`, the
privileged init process.

## Virtual memory map in kernel land

- `0x00_0000_0000` start of memory, this entire section belongs to userspace

--------------------

- `0x40_0000_0000` boundary of kernel land and userspace
- `0x60_0000_0000` start of identity map of physical memory
- `0x80_0000_0000` end of virtual space