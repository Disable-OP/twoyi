// test_guest.c — Test program for the seccomp/SIGSYS virtualization.
//
// This test verifies the full vertical slice:
//   1. Trapped syscall reaches handler
//   2. Arguments decoded correctly
//   3. Emulated state is updated
//   4. Subsequent operations observe that state
//   5. Return values match Linux semantics
//   6. Non-trapped syscalls continue normally
//   7. Child processes preserve virtualization state

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <unistd.h>
#include <sys/mount.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <signal.h>
#include <fcntl.h>

// Extern declarations from twoyi_loader.c
extern volatile int g_sigsys_count;
extern volatile int g_runtime_ready;
extern struct mount_entry {
    char source[256];
    char target[256];
    char fstype[64];
    unsigned long flags;
    int active;
} g_mount_table[];
extern const char *g_rootfs_prefix;

// Test helpers
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

// Check if a path is in the mount table
static int is_mounted(const char *target) {
    for (int i = 0; i < 32; i++) {
        if (g_mount_table[i].active &&
            strncmp(g_mount_table[i].target, target, 256) == 0) {
            return 1;
        }
    }
    return 0;
}

int main(void) {
    printf("=== Twoyi Virtualization Test ===\n");
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
    printf("Runtime ready: %d\n", g_runtime_ready);
    printf("Sigsys count: %d\n", g_sigsys_count);
    printf("\n");

    // ---- Test 1: Trapped syscall reaches handler ----
    int count_before = g_sigsys_count;
    TEST("1. trapped syscall reaches handler");
    int ret = mount("tmpfs", "/test_twoyi_dev", "tmpfs", MS_NOSUID, "mode=0755");
    int count_after = g_sigsys_count;
    if (ret == 0 && count_after > count_before) {
        PASS();
    } else {
        FAIL("ret=%d errno=%d count %d->%d", ret, errno, count_before, count_after);
    }

    // ---- Test 2: Mount table state updated ----
    TEST("2. mount table state updated");
    if (is_mounted("/test_twoyi_dev")) {
        PASS();
    } else {
        FAIL("/test_twoyi_dev not in mount table");
    }

    // ---- Test 3: Duplicate mount returns EBUSY ----
    TEST("3. duplicate mount returns EBUSY");
    errno = 0;
    ret = mount("tmpfs", "/test_twoyi_dev", "tmpfs", 0, NULL);
    if (ret == -1 && errno == EBUSY) {
        PASS();
    } else {
        FAIL("expected ret=-1 errno=EBUSY, got ret=%d errno=%d", ret, errno);
    }

    // ---- Test 4: Fresh mount succeeds ----
    TEST("4. fresh mount returns 0");
    ret = mount("proc", "/test_twoyi_proc", "proc", 0, NULL);
    if (ret == 0 && is_mounted("/test_twoyi_proc")) {
        PASS();
    } else {
        FAIL("ret=%d errno=%d", ret, errno);
    }

    // ---- Test 5: Non-trapped syscall (getpid) works ----
    TEST("5. non-trapped syscall (getpid) works");
    pid_t pid = getpid();
    if (pid > 0) {
        PASS();
    } else {
        FAIL("getpid returned %d", pid);
    }

    // ---- Test 6: Non-trapped read/write works ----
    TEST("6. non-trapped read/write works");
    {
        int pipefd[2];
        if (pipe(pipefd) == 0) {
            const char *msg = "hello";
            ssize_t w = write(pipefd[1], msg, 5);
            char buf[16] = {0};
            ssize_t r = read(pipefd[0], buf, 5);
            close(pipefd[0]);
            close(pipefd[1]);
            if (w == 5 && r == 5 && memcmp(buf, "hello", 5) == 0) {
                PASS();
            } else {
                FAIL("write=%zd read=%zd", w, r);
            }
        } else {
            FAIL("pipe failed");
        }
    }

    // ---- Test 7: chroot returns 0 (emulated) ----
    TEST("7. chroot returns 0 (emulated)");
    ret = chroot("/test_twoyi_dev");
    if (ret == 0) {
        PASS();
    } else {
        FAIL("expected ret=0, got ret=%d errno=%d", ret, errno);
    }

    // ---- Test 8: umount2 removes mount entry ----
    TEST("8. umount2 removes mount entry");
    ret = umount2("/test_twoyi_proc", 0);
    if (ret == 0 && !is_mounted("/test_twoyi_proc")) {
        PASS();
    } else {
        FAIL("ret=%d errno=%d still_mounted=%d", ret, errno,
             is_mounted("/test_twoyi_proc"));
    }

    // ---- Test 9: Child process inherits seccomp filter ----
    TEST("9. child process inherits seccomp filter");
    pid_t child = fork();
    if (child < 0) {
        FAIL("fork failed: %s", strerror(errno));
    } else if (child == 0) {
        // Child: seccomp filter inherited
        int child_ret = mount("sysfs", "/test_twoyi_sys", "sysfs", 0, NULL);
        if (child_ret != 0) {
            _exit(1);
        }
        _exit(0);
    } else {
        int status;
        waitpid(child, &status, 0);
        if (WIFEXITED(status) && WEXITSTATUS(status) == 0) {
            PASS();
        } else {
            FAIL("child exited with %d", WEXITSTATUS(status));
        }
    }

    // ---- Test 10: mknodat creates regular file ----
    TEST("10. mknodat emulated (creates regular file)");
    ret = mknod("/tmp/test_twoyi_mknod", S_IFCHR | 0666, makedev(1, 3));
    if (ret == 0) {
        // Verify it's a regular file, not a char device
        struct stat st;
        if (stat("/tmp/test_twoyi_mknod", &st) == 0 && S_ISREG(st.st_mode)) {
            PASS();
        } else {
            FAIL("file exists but wrong type");
        }
        unlink("/tmp/test_twoyi_mknod");
    } else {
        FAIL("mknod returned %d errno=%d", ret, errno);
    }

    // ---- Summary ----
    printf("\n=== Results: %d/%d tests passed ===\n", test_passed, test_count);
    printf("Total SIGSYS traps: %d\n", g_sigsys_count);
    fflush(stdout);

    if (test_passed == test_count) {
        printf("ALL TESTS PASSED\n");
        return 0;
    } else {
        printf("SOME TESTS FAILED\n");
        return 1;
    }
}
