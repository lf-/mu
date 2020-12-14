# μ

an attempt at a microkernel

## development

the kernel sources are in `kern`. we provide a bad makefile with hardcoded paths
to self-built llvm tools and qemu in there, which may or may not be of use.

it supports:

```
make qemu         # builds and runs the kernel in qemu
make qemu-gdb     # builds and runs the kernel in qemu, breaking into a gdb stub
make gdb          # connects to the gdb server exposed by qemu
```