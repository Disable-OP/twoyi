#!/usr/bin/env python3
"""Disassemble specific functions and look for syscalls/imports."""
import sys, struct
from elftools.elf.elffile import ELFFile
from capstone import Cs, CS_ARCH_ARM64, CS_MODE_ARM
from capstone import CS_OP_REG, CS_OP_IMM, CS_OP_MEM

SO = "/home/z/my-project/repos/twoyi/kr64-analysis/libkrloader64.so"

f = open(SO, "rb")
elf = ELFFile(f)

def vaddr_to_offset(vaddr):
    for seg in elf.iter_segments():
        if seg.header.p_type == "PT_LOAD":
            v0 = seg.header.p_vaddr
            sz = seg.header.p_filesz
            off = seg.header.p_offset
            if v0 <= vaddr < v0 + sz:
                return off + (vaddr - v0)
    return None

def get_section_bytes(name):
    s = elf.get_section_by_name(name)
    if not s: return None, None
    return s.data(), s.header.sh_addr

def disasm_range(vaddr, length, label=""):
    off = vaddr_to_offset(vaddr)
    if off is None:
        print(f"# Cannot map vaddr 0x{vaddr:x}")
        return
    f.seek(off)
    code = f.read(length)
    md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    md.detail = True
    print(f"\n=== Disassembly {label}: 0x{vaddr:x} - 0x{vaddr+length:x} (file off 0x{off:x}, {length} bytes) ===")
    for ins in md.disasm(code, vaddr):
        # Annotate call/jump targets
        annot = ""
        if ins.mnemonic in ("bl","b","b.eq","b.ne","cbz","cbnz","tbz","tbnz") and ins.operands:
            op = ins.operands[0]
            if op.type == CS_OP_IMM:
                target = op.imm
                # See if target is in any section
                annot = f" -> 0x{target:x}"
        print(f"  0x{ins.address:08x}:  {ins.mnemonic:<8s} {ins.op_str}{annot}")

# Disassemble __libc_init candidate at 0x40b0 (called from _start)
print("\n" + "="*80)
print("__libc_init candidate at 0x40b0 (called from _start)")
print("="*80)
disasm_range(0x40b0, 0x400, "__libc_init")

# Disassemble 0x1336c (called from init_array entries - suspected __cxa_atexit)
print("\n" + "="*80)
print("0x1336c - called from init_array entries (suspected __cxa_atexit)")
print("="*80)
disasm_range(0x1336c, 0x200, "0x1336c")

# Disassemble more around _start to find the actual main (the third function in .text)
print("\n" + "="*80)
print("Functions after _start (continuing .text from 0x2d10)")
print("="*80)
disasm_range(0x2d10, 0x300, "post-_start")
