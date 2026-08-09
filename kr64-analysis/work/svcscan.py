#!/usr/bin/env python3
"""Find all svc instructions and BL targets in the binary."""
import sys, struct
from collections import Counter, defaultdict
from elftools.elf.elffile import ELFFile
from capstone import Cs, CS_ARCH_ARM64, CS_MODE_ARM

SO = "/home/z/my-project/repos/twoyi/kr64-analysis/libkrloader64.so"
f = open(SO, "rb")
elf = ELFFile(f)

# Get .text section
text = elf.get_section_by_name(".text")
text_addr = text.header.sh_addr
text_size = text.header.sh_size
text_data = text.data()
print(f".text @ 0x{text_addr:x}, size 0x{text_size:x}")

md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
md.detail = True

# Syscall numbers (AArch64)
SYSCALLS = {
    0: "io_setup", 17: "getcwd", 21: "eventfd2", 22: "epoll_create1",
    23: "epoll_ctl", 24: "epoll_pwait", 25: "dup", 29: "dup3",
    32: "faccessat", 33: "chdir", 34: "fchdir", 35: "chroot",
    40: "mount", 41: "umount2", 43: "statfs", 44: "fstatfs",
    46: "ftruncate", 48: "fallocate", 49: "faccessat2", 50: "chdir",
    51: "openat", 52: "close", 53: "vhangup", 54: "pipe2",
    56: "openat", 57: "close", 61: "getdents64", 62: "lseek",
    63: "read", 64: "write", 65: "readv", 66: "writev",
    67: "pread64", 68: "pwrite64", 72: "pselect6", 73: "ppoll",
    78: "readlinkat", 79: "newfstatat", 80: "fstat", 81: "sync",
    82: "fsync", 83: "fdatasync", 84: "sync_file_range",
    86: "timer_settime", 89: "timer_getoverrun",
    95: "exit", 96: "exit_group", 97: "waitid",
    98: "set_tid_address", 99: "unshare", 100: "futex",
    101: "set_robust_list", 102: "get_robust_list",
    113: "clock_gettime", 114: "clock_getres", 115: "clock_nanosleep",
    116: "timerfd_create", 117: "timerfd_settime", 118: "timerfd_gettime",
    124: "rt_sigaction", 125: "rt_sigprocmask", 126: "rt_sigreturn",
    127: "rt_sigpending", 128: "rt_sigtimedwait", 129: "rt_sigqueueinfo",
    130: "rt_sigsuspend",
    134: "rt_sigaction", 135: "rt_sigprocmask",
    137: "rt_sigsuspend", 139: "rt_sigreturn",
    160: "uname", 161: "semget", 162: "semop", 163: "semctl",
    165: "msgget", 166: "msgsnd", 167: "msgrcv", 168: "msgctl",
    170: "shmget", 172: "shmctl",
    174: "getpid", 175: "getppid", 176: "getuid", 177: "geteuid",
    178: "getgid", 179: "getegid", 180: "gettid",
    181: "sysinfo", 182: "mq_open", 183: "mq_unlink",
    193: "shutdown", 195: "bind", 196: "listen", 197: "accept",
    198: "connect", 199: "getsockname", 200: "getpeername",
    201: "sendto", 202: "recvfrom", 203: "setsockopt", 204: "getsockopt",
    205: "shutdown", 206: "sendmsg", 207: "recvmsg",
    208: "accept4", 209: "recvmmsg", 211: "socket", 212: "socketpair",
    213: "bind", 214: "listen", 215: "accept",
    220: "clone", 221: "execve", 222: "mmap", 223: "fadvise64",
    224: "swapon", 225: "swapoff", 226: "mprotect", 227: "msync",
    228: "mlock", 229: "munlock", 230: "mlockall", 231: "munlockall",
    232: "mincore", 233: "madvise", 234: "remap_file_pages",
    235: "mbind", 236: "get_mempolicy", 237: "set_mempolicy",
    238: "migrate_pages", 239: "move_pages",
    260: "wait4", 261: "prlimit64", 262: "fanotify_init",
    263: "fanotify_mark", 264: "name_to_handle_at", 265: "open_by_handle_at",
    266: "clock_adjtime", 267: "syncfs", 268: "setns",
    269: "sendmmsg", 270: "process_vm_readv", 271: "process_vm_writev",
    272: "accept4", 273: "recvmmsg", 274: "sendmmsg",
    277: "seccomp", 278: "getrandom", 279: "memfd_create",
    280: "fexecve", 281: "bpf", 282: "execveat",
    283: "userfaultfd", 284: "membarrier", 285: "mlock2",
    286: "copy_file_range", 287: "preadv2", 288: "pwritev2",
    289: "pkey_mprotect", 290: "pkey_alloc", 291: "pkey_free",
    292: "statx",
    293: "io_pgetevents", 294: "rseq",
    424: "pidfd_send_signal", 425: "io_uring_setup", 426: "io_uring_enter",
    427: "io_uring_register", 428: "open_tree", 429: "move_mount",
    430: "fsopen", 431: "fsconfig", 432: "fsmount", 433: "fspick",
    434: "pidfd_open", 435: "clone3", 436: "close_range", 437: "openat2",
    438: "pidfd_getfd", 439: "faccessat2", 440: "process_madvise",
    441: "epoll_pwait2", 442: "mount_setattr", 443: "quotactl_fd",
    167: "prctl"  # NOT 167 — let me fix
}
# Actually AArch64 prctl is 167 (yes!), but it conflicts with msgsnd. Let me re-check
# AArch64 syscall table: prctl = 167, getcpu = 168
SYSCALLS[167] = "prctl"
SYSCALLS[168] = "getcpu"
SYSCALLS[98] = "futex"  # wait, 98 is set_tid_address, 98 is futex? Let me re-check
# AArch64: futex = 98, set_tid_address = 96, set_robust_list = 99
SYSCALLS[96] = "set_tid_address"
SYSCALLS[97] = "waitid"  # actually wait4=260, waitid=97? Let me use known good values
SYSCALLS[98] = "futex"
SYSCALLS[99] = "set_robust_list"
SYSCALLS[113] = "clock_gettime"
SYSCALLS[114] = "clock_getres"
SYSCALLS[115] = "clock_nanosleep"
SYSCALLS[124] = "rt_sigaction"
SYSCALLS[125] = "rt_sigprocmask"
SYSCALLS[126] = "rt_sigreturn"
SYSCALLS[129] = "rt_sigqueueinfo"
SYSCALLS[130] = "rt_sigsuspend"
SYSCALLS[131] = "rt_sigaction"  # not sure
SYSCALLS[134] = "rt_sigaction"  # don't remember exactly
SYSCALLS[167] = "prctl"
SYSCALLS[220] = "clone"
SYSCALLS[221] = "execve"
SYSCALLS[222] = "mmap"
SYSCALLS[226] = "mprotect"
SYSCALLS[261] = "prlimit64"
SYSCALLS[277] = "seccomp"
SYSCALLS[281] = "bpf"
SYSCALLS[278] = "getrandom"
SYSCALLS[279] = "memfd_create"
SYSCALLS[222] = "mmap"
SYSCALLS[215] = "munmap"
SYSCALLS[215] = "munmap"  # actually 215 is munmap on AArch64
SYSCALLS[56] = "openat"
SYSCALLS[57] = "close"
SYSCALLS[63] = "read"
SYSCALLS[64] = "write"
SYSCALLS[61] = "getdents64"
SYSCALLS[62] = "lseek"
SYSCALLS[78] = "readlinkat"
SYSCALLS[79] = "newfstatat"
SYSCALLS[80] = "fstat"
SYSCALLS[95] = "exit"
SYSCALLS[93] = "exit"  # Actually exit = 93, exit_group = 94 on AArch64
SYSCALLS[93] = "exit"
SYSCALLS[94] = "exit_group"
SYSCALLS[174] = "getpid"
SYSCALLS[175] = "getppid"
SYSCALLS[176] = "getuid"
SYSCALLS[177] = "geteuid"
SYSCALLS[178] = "getgid"
SYSCALLS[179] = "getegid"

# Find svc instructions
svc_locations = []
bl_targets = Counter()
bl_locations = defaultdict(list)

for ins in md.disasm(text_data, text_addr):
    mnem = ins.mnemonic
    if mnem == "svc":
        svc_locations.append(ins.address)
    elif mnem in ("bl","b","blr"):
        if mnem == "blr":
            bl_targets["<indirect>"] += 1
        elif ins.operands and ins.operands[0].type == 2:  # CS_OP_IMM = 2
            target = ins.operands[0].imm
            bl_targets[target] += 1
            bl_locations[target].append(ins.address)

print(f"\n=== SVC (syscall) instructions found: {len(svc_locations)} ===")
for addr in svc_locations[:200]:
    print(f"  0x{addr:08x}: svc #0")

print(f"\n=== Top BL/B targets (called functions) ===")
for target, count in sorted(bl_targets.items(), key=lambda x: -x[1])[:40]:
    if target == "<indirect>":
        print(f"  {count:4d}x  indirect (blr)")
    else:
        print(f"  {count:4d}x  0x{target:08x}  (callers: {bl_locations[target][:5]}{'...' if len(bl_locations[target])>5 else ''})")
