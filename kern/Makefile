# stolen from https://github.com/mit-pdos/xv6-riscv/blob/riscv/Makefile
QEMU = /opt/qemu/bin/qemu-system-riscv64
GDB = /opt/gdb/bin/gdb
# TODO: UP for the minute
CPUS = 3
KERNEL = target/riscv64imac-unknown-none-elf/debug/kern

QEMUOPTS = -machine virt -bios none -kernel $(KERNEL) -m 128M -smp $(CPUS) -nographic
# debug on port 1234
#QEMUOPTS += -s
#QEMUOPTS += -drive file=fs.img,if=none,format=raw,id=x0
#QEMUOPTS += -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

.PHONY: qemu cargo
cargo:
	cargo build

qemu: cargo
	$(QEMU) $(QEMUOPTS)

dbgqemu: cargo
	$(QEMU) $(QEMUOPTS) -s -S

gdb:
	$(GDB)