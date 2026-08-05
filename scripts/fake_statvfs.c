// LD_PRELOAD library that overrides statvfs/statfs to report more free disk space.
// This bypasses the Android emulator's conservative disk space check.
//
// Build: gcc -shared -fPIC -o fake_statvfs.so fake_statvfs.c -ldl
// Usage: LD_PRELOAD=./fake_statvfs.so emulator ...

#define _GNU_SOURCE
#include <sys/statvfs.h>
#include <sys/statfs.h>
#include <dlfcn.h>
#include <string.h>
#include <stdio.h>

// Multiply factor for faked free space
#define FAKE_MULT 10

static int should_fake(const char *path) {
    if (!path) return 0;
    return (strstr(path, "avd") || strstr(path, "android") || strstr(path, ".android") || strstr(path, "twoyi"));
}

// Override statvfs
int statvfs(const char *path, struct statvfs *buf) {
    static int (*real_statvfs)(const char *, struct statvfs *) = NULL;
    if (!real_statvfs) {
        real_statvfs = dlsym(RTLD_NEXT, "statvfs");
        if (!real_statvfs) return -1;
    }
    int ret = real_statvfs(path, buf);
    if (ret != 0) return ret;
    if (should_fake(path)) {
        buf->f_bfree = (fsblkcnt_t)((unsigned long)buf->f_bfree * FAKE_MULT);
        buf->f_bavail = (fsblkcnt_t)((unsigned long)buf->f_bavail * FAKE_MULT);
        buf->f_blocks = (fsblkcnt_t)((unsigned long)buf->f_blocks * FAKE_MULT);
    }
    return ret;
}

// Override statvfs64
int statvfs64(const char *path, struct statvfs64 *buf) {
    static int (*real_statvfs64)(const char *, struct statvfs64 *) = NULL;
    if (!real_statvfs64) {
        real_statvfs64 = dlsym(RTLD_NEXT, "statvfs64");
        if (!real_statvfs64) return -1;
    }
    int ret = real_statvfs64(path, buf);
    if (ret != 0) return ret;
    if (should_fake(path)) {
        buf->f_bfree = buf->f_bfree * FAKE_MULT;
        buf->f_bavail = buf->f_bavail * FAKE_MULT;
        buf->f_blocks = buf->f_blocks * FAKE_MULT;
    }
    return ret;
}

// Override statfs
int statfs(const char *path, struct statfs *buf) {
    static int (*real_statfs)(const char *, struct statfs *) = NULL;
    if (!real_statfs) {
        real_statfs = dlsym(RTLD_NEXT, "statfs");
        if (!real_statfs) return -1;
    }
    int ret = real_statfs(path, buf);
    if (ret != 0) return ret;
    if (should_fake(path)) {
        // struct statfs on Linux x86_64:
        // long f_type, f_bsize, f_blocks, f_bfree, f_bavail, f_files, f_ffree, ...
        buf->f_bfree *= FAKE_MULT;
        buf->f_bavail *= FAKE_MULT;
        buf->f_blocks *= FAKE_MULT;
    }
    return ret;
}

// Override statfs64
int statfs64(const char *path, struct statfs64 *buf) {
    static int (*real_statfs64)(const char *, struct statfs64 *) = NULL;
    if (!real_statfs64) {
        real_statfs64 = dlsym(RTLD_NEXT, "statfs64");
        if (!real_statfs64) return -1;
    }
    int ret = real_statfs64(path, buf);
    if (ret != 0) return ret;
    if (should_fake(path)) {
        buf->f_bfree *= FAKE_MULT;
        buf->f_bavail *= FAKE_MULT;
        buf->f_blocks *= FAKE_MULT;
    }
    return ret;
}
