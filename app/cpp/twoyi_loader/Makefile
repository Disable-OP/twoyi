# Makefile for twoyi_loader
#
# Builds:
#   1. libtwoyi_loader.so — the custom ELF interpreter (PT_INTERP)
#   2. test_twoyi — combined test binary (loader + test guest in one binary)
#
# Usage:
#   make                    — build test binary (x86_64 native)
#   make test               — run test binary
#   make loader             — build libtwoyi_loader.so
#   make TARGET=arm64       — cross-compile for arm64
#   make test-arm64         — run arm64 test under QEMU

UNAME_M := $(shell uname -m)

ifeq ($(TARGET),arm64)
    CC ?= aarch64-linux-gnu-gcc-14
    SYSROOT ?= /tmp/gcc-aarch64-extract/usr/aarch64-linux-gnu
    GCC_INCLUDE ?= /tmp/gcc-aarch64-extract/usr/lib/gcc-cross/aarch64-linux-gnu/14/include
    LD_LIBRARY_PATH := /tmp/gcc-aarch64-extract/usr/lib/x86_64-linux-gnu:$(LD_LIBRARY_PATH)
    ARCH_START = arch/arm64_start.S
    EXTRA_CFLAGS = --sysroot=$(SYSROOT) -I$(GCC_INCLUDE) -fno-stack-protector
    EXTRA_LDFLAGS = -static -Wl,--no-dynamic-linker
    QEMU ?= /tmp/qemu-user-extract/usr/bin/qemu-aarch64
    TARGET_BIN = test_twoyi_arm64
    LOADER_BIN = libtwoyi_loader_arm64.so
else
    CC ?= gcc
    ifeq ($(UNAME_M),x86_64)
	ARCH_START = arch/x86_64_start.S
    else ifeq ($(UNAME_M),aarch64)
	ARCH_START = arch/arm64_start.S
    endif
    EXTRA_CFLAGS =
    EXTRA_LDFLAGS =
    TARGET_BIN = test_twoyi
    LOADER_BIN = libtwoyi_loader.so
endif

CFLAGS = -Wall -Wextra -O2 -g -fno-stack-protector $(EXTRA_CFLAGS)
CFLAGS += -Iinclude -Isrc

# All source files
LOADER_SRCS = src/twoyi_loader.c
LOADER_OBJS = $(LOADER_SRCS:.c=.o)
ARCH_OBJS = $(ARCH_START:.S=.o)

# For the combined test, we also need loader_main.c (which calls main())
# and the test guest
COMBINED_SRCS = src/loader_main.c src/twoyi_loader.c
TEST_SRCS = tests/test_guest.c
COMBINED_OBJS = $(COMBINED_SRCS:.c=.o) $(TEST_SRCS:.c=.o) $(ARCH_OBJS)

.PHONY: all clean test loader test-arm64

all: $(TARGET_BIN)

# Combined test binary (loader + test guest)
# For arm64: need arm64_stub.c for _DYNAMIC etc.
ifeq ($(TARGET),arm64)
$(TARGET_BIN): $(COMBINED_OBJS) arch/arm64_stub.o
	$(CC) $(CFLAGS) -nostartfiles -no-pie $(EXTRA_LDFLAGS) -Wl,-e,_start -o $@ \
		$(COMBINED_OBJS) arch/arm64_stub.o -lc -lgcc -lgcc_eh -lpthread
	@echo "Built $(TARGET_BIN)"
	@file $@ | head -1
else
$(TARGET_BIN): $(COMBINED_OBJS)
	$(CC) $(CFLAGS) -nostartfiles -no-pie $(EXTRA_LDFLAGS) -Wl,-e,_start -o $@ \
		$(COMBINED_OBJS) -lc -lpthread
	@echo "Built $(TARGET_BIN)"
	@file $@ | head -1
endif

# Custom ELF interpreter (shared library, like a dynamic linker)
# This is what gets installed as PT_INTERP for guest binaries
loader: $(LOADER_BIN)

$(LOADER_BIN): $(LOADER_OBJS)
	$(CC) $(CFLAGS) -nostartfiles -shared -fPIC -Wl,-e,twoyi_loader_main -o $@ \
		$(LOADER_OBJS) -lc -lpthread
	@echo "Built $(LOADER_BIN)"
	@file $@ | head -1

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

%.o: %.S
	$(CC) $(CFLAGS) -c -o $@ $<

test: $(TARGET_BIN)
	@echo "=== Running vertical slice test ==="
	./$(TARGET_BIN)

test-arm64: $(TARGET_BIN)
	@echo "=== Running arm64 test under QEMU ==="
	$(QEMU) ./$(TARGET_BIN)

clean:
	rm -f $(LOADER_OBJS) $(COMBINED_OBJS) arch/arm64_stub.o $(TARGET_BIN) $(LOADER_BIN)

# Shared library (for LD_PRELOAD / DT_NEEDED)
shlib: libtwoyi_loader_shlib.so

libtwoyi_loader_shlib.so: src/twoyi_loader_shlib.c
	$(CC) $(CFLAGS) -shared -fPIC -o $@ src/twoyi_loader_shlib.c -lc -lpthread -ldl
	@echo "Built $@"
	@file $@ | head -1
