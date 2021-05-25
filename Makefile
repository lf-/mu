# stolen from https://github.com/mit-pdos/xv6-riscv/blob/riscv/Makefile
QEMU = /opt/qemu/bin/qemu-system-riscv64
GDB = /opt/gdb/bin/gdb
# TODO: UP for the minute
CPUS = 1
STAGE1 = target/riscv64imac-mu-shoo-elf/release/shoo
CARGOFLAGS = --release
# RUST_TARGET_PATH = $(shell realpath ..)
# export RUST_TARGET_PATH

QEMUOPTS = -machine virt -bios none -kernel $(STAGE1) -initrd initrd -m 128M \
			-smp $(CPUS) -nographic -trace enable=riscv_trap
# debug on port 1234
#QEMUOPTS += -s
#QEMUOPTS += -drive file=fs.img,if=none,format=raw,id=x0
#QEMUOPTS += -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

user_targets = init
user_target_prefix = target/riscv64imac-mu-user-elf/release
user_target_files = $(addprefix $(user_target_prefix)/,$(user_targets))
kern = target/riscv64imac-mu-kern-elf/release/kern
shoo = target/riscv64imac-mu-shoo-elf/release/shoo

.PHONY: qemu clean doc gdb build.rs

initrd: $(kern) $(user_target_files)
	cargo run -p uflop -- new -o initrd $^

$(shoo).d: $(shoo)
ifneq ("$(wildcard $(shoo).d)","")
include $(shoo).d
endif
$(shoo):
	(cd shoo; cargo build $(CARGOFLAGS))

$(kern).d: $(kern)
ifneq ("$(wildcard $(kern).d)","")
include $(kern).d
endif

$(kern):
	(cd kern; cargo build $(CARGOFLAGS))

$(user_target_prefix)/%.d: $(user_target_prefix)/$*
ifneq ("$(wildcard $(user_target_prefix)/*.d)","")
include $(wildcard $(user_target_prefix)/*.d)
endif

$(user_target_prefix)/%:
	(cd user/$*; cargo build $(CARGOFLAGS))

clean:
	rm initrd
	cargo clean

# always open a gdb socket but only block if we request a debugger. reasoning:
# the qemu monitor is rather broken and e.g. doesn't allow reading regs
qemu: initrd $(shoo)
	$(QEMU) $(QEMUOPTS) -s

qemu-gdb: initrd $(shoo)
	@echo "Run 'make gdb' in another terminal to connect"
	$(QEMU) $(QEMUOPTS) -s -S

gdb:
	$(GDB)

ifeq ($(OPEN),1)
DOC = --open
endif

doc:
	(cd shoo; cargo doc -p shoo $(CARGOFLAGS) $(DOC))
