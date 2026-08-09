#!/usr/bin/env python3
"""Find function starts using capstone (look for prologue patterns).
   AArch64 prologue usually starts with:
   - stp x29, x30, [sp, #imm]! (0xa9bf7bfd or 0xa9XX7bfd with bit pattern)
   - sub sp, sp, #imm (0xd1XXffXX)
"""
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

func_starts = set()
# STP x29, x30, [sp, ...] : bytes (LE) = fd 7b XX a9 (with XX depending on imm)
for i in range(0, len(text_data) - 3, 4):
    if text_data[i] == 0xfd and text_data[i+1] == 0x7b and text_data[i+3] == 0xa9:
        func_starts.add(text_addr + i)
# SUB sp, sp, #imm : bytes (LE) = ff 03 XX d1 (where XX has the imm)
for i in range(0, len(text_data) - 3, 4):
    if text_data[i] == 0xff and text_data[i+1] == 0x03 and text_data[i+3] == 0xd1:
        func_starts.add(text_addr + i)

func_starts = sorted(func_starts)
print(f"Found {len(func_starts)} function-prologue candidates")

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

def find_func_for(addr):
    best = None
    for fs in func_starts:
        if fs <= addr:
            best = fs
        else:
            break
    return best

addrs_of_interest = [
    (0x32f4, "mprotect call #1"),
    (0x3334, "mprotect call #2"),
    (0x3a28, "prctl call #1"),
    (0x3c00, "prctl call #2"),
    (0x3964, "rt_sigprocmask call"),
    (0x124f0, "rt_sigaction call"),
    (0x119a8, "exit_group call"),
    (0x3c8c, "getauxval-like?"),
    (0x40b0, "__libc_init"),
    (0x2cd0, "_start"),
]

print("\n=== Function containing each address of interest ===")
for addr, label in addrs_of_interest:
    fs = find_func_for(addr)
    print(f"  0x{addr:x} ({label}) is in function starting at 0x{fs:x}" if fs else f"  0x{addr:x} ({label}): NO PROLOGUE FOUND")

# Find function bounds (next func_start - 4)
def func_bounds(addr):
    fs = find_func_for(addr)
    if fs is None: return None, None
    idx = func_starts.index(fs)
    if idx + 1 < len(func_starts):
        return fs, func_starts[idx+1] - 4
    return fs, text_addr + text_size - 4

# For each function of interest, disassemble
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

# Function 0x338c - likely the main loader init (contains prctl + sigprocmask)
fs, fe = func_bounds(0x3a28)
print(f"\nFunction containing prctl call (0x3a28): 0x{fs:x} - 0x{fe:x}, size {fe-fs+4:#x}")
# Disassemble just the region around the prctl and sigprocmask calls
disasm_range(0x3940, 0x100, "sigprocmask_caller_region")
disasm_range(0x3a00, 0x80, "prctl_call1_region")
disasm_range(0x3bd0, 0x80, "prctl_call2_region")

# Function containing mprotect calls at 0x32f4 and 0x3334
fs1, fe1 = func_bounds(0x32f4)
print(f"\nFunction containing mprotect call #1 (0x32f4): 0x{fs1:x} - 0x{fe1:x}, size {fe1-fs1+4:#x}")
disasm_range(fs1, min(0x800, fe1-fs1+4), "mprotect_caller_func")

# Function containing rt_sigaction call at 0x124f0
fs2, fe2 = func_bounds(0x124f0)
print(f"\nFunction containing rt_sigaction call (0x124f0): 0x{fs2:x} - 0x{fe2:x}, size {fe2-fs2+4:#x}")
disasm_range(fs2, min(0x300, fe2-fs2+4), "rt_sigaction_caller_func")

# Function containing exit_group call at 0x119a8
fs3, fe3 = func_bounds(0x119a8)
print(f"\nFunction containing exit_group call (0x119a8): 0x{fs3:x} - 0x{fe3:x}, size {fe3-fs3+4:#x}")
disasm_range(fs3, min(0x300, fe3-fs3+4), "exit_group_caller_func")
