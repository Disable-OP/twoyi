// mount_table.h — Virtual mount table for seccomp/SIGSYS mount() emulation.
//
// This is NOT a fake "return 0" stub. It implements the minimum real
// virtual mount-table semantics demonstrated by the VM reverse-engineering:
//
//   - Records mount entries (source, target, fstype, flags)
//   - Returns EBUSY if target is already mounted with a different source
//   - Returns 0 on success and actually adds the entry
//   - Can be queried by future operations (e.g., /proc/mounts)
//
// Source: VM libkr64.so mount_mgr at 0x8618, which maintains a virtual
// mount table with strings like "mount_mgr: /dev is special, skip",
// "mount_mgr: %s -> %s -> %s", "mount_mgr: bind loop detected %s".
//
// This table is designed to be async-signal-safe (no malloc, no locks,
// fixed-size array) because it's accessed from the SIGSYS handler.

#ifndef TWOYI_LOADER_MOUNT_TABLE_H
#define TWOYI_LOADER_MOUNT_TABLE_H

#include <stdbool.h>
#include <stddef.h>

// Maximum path length for mount entries.
// Linux PATH_MAX is typically 4096, but we use 256 for the fixed-size
// table to keep memory usage reasonable (32 entries * ~600 bytes = ~19KB).
#define TWOYI_MOUNT_PATH_MAX 256

// Maximum filesystem type length.
#define TWOYI_MOUNT_FSTYPE_MAX 64

// Maximum number of mount entries in the table.
#define TWOYI_MOUNT_TABLE_SIZE 32

// Mount flags (subset of Linux MS_* flags, from <sys/mount.h>)
#define TWOYI_MS_RDONLY       1
#define TWOYI_MS_NOSUID       2
#define TWOYI_MS_NODEV        4
#define TWOYI_MS_NOEXEC       8
#define TWOYI_MS_BIND         4096
#define TWOYI_MS_REC          16384
#define TWOYI_MS_REMOUNT      32
// 6-Z214: propagation flags — mount-namespace reconfiguration ops.
#define TWOYI_MS_UNBINDABLE   131072  /* 1 << 17 */
#define TWOYI_MS_PRIVATE      262144  /* 1 << 18 */
#define TWOYI_MS_SLAVE        524288  /* 1 << 19 */
#define TWOYI_MS_SHARED       1048576 /* 1 << 20 */
#define TWOYI_MS_MOVE         8192    /* 1 << 13 */

// A single mount entry in the virtual mount table.
struct twoyi_mount_entry {
    char source[TWOYI_MOUNT_PATH_MAX];  // mount source (e.g., "tmpfs", "proc")
    char target[TWOYI_MOUNT_PATH_MAX];  // mount target (e.g., "/dev", "/proc")
    char fstype[TWOYI_MOUNT_FSTYPE_MAX]; // filesystem type (e.g., "tmpfs", "proc")
    unsigned long flags;                 // mount flags (MS_*)
    bool active;                         // is this slot in use?
};

// Initialize the mount table. Must be called once before any mount()
// emulation. Safe to call from the loader before seccomp is installed.
void twoyi_mount_table_init(void);

// Emulate the mount() syscall with real virtual mount-table semantics.
//
// This function is ASYNC-SIGNAL-SAFE (no malloc, no locks, no stdio).
// It is called from the SIGSYS handler.
//
// Semantics (based on VM mount_mgr + Linux mount(2) man page):
//   1. If target is a "special" path (/dev, /mnt, /storage), skip (no-op, return 0).
//      VM does this — "mount_mgr: /dev is special, skip".
//   2. If target is already in the table:
//      - If MS_REMOUNT flag is set, update the existing entry's flags. Return 0.
//      - Otherwise return -1 with errno=EBUSY.
//   3. If the table is full, return -1 with errno=ENOMEM.
//   4. Add a new entry to the table. Return 0.
//
// Returns: 0 on success, -1 on error (errno set via the return value —
//          the caller writes the return value to ucontext->regs[0]).
//
// Args:
//   source: mount source string (may be NULL for virtual filesystems)
//   target: mount target path (must not be NULL)
//   fstype: filesystem type string (may be NULL for bind mounts)
//   flags:  mount flags bitmask
//   data:   mount data string (ignored for now, future: mode= etc.)
long twoyi_mount_emulate(const char *source, const char *target,
                         const char *fstype, unsigned long flags,
                         const void *data);

// Emulate umount2() — remove a mount entry from the table.
// Returns 0 on success, -1 with errno=EINVAL if target not found.
long twoyi_umount2_emulate(const char *target, int flags);

// Query: check if a target is mounted.
// Returns true if the target has an active mount entry.
bool twoyi_is_mounted(const char *target);

// Query: get the number of active mount entries.
// Used by tests to verify state was updated.
int twoyi_mount_count(void);

// Query: get a mount entry by index (for /proc/mounts emulation).
// Returns NULL if index is out of range or entry is inactive.
const struct twoyi_mount_entry *twoyi_get_mount(int index);

#endif // TWOYI_LOADER_MOUNT_TABLE_H
