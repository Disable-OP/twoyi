# Makefile for twoyi_loader vertical slice test
#
# Supports two modes:
#   make            — x86_64 native build
#   make arm64      — AArch64 cross-compile (run with qemu-aarch64)
#   make test       — run x86_64 test
#   make test-arm64 — run AArch64 test under QEMU

UNAME_M := $(shell uname -m)

ifeq ($(TARGET),arm64)
    # AArch64 cross-compilation mode
    CC ?= aarch64-linux-gnu-gcc-14
    SYSROOT ?= /tmp/gcc-aarch64-extract/usr/aarch64-linux-gnu
    GCC_INCLUDE ?= /tmp/gcc-aarch64-extract/usr/lib/gcc-cross/aarch64-linux-gnu/14/include
    LD_LIBRARY_PATH := /tmp/gcc-aarch64-extract/usr/lib/x86_64-linux-gnu:$(LD_LIBRARY_PATH)
    ARCH_START = arch/arm64_start.S
    EXTRA_CFLAGS = --sysroot=$(SYSROOT) -I$(GCC_INCLUDE) -fno-stack-protector
    EXTRA_LDFLAGS = -static -Wl,--no-dynamic-linker
    QEMU ?= /tmp/qemu-user-extract/usr/bin/qemu-aarch64
    TARGET_BIN = test_twoyi_arm64
else
    # x86_64 native mode
    CC ?= gcc
    ifeq ($(UNAME_M),x86_64)
	ARCH_START = arch/x86_64_start.S
    else ifeq ($(UNAME_M),aarch64)
	ARCH_START = arch/arm64_start.S
    endif
    EXTRA_CFLAGS =
    EXTRA_LDFLAGS =
    TARGET_BIN = test_twoyi
endif

CFLAGS = -Wall -Wextra -O2 -g -fno-stack-protector $(EXTRA_CFLAGS)
CFLAGS += -Iinclude -Itests

LOADER_OBJS = src/loader_main.o src/sigsys_handler.o src/seccomp_filter.o src/mount_table.o
TEST_OBJS = tests/test_guest.o
ARCH_OBJS = $(ARCH_START:.S=.o)
ALL_OBJS = $(LOADER_OBJS) $(TEST_OBJS) $(ARCH_OBJS)

.PHONY: all clean test arm64 test-arm64

all: $(TARGET_BIN)

arm64:
	$(MAKE) TARGET=arm64 $(TARGET_BIN)

# Combined test binary.
# For arm64: provide a stub _DYNAMIC to satisfy dl-reloc-static-pie.o
ifeq ($(TARGET),arm64)
$(TARGET_BIN): $(ALL_OBJS) arch/arm64_stub.o
	$(CC) $(CFLAGS) -nostartfiles -no-pie $(EXTRA_LDFLAGS) -Wl,-e,_start -o $@ $(ALL_OBJS) arch/arm64_stub.o -lc -lgcc -lgcc_eh
	@echo "Built $(TARGET_BIN)"
	@file $@ | head -1
else
$(TARGET_BIN): $(ALL_OBJS)
	$(CC) $(CFLAGS) -nostartfiles -no-pie $(EXTRA_LDFLAGS) -Wl,-e,_start -o $@ $(ALL_OBJS) -lc
	@echo "Built $(TARGET_BIN)"
	@file $@ | head -1
endif

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

%.o: %.S
	$(CC) $(CFLAGS) -c -o $@ $<

# Run x86_64 test natively
test: test_twoyi
	@echo "=== Running vertical slice test (x86_64 native) ==="
	./test_twoyi

# Run AArch64 test under QEMU
test-arm64: test_twoyi_arm64
	@echo "=== Running vertical slice test (AArch64 via QEMU) ==="
	$(QEMU) ./test_twoyi_arm64

clean:
	rm -f $(ALL_OBJS) test_twoyi test_twoyi_arm64
