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

# initrd is phony because it is very cheap to build but it's super hard to
# track its deps with make
.PHONY: qemu cargo clean initrd $(kern) fake_userspace_target

initrd: $(kern) $(user_target_files)
	cargo run -p uflop -- new -o initrd $^

cargo:
	(cd shoo; cargo build $(CARGOFLAGS))

$(kern):
	(cd kern; cargo build $(CARGOFLAGS))

fake_userspace_target:

$(user_target_prefix)/%: fake_userspace_target
	(cd user/$*; cargo build $(CARGOFLAGS))

clean:
	rm initrd
	cargo clean

# always open a gdb socket but only block if we request a debugger. reasoning:
# the qemu monitor is rather broken and e.g. doesn't allow reading regs
qemu: cargo initrd
	$(QEMU) $(QEMUOPTS) -s

qemu-gdb: cargo initrd
	@echo "Run 'make gdb' in another terminal to connect"
	$(QEMU) $(QEMUOPTS) -s -S

gdb:
	$(GDB)

ifeq ($(OPEN),1)
DOC = --open
endif

doc:
	(cd shoo; cargo doc -p shoo $(CARGOFLAGS) $(DOC))