# Makefile for twoyi_loader vertical slice test

CC ?= gcc
CFLAGS = -Wall -Wextra -O2 -g -fno-stack-protector
CFLAGS += -Iinclude -Itests

UNAME_M := $(shell uname -m)
ifeq ($(UNAME_M),x86_64)
    ARCH_START = arch/x86_64_start.S
else ifeq ($(UNAME_M),aarch64)
    ARCH_START = arch/arm64_start.S
endif

LOADER_OBJS = src/loader_main.o src/sigsys_handler.o src/seccomp_filter.o src/mount_table.o
TEST_OBJS = tests/test_guest.o
ARCH_OBJS = $(ARCH_START:.S=.o)
ALL_OBJS = $(LOADER_OBJS) $(TEST_OBJS) $(ARCH_OBJS)

.PHONY: all clean test

all: test_twoyi

# Combined test binary.
# -nostartfiles: no crt0 — we provide _start
# -no-pie: disable PIE (we want a fixed load address for static)
# -Wl,-e,_start: entry point is _start
test_twoyi: $(ALL_OBJS)
	$(CC) $(CFLAGS) -nostartfiles -no-pie -Wl,-e,_start -o $@ $(ALL_OBJS) -lc
	@echo "Built test_twoyi"

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

%.o: %.S
	$(CC) $(CFLAGS) -c -o $@ $<

test: test_twoyi
	@echo "=== Running vertical slice test ==="
	./test_twoyi

clean:
	rm -f $(ALL_OBJS) test_twoyi
