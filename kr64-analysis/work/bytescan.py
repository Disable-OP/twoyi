#!/usr/bin/env python3
"""Byte-pattern scan for BL instructions to find ALL callers of syscall stubs.
   AArch64 BL opcode: 1001 01 + imm26 (top 6 bits = 0b100101 = 0x25).
   The instruction byte (little-endian) starts with bits 0b100101.
"""
import sys, struct
from collections import Counter, defaultdict
from elftools.elf.elffile import ELFFile
from capstone import Cs, CS_ARCH_ARM64, CS_MODE_ARM

SO = "/home/z/my-project/repos/twoyi/kr64-analysis/libkrloader64.so"
f = open(SO, "rb")
elf = ELFFile(f)

text = elf.get_section_by_name(".text")
text_addr = text.header.sh_addr
text_size = text.header.sh_size
text_data = text.data()

# Byte-pattern scan for BL: top 6 bits = 0b100101
# In LE, the bottom byte has the lowest bits. Instruction encoding:
# 31..26: 100101 (BL) | 25..0: imm26 (signed)
# So instr = 0x94000000 | (imm26 & 0x3FFFFFF)
# In little-endian bytes: byte0 = bits 0..7, byte1 = bits 8..15, byte2 = bits 16..23, byte3 = bits 24..31
# bits 24..31 of BL: 1001 0100 = 0x94

bl_callers = defaultdict(list)
for i in range(0, len(text_data) - 3, 4):
    if text_data[i+3] == 0x94 and (text_data[i+3] >> 6) == 0b10:
        # Top 6 bits check: text_data[i+3] = 1001 01 xx where xx are top 2 bits of imm26
        # Need to verify: bits 31-26 = 100101
        # byte3 = bits 31-24 = 1001 01 i(i) where i(i) are top 2 bits of imm26
        # So byte3 must have bits 7..2 = 100101 = 0x25
        if (text_data[i+3] & 0xFC) == 0x94:
            # Decode imm26 (signed)
            imm26 = ((text_data[i+3] & 0x03) << 24) | (text_data[i+2] << 16) | (text_data[i+1] << 8) | text_data[i]
            # Sign extend from 26 bits
            if imm26 & 0x02000000:
                imm26 -= 0x04000000
            offset = imm26 * 4
            bl_addr = text_addr + i
            target = bl_addr + offset
            bl_callers[target].append(bl_addr)

print(f"Found {sum(len(v) for v in bl_callers.values())} BL instructions total")
print(f"Unique BL targets: {len(bl_callers)}")

syscall_stubs = {
    0xff38: (0x39, "close"),
    0xff50: (0x38, "openat"),
    0xff68: (0x87, "rt_sigprocmask"),
    0xff80: (0x19, "fcntl"),
    0xff98: (0x50, "fstat"),
    0xffb0: (0x2e, "ftruncate"),
    0xffc8: (0xae, "getuid"),
    0xffe0: (0xde, "mmap"),
    0xfff8: (0xe2, "mprotect"),
    0x10010: (0xd7, "munmap"),
    0x10028: (0xa7, "prctl"),
    0x10040: (0x42, "writev"),
    0x1351c: (0x30, "faccessat"),
    0x13534: (0x07, "fsetxattr"),
    0x1354c: (0xf2, "accept4"),
    0x13564: (0x71, "clock_gettime"),
    0x1357c: (0xcb, "connect"),
    0x13594: (0xac, "getpid"),
    0x135ac: (0xa9, "gettimeofday"),
    0x135c4: (0x49, "ppoll"),
    0x135dc: (0x48, "pselect6"),
    0x135f4: (0x86, "rt_sigaction"),
    0x1360c: (0xc6, "socket"),
    0x13624: (0x5e, "exit_group"),
    0x1363c: (0x18, "dup3"),
    0xbd20: (None, "syscall(generic)"),
}

print("\n=== Syscall stub callers (byte-scan) ===")
for stub_addr in sorted(syscall_stubs):
    num, name = syscall_stubs[stub_addr]
    callers = bl_callers.get(stub_addr, [])
    if callers:
        print(f"  0x{stub_addr:08x}  {name}({num})  called {len(callers)}x: {[hex(c) for c in callers[:20]]}")
    else:
        print(f"  0x{stub_addr:08x}  {name}({num})  called 0x  *** NEVER CALLED ***")
