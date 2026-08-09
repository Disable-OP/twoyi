// test_guest.c — Test program for the seccomp/SIGSYS vertical slice.
//
// This program is compiled as a single static binary that:
//   1. Has _start as the entry point (assembly stub)
//   2. _start calls twoyi_loader_main() to install seccomp + SIGSYS
//   3. twoyi_loader_main() returns, _start calls main()
//   4. main() calls mount() and verifies the SIGSYS handler emulated it
//
// This tests the full vertical slice:
//   seccomp install → SIGSYS handler → syscall identification →
//   argument decoding → mount table update → return value emulation →
//   execution continues
//
// NOTE: In the real Twoyi system, the loader and guest are separate
// binaries (PT_INTERP). For this test, we combine them into one binary
// to share the address space (so main() can query the mount table).
// The seccomp/SIGSYS mechanism is identical either way.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <unistd.h>
#include <sys/mount.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <sys/stat.h>
#include <signal.h>

#include "sigsys_handler.h"
#include "mount_table.h"
#include "seccomp_filter.h"

static int test_count = 0;
static int test_passed = 0;

#define TEST(name) do { \
    test_count++; \
    printf("[TEST %d] %s ... ", test_count, name); \
    fflush(stdout); \
} while(0)

#define PASS() do { \
    printf("PASS\n"); \
    fflush(stdout); \
    test_passed++; \
} while(0)

#define FAIL(fmt, ...) do { \
    printf("FAIL: " fmt "\n", ##__VA_ARGS__); \
    fflush(stdout); \
} while(0)

int main(void) {
    printf("=== Twoyi Vertical Slice Test ===\n");
    printf("Architecture: %s\n",
#ifdef __aarch64__
        "arm64-v8a"
#elif defined(__x86_64__)
        "x86_64"
#else
        "unknown"
#endif
    );
    printf("PID: %d\n", getpid());
    printf("\n");

    // ---- Test 1: Trapped syscall reaches handler ----
    // mount() should be trapped by seccomp and handled by SIGSYS handler.
    unsigned int count_before = twoyi_sigsys_get_invoke_count();

    TEST("1. trapped syscall reaches handler");
    int ret = mount("tmpfs", "/test_twoyi_dev", "tmpfs", MS_NOSUID, "mode=0755");

    unsigned int count_after = twoyi_sigsys_get_invoke_count();
    if (ret == 0 && count_after == count_before + 1) {
        PASS();
    } else {
        FAIL("ret=%d errno=%d invoke_count %u->%u", ret, errno, count_before, count_after);
    }

    // ---- Test 2: Arguments decoded correctly ----
    // Verify the handler identified mount() (syscall number).
    TEST("2. syscall identified correctly");
    long last_nr = twoyi_sigsys_get_last_syscall_nr();
#ifdef __aarch64__
    int expected_nr = 40;  // mount on arm64
#elif defined(__x86_64__)
    int expected_nr = 165; // mount on x86_64
#endif
    if (last_nr == expected_nr) {
        PASS();
    } else {
        FAIL("expected syscall nr %d, got %ld", expected_nr, last_nr);
    }

    // ---- Test 3: Emulated state is updated ----
    TEST("3. mount table state updated");
    if (twoyi_is_mounted("/test_twoyi_dev")) {
        PASS();
    } else {
        FAIL("/test_twoyi_dev not in mount table");
    }

    // ---- Test 4: Subsequent operations observe that state ----
    // A second mount on the same target should return EBUSY.
    TEST("4. duplicate mount returns EBUSY");
    errno = 0;
    ret = mount("tmpfs", "/test_twoyi_dev", "tmpfs", 0, NULL);
    if (ret == -1 && errno == EBUSY) {
        PASS();
    } else {
        FAIL("expected ret=-1 errno=EBUSY(%d), got ret=%d errno=%d", EBUSY, ret, errno);
    }

    // ---- Test 5: Return values match Linux semantics ----
    TEST("5. fresh mount returns 0 (success)");
    ret = mount("proc", "/test_twoyi_proc", "proc", 0, NULL);
    if (ret == 0 && twoyi_is_mounted("/test_twoyi_proc")) {
        PASS();
    } else {
        FAIL("ret=%d errno=%d", ret, errno);
    }

    // ---- Test 5b: Mount table entry has correct data ----
    TEST("5b. mount entry has correct source/target/fstype");
    int found = 0;
    for (int i = 0; i < 32; i++) {
        const struct twoyi_mount_entry *e = twoyi_get_mount(i);
        if (e && strcmp(e->target, "/test_twoyi_proc") == 0) {
            if (strcmp(e->source, "proc") == 0 && strcmp(e->fstype, "proc") == 0) {
                found = 1;
            }
            break;
        }
    }
    if (found) {
        PASS();
    } else {
        FAIL("mount entry not found or has wrong data");
    }

    // ---- Test 6: Non-trapped syscalls continue normally ----
    TEST("6. non-trapped syscall (getpid) works");
    pid_t pid = getpid();
    if (pid > 0) {
        PASS();
    } else {
        FAIL("getpid returned %d", pid);
    }

    // ---- Test 6b: Non-trapped syscall (read/write) works ----
    // Verify that the BPF filter allows non-trapped syscalls to pass through.
    // read() and write() should work normally (not trigger SIGSYS).
    TEST("6b. non-trapped syscalls (read/write) work");
    {
        int pipefd[2];
        int pipe_ret = pipe(pipefd);
        if (pipe_ret != 0) {
            FAIL("pipe failed: %s", strerror(errno));
        } else {
            const char *msg = "hello";
            ssize_t wret = write(pipefd[1], msg, 5);
            char buf[16] = {0};
            ssize_t rret = read(pipefd[0], buf, 5);
            close(pipefd[0]);
            close(pipefd[1]);
            if (wret == 5 && rret == 5 && memcmp(buf, "hello", 5) == 0) {
                PASS();
            } else {
                FAIL("write=%zd read=%zd buf=%.*s", wret, rret, 5, buf);
            }
        }
    }

    // ---- Test 6c: Non-trapped syscall (stat) works ----
    TEST("6c. non-trapped syscall (stat) works");
    {
        struct stat st;
        int stat_ret = stat("/proc/self", &st);
        if (stat_ret == 0 && S_ISDIR(st.st_mode)) {
            PASS();
        } else {
            FAIL("stat returned %d errno=%d", stat_ret, errno);
        }
    }

    // ---- Test 7: Child processes preserve virtualization ----
    // The seccomp filter is inherited across fork(). The mount table is
    // per-process (fork copies memory), so the child has its OWN copy.
    // We verify that the child's mount() is still trapped (returns 0,
    // not EPERM), proving the seccomp filter was inherited.
    TEST("7. child process inherits seccomp filter");
    pid_t child = fork();
    if (child < 0) {
        FAIL("fork failed: %s", strerror(errno));
    } else if (child == 0) {
        // Child: seccomp filter is inherited from parent.
        // mount() should be trapped and emulated (return 0), not EPERM.
        int child_ret = mount("sysfs", "/test_twoyi_sys", "sysfs", 0, NULL);
        if (child_ret != 0) {
            // If mount returned -1 with errno=EPERM, the seccomp filter
            // was NOT inherited (the real syscall reached the kernel).
            fprintf(stderr, "CHILD: mount failed ret=%d errno=%d\n", child_ret, errno);
            _exit(1);
        }
        // Verify the mount was recorded in the child's mount table.
        if (!twoyi_is_mounted("/test_twoyi_sys")) {
            fprintf(stderr, "CHILD: /test_twoyi_sys not in mount table\n");
            _exit(2);
        }
        _exit(0); // success
    } else {
        int status;
        waitpid(child, &status, 0);
        if (WIFEXITED(status) && WEXITSTATUS(status) == 0) {
            // Child successfully had mount() trapped and emulated.
            // This proves the seccomp filter was inherited.
            // Note: the child's mount table is a COPY of the parent's
            // (fork uses copy-on-write), so the parent can't see the
            // child's mount. That's expected — in production, the mount
            // table would be in shared memory (MAP_SHARED mmap).
            PASS();
        } else {
            FAIL("child exited with status %d (seccomp not inherited?)",
                 WEXITSTATUS(status));
        }
    }

    // ---- Test 8: umount2 removes from mount table ----
    TEST("8. umount2 removes mount entry");
    ret = umount2("/test_twoyi_proc", 0);
    if (ret == 0 && !twoyi_is_mounted("/test_twoyi_proc")) {
        PASS();
    } else {
        FAIL("ret=%d errno=%d, still mounted=%d", ret, errno,
             twoyi_is_mounted("/test_twoyi_proc"));
    }

    // ---- Test 9: chroot returns 0 (emulated as no-op) ----
    TEST("9. chroot returns 0 (emulated)");
    ret = chroot("/test_twoyi_dev");
    if (ret == 0) {
        PASS();
    } else {
        FAIL("expected ret=0, got ret=%d errno=%d", ret, errno);
    }

    // ---- Summary ----
    printf("\n=== Results: %d/%d tests passed ===\n", test_passed, test_count);
    fflush(stdout);

    if (test_passed == test_count) {
        printf("ALL TESTS PASSED\n");
        fflush(stdout);
        return 0;
    } else {
        printf("SOME TESTS FAILED\n");
        fflush(stdout);
        return 1;
    }
}
