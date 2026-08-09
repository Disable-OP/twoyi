#!/usr/bin/env python3
"""Disassemble libkrloader64.so using capstone + pyelftools."""
import sys, struct
from elftools.elf.elffile import ELFFile
from elftools.elf.relocation import RelocationSection
from capstone import Cs, CS_ARCH_ARM64, CS_MODE_ARM

SO = "/home/z/my-project/repos/twoyi/kr64-analysis/libkrloader64.so"

def load():
    f = open(SO, "rb")
    elf = ELFFile(f)
    return f, elf

def vaddr_to_offset(elf, vaddr):
    for seg in elf.iter_segments():
        if seg.header.p_type == "PT_LOAD":
            v0 = seg.header.p_vaddr
            sz = seg.header.p_filesz
            off = seg.header.p_offset
            if v0 <= vaddr < v0 + sz:
                return off + (vaddr - v0)
    return None

def get_section_bytes(elf, name):
    s = elf.get_section_by_name(name)
    if not s: return None, None, None
    return s.data(), s.header.sh_addr, s.header.sh_size

def disasm_range(elf, f, vaddr, length, label=""):
    off = vaddr_to_offset(elf, vaddr)
    if off is None:
        print(f"# Cannot map vaddr 0x{vaddr:x}")
        return
    f.seek(off)
    code = f.read(length)
    md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    md.detail = False
    print(f"\n=== Disassembly {label}: 0x{vaddr:x} - 0x{vaddr+length:x} (file off 0x{off:x}, {length} bytes) ===")
    for ins in md.disasm(code, vaddr):
        print(f"  0x{ins.address:08x}:  {ins.mnemonic:<8s} {ins.op_str}")

def main():
    f, elf = load()
    # Show relocations so we can resolve PLT/GOT references
    rela_dyn = elf.get_section_by_name(".rela.dyn")
    rela_plt = elf.get_section_by_name(".rela.plt")
    dynsym = elf.get_section_by_name(".dynsym")
    
    # Build symbol map: name -> addr
    sym_by_addr = {}
    sym_by_name = {}
    for i, sym in enumerate(dynsym.iter_symbols()):
        if sym.entry.st_value:
            sym_by_addr[sym.entry.st_value] = sym.name
            sym_by_name[sym.name] = (sym.entry.st_value, sym.entry.st_info.type)
    
    # Build relocation map: offset -> symbol name
    reloc_map = {}
    for rsection in [rela_dyn, rela_plt]:
        if rsection is None: continue
        for r in rsection.iter_relocations():
            off = r.entry.r_offset
            symidx = r.entry.r_info_sym
            sym = dynsym.get_symbol(symidx)
            reloc_map[off] = sym.name
    
    print("=== Dynamic symbols with values ===")
    for name, (val, typ) in sorted(sym_by_name.items(), key=lambda x: x[1][0] if x[1][0] else 0):
        if val and name not in ("_edata","_end","__bss_start"):
            print(f"  0x{val:08x}: {name}")
    
    print("\n=== Relocations (offset -> symbol) ===")
    for off in sorted(reloc_map):
        print(f"  0x{off:08x} -> {reloc_map[off]}")
    
    # Dump init_array
    init_arr_data, init_addr, init_sz = get_section_bytes(elf, ".init_array")
    print(f"\n=== .init_array at 0x{init_addr:x} size {init_sz} ===")
    for i in range(0, init_sz, 8):
        val = struct.unpack("<Q", init_arr_data[i:i+8])[0]
        print(f"  [0x{init_addr+i:x}] = 0x{val:x}")
    
    # Check for .preinit_array
    pia = elf.get_section_by_name(".preinit_array")
    if pia:
        print(f"\n=== .preinit_array at 0x{pia.header.sh_addr:x} size {pia.header.sh_size} ===")
        d = pia.data()
        for i in range(0, pia.header.sh_size, 8):
            val = struct.unpack("<Q", d[i:i+8])[0]
            print(f"  [0x{pia.header.sh_addr+i:x}] = 0x{val:x}")
    else:
        print("\n=== No .preinit_array ===")
    
    # Disassemble entry point
    disasm_range(elf, f, 0x2cd0, 64, "_start")
    # Disassemble init_array functions
    for i in range(0, init_sz, 8):
        val = struct.unpack("<Q", init_arr_data[i:i+8])[0]
        disasm_range(elf, f, val, 128, f"init_array[{i//8}]")

if __name__ == "__main__":
    main()
