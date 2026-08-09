#!/usr/bin/env python3
"""
XOR brute-force all 256 single-byte keys against a binary file,
looking for known substrings.
"""
import sys
import os
import re

# Interesting substrings — case-insensitive where appropriate
TARGETS = [
    # Binder / IPC
    b"/dev/binder", b"/vm%d/dev/binder", b"/dev/binder/vm",
    b"binder", b"BINDER_WRITE_READ", b"BINDER_VERSION",
    b"BINDER_SET_MAX_THREADS", b"servicemanager", b"ServiceManager",
    b"IBinder", b"Parcel", b"BpBinder", b"BBinder",
    b"/dev/vndbinder", b"/dev/hwbinder",
    # Graphics
    b"/dev/gb", b"/dev/gb2", b"/dev/graphics",
    b"gralloc", b"GraphicBuffer", b"ION", b"ion",
    b"qemu_pipe", b"/dev/qemu_pipe", b"opengl", b"OpenGL",
    b"/dev/dri", b"/dev/fb",
    # Input / audio
    b"/dev/input", b"/dev/input/touch", b"uinput", b"evdev",
    b"/dev/audio", b"/dev/snd",
    # Devices
    b"/dev/socket", b"/dev/__properties__", b"/dev/null",
    b"/dev/random", b"/dev/urandom", b"/dev/ashmem",
    b"/dev/__null__", b"/dev/properties",
    # Android paths
    b"/system/", b"/vendor/", b"/data/", b"/proc/",
    b"/sys/", b"/dev/", b"/tmp/", b"/mnt/",
    b"android.app.IActivityManager", b"android.os.",
    b"package", b"android.content.",
    # Common error / log tags
    b"failed", b"Failed", b"FAILED",
    b"error", b"Error", b"ERROR",
    b"init", b"Init", b"INIT",
    b"libkr", b"libvm", b"libbinder",
    # Syscalls / Linux
    b"ioctl", b"mmap", b"socket", b"bind", b"listen", b"accept",
    b"connect", b"socketpair", b"mknod", b"mknodat",
    b"open", b"close", b"read", b"write",
    b"fork", b"vfork", b"clone", b"execve",
    b"ptrace", b"seccomp", b"prctl",
    # Loader-related
    b"linker", b"dlopen", b"dlsym", b"dlclose",
    b"__dl__", b"soinfo",
    # Properties
    b"ro.build.", b"persist.", b"sys.",
    # Misc
    b"VMT", b"VM-", b"vm-", b"vmsvc", b"krservice",
    b"krloader", b"kr64",
    b"property", b"prop",
    b"shmem", b"ashmem",
    b"tmpfs", b"ramfs",
    b"mount", b"umount",
    b"chroot", b"pivot_root",
    # Magic numbers / signatures
    b"\\x7fELF", b"\\.so", b"\\.img",
    # /proc
    b"/proc/self/", b"/proc/%d/", b"/proc/self/maps",
    b"/proc/self/exe", b"/proc/self/cmdline",
    b"/proc/self/status", b"/proc/self/auxv",
    # Event socket (from Java analysis)
    b"/dev/event", b"event",
    # Boot / shutdown
    b"BOOT_COMPLETED", b"SHUTDOWN", b"init.svc.",
    # Generic
    b"cannot ", b"unable to ",
    b"signal", b"SIGCHLD",
    b"thread", b"Thread", b"pthread",
    b"mutex", b"Mutex",
]

def find_strings(data, key):
    """XOR data with key, find target substrings."""
    decoded = bytes(b ^ key for b in data)
    hits = []
    for target in TARGETS:
        idx = 0
        while True:
            pos = decoded.find(target, idx)
            if pos == -1:
                break
            # Get some context: ±20 bytes
            start = max(0, pos - 20)
            end = min(len(decoded), pos + len(target) + 40)
            ctx = decoded[start:end]
            # Clean: replace non-printable with dots
            printable = ''.join(chr(c) if 32 <= c < 127 else '.' for c in ctx)
            hits.append((target.decode('latin-1'), pos, printable))
            idx = pos + 1
    return hits

def main():
    if len(sys.argv) < 2:
        print("Usage: xor_brute.py <file>")
        sys.exit(1)
    fname = sys.argv[1]
    with open(fname, 'rb') as f:
        data = f.read()
    print(f"=== XOR brute-force of {fname} ({len(data)} bytes) ===")
    for key in range(256):
        hits = find_strings(data, key)
        if hits:
            print(f"\n--- Key 0x{key:02x} ({key}) — {len(hits)} hits ---")
            for target, pos, ctx in hits[:80]:
                print(f"  @0x{pos:08x} [{target:30s}] {ctx}")
            if len(hits) > 80:
                print(f"  ... and {len(hits)-80} more")

if __name__ == "__main__":
    main()
