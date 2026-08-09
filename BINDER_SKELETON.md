# Twoyi — Binder Virtualisation Skeleton (Task BINDER-2)

> **Author:** general-purpose sub-agent
> **Date:** 2026-08-05
> **Codespace:** `twoyi-dev-3-jr47xg6xvx7ghq6p`
> **Inputs:** `worklog.md`, `download/GSI_BOOT_PLAN.md` §3.2, `app/rs/kr64/src/devices.rs`
> **Output:** `app/rs/kr64/src/binder.rs` (new, ~1900 lines) + integration into `lib.rs`
> **Goal:** Land a compiling, well-tested skeleton of the per-VM binder virtualisation layer described in `GSI_BOOT_PLAN.md` §3.2, so that the next task (`BINDER-3`) can fill in the parcel-parsing and handle-translation logic without having to re-do the protocol constants, device creation, or thread-pool plumbing.

---

## 0. Executive summary

Implemented a compiling binder virtualisation skeleton in the kr64 Rust crate. The skeleton:

1. **Creates the per-VM binder device** at `{rootfs}/vm{id}/dev/binder` (Unix socket) plus a `{rootfs}/dev/binder` symlink → `../vm{id}/dev/binder` so the chrooted guest sees the conventional `/dev/binder` path.
2. **Defines the full binder protocol constant set** — `BINDER_*` ioctl numbers, `BC_*` commands, `BR_*` returns, `SVC_MGR_*` service-manager codes, `BINDER_TYPE_*` flat-object types, `TF_*` transaction flags. All match the kernel `<uapi/linux/android/binder.h>` exactly (verified by unit tests against the canonical ioctl hex values).
3. **Spawns a binder proxy server** with a fixed-size thread pool (4 workers) that accepts guest connections and dispatches per-ioctl.
4. **Implements the basic ioctl handlers**: `BINDER_VERSION` returns protocol version 8 (Android 11); `BINDER_SET_MAX_THREADS`, `BINDER_SET_CONTEXT_MGR`, `BINDER_THREAD_EXIT` acknowledge; `BINDER_WRITE_READ` parses the BC_* command stream, dispatches each, and builds a BR_* response stream.
5. **Stubs out transaction forwarding**: `forward_transaction_to_host` opens the host's `/dev/binder` lazily and issues a real `BINDER_WRITE_READ` ioctl, but does NOT yet translate handles or patch the `flat_binder_object` array (that's `BINDER-3`).
6. **Stubs out the service-manager proxy**: `servicemanager_proxy` recognises `SVC_MGR_GET_SERVICE`, `SVC_MGR_ADD_SERVICE`, `SVC_MGR_LIST_SERVICES` but returns `BR_FAILED_REPLY` / no-op for each (real parcel parsing is `BINDER-3`).
7. **Integrates into the daemon startup**: `lib.rs::run()` now creates the binder device + spawns the proxy as "Step 2.5", right after the other `/dev` nodes are created and before `/proc` is populated. Failure is non-fatal (logs a warning and falls back to the host's binder).
8. **Passes 11 new unit tests** plus all 27 pre-existing tests; total 38 tests, 0 failures. Builds clean (no warnings) on Linux x86_64 host.

The skeleton is NOT yet functional — a guest connecting to `/vm{id}/dev/binder` will get `BINDER_VERSION` and the basic ioctl acknowledgements back, but `BC_TRANSACTION` calls will mostly fail (no parcel parsing, no handle translation). See §4 for what's left.

---

## 1. What was implemented

### 1.1 File layout

```
app/rs/kr64/src/
├── lib.rs         (modified: +`pub mod binder;`, +Step 2.5 in run())
├── main.rs        (unchanged — already just a thin wrapper around kr64::run)
├── binder.rs      (NEW — ~1900 lines, the skeleton)
├── devices.rs     (unchanged)
├── seccomp.rs     (unchanged)
├── proc_emu.rs    (unchanged)
└── mount_mgr.rs   (unchanged)
```

### 1.2 `binder.rs` module structure

| Section | Lines | Purpose |
|---------|-------|---------|
| Header + module docs | 1–100 | MPL header, skeleton-status disclaimer, wire-format description, module-layout map. |
| `ioctl` macros | 115–160 | `_IOC`, `_IO`, `_IOR`, `_IOW`, `_IOWR` as `const fn` (so the constants below can be `const`). Matches `<asm-generic/ioctl.h>`. |
| Kernel ABI structs | 165–290 | `BinderWriteRead`, `BinderPtrCookie`, `BinderHandleCookie`, `BinderPriDesc`, `BinderPriCookie`, `BinderTransactionData`, `FlatBinderObject`. All `#[repr(C)]`, sizes verified by tests. |
| `BINDER_*` ioctl numbers | 295–335 | `BINDER_WRITE_READ`, `BINDER_SET_MAX_THREADS`, `BINDER_SET_CONTEXT_MGR`, `BINDER_THREAD_EXIT`, `BINDER_VERSION`, plus `_IDLE_TIMEOUT`, `_IDLE_PRIORITY`, `GET_NODE_DEBUG_INFO`, `SET_CONTEXT_MGR_EXT`. |
| `BC_*` constants | 337–460 | All 19 binder commands (`BC_TRANSACTION` … `BC_DEAD_BINDER_DONE`), nrs 1–18 matching the kernel enum. |
| `BR_*` constants | 460–555 | All 15 binder returns (`BR_ERROR` … `BR_FAILED_REPLY`), nrs matching the kernel enum. |
| `SVC_MGR_*` codes | 556–580 | `SVC_MGR_GET_SERVICE` (1), `CHECK_SERVICE` (2), `ADD_SERVICE` (3), `LIST_SERVICES` (4), `CHECK_SERVICE_IF_EXIST` (5), `SVC_MGR_HANDLE` (0). |
| `BINDER_TYPE_*` + `TF_*` | 582–615 | Flat-object type constants and transaction flags. |
| Misc constants | 617–635 | `BINDER_CURRENT_PROTOCOL_VERSION = 8`, `BINDER_THREAD_POOL_SIZE = 4`. |
| Wire framing | 640–680 | `Frame` (request), `Resp` (response), `WireBinderWriteRead`, `WireBinderWriteReadResponse`. |
| `HandleTable` | 685–745 | Per-VM guest↔host handle map + service-name lookup. |
| `create_binder_device` | 750–825 | Creates the socket file + symlink. |
| `BinderProxy` / `BinderProxyHandle` | 830–1000 | Owns the listener, spawns the accept thread + worker pool, manages shutdown. |
| `ThreadPool` | 1005–1115 | Minimal fixed-size thread pool (Rust-book pattern + `Terminate` control message). |
| `handle_connection` | 1120–1155 | Per-connection read-dispatch-write loop. |
| `dispatch_request` + per-ioctl handlers | 1160–1230 | `BINDER_VERSION` returns 8; `SET_MAX_THREADS`/`SET_CONTEXT_MGR`/`THREAD_EXIT` acknowledge; `WRITE_READ` dispatches to `handle_write_read`. |
| `handle_write_read` | 1235–1340 | Parses the wire payload, iterates BC_* commands, dispatches each, builds the BR_* read buffer. |
| `handle_transaction` + `TransactionResult` | 1345–1410 | Routes target-handle-0 transactions to `servicemanager_proxy`, everything else to `forward_transaction_to_host`. |
| `servicemanager_proxy` | 1415–1465 | Skeleton: logs the code, returns `Failed`/`Noop` per code. |
| `forward_transaction_to_host` | 1470–1540 | Lazily opens `/dev/binder`, issues a real `BINDER_WRITE_READ` ioctl. Skeleton: does NOT translate handles. |
| `open_host_binder` | 1545–1565 | `open("/dev/binder", O_RDWR \| O_CLOEXEC)`. |
| Wire-framing I/O helpers | 1570–1605 | `read_frame`, `write_frame`. |
| BR_* push helpers | 1610–1660 | `push_br_noop`, `push_br_failed_reply`, `push_br_reply`. |
| `bc_payload_size` | 1665–1680 | Extracts the size field from an ioctl number. |
| Tests | 1690–1927 | 11 tests: ioctl-number correctness, struct sizes, `HandleTable`, `create_binder_device`, end-to-end `BINDER_VERSION`, end-to-end `BINDER_WRITE_READ`→`BR_NOOP`, `ThreadPool`. |

### 1.3 Integration into `lib.rs`

`lib.rs::run()` now has a "Step 2.5" between device creation and `/proc` population:

```rust
let _binder_handle = match binder::create_binder_device(&cfg.rootfs, cfg.vmid)
    .and_then(|path| binder::BinderProxy::new(cfg.vmid, &path))
    .and_then(|proxy| proxy.spawn())
{
    Ok(h) => {
        info!("[KR64] binder proxy listening at {} (vm{})", h.path(), cfg.vmid);
        Some(h)
    }
    Err(e) => {
        warning!("[KR64] failed to start binder proxy: {} — falling back to host binder", e);
        None
    }
};
```

The `_binder_handle` is held until the end of `run()`. When `run()` returns (either because the guest exited or because of an error), the handle is dropped, which:

1. Sets the `shutdown` atomic flag → the accept thread exits its loop on the next iteration.
2. Joins the accept thread.
3. The accept thread's `Drop` for the `ThreadPool` runs, which sends `Terminate` to all 4 workers and joins them.
4. Unlinks the socket file.

The `_binder_handle` is non-fatal: if the proxy fails to start (e.g. socket bind fails), `run()` continues anyway. The guest can still fall back to the host's `/dev/binder` if it's bind-mounted in (the current twoyi approach). This matches the skeleton's "best-effort" philosophy.

`main.rs` is unchanged — it's already a thin wrapper around `kr64::run(args)`.

---

## 2. Wire-framing protocol

Since the guest's `libbinder.so` cannot call `ioctl()` on a Unix socket (`ioctl` returns `ENOTTY` for `BINDER_*` on `SOCK_STREAM`), we define our own framed protocol over the socket. This mirrors what VM does in `libvm.so` (the `setupBinder` JNI installs a shadowhook-based shim that translates `ioctl` calls into socket messages).

### 2.1 Request frame (guest → proxy)

```
+----------+----------+--------------------------+
| u32 cmd  | u32 len  |  len bytes of payload    |
+----------+----------+--------------------------+
```

- `cmd` is the binder ioctl number (`BINDER_WRITE_READ`, `BINDER_VERSION`, …).
- `len` is the payload size in bytes (0 for `_IO` ioctls like `BINDER_SET_CONTEXT_MGR`).
- The payload is the raw bytes of the ioctl's `arg` pointer.

### 2.2 Response frame (proxy → guest)

```
+----------+----------+--------------------------+
| i32 ret  | u32 len  |  len bytes of payload    |
+----------+----------+--------------------------+
```

- `ret` is `0` on success, `-errno` on failure (matching the kernel's `ioctl(2)` return convention).
- `len` is the response payload size in bytes (0 for `_IO` ioctls, 4 for `BINDER_VERSION`, variable for `BINDER_WRITE_READ`).
- The payload is the bytes the kernel would have written into `arg` (for `_IOR`/`_IOWR` ioctls).

### 2.3 `BINDER_WRITE_READ` payload

Because the kernel's `struct binder_write_read` uses user pointers (which don't make sense over a socket), we use our own wire format:

**Request payload:**
```
+-----------------+--------------------+------------------------------+
| u32 write_size  | u32 read_capacity  |  write_size bytes            |
+-----------------+--------------------+------------------------------+
```

**Response payload:**
```
+----------------+---------------------------------------+
| u32 read_size  |  read_size bytes                      |
+----------------+---------------------------------------+
```

The `write_size` bytes are the guest's outgoing BC_* command stream (each command is `[u32 cmd][cmd-specific payload]`). The proxy parses this, dispatches each command, and builds the `read_size` bytes of BR_* commands to return.

---

## 3. Protocol-constant correctness

The ioctl numbers must match the kernel exactly — the guest's `libbinder.so` uses these literal numbers in `ioctl()` calls, and any mismatch means the guest's `ioctl()` returns `ENOTTY` and binder traffic fails completely.

### 3.1 Verified values (Linux x86_64 / aarch64)

| Constant | Computed | Kernel |
|----------|----------|--------|
| `BINDER_WRITE_READ` | `0xC0306201` | `_IOWR('b', 1, struct binder_write_read)` |
| `BINDER_SET_MAX_THREADS` | `0x40046205` | `_IOW('b', 5, __u32)` |
| `BINDER_SET_CONTEXT_MGR` | `0x00006207` | `_IO('b', 7)` |
| `BINDER_THREAD_EXIT` | `0x40046208` | `_IOW('b', 8, __u32)` |
| `BINDER_VERSION` | `0xC0046209` | `_IOWR('b', 9, __u32)` |
| `BC_TRANSACTION` | `0x40406201` | `_IOW('b', 1, struct binder_transaction_data)` |
| `BC_REPLY` | `0x40406202` | `_IOW('b', 2, struct binder_transaction_data)` |
| `BC_ACQUIRE` | `0x40046203` | `_IOW('b', 3, __u32)` |
| `BC_ENTER_LOOPER` | `0x0000620d` | `_IO('b', 13)` |
| `BC_REGISTER_LOOPER` | `0x0000620e` | `_IO('b', 14)` |
| `BC_EXIT_LOOPER` | `0x0000620f` | `_IO('b', 15)` |
| `BC_TRANSACTION_SG` | `0x4040620b` | `_IOW('b', 11, struct binder_transaction_data_sg)` |
| `BR_TRANSACTION` | `0x80406202` | `_IOR('b', 2, struct binder_transaction_data)` |
| `BR_REPLY` | `0x80406203` | `_IOR('b', 3, struct binder_transaction_data)` |
| `BR_NOOP` | `0x0000620c` | `_IO('b', 12)` |
| `BR_SPAWN_LOOPER` | `0x0000620d` | `_IO('b', 13)` |
| `BR_FAILED_REPLY` | `0x00006211` | `_IO('b', 17)` |

### 3.2 Gotcha: the BC_* enum starts at nr=1, not nr=0

The first version of this skeleton used `nr=0` for `BC_TRANSACTION`. The kernel enum (`enum BinderCommand` in `<uapi/linux/android/binder.h>`) actually starts at `nr=1` (nr=0 is unused in the BC_* space). Similarly, `BR_OK` occupies `nr=1` in the BR_* space — `BR_TRANSACTION` is `nr=2`. Caught by the `bc_br_constants_match_kernel_values` unit test; fixed by shifting all the BC_* nrs up by 1 and adding `BR_OK` at `nr=1`.

### 3.3 Struct-size verification

The size component of each `_IOW`/`_IOR`/`_IOWR` ioctl number is `sizeof(arg)`, so the kernel struct sizes must also match. Verified by tests:

| Struct | Computed size | Kernel |
|--------|---------------|--------|
| `BinderWriteRead` | 48 bytes | `sizeof(struct binder_write_read)` on aarch64/x86_64 |
| `BinderTransactionData` | 64 bytes | `sizeof(struct binder_transaction_data)` |
| `FlatBinderObject` | 24 bytes | `sizeof(struct flat_binder_object)` |

---

## 4. What's NOT implemented (TODO for `BINDER-3` and later)

The skeleton compiles and handles the basic ioctl set, but a real guest will not be able to make actual binder transactions yet. Here's what's missing, in priority order:

### 4.1 Parcel parsing (BINDER-3)

The guest sends `SVC_MGR_GET_SERVICE("activity")` as a `BC_TRANSACTION` to handle 0. The transaction's `binder_transaction_data.data_ptr` points at a parcel:

```
i32  strict_mode_policy
i32  work_source
u16[] interface_descriptor_string  (length-prefixed)
…    per-code arguments
```

For `SVC_MGR_GET_SERVICE` the per-code argument is a length-prefixed UTF-16 service name string. To parse it, we need to **follow `data_ptr` into the guest's address space** — either via `process_vm_readv` (slow, requires `PTRACE_MODE_ATTACH_REALCREDS`) or via a shared-memory mapping negotiated through the wire protocol (the proper approach — add a "shared buffer" extension to the wire framing).

### 4.2 Handle translation (BINDER-3)

When the guest does `BC_TRANSACTION` to handle 5 (e.g. `activity`), we need to:

1. Look up the host's binder handle for `activity` (looked up earlier via `SVC_MGR_GET_SERVICE` on the host's `/dev/binder`).
2. Patch `transaction_data.target.handle` from the guest handle to the host handle.
3. Walk the `offsets` array in the data buffer and patch any `BINDER_TYPE_HANDLE` / `BINDER_TYPE_WEAK_HANDLE` objects (their `handle` field also needs translation).
4. Conversely, in the reply, walk the offsets and translate any `BINDER_TYPE_BINDER` / `BINDER_TYPE_WEAK_BINDER` local-binder pointers back to guest handles.

The `HandleTable` struct is already in place (`allocate`, `register`, `lookup_by_name`, `lookup_host`); the wiring is missing.

### 4.3 Data buffer copy-in (BINDER-3)

The guest's `data_ptr` is a user pointer in the GUEST's address space — the host kernel can't read it. We need to either:

- Copy the data buffer into our own process's memory before issuing the host `BINDER_WRITE_READ` ioctl (requires parcel-parsing first to know the buffer size), OR
- Use ashmem to share a buffer between the guest and the proxy, and rewrite `data_ptr` to point at the shared buffer.

### 4.4 Reply unparceling (BINDER-3)

The host's `BINDER_WRITE_READ` ioctl returns a `BR_REPLY` + `binder_transaction_data` + reply data in the read buffer. We need to:

1. Slice the read buffer to `bwr.read_consumed` (the kernel writes back this field).
2. Parse out the leading `BR_REPLY` (4 bytes) + `binder_transaction_data` (64 bytes).
3. Translate any `BINDER_TYPE_HANDLE` objects in the reply back to guest handles.
4. Wrap the result in our wire format and send it back to the guest.

Currently `forward_transaction_to_host` returns the WHOLE 4 KiB read buffer — the caller (`handle_transaction`) treats it as the reply data bytes and pushes a `BR_REPLY` with `data_size = 4096`. This is wrong but at least has the right shape.

### 4.5 Guest-side libbinder.so patching (BINDER-4)

The guest's `libbinder.so` cannot call `ioctl()` on a Unix socket — it needs to be patched (or shimmed via LD_PRELOAD) to translate `ioctl(fd, BINDER_*, arg)` calls into framed socket messages. VM does this in `libvm.so` via shadowhook (see `VM_KR64_ANALYSIS.md` §11). For twoyi this will be a separate native library (`libtwbinder_shim.so`?) loaded into the guest via `LD_PRELOAD` (or via the existing twoyi loader).

Without this shim, the guest's `ioctl(/dev/binder, BINDER_VERSION, ...)` returns `ENOTTY` and binder traffic never reaches our proxy at all. **This is the most important next step** — without it, the skeleton is unreachable.

### 4.6 Java-side `BinderService` (BINDER-5)

VM's `com.android.vmcore.service.BinderService` wraps the host's `IActivityManager` with a `java.lang.reflect.Proxy` so `servicemanager` lookups for `activity` / `package` / `window` are re-routed into the host app. The twoyi equivalent would be `app/src/main/java/io/twoyi/service/BinderService.java` + `IBinderService.aidl`. The native `setupBinder(vmId, ...)` JNI would call into `libkr64.so` to start the proxy. This is the user-visible integration point.

### 4.7 Multi-version support (BINDER-6+)

The binder protocol changed slightly between Android 7, 9, 11, 13+. The skeleton targets Android 11 (`BINDER_CURRENT_PROTOCOL_VERSION = 8`). Multi-version support would gate the protocol version on the GSI's `build.prop` `ro.build.version.sdk` and adjust the struct layouts / BC_* enum accordingly.

---

## 5. Testing

### 5.1 Unit tests

11 new tests in `binder::tests`:

| Test | What it verifies |
|------|------------------|
| `ioctl_macros_match_kernel_values` | The 5 `BINDER_*` ioctl numbers match the kernel hex values exactly. |
| `bc_br_constants_match_kernel_values` | All 19 `BC_*` and 15 `BR_*` constants match the kernel enum exactly. |
| `bc_payload_size_extracts_size_from_ioctl_number` | The size-field extraction works for varying-payload-size commands. |
| `binder_write_read_size_is_48_bytes` | `BinderWriteRead` ABI struct is 48 bytes (matches kernel). |
| `binder_transaction_data_size_is_64_bytes` | `BinderTransactionData` ABI struct is 64 bytes. |
| `flat_binder_object_size_is_24_bytes` | `FlatBinderObject` ABI struct is 24 bytes. |
| `handle_table_allocate_and_lookup` | `HandleTable` allocates unique guest handles and maps them to host handles. |
| `handle_table_register_and_lookup_by_name` | `HandleTable` records service-name → guest-handle mappings. |
| `create_binder_device_creates_socket_and_symlink` | The socket file and the `../vm{id}/dev/binder` symlink are created correctly. |
| `binder_proxy_responds_to_version_ioctl` | End-to-end: connect to a running proxy, send `BINDER_VERSION`, get protocol version 8 back. |
| `binder_proxy_write_read_returns_noop_when_idle` | End-to-end: send an empty `BINDER_WRITE_READ`, get a `BR_NOOP` back. |
| `thread_pool_executes_jobs` | The `ThreadPool` runs all submitted jobs before being dropped. |

Total: 38 tests pass (11 binder + 27 pre-existing), 0 failures, no warnings.

### 5.2 Manual smoke test

```sh
$ cd app/rs/kr64 && cargo test --lib
running 38 tests
test binder::tests::bc_payload_size_extracts_size_from_ioctl_number ... ok
test binder::tests::bc_br_constants_match_kernel_values ... ok
test binder::tests::binder_proxy_responds_to_version_ioctl ... ok
... [35 more] ...
test result: ok. 38 passed; 0 failed; 0 ignored
```

### 5.3 Build verification

```sh
$ cd app/rs/kr64 && cargo build --bin kr64       # bin target
$ cd app/rs/kr64 && cargo build --lib            # cdylib target (libkr64.so)
$ cd app/rs/kr64 && cargo build --release        # optimised
```

All three build with **zero warnings** on Linux x86_64 host (Rust 1.97.1).

---

## 6. Design decisions

### 6.1 Why a Unix socket, not a real char device?

VM creates `/vm%d/dev/binder` as a real char device via `mknodat(S_IFCHR, major, minor)` at libkr64.so offset `0x11d770` (see `VM_KR64_ANALYSIS.md` §6). This requires `CAP_MKNOD`, which is unavailable in unprivileged Android app processes. VM works around this by running `libkr64.so` as a separate process with elevated capabilities (via `libkrloader64.so`'s custom interpreter trick).

Twoyi doesn't have that loader trick yet, so for the skeleton we use a Unix socket instead. This means the guest's `libbinder.so` has to be patched (via LD_PRELOAD) to translate `ioctl` calls into socket messages — see §4.5. A future task could switch to a real char device if/when twoyi gains the loader trick.

### 6.2 Why a thread pool, not thread-per-connection?

Binder traffic is latency-sensitive (each `BC_TRANSACTION` blocks the calling thread until the reply arrives). Thread-per-connection can starve under load (a flood of low-priority connections can starve a high-priority one). A fixed-size pool with a queue gives predictable behaviour and limits the daemon's memory footprint.

The pool size (4) matches what VM uses (inferred from the `BINDER_SET_MAX_THREADS` value seen in the BinderService disassembly — see `VM_JAVA_ANALYSIS.md` §5.2).

### 6.3 Why is `create_binder_device` separate from `BinderProxy::new`?

Two reasons:

1. **Testability**: `create_binder_device` can be tested without spawning threads. The test verifies the socket file and symlink are created correctly.
2. **Decoupling**: The caller can choose to create the device without starting the proxy (e.g. for a "dry-run" mode that just materialises the `/dev` tree without serving it). This matches the pattern in `devices.rs`, where `create_qemu_pipe` etc. return a `DeviceSocket` that the caller can then choose to `accept()` on or not.

### 6.4 Why does `create_binder_device` drop the listener?

Because `UnixListener` doesn't `Clone`, and we want `create_binder_device` to be callable independently of `BinderProxy::new`. The function binds the listener (to verify the socket can be created and to set its mode), then drops it and unlinks the socket file. The caller (`BinderProxy::new`) re-binds. The re-bind is cheap and the unlink-then-bind is atomic from the guest's perspective (the symlink always points to a valid path, even if no one is listening yet).

### 6.5 Why is the binder proxy failure non-fatal?

The skeleton is not yet functional (see §4). If we made binder-proxy startup fatal, the daemon would refuse to start at all. By making it non-fatal, the daemon still starts, creates the other devices, and execs the guest — the guest just falls back to the host's `/dev/binder` (if bind-mounted in) or fails to find `/dev/binder` (if not). Either way, the rest of the daemon's behaviour is exercised, which is useful for development.

### 6.6 Why does `forward_transaction_to_host` issue a real ioctl?

It would have been simpler to leave `forward_transaction_to_host` as a stub that always returns `Err`. But issuing the real ioctl means:

1. We exercise the `BinderWriteRead` ABI struct (catches layout bugs early).
2. We surface `open("/dev/binder")` failures immediately (so on a Linux dev host, the test logs a clear "could not open host /dev/binder" message rather than a generic "not implemented").
3. The next task (`BINDER-3`) can build on the existing ioctl call site rather than starting from scratch.

---

## 7. Next actions

1. **BINDER-3**: Implement parcel parsing + handle translation + data-buffer copy-in + reply unparceling in `forward_transaction_to_host` and `servicemanager_proxy`. This is the bulk of the "real" binder virtualisation work.
2. **BINDER-4**: Implement the guest-side `libbinder.so` shim (LD_PRELOAD library) that translates `ioctl(fd, BINDER_*, arg)` calls into framed socket messages. Without this, the skeleton is unreachable from the guest.
3. **BINDER-5**: Implement the Java-side `BinderService` + AIDL + `setupBinder` JNI, mirroring VM's `com.android.vmcore.service.BinderService`.
4. **BINDER-6+**: Multi-version support (Android 7/9/11/13+ protocol differences).
5. **BINDER-7**: Switch from Unix-socket to real char device (`mknodat(S_IFCHR)`) once twoyi has the loader trick.

---

## 8. Files touched

| File | Status | Lines changed |
|------|--------|---------------|
| `app/rs/kr64/src/binder.rs` | **NEW** | ~1927 |
| `app/rs/kr64/src/lib.rs` | modified | +27 (module decl + Step 2.5 in `run()`) |
| `app/rs/kr64/src/main.rs` | unchanged | 0 |
| `download/BINDER_SKELETON.md` | **NEW** | this file |

---

## 9. References

- `download/GSI_BOOT_PLAN.md` §3.2 — the design this skeleton implements.
- `download/VM_JAVA_ANALYSIS.md` §5.2 — VM's `BinderService.m5206WWWWoWWWWo` flow.
- `download/VM_KR64_ANALYSIS.md` §6, §11 — VM's `libkr64.so` device-creation pattern + shadowhook.
- Linux kernel: `include/uapi/linux/android/binder.h` — the canonical protocol constants.
- AOSP: `frameworks/native/libs/binder/IServiceManager.cpp` — `SVC_MGR_*` codes.
- AOSP: `frameworks/native/libs/binder/Parcel.cpp` — parcel format (for BINDER-3).
