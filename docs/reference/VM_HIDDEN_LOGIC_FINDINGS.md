# VM Re-Disassembly: Hidden Logic Findings

## Date: 2026-08-10
## Method: Direct disassembly of libkr64.so + libkr64.11.so

---

## 1. CHROOT Handler (0x11c928) — NOT a simple no-op

### Previous understanding: "returns 0 (2-instruction no-op)"
### CORRECTED: Synchronization barrier with infinite loop

```asm
0x11c928:  adrp x8, 1bd000        ; load BSS base
0x11c92c:  ldr  w8, [x8, #88]     ; w8 = *(BSS + 0x58) — state variable 1
0x11c930:  mov  w10, #0x330b      ; OLLVM magic constant
0x11c934:  movk w10, #0xdf70, lsl #16
0x11c938:  adrp x9, 1bd000
0x11c93c:  add  w11, w8, w10     ; OLLVM obfuscation
0x11c940:  sub  w11, w11, #0x1
0x11c944:  sub  w10, w11, w10    ; w10 = w8 - 1 (deobfuscated)
0x11c948:  ldr  w9, [x9, #2724]  ; w9 = *(BSS + 0xAA4) — state variable 2
0x11c94c:  mul  w8, w10, w8      ; w8 = (w8-1) * w8
0x11c950:  mvn  w8, w8            ; OLLVM
0x11c954:  orr  w8, w8, #0xfffffffe
0x11c958:  cmn  w8, #0x1         ; check if w8 == -1
0x11c95c:  cset w8, ne            ; w8 = (state != some_value)
0x11c960:  cmp  w9, #0x9          ; compare state2 with 9
0x11c964:  cset w9, gt            ; w9 = (state2 > 9)
0x11c968:  eor  w10, w9, w8       ; XOR the conditions
0x11c96c:  tbnz w10, #0, 0x11c980 ; if either true, go to return
0x11c970:  orr  w8, w9, w8
0x11c974:  eor  w8, w8, #0x1
0x11c978:  tbnz w8, #0, 0x11c980 ; if other condition, go to return
0x11c97c:  b    0x11c97c          ; INFINITE LOOP — wait for state
0x11c980:  mov  w0, wzr           ; return 0
0x11c984:  ret
```

### Hidden Logic:
- Reads two BSS state variables: BSS+0x58 and BSS+0xAA4
- If runtime is NOT ready (state variables don't meet conditions):
  **INFINITE LOOP** — the handler blocks until state is initialized
- If runtime IS ready: return 0
- This is a **synchronization barrier** — chroot blocks until VM runtime is ready

### Twoyi equivalent needed:
```c
// In SIGSYS handler for chroot:
while (!runtime_ready) {
    // Spin until VM runtime is initialized
    // runtime_ready is set by the loader after all setup is done
}
return 0;
```

---

## 2. MOUNT Handler (0x113380 → 0x13d1f8 → 0x8618) — Full mount_mgr

### Previous understanding: "virtual mount table, returns 0"
### CORRECTED: Real filesystem operations + lock + fstype validation

```asm
0x8618:  stp x28, x27, [sp, #-96]!  ; large stack frame
0x8634:  sub sp, sp, #0x3020         ; 12KB+ stack (for path buffers)
0x863c:  mrs x24, tpidr_el0          ; TLS base (stack canary)
0x8658:  bl  0x8e14                  ; ACQUIRE LOCK (mutex)
0x865c:  mov w0, #0x441              ; 1089 = mode for mkdir?
0x8660:  orr w1, wzr, #0x1ff         ; 511 = 0777 permissions
0x8664:  bl  0x8cac                  ; mkdir or similar
0x86c8:  add x0, sp, #0x2018         ; buffer on stack
0x86d0:  orr w2, wzr, #0x1000        ; 4096 bytes
0x86d8:  bl  memset                  ; clear 4KB buffer

; Check state variables
0x86e0:  ldr w8, [BSS + 0x404]      ; mount count?
0x86e8:  ldr w9, [BSS + 0xE68]      ; another state

; Compare fstype against known types (3 strcmp)
0x8758:  adrp x1, 170000; add x1, x1, #0x514  ; fstype string 1
0x8764:  bl  strcmp
0x876c:  adrp x1, 170000; add x1, x1, #0x51c  ; fstype string 2
0x8778:  bl  strcmp
0x8780:  adrp x1, 170000; add x1, x1, #0x524  ; fstype string 3
0x878c:  bl  strcmp
```

### Hidden Logic:
1. **Acquires a lock** (0x8e14) — thread-safe mount table access
2. **Creates directories** if needed (mkdir with mode 0x441)
3. **Clears a 4KB buffer** on stack (for path manipulation)
4. **Checks state variables** BSS+0x404 and BSS+0xE68
5. **Compares fstype** against 3 known types (could not decode — may use multi-byte XOR)
6. **Handles unsupported filesystemtype** — logs error
7. **Handles bind mounts** — detects loops
8. **Handles remount** — updates flags on existing entry
9. Has an **infinite loop** at 0x8718 (same wait pattern as chroot)

### Decoded mount_mgr strings:
- "mount_mgr: /dev is special, skip" (key 0x0d)
- "mount_mgr: /mnt is special, skip" (key 0x25)
- "mount_mgr: /storage is special, skip" (key 0xdc)
- "mount_mgr: unsupported filesystemtype %s" (key 0xb3)
- "mount_mgr: bind loop detected %s" (key 0x56)
- "mount_mgr: already latest" (key 0xbc)
- "mount_mgr: no mounts" (key 0x26)
- "mount_mgr: mount arg source %s is bad" (key 0x02)
- "mount_mgr: mount arg target %s is bad" (key 0xff)
- "mount_mgr: umount arg target %s is bad" (key 0xe5)
- "mount_mgr: propagation %s not supported" (key 0xf7)
- "mount_mgr: %s -> %s -> %s" (key 0x86)
- "mount_mgr: %s -> %s" (key 0x69)
- "mount_mgr: %s not mounted" (key 0xab)

### Twoyi equivalent needed:
```c
// mount emulation needs:
// 1. Lock (mutex) for thread-safe table access
// 2. mkdir() for target paths that don't exist
// 3. fstype validation (check against known types)
// 4. Bind mount loop detection
// 5. Remount support (update flags)
// 6. Special path handling (/dev, /mnt, /storage)
// 7. Wait for runtime readiness (infinite loop if not ready)
```

---

## 3. MKNODAT Handler (0x1139e4 → 0x11d598) — Creates regular files

### Previous understanding: "return 0 (don't create anything)"
### CORRECTED: Creates a regular file containing the device number

```asm
0x11d638:  mov x0, x23             ; pathname
0x11d640:  mov x2, x20             ; translated path buffer
0x11d648:  bl  0x11baa4            ; TRANSLATE PATH (prepend rootfs prefix)

; Check if it's a device node
0x11d69c:  and w8, w22, #0xf000    ; extract S_IFMT from mode
0x11d6a0:  orr w8, w8, #0x4000     ; set S_IFDIR bit
0x11d6a4:  cmp w8, #0x6000         ; compare with S_IFBLK
0x11d6a8:  b.ne 0x11d760           ; if not device, skip

; Create a regular file instead of device node
0x11d6b8:  mov w0, #0x38           ; 56 = __NR_openat (arm64)
0x11d6bc:  mov w1, #-100           ; AT_FDCWD
0x11d6c0:  mov w3, #0x42           ; O_RDWR | O_CREAT
0x11d6c4:  mov w4, #0x1b6          ; 0666 permissions
0x11d6c8:  mov x2, x20             ; translated path
0x11d6cc:  bl  syscall             ; openat(AT_FDCWD, path, O_RDWR|O_CREAT, 0666)

; Write 8 bytes (the dev_t device number) to the file
0x11d6d0:  mov w2, #8              ; 8 bytes
0x11d6d4:  mov w3, #8              ; 8 bytes (write limit)
0x11d6d8:  mov x1, x19             ; data = dev_t value
0x11d6dc:  bl  __write_chk         ; write(fd, &dev, 8)
0x11d6e0:  mov w0, w21             ; fd
0x11d6e8:  bl  close               ; close(fd)

; Do it AGAIN (second open+write+close — possibly for a second file)
0x11d6ec:  mov w0, #0x38           ; openat again
0x11d6f0:  ...
```

### Hidden Logic:
1. **Translates the path** (prepends rootfs prefix) via 0x11baa4
2. **Checks if mode is S_IFCHR or S_IFBLK** (device node)
3. If device node: **creates a REGULAR FILE** at the translated path:
   - `openat(AT_FDCWD, path, O_RDWR|O_CREAT, 0666)`
   - `write(fd, &dev_t, 8)` — writes the device number as 8 bytes
   - `close(fd)`
4. Does this **TWICE** (possibly for major+minor device numbers, or two paths)
5. The "device" is actually a **regular file containing the device number**

### Twoyi equivalent needed:
```c
// mknodat emulation:
// 1. Translate path (prepend rootfs prefix)
// 2. If mode & S_IFMT == S_IFCHR or S_IFBLK:
//    a. openat(AT_FDCWD, translated_path, O_RDWR|O_CREAT, 0666)
//    b. write(fd, &dev, sizeof(dev_t))
//    c. close(fd)
// 3. Return 0
```

---

## 4. rt_sigaction Guard (0x114650) — Prevents SIGSYS override

### CONFIRMED: Simple but critical check

```asm
0x114650:  ldr  w10, [x19]        ; load signal number from args
0x114654:  cmp  w10, #0x1f        ; compare with 31 (SIGSYS)
0x114658:  b.ne 0x114664          ; if NOT SIGSYS, go to default handler
0x11465c:  mov  x0, xzr           ; return 0 (fake success)
0x114660:  b    0x115538          ; jump to return path
```

### Hidden Logic:
- If guest calls `rt_sigaction(SIGSYS=31, ...)`:
  - Return 0 (success) **WITHOUT** calling real sigaction
  - Guest thinks it installed its handler, but VM's handler remains
- If guest calls `rt_sigaction(any_other_signal, ...)`:
  - Fall through to DEFAULT handler
  - Re-execute real `rt_sigaction` syscall via `syscall@plt`

### Twoyi equivalent needed:
```c
// In SIGSYS handler for rt_sigaction:
if (signal_number == SIGSYS) {
    return 0;  // fake success, don't register guest's handler
} else {
    // re-execute real rt_sigaction
    return syscall(SYS_rt_sigaction, signal, act, oldact, sigsetsize);
}
```

---

## 5. openat Path Translation (0x118320 → 0x119080)

### CONFIRMED: /proc/ special handling + rootfs prefix

The path translation function at 0x119080:
1. `strncmp(path, "/proc/", 6)` — check if path starts with /proc/
2. If yes: special /proc handling (per-VM files)
3. If no: prepend rootfs prefix (`/data/data/com.clone.android.dual.space/vm/vm%d/fs`)

### /proc/ special paths (from decoded strings):
- `/proc/self/maps` → per-VM: `%s/proc/maps_%d_%d`
- `/proc/self/exe` → per-VM: `%s/proc/exe_%d`
- `/proc/self/status` → per-VM: `%s/proc/status_%d_%d`
- `/proc/self/mounts` → per-VM: `%s/proc/mounts_%d_%d`
- `/proc/self/fd/%d` → per-VM
- `/proc/%d/cmdline` → per-VM
- `/proc/%d/status` → per-VM
- `/proc/%d/maps` → per-VM
- `/proc/%d/mounts` → per-VM
- `/proc/%d/fd/%d` → per-VM
- `/proc/1` → special (init PID)
- `/proc/cmdline` → virtual (guest's cmdline)
- `/proc/version` → virtual
- `/proc/mounts` → virtual (from mount table)
- `/proc/mnt_points` → virtual
- `/proc/net/if_inet6` → virtual

### Twoyi equivalent needed:
```c
// openat path translation:
// 1. If path starts with "/proc/":
//    a. Parse the PID from the path
//    b. Translate to per-VM file: {TWOYI_ROOTFS}/proc/{type}_{vmid}_{pid}
// 2. Else: prepend {TWOYI_ROOTFS} to the path
// 3. Call real openat with translated path
```

---

## 6. BSS State Variables (synchronization)

### Confirmed: VM uses BSS variables for runtime readiness checks

| BSS Offset | Used By | Purpose (inferred) |
|------------|---------|-------------------|
| BSS+0x58 | chroot handler | Runtime init counter (wait until > 0) |
| BSS+0xAA4 | chroot handler | Ready flag (wait until > 9) |
| BSS+0x404 | mount_mgr | Mount count |
| BSS+0xE68 | mount_mgr | Mount state |
| BSS+0x80 | mknodat | Runtime init state |
| BSS+0xACC | mknodat | Ready flag |
| BSS+0x3B8 | mount wrapper | State (BSS+3512 decimal) |
| BSS+0x808 | mount wrapper | State (BSS+2056 decimal) |

### Pattern: Multiple handlers check the same state variables
- The chroot, mount, and mknodat handlers ALL check BSS state variables
- They all have **infinite loops** that wait for the runtime to be ready
- This is a **global initialization barrier** — no syscall emulation
  proceeds until the VM runtime is fully set up

### Twoyi equivalent needed:
```c
// Global runtime readiness flag
static volatile int runtime_ready = 0;

// Set by the loader after all initialization is complete
void mark_runtime_ready(void) {
    runtime_ready = 1;
}

// Called by every SIGSYS handler before emulating
void wait_for_runtime(void) {
    while (!runtime_ready) {
        // Spin until runtime is ready
        sched_yield();
    }
}
```

---

## Summary of Hidden Logic

| Handler | Previous Understanding | CORRECTED Understanding |
|---------|----------------------|------------------------|
| chroot | Simple no-op (return 0) | Synchronization barrier — waits for runtime readiness via infinite loop |
| mount | Virtual table, return 0 | Real lock + mkdir + fstype validation + bind loop detection + remount + wait for readiness |
| mknodat | Return 0 (don't create) | Creates a regular file containing the dev_t value via openat+write+close |
| rt_sigaction | (not analyzed) | Guards SIGSYS — returns fake success if guest tries to override SIGSYS handler |
| openat | (basic path translation) | Extensive /proc/ virtualization with per-VM files |
| ALL handlers | (no synchronization) | ALL check BSS state variables and wait for runtime readiness |

### Key Insight: The VM runtime has a **global initialization phase**
The loader sets up the seccomp filter and SIGSYS handler, but the handlers
DON'T immediately start emulating. They wait (infinite loop) until the
VM runtime (libkr64.so's init_array functions) sets BSS state variables
indicating readiness. This prevents race conditions where a guest syscall
is trapped before the VM's internal state (mount table, /proc virtual
files, etc.) is fully initialized.
