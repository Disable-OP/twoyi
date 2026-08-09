#!/usr/bin/env python3
"""Find callers of specific syscall stubs and analyze key functions."""
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

def vaddr_to_offset(vaddr):
    for seg in elf.iter_segments():
        if seg.header.p_type == "PT_LOAD":
            v0, sz, off = seg.header.p_vaddr, seg.header.p_filesz, seg.header.p_offset
            if v0 <= vaddr < v0 + sz:
                return off + (vaddr - v0)
    return None

def disasm_at(vaddr, length):
    off = vaddr_to_offset(vaddr)
    if off is None: return []
    f.seek(off)
    code = f.read(length)
    md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    md.detail = True
    return list(md.disasm(code, vaddr))

# Map: each BL target -> list of caller addresses
bl_callers = defaultdict(list)
md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
md.detail = True
for ins in md.disasm(text_data, text_addr):
    if ins.mnemonic == "bl" and ins.operands and ins.operands[0].type == 2:
        bl_callers[ins.operands[0].imm].append(ins.address)

# Key syscall stubs and their syscall numbers (from previous analysis)
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
}

print("=== Syscall stub callers ===")
for stub_addr, (num, name) in sorted(syscall_stubs.items()):
    callers = bl_callers.get(stub_addr, [])
    if callers:
        print(f"  0x{stub_addr:08x}  sys_{name}({num:#x})  called {len(callers)}x: {[hex(c) for c in callers[:10]]}")
    else:
        print(f"  0x{stub_addr:08x}  sys_{name}({num:#x})  called 0x  *** NEVER CALLED ***")

# Disassemble the lone SVC at 0xbd20 context
print("\n=== Context around lone SVC at 0xbd20 ===")
for ins in disasm_at(0xbd00, 0x80):
    annot = ""
    if ins.mnemonic == "svc":
        annot = " << SVC >>"
    elif ins.mnemonic == "bl" and ins.operands and ins.operands[0].type == 2:
        target = ins.operands[0].imm
        # Resolve target name
        if target in syscall_stubs:
            num, name = syscall_stubs[target]
            annot = f" -> 0x{target:x} ({name})"
        else:
            annot = f" -> 0x{target:x}"
    print(f"  0x{ins.address:08x}:  {ins.bytes.hex():<10s}  {ins.mnemonic:<8s} {ins.op_str}{annot}")

# Find function containing a given address (look back for prologue)
def find_function_start(addr, max_lookback=0x1000):
    """Walk back to find a stack-saving instruction (function prologue)."""
    cur = addr & ~3
    while cur > text_addr and (addr - cur) < max_lookback:
        # Look for typical function prologue patterns
        instrs = disasm_at(cur, 16)
        if instrs:
            i0 = instrs[0]
            # Look for: stp x29, x30, [sp, #...]! or sub sp, sp, #...
            if i0.mnemonic == "stp" and "x29, x30" in i0.op_str:
                return cur
            if i0.mnemonic == "sub" and "sp, sp" in i0.op_str:
                # Could be prologue
                return cur
            if i0.mnemonic == "stp" and "!" in i0.op_str:
                return cur
        cur -= 4
    return None

# Now find what function calls prctl (0x10028) - we want to see if it's seccomp
print("\n=== Function that calls prctl (0x10028) - seccomp check ===")
prctl_callers = bl_callers.get(0x10028, [])
for caller in prctl_callers[:5]:
    func_start = find_function_start(caller)
    print(f"\n--- prctl caller at 0x{caller:x} (func start ~0x{func_start:x}) ---")
    if func_start:
        for ins in disasm_at(func_start, 0x200):
            annot = ""
            if ins.mnemonic == "bl" and ins.operands and ins.operands[0].type == 2:
                target = ins.operands[0].imm
                if target in syscall_stubs:
                    num, name = syscall_stubs[target]
                    annot = f" -> {name}()"
                else:
                    annot = f" -> 0x{target:x}"
            if ins.address == caller:
                annot += " <-- PRCTL CALL"
            print(f"  0x{ins.address:08x}:  {ins.mnemonic:<8s} {ins.op_str}{annot}")
            if ins.mnemonic == "ret":
                break

print("\n=== Function that calls rt_sigaction (0x135f4) - SIGSYS check ===")
sigaction_callers = bl_callers.get(0x135f4, [])
for caller in sigaction_callers[:5]:
    func_start = find_function_start(caller)
    print(f"\n--- rt_sigaction caller at 0x{caller:x} (func start ~0x{func_start:x}) ---")
    if func_start:
        for ins in disasm_at(func_start, 0x100):
            annot = ""
            if ins.mnemonic == "bl" and ins.operands and ins.operands[0].type == 2:
                target = ins.operands[0].imm
                if target in syscall_stubs:
                    num, name = syscall_stubs[target]
                    annot = f" -> {name}()"
                else:
                    annot = f" -> 0x{target:x}"
            if ins.address == caller:
                annot += " <-- SIGACTION CALL"
            print(f"  0x{ins.address:08x}:  {ins.mnemonic:<8s} {ins.op_str}{annot}")
            if ins.mnemonic == "ret":
                break
