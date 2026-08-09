#!/usr/bin/env python3
"""Look at rt_sigaction caller, mprotect caller, and find the main function."""
import sys, struct
from collections import defaultdict
from elftools.elf.elffile import ELFFile
from capstone import Cs, CS_ARCH_ARM64, CS_MODE_ARM

SO = "/home/z/my-project/repos/twoyi/kr64-analysis/libkrloader64.so"
f = open(SO, "rb")
elf = ELFFile(f)

text = elf.get_section_by_name(".text")
text_addr = text.header.sh_addr
text_data = text.data()

syscall_stubs = {
    0xff38: "close", 0xff50: "openat", 0xff68: "rt_sigprocmask",
    0xff80: "fcntl", 0xff98: "fstat", 0xffb0: "ftruncate",
    0xffc8: "getuid", 0xffe0: "mmap", 0xfff8: "mprotect",
    0x10010: "munmap", 0x10028: "prctl", 0x10040: "writev",
    0x1351c: "faccessat", 0x13534: "fsetxattr", 0x1354c: "accept4",
    0x13564: "clock_gettime", 0x1357c: "connect", 0x13594: "getpid",
    0x135ac: "gettimeofday", 0x135c4: "ppoll", 0x135dc: "pselect6",
    0x135f4: "rt_sigaction", 0x1360c: "socket", 0x13624: "exit_group",
    0x1363c: "dup3", 0xbd20: "syscall_generic",
}

# Byte-scan for BL
bl_callers = defaultdict(list)
for i in range(0, len(text_data) - 3, 4):
    if (text_data[i+3] & 0xFC) == 0x94:
        imm26 = ((text_data[i+3] & 0x03) << 24) | (text_data[i+2] << 16) | (text_data[i+1] << 8) | text_data[i]
        if imm26 & 0x02000000: imm26 -= 0x04000000
        offset = imm26 * 4
        bl_addr = text_addr + i
        target = bl_addr + offset
        bl_callers[target].append(bl_addr)

# Print callers of key functions
key_funcs = [0x31a8, 0x3384, 0x3c70, 0x4090, 0x12490, 0x1192c, 0x2d10]
print("=== Callers of key functions ===")
for kf in key_funcs:
    callers = bl_callers.get(kf, [])
    print(f"  0x{kf:08x}  called {len(callers)}x: {[hex(c) for c in callers[:15]]}")

def vaddr_to_offset(vaddr):
    for seg in elf.iter_segments():
        if seg.header.p_type == "PT_LOAD":
            v0, sz, off = seg.header.p_vaddr, seg.header.p_filesz, seg.header.p_offset
            if v0 <= vaddr < v0 + sz:
                return off + (vaddr - v0)
    return None

def disasm_range(vaddr, length, label=""):
    off = vaddr_to_offset(vaddr)
    if off is None: return
    f.seek(off)
    code = f.read(length)
    md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    md.detail = True
    print(f"\n--- {label}: 0x{vaddr:x} - 0x{vaddr+length:x} ---")
    for ins in md.disasm(code, vaddr):
        annot = ""
        if ins.mnemonic == "bl" and ins.operands and ins.operands[0].type == 2:
            t = ins.operands[0].imm
            if t in syscall_stubs:
                annot = f" -> {syscall_stubs[t]}"
            else:
                annot = f" -> 0x{t:x}"
        elif ins.mnemonic == "svc":
            annot = " << SVC >>"
        elif ins.mnemonic == "blr":
            annot = " <indirect>"
        print(f"  0x{ins.address:08x}:  {ins.mnemonic:<8s} {ins.op_str}{annot}")

# Disassemble rt_sigaction caller function (0x12490)
print("\n" + "="*80)
print("rt_sigaction caller function (starts at 0x12490)")
print("="*80)
disasm_range(0x12490, 0x100, "rt_sigaction_caller")

# Disassemble the mprotect calls within function 0x31a8
print("\n" + "="*80)
print("mprotect call sites within function 0x31a8")
print("="*80)
disasm_range(0x32c0, 0xa0, "mprotect_call1_region")
disasm_range(0x3300, 0xa0, "mprotect_call2_region")

# Look at __libc_init (0x4090) - find what it returns as main
print("\n" + "="*80)
print("__libc_init function (starts at 0x4090)")
print("="*80)
disasm_range(0x4090, 0x80, "__libc_init_prologue")

# Look at function 0x1192c (exit_group caller) - likely the "exit" function
print("\n" + "="*80)
print("exit_group caller function (0x1192c)")
print("="*80)
disasm_range(0x1192c, 0x100, "exit_group_caller")
