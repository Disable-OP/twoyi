// 6-Z214 regression test: mount-table semantics for propagation ops.
//
// Reproduces the OrangeFox R12.1 lavender / Lineage 22.2 init
// SetupMountNamespaces blocker: mount(nullptr, "/apex", nullptr,
// MS_PRIVATE) on an ALREADY-RECORDED /apex target used to return
// -EBUSY (only MS_REMOUNT was special-cased), which init treats as a
// FATAL SetupMountNamespaces failure → InitFatalReboot → exit 127.
//
// Build + run (host gcc, no NDK needed — mount_table.c is portable):
//   gcc -D_GNU_SOURCE -I../include -o /tmp/test_mount_table \
//       test_mount_table.c mount_table.c && /tmp/test_mount_table

#include <stdio.h>
#include <string.h>
#include <errno.h>
#include "mount_table.h"

static int failures = 0;

#define CHECK(desc, expr) do { \
    if (!(expr)) { \
        printf("FAIL: %s\n", desc); \
        failures++; \
    } else { \
        printf("ok:   %s\n", desc); \
    } \
} while (0)

int main(void) {
    twoyi_mount_table_init();

    // 1. First mount of /apex (what an earlier guest init phase does —
    //    e.g. first-stage flattened-APEX setup) — recorded, success.
    long r1 = twoyi_mount_emulate("tmpfs", "/apex", "tmpfs", 0, NULL);
    CHECK("initial mount of /apex succeeds", r1 == 0);
    CHECK("/apex is mounted", twoyi_is_mounted("/apex"));

    // 2. THE REGRESSION: SetupMountNamespaces propagation op on the
    //    already-recorded target. Old code: -EBUSY → init aborts.
    long r2 = twoyi_mount_emulate(NULL, "/apex", NULL, TWOYI_MS_PRIVATE, NULL);
    CHECK("MS_PRIVATE on already-mounted /apex returns 0 (was EBUSY)", r2 == 0);

    // 3. MS_PRIVATE | MS_REC — the recursive variant init actually uses.
    long r3 = twoyi_mount_emulate(NULL, "/apex", NULL,
                                  TWOYI_MS_PRIVATE | TWOYI_MS_REC, NULL);
    CHECK("MS_PRIVATE|MS_REC on already-mounted /apex returns 0", r3 == 0);

    // 4. MS_SLAVE / MS_SHARED / MS_UNBINDABLE propagation ops.
    CHECK("MS_SLAVE on mounted target returns 0",
          twoyi_mount_emulate(NULL, "/apex", NULL, TWOYI_MS_SLAVE, NULL) == 0);
    CHECK("MS_SHARED on mounted target returns 0",
          twoyi_mount_emulate(NULL, "/apex", NULL, TWOYI_MS_SHARED, NULL) == 0);
    CHECK("MS_UNBINDABLE on mounted target returns 0",
          twoyi_mount_emulate(NULL, "/apex", NULL, TWOYI_MS_UNBINDABLE, NULL) == 0);

    // 5. MS_REMOUNT — old behaviour kept.
    CHECK("MS_REMOUNT on mounted target returns 0",
          twoyi_mount_emulate(NULL, "/apex", NULL, TWOYI_MS_REMOUNT, NULL) == 0);

    // 6. Bind-mount ONTO an already-mounted target — stacked binds are
    //    legal Linux semantics (AOSP init MountDir does mkdir + MS_BIND).
    long r6 = twoyi_mount_emulate("/some/source", "/apex", NULL,
                                  TWOYI_MS_BIND, NULL);
    CHECK("MS_BIND onto already-mounted target returns 0 (was EBUSY)", r6 == 0);

    // 7. Self-bind loop detection kept.
    long r7 = twoyi_mount_emulate("/apex", "/apex", NULL, TWOYI_MS_BIND, NULL);
    CHECK("self-bind loop still returns -EINVAL", r7 == -EINVAL);

    // 8. Plain duplicate mount (no bind, no propagation) — real kernel
    //    returns EBUSY; semantics kept.
    long r8 = twoyi_mount_emulate("tmpfs", "/apex", "tmpfs", 0, NULL);
    CHECK("plain duplicate mount still returns -EBUSY", r8 == -EBUSY);

    // 9. Special paths unchanged.
    CHECK("/dev mount is a no-op success",
          twoyi_mount_emulate("tmpfs", "/dev", "tmpfs", 0, NULL) == 0);

    // 10. umount then re-mount.
    CHECK("umount2 of mounted target returns 0",
          twoyi_umount2_emulate("/apex", 0) == 0);
    CHECK("re-mount after umount returns 0",
          twoyi_mount_emulate("tmpfs", "/apex", "tmpfs", 0, NULL) == 0);

    // 11. Propagation op on a NOT-yet-mounted target: lenient virtualized
    //     success (recorded) — init must never wedge on it.
    long r11 = twoyi_mount_emulate(NULL, "/cache", NULL, TWOYI_MS_PRIVATE, NULL);
    CHECK("MS_PRIVATE on unmounted target returns 0 (lenient)", r11 == 0);

    if (failures) {
        printf("\n%d FAILURE(S)\n", failures);
        return 1;
    }
    printf("\nall mount-table 6-Z214 semantics pass\n");
    return 0;
}
