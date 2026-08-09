#!/usr/bin/env python3
"""
Faster XOR scan for .text — only scan with key 0 (no XOR) and a few common keys,
looking for short substrings. The .text section is large, so use a simple approach.
"""
import sys
import re

# Common XOR keys to try (in addition to 0)
# These are based on what we found in .data:
#   0x00 (no xor), 0x03, 0x05, 0x0a, 0x0c, 0x0d, 0x15, 0x17, 0x19, 0x1a, 0x1c,
#   0x1d, 0x1e, 0x1f, 0x21, 0x25, 0x27, 0x2a, 0x2c, 0x2d, 0x2e, 0x32, 0x33,
#   0x34, 0x37, 0x38, 0x3c, 0x3d, 0x3e, 0x44, 0x47, 0x48, 0x4f, 0x55, 0x56,
#   0x5a, 0x5d, 0x5e, 0x63, 0x64, 0xa1, 0xa2, 0xa3, 0xa6, 0xa7, 0xb0, 0xb1,
#   0xba, 0xbb, 0xbc, 0xbd, 0xc2, 0xc7, 0xc8, 0xca, 0xcb, 0xd2, 0xd4, 0xd9,
#   0xda, 0xdb, 0xdc, 0xe2, 0xe3, 0xe7, 0xe8, 0xee, 0xf1, 0xf3, 0xf4, 0xf7,
#   0xf8, 0xfc, 0xfe, 0xff

# Specific targets relevant to libkr64.so's job
TARGETS = [
    b"binder", b"BINDER", b"Binder",
    b"/dev/binder", b"/vm%d/dev/binder",
    b"BINDER_WRITE_READ", b"BINDER_VERSION",
    b"BINDER_SET_MAX_THREADS", b"BINDER_SET_CONTEXT_MGR",
    b"BINDER_TYPE_BINDER", b"BINDER_TYPE_HANDLE",
    b"BINDER_TYPE_WEAK_BINDER", b"BINDER_TYPE_WEAK_HANDLE",
    b"servicemanager", b"ServiceManager",
    b"IBinder", b"Parcel", b"BpBinder", b"BBinder",
    b"IInterface", b"IActivityManager",
    b"transaction", b"transact",
    b"flat_binder", b"binder_write_read",
    b"/dev/vndbinder", b"/dev/hwbinder",
    # Graphics
    b"/dev/gb", b"/dev/gb2",
    b"gralloc", b"GraphicBuffer",
    b"graphic_buffer", b"graphics_buffer",
    b"gb_device", b"gb2_device",
    b"gb_open", b"gb_close", b"gb_ioctl",
    b"gb2_open", b"gb2_close", b"gb2_ioctl",
    # ion
    b"ION", b"ion", b"ion_alloc", b"ion_free",
    b"ion_ioctl", b"ion_handle",
    # Binder protocol
    b"BC_TRANSACTION", b"BC_REPLY", b"BC_ACQUIRE",
    b"BC_RELEASE", b"BC_INCREFS", b"BC_DECREFS",
    b"BR_TRANSACTION", b"BR_REPLY", b"BR_DEAD_REPLY",
    b"BR_NOOP", b"BR_SPAWN_LOOPER",
    # HAL
    b"hw_get_module", b"hw_module",
    # Hidl
    b"hidl", b"Hidl",
    # Various
    b"app_process", b"app_process64",
    b"zygote", b"Zygote", b"ZygoteInit",
    b"system_server", b"SystemServer",
    b"init.rc", b"init.",
    b"/init", b"/init.rc",
    b"ueventd", b"vold", b"surfaceflinger",
    b" SurfaceFlinger",
    # Process management
    b"forkAndSpecialize",
    b"nativeForkAndSpecialize",
    # Mounts
    b"/proc/self/mounts", b"/proc/mounts",
    b"bind mount", b"tmpfs",
    # Multi-VM
    b"/vm%d/", b"vm%d", b"vmid", b"VM_ID",
    # Seccomp
    b"seccomp", b"SECCOMP",
    # Boot
    b"boot", b"BOOT",
    b"BOOT_COMPLETED", b"SHUTDOWN",
    b"init.svc.", b"sys.boot_completed",
    # Properties
    b"property_get", b"property_set",
    b"__system_property_get",
    # Sockets
    b"socketpair", b"socket", b"AF_UNIX", b"AF_INET",
    # Logging
    b"__android_log_print",
    b"krlog", b"kmsg",
    # Hooks
    b"shadowhook", b"shadowhook_",
    b"hook", b"Hook",
    # Misc
    b"VMINIT_PID", b"vminit.pid",
    b"VM_", b"VM-", b"vm_",
    # netlink
    b"netlink", b"netdevice",
    # Network namespaces
    b"net.ns", b"netns",
    # Touch input
    b"/dev/input/touch", b"uinput",
    # Audio
    b"/dev/audio", b"/dev/snd",
    # Camera
    b"/dev/video",
    # Drm
    b"/dev/dri",
    # Display
    b"/dev/graphics",
    b"/dev/fb",
    # Sockets/IPC
    b"/dev/socket",
    # Loader-related
    b"linker", b"dlopen", b"dlsym",
    b"soinfo", b"__dl__",
    # Property area
    b"__properties__",
    # Service manager
    b"service_manager", b"ServiceManager",
    # Kernel-related
    b"sys_call", b"syscall",
    # Common error patterns
    b"setup_binder", b"setup_gb", b"setup_touch",
    b"setup_audio", b"setup_input",
    b"setupBinder", b"setupGb", b"setupTouch",
    # Misc
    b"VMManager", b"VMInstance",
    # /dev/qemu_pipe
    b"qemu_pipe", b"GLTransport",
    # OpenGL
    b"OpenGL", b"opengl",
    # /system/lib64
    b"/system/lib64/", b"/system/lib/",
    # System path
    b"/system/bin/", b"/system/xbin/",
    # Various
    b"exit", b"abort",
    b"fork", b"vfork", b"clone",
    # syscall numbers
    b"syscall(", b"syscall_",
]

def find_strings(data, key):
    decoded = bytes(b ^ key for b in data)
    hits = []
    for target in TARGETS:
        idx = 0
        while True:
            pos = decoded.find(target, idx)
            if pos == -1:
                break
            start = max(0, pos - 15)
            end = min(len(decoded), pos + len(target) + 50)
            ctx = decoded[start:end]
            printable = ''.join(chr(c) if 32 <= c < 127 else '.' for c in ctx)
            hits.append((target.decode('latin-1'), pos, printable))
            idx = pos + 1
            if len(hits) > 30:
                break
        if len(hits) > 30:
            break
    return hits

def main():
    fname = sys.argv[1]
    keys = [int(x, 0) for x in sys.argv[2:]] if len(sys.argv) > 2 else list(range(256))
    with open(fname, 'rb') as f:
        data = f.read()
    print(f"=== XOR scan {fname} ({len(data)} bytes) ===")
    for key in keys:
        hits = find_strings(data, key)
        if hits:
            print(f"\n--- Key 0x{key:02x} ({key}) — {len(hits)} hits ---")
            for target, pos, ctx in hits:
                print(f"  @0x{pos:08x} [{target:30s}] {ctx}")

if __name__ == "__main__":
    main()
