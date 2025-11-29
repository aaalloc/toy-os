TARGET := riscv64gc-unknown-none-elf
MODE := release
TARGET_DIR := target/$(TARGET)/$(MODE)
KERNEL_ELF := $(TARGET_DIR)/toy-os
KERNEL_BIN := $(KERNEL_ELF).bin
DISASM_TMP := $(TARGET_DIR)/asm

FS_IMG := ../user/$(TARGET_DIR)/fs.img
APPS := ../user/src/bin/*

ifeq ($(MODE), release)
	MODE_ARG := --release
endif

BOARD := qemu
SBI ?= rustsbi
BOOTLOADER := ../bootloader/$(SBI)-qemu.bin

KERNEL_ENTRY_PA := 0x80200000

OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

# Disassembly
DISASM ?= -x
TEST ?=

ENTRY_POINT=kmain
ifeq ($(BOARD), qemu)
	ENTRY_POINT := kmain
else ifeq ($(BOARD), nobios)
	ENTRY_POINT := start
else ifeq ($(BOARD), rvemu)
	ENTRY_POINT := start
endif

build: env $(KERNEL_BIN) fs-img

env:
	(rustup target list | grep "riscv64gc-unknown-none-elf (installed)") || rustup target add $(TARGET)
	cargo install cargo-binutils

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

kernel:
	@cd ../user && make build
	@echo Platform: $(BOARD)
	@cp src/linker-$(BOARD).ld src/linker.ld
	@ENTRY_POINT=$(ENTRY_POINT) cargo build $(MODE_ARG)
	@rm src/linker.ld


clean:
	@cargo clean

disasm: kernel
	@$(OBJDUMP) $(DISASM) $(KERNEL_ELF) | less

disasm-vim: kernel
	@$(OBJDUMP) $(DISASM) $(KERNEL_ELF) > $(DISASM_TMP)
	@vim $(DISASM_TMP)
	@rm $(DISASM_TMP)

run: run-inner

$(APPS):

fs-img: $(APPS)
	@cd ../user && make build
	@rm -f $(FS_IMG)
	@cd ../easy-fs-fuse && cargo run --release -- -s ../user/src/bin/ -t ../user/$(TARGET_DIR)/

QEMU_ARGS := -machine virt \
			 -cpu rv64 \
			 -nographic \
			 -bios /home/yanovskyy/Téléchargements/opensbi-1.7-rv-bin/share/opensbi/lp64/generic/firmware/fw_dynamic.bin \
			 -global virtio-mmio.force-legacy=false \
			 -dtb /home/yanovskyy/Documents/projects/toy-os/os/virt.dtb \
			 -kernel $(KERNEL_BIN) \
			 -device virtio-blk-device,drive=x0 \
			 -drive file=$(FS_IMG),if=none,format=raw,id=x0

QEMU_ARGS_NO_BOOTLOADER := -machine virt \
			 -cpu rv64 \
			 -nographic \
			 -bios none \
			 -global virtio-mmio.force-legacy=false \
			 -kernel $(KERNEL_BIN) \
			 -device virtio-blk-device,drive=x0 \
			 -drive file=$(FS_IMG),if=none,format=raw,id=x0
# NOTE: without bios none, it default to OpenSBI, so base addr for kmain needs to be 0x80200000.


run-inner: build
ifeq ($(BOARD), qemu)
	@qemu-system-riscv64 $(QEMU_ARGS)
else ifeq ($(BOARD), nobios)
	@qemu-system-riscv64 $(QEMU_ARGS_NO_BOOTLOADER)
else ifeq ($(BOARD), rvemu)
	@/home/yanovskyy/Documents/projects/rvemu-0.0.11/target/release/rvemu-cli -k $(KERNEL_BIN) -f $(FS_IMG)
endif

gdbserver: build
ifeq ($(BOARD), qemu)
	@qemu-system-riscv64 $(QEMU_ARGS) -s -S
else ifeq ($(BOARD), nobios)
	@qemu-system-riscv64 $(QEMU_ARGS_NO_BOOTLOADER) -s -S
endif

gdbclient:
	@gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'

.PHONY: build env kernel clean disasm disasm-vim run-inner gdbserver gdbclient