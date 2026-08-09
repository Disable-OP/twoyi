#!/usr/bin/env python3
"""Look at function 0x2a90c (called after mprotect with PROT_EXEC) and
   trace more of the prctl-caller function 0x3384 to find BPF filter data."""
import sys, struct
from elftools.elf.elffile import ELFFile
from capstone import Cs, CS_ARCH_ARM64, CS_MODE_ARM

SO = "/home/z/my-project/repos/twoyi/kr64-analysis/libkrloader64.so"
f = open(SO, "rb")
elf = ELFFile(f)

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
            annot = f" -> 0x{t:x}"
        elif ins.mnemonic == "svc":
            annot = " << SVC >>"
        elif ins.mnemonic == "blr":
            annot = " <indirect>"
        print(f"  0x{ins.address:08x}:  {ins.mnemonic:<8s} {ins.op_str}{annot}")

# Function 0x2a90c (called after mprotect with PROT_READ|PROT_EXEC)
print("="*80)
print("Function 0x2a90c (called after mprotect PROT_EXEC)")
print("="*80)
disasm_range(0x2a90c, 0x100, "func_0x2a90c")

# Function 0xb5c8 (called from 0x31a8 mprotect caller)
print("="*80)
print("Function 0xb5c8 (helper called from 0x31a8)")
print("="*80)
disasm_range(0xb5c8, 0x80, "func_0xb5c8")

# Function 0xb178 (called from 0x31a8 mprotect caller)
print("="*80)
print("Function 0xb178 (helper called from 0x31a8)")
print("="*80)
disasm_range(0xb178, 0x80, "func_0xb178")

# Look at the actual prctl-caller function 0x3384 start
print("="*80)
print("Function 0x3384 START (prctl + rt_sigprocmask caller)")
print("="*80)
disasm_range(0x3384, 0x300, "func_0x3384_start")

# Look at the data layout around the prctl call to see BPF filter setup
# Specifically what's at sp+0x80, sp+0x88, sp+0x98, sp+0xa0 (where BPF might be)
print("="*80)
print("BPF filter setup area (around 0x3a3c)")
print("="*80)
disasm_range(0x3a3c, 0x200, "bpf_setup_region")

# Look at .data.rel.ro for any function pointer tables (vtables etc)
import struct
s = elf.get_section_by_name('.data.rel.ro')
saddr = s.header.sh_addr
sdata = s.data()
sz = s.header.sh_size
print(f"\n=== .data.rel.ro full dump (16 bytes per line) @ 0x{saddr:x} size 0x{sz:x} ===")
for i in range(0, min(sz, 0x300), 16):
    bytes_str = ' '.join(f'{b:02x}' for b in sdata[i:i+16])
    print(f"  0x{saddr+i:08x}: {bytes_str}")
