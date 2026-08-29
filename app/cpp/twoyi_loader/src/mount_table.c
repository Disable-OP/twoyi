// mount_table.c — Virtual mount table for seccomp/SIGSYS mount() emulation.
//
// Implements REAL virtual mount-table semantics, not a fake "return 0".
//
// Based on VM libkr64.so mount_mgr (at 0x8618) which:
//   - Maintains a virtual mount table
//   - Handles special paths (/dev, /mnt, /storage) as no-ops
//   - Returns 0 on success, actually records the mount
//   - Detects bind mount loops
//   - Supports remount (MS_REMOUNT)
//
// This implementation is async-signal-safe: no malloc, no locks, no stdio.
// The table is a fixed-size array with a simple linear scan.

#include "mount_table.h"

#include <errno.h>
#include <string.h>

// Global mount table (static storage, zero-initialized).
// This is shared between the loader and the SIGSYS handler.
static struct twoyi_mount_entry mount_table[TWOYI_MOUNT_TABLE_SIZE];

// Special paths that VM's mount_mgr skips (no-op, return 0).
// Source: VM strings "mount_mgr: /dev is special, skip", etc.
static const char *special_paths[] = {
    "/dev",
    "/mnt",
    "/storage",
    NULL
};

// Async-signal-safe string copy with length limit.
static void safe_strncpy(char *dst, const char *src, size_t n) {
    if (!src) {
        dst[0] = '\0';
        return;
    }
    size_t i;
    for (i = 0; i < n - 1 && src[i]; i++) {
        dst[i] = src[i];
    }
    dst[i] = '\0';
}

// 6-Z214: mount-PROPAGATION / REMOUNT / MOVE flag detection — the SIGSYS
// mirror of the PLT-interposer fix in twoyi_loader_shlib.c (same root
// cause: AOSP init SetupMountNamespaces does mount(nullptr, "/apex",
// nullptr, MS_PRIVATE) on an already-recorded target; the old emulation
// returned -EBUSY for it because only MS_REMOUNT was special-cased).
// Propagation ops reconfigure an EXISTING mount — they never create a
// duplicate — so the "duplicate mount → -EBUSY" path must NOT fire.
static int is_propagation_or_remount(unsigned long flags) {
    const unsigned long PROP_MASK = TWOYI_MS_REMOUNT | TWOYI_MS_MOVE |
                                    TWOYI_MS_UNBINDABLE | TWOYI_MS_PRIVATE |
                                    TWOYI_MS_SLAVE | TWOYI_MS_SHARED;
    return (flags & PROP_MASK) != 0;
}

// Check if a path is in the special_paths list.
static bool is_special_path(const char *target) {
    if (!target) return false;
    for (int i = 0; special_paths[i]; i++) {
        // Exact match or path prefix match (target/...)
        const char *sp = special_paths[i];
        size_t splen = strlen(sp);
        if (strncmp(target, sp, splen) == 0) {
            // Match is either exact or target has '/' after the prefix
            if (target[splen] == '\0' || target[splen] == '/') {
                return true;
            }
        }
    }
    return false;
}

// Find a mount entry by target path. Returns index or -1 if not found.
static int find_mount_by_target(const char *target) {
    if (!target) return -1;
    for (int i = 0; i < TWOYI_MOUNT_TABLE_SIZE; i++) {
        if (mount_table[i].active &&
            strncmp(mount_table[i].target, target, TWOYI_MOUNT_PATH_MAX) == 0) {
            return i;
        }
    }
    return -1;
}

// Find a free slot in the mount table. Returns index or -1 if full.
static int find_free_slot(void) {
    for (int i = 0; i < TWOYI_MOUNT_TABLE_SIZE; i++) {
        if (!mount_table[i].active) {
            return i;
        }
    }
    return -1;
}

void twoyi_mount_table_init(void) {
    // Zero the entire table. This is safe because static storage is
    // already zero-initialized, but we call this explicitly for clarity.
    memset(mount_table, 0, sizeof(mount_table));
}

long twoyi_mount_emulate(const char *source, const char *target,
                         const char *fstype, unsigned long flags,
                         const void *data) {
    (void)data; // mount data not yet implemented (future: parse mode= for tmpfs)

    // Validate target — mount(2) returns EFAULT for NULL target.
    if (!target) {
        return -EFAULT;
    }

    // Check special paths — VM's mount_mgr skips these.
    // This matches VM behavior: "mount_mgr: /dev is special, skip".
    // The guest's init mounts tmpfs on /dev, but we already have /dev
    // populated by kr64's device creation, so we skip the actual mount.
    if (is_special_path(target)) {
        return 0;
    }

    // Check if target is already mounted.
    int existing = find_mount_by_target(target);
    if (existing >= 0) {
        // 6-Z214: propagation / remount / move ops reconfigure the
        // EXISTING entry — update flags and succeed. These are NOT
        // duplicate mounts: AOSP init's SetupMountNamespaces issues
        // mount(nullptr, "/apex", nullptr, MS_PRIVATE) on an
        // already-recorded /apex, and the old code returned -EBUSY,
        // which init treats as a FATAL SetupMountNamespaces failure
        // (InitFatalReboot — the r14-r25 OrangeFox/Lineage blocker).
        if (is_propagation_or_remount(flags)) {
            mount_table[existing].flags = flags;
            return 0;
        }
        // Bind-mount ONTO an already-mounted target is legal Linux
        // semantics (stacked bind mounts — AOSP init's MountDir does
        // mkdir + MS_BIND onto live dirs). Virtualize as success.
        if (flags & TWOYI_MS_BIND) {
            if (source && strncmp(source, target, TWOYI_MOUNT_PATH_MAX) == 0) {
                return -EINVAL; // self-bind loop detected (unchanged)
            }
            mount_table[existing].flags = flags;
            return 0;
        }
        // Plain (non-bind, non-propagation) re-mount of a live target:
        // real kernel returns EBUSY — keep that semantic.
        return -EBUSY;
    }

    // Find a free slot in the table.
    int slot = find_free_slot();
    if (slot < 0) {
        return -ENOMEM; // table full
    }

    // Record the mount entry.
    safe_strncpy(mount_table[slot].source, source, TWOYI_MOUNT_PATH_MAX);
    safe_strncpy(mount_table[slot].target, target, TWOYI_MOUNT_PATH_MAX);
    safe_strncpy(mount_table[slot].fstype, fstype, TWOYI_MOUNT_FSTYPE_MAX);
    mount_table[slot].flags = flags;
    mount_table[slot].active = true;

    return 0; // success
}

long twoyi_umount2_emulate(const char *target, int flags) {
    (void)flags; // MNT_FORCE, MNT_DETACH not yet implemented

    if (!target) {
        return -EFAULT;
    }

    int idx = find_mount_by_target(target);
    if (idx < 0) {
        return -EINVAL; // not mounted
    }

    // Mark the slot as inactive.
    mount_table[idx].active = false;
    mount_table[idx].source[0] = '\0';
    mount_table[idx].target[0] = '\0';
    mount_table[idx].fstype[0] = '\0';
    mount_table[idx].flags = 0;

    return 0;
}

bool twoyi_is_mounted(const char *target) {
    return find_mount_by_target(target) >= 0;
}

int twoyi_mount_count(void) {
    int count = 0;
    for (int i = 0; i < TWOYI_MOUNT_TABLE_SIZE; i++) {
        if (mount_table[i].active) {
            count++;
        }
    }
    return count;
}

const struct twoyi_mount_entry *twoyi_get_mount(int index) {
    if (index < 0 || index >= TWOYI_MOUNT_TABLE_SIZE) {
        return NULL;
    }
    if (!mount_table[index].active) {
        return NULL;
    }
    return &mount_table[index];
}
