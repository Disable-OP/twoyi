#!/usr/bin/env python3
"""Find callers of sigaction wrapper (0x12490), prctl-caller function (0x3384),
   and mprotect-caller function (0x31a8). Also look for function pointers in
   .data.rel.ro and .got that reference these addresses."""
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

# Look at all sections that could contain function pointers
sections_to_scan = [".data.rel.ro", ".got", ".got.plt", ".data", ".init_array", ".rodata"]
print("=== Function pointer references in data sections ===")
targets_to_find = [0x31a8, 0x3384, 0x3c70, 0x4090, 0x40b0, 0x12490, 0x1192c, 0x2d10, 0x2cd0, 0x3c8c, 0xceac, 0xb5c8, 0xb178, 0x2a90c, 0x125e4, 0x12538, 0x12620, 0x1243c, 0x125ac, 0x1248c, 0x1336c, 0x2c80, 0x2c94]
for sname in sections_to_scan:
    s = elf.get_section_by_name(sname)
    if not s: continue
    saddr = s.header.sh_addr
    sdata = s.data()
    sz = s.header.sh_size
    for i in range(0, sz - 7, 8):
        val = struct.unpack("<Q", sdata[i:i+8])[0]
        if val in targets_to_find:
            print(f"  Section {sname}+0x{i:x} (vaddr 0x{saddr+i:x}): 0x{val:x}")

# Look at .data.rel.ro in detail (it's where bionic puts struct link_map etc)
print("\n=== .data.rel.ro contents (function pointers and structures) ===")
s = elf.get_section_by_name(".data.rel.ro")
saddr = s.header.sh_addr
sdata = s.data()
sz = s.header.sh_size
print(f".data.rel.ro @ 0x{saddr:x}, size 0x{sz:x}")
# Print all 8-byte values that look like text-section pointers
text_start = text_addr
text_end = text_addr + text.header.sh_size
print(f"  (text range: 0x{text_start:x} - 0x{text_end:x})")
for i in range(0, sz - 7, 8):
    val = struct.unpack("<Q", sdata[i:i+8])[0]
    if text_start <= val < text_end:
        print(f"  +0x{i:x} (vaddr 0x{saddr+i:x}): 0x{val:x}  <- text pointer")

# Print callers of 0x12490 (sigaction wrapper)
print("\n=== Callers of sigaction() wrapper (0x12490) ===")
print(f"  bl_callers[0x12490] = {bl_callers.get(0x12490, [])}")

# Print callers of 0x1192c (abort) and 0x1243c (sigprocmask wrapper?)
for fn in [0x12490, 0x1192c, 0x1243c, 0x1248c, 0x12538, 0x125e4, 0x12620, 0x125ac, 0x12588]:
    callers = bl_callers.get(fn, [])
    print(f"  0x{fn:x} called {len(callers)}x: {[hex(c) for c in callers[:15]]}")

# Now look at what's around the prctl-caller function 0x3384 -- specifically, does anything
# in __libc_init (0x40b0) reference it?
print("\n=== Looking for references to 0x3384 (prctl-caller) ===")
# Check BL targets
print(f"  BL callers of 0x3384: {bl_callers.get(0x3384, [])}")
# Check direct address mentions in text
for i in range(0, len(text_data) - 3, 4):
    inst_bytes = text_data[i:i+4]
    # ADRP+ADD pattern can construct addresses
    # Just check if the 4-byte instruction has any imm matching 0x3384
    pass

# Look at __libc_init (0x40b0) full disassembly to find what it returns as main
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

# Find what __libc_init returns. Look at its 'ret' instructions
print("\n=== __libc_init full disassembly (look for return path) ===")
# It's a long function, let's dump the first 0x500 bytes to find structure
disasm_range(0x40b0, 0x500, "__libc_init_start")
