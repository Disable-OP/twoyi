#!/usr/bin/env python3
"""Disassemble syscall stubs and key functions."""
import sys, struct
from elftools.elf.elffile import ELFFile
from capstone import Cs, CS_ARCH_ARM64, CS_MODE_ARM

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
        annot = ""
        # Decode svc - get syscall number from x8 if visible in prior instructions
        if ins.mnemonic == "svc":
            annot = " << SYSCALL >>"
        elif ins.mnemonic in ("bl","b","b.eq","b.ne","cbz","cbnz","tbz","tbnz") and ins.operands:
            op = ins.operands[0]
            if op.type == 2:  # CS_OP_IMM
                annot = f" -> 0x{op.imm:x}"
        elif ins.mnemonic == "blr":
            annot = " <indirect>"
        elif ins.mnemonic == "br":
            annot = " <indirect-jump>"
        print(f"  0x{ins.address:08x}:  {ins.bytes.hex():<10s}  {ins.mnemonic:<8s} {ins.op_str}{annot}")

# Disassemble the syscall stub regions
print("\n" + "="*80)
print("SYSCALL STUB CLUSTER 1 @ 0xff00-0x10060")
print("="*80)
disasm_range(0xff00, 0x180, "syscall_cluster_1")

print("\n" + "="*80)
print("SYSCALL STUB CLUSTER 2 @ 0x13500-0x13620")
print("="*80)
disasm_range(0x13500, 0x140, "syscall_cluster_2")

print("\n" + "="*80)
print("0x3c8c - called from __libc_init as getauxval-like")
print("="*80)
disasm_range(0x3c8c, 0x80, "0x3c8c")

print("\n" + "="*80)
print("0xbd20 - lone SVC")
print("="*80)
disasm_range(0xbd00, 0x80, "0xbd20_context")

# Look at PLT (0x2c20-0x2c80)
print("\n" + "="*80)
print(".plt entries")
print("="*80)
disasm_range(0x2c20, 0x60, ".plt")
