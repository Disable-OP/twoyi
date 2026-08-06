// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of
// merchantability, fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://www.mozilla.org/MPL/2.0/.

//! Binder virtualisation skeleton — per-VM `/vm%d/dev/binder` Unix socket
//! plus a userspace proxy that forwards binder transactions between the
//! guest and the host's real `/dev/binder` driver.
//!
//! # What this mirrors
//!
//! Virtual Master's `libvm.so` (loaded into the Java app process) creates
//! `/vm%d/dev/binder` via the `setupBinder()` JNI called from
//! `com.android.vmcore.service.BinderService.m5206WWWWoWWWWo` (see
//! `VM_JAVA_ANALYSIS.md` §5.2 and `GSI_BOOT_PLAN.md` §2.5 / §3.2). The
//! guest's `servicemanager` then talks to this virtual binder instead of
//! the host's `/dev/binder`, and the Java side wraps the host's
//! `IActivityManager` with a `java.lang.reflect.Proxy` so servicemanager
//! lookups for `activity` / `package` / `window` / etc. are re-routed
//! back into the host app's `BinderService`.
//!
//! VM puts this in `libvm.so` (not `libkr64.so`) because binder is
//! latency-sensitive and going through a separate process would add a
//! context switch per transaction. Twoyi doesn't have a `libvm.so` yet,
//! so for the skeleton we host the binder proxy inside the kr64 daemon
//! process. A future task can split it out into a per-VM `libvm.so`
//! analogue if the latency becomes a problem.
//!
//! # Skeleton status
//!
//! This is a **skeleton**: it compiles, has the right protocol constants,
//! creates the device, accepts connections, and dispatches the standard
//! ioctl set. What it does NOT do yet:
//!
//! * **Parcel parsing** for `SVC_MGR_GET_SERVICE` / `SVC_MGR_ADD_SERVICE`.
//!   The guest sends a parcelled interface-descriptor + service-name
//!   string; we'd need to follow the `binder_transaction_data.data.ptr`
//!   pointer into the guest's write buffer to read it. The skeleton just
//!   logs the request and returns `BR_FAILED_REPLY`.
//! * **Handle translation.** When the guest does `BC_TRANSACTION` to
//!   handle 5 (e.g. `activity`), we'd need to look up the *host's*
//!   binder handle for `activity` (looked up earlier via
//!   `SVC_MGR_GET_SERVICE` on the host's `/dev/binder`), patch the
//!   transaction_data.target.handle, and forward via a real
//!   `BINDER_WRITE_READ` ioctl on the host. The skeleton's
//!   `forward_transaction_to_host` opens `/dev/binder` lazily and issues
//!   the ioctl but does NOT yet translate handles or patch the offsets /
//!   `flat_binder_object` array — that's the hard part and is left for
//!   the next task (proposed: `BINDER-3`).
//! * **Death notifications**, **BC_ACQUIRE_DONE** / **BC_INCREFS_DONE**
//!   acknowledgements, **file-descriptor passing**, **async (one-way)
//!   transactions**, **BC_TRANSACTION_SG** scatter-gather offsets. All
//!   are accepted and logged but not actually processed.
//!
//! # Wire framing
//!
//! The guest's `libbinder.so` cannot call `ioctl()` on a Unix socket —
//! `ioctl` on a `SOCK_STREAM` returns ENOTTY for binder ioctls. So
//! either:
//!
//! 1. The guest's `libbinder.so` is patched (via LD_PRELOAD or a custom
//!    `libc.so` shim) to translate `ioctl(fd, BINDER_*, arg)` calls into
//!    framed socket messages on `/dev/binder`. This is what VM does
//!    (the patching happens in `libvm.so` via shadowhook — see
//!    `VM_KR64_ANALYSIS.md` §11), OR
//! 2. The daemon creates `/dev/binder` as a real char device via
//!    `mknodat(S_IFCHR, …)` and uses `binderfs` / a kernel module to
//!    actually handle the ioctls. This is the "real driver" approach
//!    and requires `CAP_MKNOD` + a per-VM binder context in the kernel.
//!
//! This skeleton uses approach (1). The wire format is documented in
//! the [`Frame`] / [`Resp`] docs below. The guest-side patching code is
//! a separate task (proposed: `BINDER-4`).
//!
//! # Module layout
//!
//! * [`create_binder_device`] — creates `{rootfs}/vm{id}/dev/binder` as
//!   a Unix socket plus a `{rootfs}/dev/binder` symlink to it.
//! * [`BinderProxy`] / [`BinderProxyHandle`] — owns the listener and
//!   spawns the accept + worker threads.
//! * [`ThreadPool`] — minimal fixed-size thread pool used by
//!   `BinderProxy` for concurrent connection handling.
//! * [`HandleTable`] — guest-handle ↔ host-handle ↔ service-name map.
//! * `dispatch_request` / `handle_*` — per-ioctls handlers.
//! * `servicemanager_proxy` / `forward_transaction_to_host` — the two
//!   transaction dispatch paths (handle 0 → servicemanager, else → host).
//! * Protocol constants (`BINDER_*`, `BC_*`, `BR_*`, `SVC_MGR_*`) —
//!   exact matches of the kernel `<uapi/linux/android/binder.h>` and
//!   AOSP `frameworks/native/libs/binder/IServiceManager.cpp`.

use libc::c_void;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};

// Crate-local logging macros (defined in lib.rs) — no external `log` crate.
// `error` is not currently used in this skeleton (all error paths use
// `warning!`), but we keep the import so future code can use it without
// touching the use list.
#[allow(unused_imports)]
use crate::{error, info, warning};

// ============================================================================
// ioctl number macros — identical to Linux <asm-generic/ioctl.h>.
//
// All binder ioctls use type 'b' (0x62). The ioctl number encodes the
// direction (_IOC_NONE/_IOC_WRITE/_IOC_READ), the type, the nr, and the
// argument size in a single u32. We compute these at compile time so
// they can be `const` and used in match arms.
// ============================================================================

const _IOC_NONE: u32 = 0;
const _IOC_WRITE: u32 = 1;
const _IOC_READ: u32 = 2;

#[inline]
#[allow(non_snake_case)]
const fn _IOC(dir: u32, type_: u32, nr: u32, size: u32) -> u32 {
    (dir << 30) | (size << 16) | (type_ << 8) | nr
}

#[inline]
#[allow(non_snake_case)]
const fn _IO(t: u32, nr: u32) -> u32 {
    _IOC(_IOC_NONE, t, nr, 0)
}

#[inline]
#[allow(non_snake_case)]
const fn _IOR(t: u32, nr: u32, size: u32) -> u32 {
    _IOC(_IOC_READ, t, nr, size)
}

#[inline]
#[allow(non_snake_case)]
const fn _IOW(t: u32, nr: u32, size: u32) -> u32 {
    _IOC(_IOC_WRITE, t, nr, size)
}

#[inline]
#[allow(non_snake_case)]
const fn _IOWR(t: u32, nr: u32, size: u32) -> u32 {
    _IOC(_IOC_READ | _IOC_WRITE, t, nr, size)
}

/// Binder ioctl type character — `'b'` = 0x62.
const BINDER_IOC_TYPE: u32 = b'b' as u32;

// ============================================================================
// Kernel-side ABI structs (drivers/android/binder.h).
//
// These are `#[repr(C)]` so their layout matches the kernel struct
// exactly. They're used:
//   * To compute the size component of the ioctl numbers (via
//     `std::mem::size_of::<T>()`).
//   * To (de)serialise BINDER_WRITE_READ payloads when forwarding to
//     the host's real /dev/binder.
//
// On aarch64 / x86_64 the layouts match the kernel. On 32-bit ABIs the
// `binder_size_t` / `binder_uintptr_t` typedefs resolve to __u32 instead
// of __u64, so the structs would need a separate definition — but twoyi
// is 64-bit only, so we don't bother.
// ============================================================================

/// `struct binder_write_read` — the argument to `BINDER_WRITE_READ`.
///
/// The guest writes BC_* commands into `write_buffer` (size `write_size`,
/// consumed-so-far `write_consumed`) and the kernel writes BR_* commands
/// into `read_buffer` (size `read_size`, consumed-so-far `read_consumed`)
/// in the same call. Either size can be 0 for a one-directional call.
#[repr(C)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct BinderWriteRead {
    pub write_size:      u64, // binder_size_t
    pub write_consumed:  u64, // binder_size_t
    pub write_buffer:    u64, // binder_uintptr_t (user pointer)
    pub read_size:       u64, // binder_size_t
    pub read_consumed:   u64, // binder_size_t
    pub read_buffer:     u64, // binder_uintptr_t (user pointer)
}

/// `struct binder_ptr_cookie` — payload of `BC_ACQUIRE_DONE`,
/// `BC_INCREFS_DONE`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct BinderPtrCookie {
    pub ptr:    u64,
    pub cookie: u64,
}

/// `struct binder_handle_cookie` — payload of
/// `BC_REQUEST_DEATH_NOTIFICATION`, `BC_CLEAR_DEATH_NOTIFICATION`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct BinderHandleCookie {
    pub handle: u32,
    pub pad:    u32,
    pub cookie: u64,
}

/// `struct binder_pri_desc` — payload of `BR_ACQUIRE`, `BR_RELEASE`,
/// `BR_INCREFS`, `BR_DECREFS`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct BinderPriDesc {
    pub priority: u32,
    pub desc:     u32,
    pub pad:      u32,
}

/// `struct binder_pri_cookie` — payload of `BR_ATTEMPT_ACQUIRE`,
/// `BC_ACQUIRE_FAILED`, `BC_INCREFS_FAILED`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct BinderPriCookie {
    pub priority: u32,
    pub desc:     u32,
    pub pad:      u32,
    pub cookie:   u64,
}

/// `struct binder_transaction_data` — payload of `BC_TRANSACTION`,
/// `BC_REPLY`, `BC_TRANSACTION_SG`, `BC_REPLY_SG`, and the corresponding
/// `BR_TRANSACTION` / `BR_REPLY`.
///
/// The kernel struct has a union for `target` (4-byte `handle` OR 8-byte
/// `ptr`) followed by an 8-byte `cookie`, totalling 16 bytes. We model
/// that as `target_handle` (u32) + `target_pad` (u32) + `target_cookie`
/// (u64), so the layout matches whether the sender used the handle form
/// or the ptr form. The `data` union is similarly modelled as the larger
/// `ptr` form (16 bytes).
///
/// Total: 64 bytes on aarch64 / x86_64, matching the kernel.
#[repr(C)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct BinderTransactionData {
    pub target_handle: u32, // [ 0.. 4] — when target is a remote handle
    pub target_pad:    u32, // [ 4.. 8] — padding (or low 4 bytes of ptr)
    pub target_cookie: u64, // [ 8..16] — cookie (or high 4 bytes of ptr)
    pub code:          u32, // [16..20] — transaction code (method id)
    pub flags:         u32, // [20..24] — TF_* flags
    pub sender_pid:    i32, // [24..28] — pid_t (signed)
    pub sender_euid:   u32, // [28..32] — uid_t (unsigned)
    pub data_size:     u64, // [32..40] — size of data buffer
    pub offsets_size:  u64, // [40..48] — size of offsets array
    pub data_ptr:      u64, // [48..56] — user pointer to data buffer
    pub offsets_ptr:   u64, // [56..64] — user pointer to offsets array
}

/// `struct binder_flat_binder_object` — the structured object that
/// appears in a transaction's offsets array. Carries a strong/weak
/// binder reference (local or remote), an FD, or a scatter-gather
/// descriptor.
#[repr(C)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct FlatBinderObject {
    pub r#type: u32, // BINDER_TYPE_{BINDER,WEAK_BINDER,HANDLE,…}
    pub flags:  u32,
    pub binder: u64, // union: handle (u32) or ptr (binder_uintptr_t)
    pub cookie: u64,
}

// ============================================================================
// Binder ioctl numbers (drivers/android/binder.h).
//
// These MUST match the kernel exactly — the guest's libbinder.so uses
// these literal numbers in `ioctl()` calls, and any translation layer
// has to recognise them.
// ============================================================================

/// `BINDER_WRITE_READ` — the workhorse ioctl. Sends BC_* commands and
/// receives BR_* commands in one call.
pub const BINDER_WRITE_READ: u32 = _IOWR(
    BINDER_IOC_TYPE,
    1,
    std::mem::size_of::<BinderWriteRead>() as u32,
);

/// `BINDER_SET_IDLE_TIMEOUT` — historical, no longer used by the kernel.
#[allow(dead_code)]
pub const BINDER_SET_IDLE_TIMEOUT: u32 = _IOW(BINDER_IOC_TYPE, 3, 8);

/// `BINDER_SET_MAX_THREADS` — tell the kernel the max number of binder
/// threads the process is willing to spawn. The kernel uses this to
/// decide when to send `BR_SPAWN_LOOPER`.
pub const BINDER_SET_MAX_THREADS: u32 = _IOW(BINDER_IOC_TYPE, 5, 4);

/// `BINDER_SET_IDLE_PRIORITY` — historical, no longer used.
#[allow(dead_code)]
pub const BINDER_SET_IDLE_PRIORITY: u32 = _IOW(BINDER_IOC_TYPE, 6, 4);

/// `BINDER_SET_CONTEXT_MGR` — become the servicemanager for this binder
/// context. Only one process per context may set this. The modern kernel
/// uses `BINDER_SET_CONTEXT_MGR_EXT` (with a flat_binder_object arg);
/// `BINDER_SET_CONTEXT_MGR` is the legacy form with no arg (`_IO`).
pub const BINDER_SET_CONTEXT_MGR: u32 = _IO(BINDER_IOC_TYPE, 7);

/// `BINDER_THREAD_EXIT` — tell the kernel a binder thread is exiting
/// (so it can clean up its per-thread state).
pub const BINDER_THREAD_EXIT: u32 = _IOW(BINDER_IOC_TYPE, 8, 4);

/// `BINDER_VERSION` — query the binder protocol version. Returns a
/// `__u32` that should match `BINDER_CURRENT_PROTOCOL_VERSION`.
pub const BINDER_VERSION: u32 = _IOWR(BINDER_IOC_TYPE, 9, 4);

/// `BINDER_GET_NODE_DEBUG_INFO` — for debuggerd / dumpstate.
#[allow(dead_code)]
pub const BINDER_GET_NODE_DEBUG_INFO: u32 =
    _IOWR(BINDER_IOC_TYPE, 11, std::mem::size_of::<FlatBinderObject>() as u32);

/// `BINDER_SET_CONTEXT_MGR_EXT` — modern form of SET_CONTEXT_MGR, takes
/// a `flat_binder_object` to specify the priority and policy of the
/// manager.
#[allow(dead_code)]
pub const BINDER_SET_CONTEXT_MGR_EXT: u32 =
    _IOW(BINDER_IOC_TYPE, 13, std::mem::size_of::<FlatBinderObject>() as u32);

// ============================================================================
// BC_* — binder commands (written by userspace into the write_buffer of
// BINDER_WRITE_READ).
//
// Each BC_* is encoded in the write_buffer as a [u32 cmd][cmd-specific
// payload] pair. The cmd u32 IS the ioctl number (so the payload size
// can be extracted from bits 16..29 via [`bc_payload_size`]).
//
// The kernel enum (`enum BinderCommand` in `<uapi/linux/android/binder.h>`)
// starts at `nr=1` (nr=0 is unused in the BC_* space). We match the
// kernel exactly so the guest's libbinder.so (which uses these literal
// numbers) recognises our commands.
// ============================================================================

/// `BC_TRANSACTION` — send a synchronous (or one-way, if TF_ONE_WAY)
/// transaction to a remote binder.
pub const BC_TRANSACTION: u32 = _IOW(
    BINDER_IOC_TYPE,
    1,
    std::mem::size_of::<BinderTransactionData>() as u32,
);

/// `BC_REPLY` — send the reply to a received `BR_TRANSACTION`.
pub const BC_REPLY: u32 = _IOW(
    BINDER_IOC_TYPE,
    2,
    std::mem::size_of::<BinderTransactionData>() as u32,
);

/// `BC_ACQUIRE` — acquire a strong reference on a remote handle.
pub const BC_ACQUIRE: u32 = _IOW(BINDER_IOC_TYPE, 3, 4);

/// `BC_RELEASE` — release a strong reference on a remote handle.
pub const BC_RELEASE: u32 = _IOW(BINDER_IOC_TYPE, 4, 4);

/// `BC_INCREFS` — acquire a weak reference on a remote handle.
pub const BC_INCREFS: u32 = _IOW(BINDER_IOC_TYPE, 5, 4);

/// `BC_ACQUIRE_DONE` — acknowledge completion of a `BR_ACQUIRE`.
pub const BC_ACQUIRE_DONE: u32 = _IOW(
    BINDER_IOC_TYPE,
    6,
    std::mem::size_of::<BinderPtrCookie>() as u32,
);

/// `BC_ACQUIRE_FAILED` — failed acknowledgement (removed from modern
/// kernels but the ioctl number is still reserved).
#[allow(dead_code)]
pub const BC_ACQUIRE_FAILED: u32 = _IOW(
    BINDER_IOC_TYPE,
    7,
    std::mem::size_of::<BinderPriCookie>() as u32,
);

/// `BC_INCREFS_DONE` — acknowledge completion of a `BR_INCREFS`.
pub const BC_INCREFS_DONE: u32 = _IOW(
    BINDER_IOC_TYPE,
    8,
    std::mem::size_of::<BinderPtrCookie>() as u32,
);

/// `BC_INCREFS_FAILED` — failed acknowledgement (removed from modern
/// kernels but the ioctl number is still reserved).
#[allow(dead_code)]
pub const BC_INCREFS_FAILED: u32 = _IOW(
    BINDER_IOC_TYPE,
    9,
    std::mem::size_of::<BinderPriCookie>() as u32,
);

/// `BC_FREE_BUFFER` — return a transaction-data buffer received via
/// `BR_TRANSACTION` / `BR_REPLY` to the kernel for reuse.
pub const BC_FREE_BUFFER: u32 = _IOW(BINDER_IOC_TYPE, 10, 8);

/// `BC_TRANSACTION_SG` — scatter-gather variant of `BC_TRANSACTION`.
/// Payload is `binder_transaction_data_sg` = `binder_transaction_data`
/// (64 bytes) + `binder_size_t buffers_size` (8 bytes) = 72 bytes total.
///
/// The kernel defines this as `_IOW('b', 11, struct binder_transaction_data_sg)`
/// where `binder_transaction_data_sg` is 72 bytes. Using 64 (just the base
/// struct) produces the wrong ioctl number and the guest's `libbinder.so`
/// (which uses the kernel literal) won't match — silently dropping ALL
/// scatter-gather transactions.
pub const BC_TRANSACTION_SG: u32 = _IOW(
    BINDER_IOC_TYPE,
    11,
    // 72 = size_of::<BinderTransactionData>() + size_of::<u64>()
    // (binder_transaction_data + buffers_size)
    (std::mem::size_of::<BinderTransactionData>() + 8) as u32,
);

/// `BC_REPLY_SG` — scatter-gather variant of `BC_REPLY`.
/// Same 72-byte payload as `BC_TRANSACTION_SG`.
pub const BC_REPLY_SG: u32 = _IOW(
    BINDER_IOC_TYPE,
    12,
    (std::mem::size_of::<BinderTransactionData>() + 8) as u32,
);

/// `BC_ENTER_LOOPER` — declare this thread a binder looper (it'll call
/// `BINDER_WRITE_READ` with `read_size > 0` to block waiting for work).
pub const BC_ENTER_LOOPER: u32 = _IO(BINDER_IOC_TYPE, 13);

/// `BC_REGISTER_LOOPER` — declare this thread was spawned by the
/// application in response to `BR_SPAWN_LOOPER`.
pub const BC_REGISTER_LOOPER: u32 = _IO(BINDER_IOC_TYPE, 14);

/// `BC_EXIT_LOOPER` — declare this thread is exiting the looper.
pub const BC_EXIT_LOOPER: u32 = _IO(BINDER_IOC_TYPE, 15);

/// `BC_REQUEST_DEATH_NOTIFICATION` — ask the kernel to send
/// `BR_DEAD_BINDER` when the referenced handle dies.
pub const BC_REQUEST_DEATH_NOTIFICATION: u32 = _IOW(
    BINDER_IOC_TYPE,
    16,
    std::mem::size_of::<BinderHandleCookie>() as u32,
);

/// `BC_CLEAR_DEATH_NOTIFICATION` — cancel a previous
/// `BC_REQUEST_DEATH_NOTIFICATION`.
pub const BC_CLEAR_DEATH_NOTIFICATION: u32 = _IOW(
    BINDER_IOC_TYPE,
    17,
    std::mem::size_of::<BinderHandleCookie>() as u32,
);

/// `BC_DEAD_BINDER_DONE` — acknowledge receipt of a `BR_DEAD_BINDER`.
pub const BC_DEAD_BINDER_DONE: u32 = _IOW(BINDER_IOC_TYPE, 18, 8);

// ============================================================================
// BR_* — binder returns (written by the kernel into the read_buffer of
// BINDER_WRITE_READ, and by our proxy into the read_buffer of our wire
// BINDER_WRITE_READ response).
//
// The kernel enum (`enum BinderReturn`) uses nr=0 for BR_ERROR, nr=1
// for BR_OK, then nr=2..8 for BR_TRANSACTION..BR_ATTEMPT_ACQUIRE, and
// nr=12..17 for the rest. We match the kernel exactly.
// ============================================================================

/// `BR_ERROR` — kernel returned an error.
#[allow(dead_code)]
pub const BR_ERROR: u32 = _IOR(BINDER_IOC_TYPE, 0, 4);

/// `BR_OK` — success (kernel often sends this as a heartbeat).
#[allow(dead_code)]
pub const BR_OK: u32 = _IO(BINDER_IOC_TYPE, 1);

/// `BR_TRANSACTION` — incoming transaction from another process.
pub const BR_TRANSACTION: u32 = _IOR(
    BINDER_IOC_TYPE,
    2,
    std::mem::size_of::<BinderTransactionData>() as u32,
);

/// `BR_REPLY` — reply to a previously-sent `BC_TRANSACTION`.
pub const BR_REPLY: u32 = _IOR(
    BINDER_IOC_TYPE,
    3,
    std::mem::size_of::<BinderTransactionData>() as u32,
);

/// `BR_ACQUIRE` — acquire a strong reference on a local binder.
#[allow(dead_code)]
pub const BR_ACQUIRE: u32 = _IOR(
    BINDER_IOC_TYPE,
    4,
    std::mem::size_of::<BinderPriDesc>() as u32,
);

/// `BR_RELEASE` — release a strong reference on a local binder.
#[allow(dead_code)]
pub const BR_RELEASE: u32 = _IOR(
    BINDER_IOC_TYPE,
    5,
    std::mem::size_of::<BinderPriDesc>() as u32,
);

/// `BR_INCREFS` — acquire a weak reference on a local binder.
#[allow(dead_code)]
pub const BR_INCREFS: u32 = _IOR(
    BINDER_IOC_TYPE,
    6,
    std::mem::size_of::<BinderPriDesc>() as u32,
);

/// `BR_DECREFS` — release a weak reference on a local binder.
#[allow(dead_code)]
pub const BR_DECREFS: u32 = _IOR(
    BINDER_IOC_TYPE,
    7,
    std::mem::size_of::<BinderPriDesc>() as u32,
);

/// `BR_ATTEMPT_ACQUIRE` — try-acquire (rare).
#[allow(dead_code)]
pub const BR_ATTEMPT_ACQUIRE: u32 = _IOR(
    BINDER_IOC_TYPE,
    8,
    std::mem::size_of::<BinderPriCookie>() as u32,
);

/// `BR_NOOP` — no-op. The looper consumes this and loops again.
pub const BR_NOOP: u32 = _IO(BINDER_IOC_TYPE, 12);

/// `BR_SPAWN_LOOPER` — kernel wants the process to spawn another binder
/// thread (up to the `BINDER_SET_MAX_THREADS` limit).
pub const BR_SPAWN_LOOPER: u32 = _IO(BINDER_IOC_TYPE, 13);

/// `BR_FINISHED` — historical, no longer sent by the kernel.
#[allow(dead_code)]
pub const BR_FINISHED: u32 = _IO(BINDER_IOC_TYPE, 14);

/// `BR_DEAD_BINDER` — a remote binder we requested death notification
/// for has died.
#[allow(dead_code)]
pub const BR_DEAD_BINDER: u32 = _IOR(BINDER_IOC_TYPE, 15, 8);

/// `BR_CLEAR_DEATH_NOTIFICATION_DONE` — ack of `BC_CLEAR_DEATH_NOTIFICATION`.
#[allow(dead_code)]
pub const BR_CLEAR_DEATH_NOTIFICATION_DONE: u32 = _IOR(BINDER_IOC_TYPE, 16, 8);

/// `BR_FAILED_REPLY` — the last `BC_TRANSACTION` failed (e.g. the
/// target handle is invalid, or the target process died).
pub const BR_FAILED_REPLY: u32 = _IO(BINDER_IOC_TYPE, 17);

// ============================================================================
// Service manager transaction codes
// (frameworks/native/libs/binder/IServiceManager.cpp).
//
// These are the `code` field of `binder_transaction_data` when the
// target handle is 0 (the servicemanager's well-known handle).
// ============================================================================

/// `SVC_MGR_GET_SERVICE` — look up a service by name, return a strong
/// binder. Old name; `SVC_MGR_CHECK_SERVICE` is the same code.
pub const SVC_MGR_GET_SERVICE: u32 = 1;

/// `SVC_MGR_CHECK_SERVICE` — same as `SVC_MGR_GET_SERVICE` (alias).
pub const SVC_MGR_CHECK_SERVICE: u32 = 2;

/// `SVC_MGR_ADD_SERVICE` — register a service by name. The transaction
/// carries the service name + a strong binder reference.
pub const SVC_MGR_ADD_SERVICE: u32 = 3;

/// `SVC_MGR_LIST_SERVICES` — enumerate registered services by index.
pub const SVC_MGR_LIST_SERVICES: u32 = 4;

/// `SVC_MGR_CHECK_SERVICE_IF_EXIST` — existence check (rare).
#[allow(dead_code)]
pub const SVC_MGR_CHECK_SERVICE_IF_EXIST: u32 = 5;

/// The well-known binder handle of the servicemanager itself.
pub const SVC_MGR_HANDLE: u32 = 0;

// ============================================================================
// Flat-binder-object type constants (drivers/android/binder.h).
// ============================================================================

#[allow(dead_code)]
pub const BINDER_TYPE_BINDER:       u32 = 1; // strong local binder
#[allow(dead_code)]
pub const BINDER_TYPE_WEAK_BINDER:  u32 = 2; // weak local binder
#[allow(dead_code)]
pub const BINDER_TYPE_HANDLE:       u32 = 3; // strong remote ref
#[allow(dead_code)]
pub const BINDER_TYPE_WEAK_HANDLE:  u32 = 4; // weak remote ref
#[allow(dead_code)]
pub const BINDER_TYPE_FD:           u32 = 5; // file descriptor
#[allow(dead_code)]
pub const BINDER_TYPE_FDA:          u32 = 6; // FD array
#[allow(dead_code)]
pub const BINDER_TYPE_PTR:          u32 = 7; // scatter-gather pointer

// ============================================================================
// Transaction flags (binder_transaction_data.flags).
// ============================================================================

/// `TF_ONE_WAY` — the transaction is asynchronous (no reply expected).
pub const TF_ONE_WAY: u32 = 0x01;
/// `TF_ROOT_OBJECT` — the data buffer's first offset is the root object.
#[allow(dead_code)]
pub const TF_ROOT_OBJECT: u32 = 0x04;
/// `TF_STATUS_CODE` — the data buffer is a single `i32` status code.
#[allow(dead_code)]
pub const TF_STATUS_CODE: u32 = 0x08;
/// `TF_ACCEPT_FDS` — the sender is willing to receive FDs in the reply.
pub const TF_ACCEPT_FDS: u32 = 0x10;

// ============================================================================
// Misc constants.
// ============================================================================

/// Binder protocol version returned by `BINDER_VERSION`. Matches
/// `CURRENT_PROTOCOL_VERSION` in `drivers/android/binder.c`. Android 11
/// ships protocol version 8.
pub const BINDER_CURRENT_PROTOCOL_VERSION: u32 = 8;

/// Number of worker threads in the binder proxy's thread pool. Each
/// worker handles one concurrent guest connection. 4 matches what VM's
/// `libvm.so` uses (based on the `BINDER_SET_MAX_THREADS` value seen in
/// the BinderService disassembly — see `VM_JAVA_ANALYSIS.md` §5.2).
pub const BINDER_THREAD_POOL_SIZE: usize = 4;

// ============================================================================
// Wire framing for the Unix-socket proxy protocol.
//
// The guest's libbinder.so is patched (or shimmed via LD_PRELOAD) to
// translate `ioctl(fd, BINDER_*, arg)` calls into framed socket messages
// on the per-VM /vm%d/dev/binder Unix socket. Each frame is:
//
//   [u32 cmd]      — the binder ioctl number (BINDER_WRITE_READ, …)
//   [u32 arg_len]  — payload size in bytes (0 for _IO ioctls)
//   [u32 arg_len bytes of payload]
//
// The server responds with:
//
//   [i32 ret]      — 0 on success, -errno on failure
//   [u32 arg_len]  — response payload size in bytes
//   [u32 arg_len bytes of payload]
//
// For `BINDER_WRITE_READ` specifically, the request payload is our own
// [`WireBinderWriteRead`] (NOT the kernel struct, because the kernel
// struct uses pointers that don't make sense over a socket), and the
// response payload is [`WireBinderWriteReadResponse`].
// ============================================================================

/// A parsed request frame received from the guest.
struct Frame {
    /// The binder ioctl number (`BINDER_WRITE_READ`, `BINDER_VERSION`, …).
    cmd: u32,
    /// Variable-length payload (the ioctl's `arg` bytes).
    payload: Vec<u8>,
}

/// A response frame sent back to the guest.
struct Resp {
    /// Return value: 0 on success, negative errno on failure.
    ret: i32,
    /// Variable-length response payload (the bytes the ioctl would have
    /// written into `arg`).
    payload: Vec<u8>,
}

/// Serialised `BINDER_WRITE_READ` request payload (our own wire format —
/// NOT the kernel struct, because the kernel struct uses user pointers
/// that don't make sense over a socket).
///
/// Layout: `[u32 write_size][u32 read_capacity][write_size bytes]`.
#[derive(Default)]
#[allow(dead_code)]
struct WireBinderWriteRead {
    /// The guest's outgoing BC_* command stream.
    write_buffer: Vec<u8>,
    /// Maximum bytes the guest is willing to receive in the read_buffer.
    /// The server may return fewer.
    read_capacity: u32,
}

/// Serialised `BINDER_WRITE_READ` response payload.
///
/// Layout: `[u32 read_size][read_size bytes]`.
#[derive(Default)]
#[allow(dead_code)]
struct WireBinderWriteReadResponse {
    /// The BR_* command stream the server wants the guest to consume.
    read_buffer: Vec<u8>,
}

// ============================================================================
// Service-manager handle table.
// ============================================================================

/// Per-VM handle table — maps guest-visible binder handles to host
/// binder handles and to service names.
///
/// When the guest calls `SVC_MGR_GET_SERVICE("activity")`, the proxy:
///   1. Looks up "activity" in `by_name` to find the guest handle.
///   2. If not present, calls `SVC_MGR_GET_SERVICE("activity")` on the
///      host's `/dev/binder` to get the host handle, allocates a new
///      guest handle, and records both mappings.
///   3. Returns the guest handle to the guest as a strong binder in the
///      reply parcel.
///
/// Subsequent `BC_TRANSACTION` calls from the guest to that guest handle
/// are translated: `target.handle` is rewritten from guest handle to
/// host handle before forwarding to the host's `/dev/binder`.
#[derive(Default)]
pub struct HandleTable {
    /// guest_handle → host_handle.
    by_guest: HashMap<u32, u32>,
    /// service_name → guest_handle.
    by_name: HashMap<String, u32>,
    /// Next guest handle to allocate (starts at 1; 0 is reserved for
    /// the servicemanager itself).
    next: u32,
}

impl HandleTable {
    /// Create an empty handle table. Handle 0 is reserved for the
    /// servicemanager; the first allocated handle is 1.
    pub fn new() -> Self {
        HandleTable {
            by_guest: HashMap::new(),
            by_name: HashMap::new(),
            next: 1,
        }
    }

    /// Allocate a new guest handle bound to `host_handle` and return it.
    pub fn allocate(&mut self, host_handle: u32) -> u32 {
        let g = self.next;
        self.next += 1;
        self.by_guest.insert(g, host_handle);
        g
    }

    /// Record that service `name` is reachable via guest handle `g`.
    pub fn register(&mut self, name: &str, guest_handle: u32) {
        self.by_name.insert(name.to_string(), guest_handle);
    }

    /// Look up a guest handle by service name.
    pub fn lookup_by_name(&self, name: &str) -> Option<u32> {
        self.by_name.get(name).copied()
    }

    /// Translate a guest handle to the corresponding host handle.
    pub fn lookup_host(&self, guest_handle: u32) -> Option<u32> {
        self.by_guest.get(&guest_handle).copied()
    }
}

// ============================================================================
// Device creation.
// ============================================================================

/// Create the per-VM binder device.
///
/// * Creates `{rootfs}/vm{vm_id}/dev/binder` as a Unix-domain socket —
///   this is the actual socket the guest connects to. (`rootfs` here is
///   treated as the per-VM data directory; the guest's chroot rootfs is
///   a sibling, not a parent.)
/// * Creates `{rootfs}/dev/binder` as a symlink to
///   `../vm{vm_id}/dev/binder` so the guest (chrooted into `rootfs`)
///   sees the conventional `/dev/binder` path. The symlink target is
///   relative so it resolves correctly inside the chroot.
///
/// The function binds the socket listener, sets mode 0666 on it (so the
/// guest process — which may run as a different uid inside the chroot —
/// can `connect()`), then drops the listener and returns the path. The
/// caller is expected to immediately pass the path to
/// [`BinderProxy::new`], which re-binds it. (The alternative — returning
/// the listener itself — is awkward because `UnixListener` doesn't
/// `Clone`, and we want `create_binder_device` to be callable
/// independently of `BinderProxy`.)
///
/// # Errors
///
/// Returns an error if directory creation, `UnixListener::bind`, or
/// `symlink` fails. Stale socket files / symlinks from a previous run
/// are best-effort removed before bind (errors are logged but not
/// propagated).
pub fn create_binder_device(rootfs: &str, vm_id: u32) -> std::io::Result<String> {
    let vm_dir = format!("{}/vm{}", rootfs, vm_id);
    let vm_dev = format!("{}/dev", vm_dir);
    let sock_path = format!("{}/dev/binder", vm_dir);
    let link_path = format!("{}/dev/binder", rootfs);

    // Make sure /vm{id}/dev and /dev exist.
    fs::create_dir_all(&vm_dev)?;
    fs::create_dir_all(format!("{}/dev", rootfs))?;

    #[cfg(unix)]
    {
        let _ = fs::set_permissions(&vm_dev, fs::Permissions::from_mode(0o755));
        let _ = fs::set_permissions(format!("{}/dev", rootfs), fs::Permissions::from_mode(0o755));
    }

    // Remove stale socket / symlink from a previous run.
    match fs::remove_file(&sock_path) {
        Ok(()) => info!("[KR64][binder] removed stale socket: {}", sock_path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warning!("[KR64][binder] could not remove {}: {}", sock_path, e),
    }
    match fs::remove_file(&link_path) {
        Ok(()) => info!("[KR64][binder] removed stale symlink: {}", link_path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warning!("[KR64][binder] could not remove {}: {}", link_path, e),
    }

    // Bind the Unix listener. This creates the socket file as a side
    // effect. (A production version would first `mknodat(S_IFSOCK|0666)`
    // then `bind()` to it — matching VM's exact pattern at libkr64.so
    // offset 0x11d770 — but `mknodat` requires CAP_MKNOD which is
    // unavailable in many sandboxes. `UnixListener::bind` is the
    // unprivileged fallback and works fine for the skeleton.)
    let listener = UnixListener::bind(&sock_path)?;
    #[cfg(unix)]
    {
        let _ = fs::set_permissions(&sock_path, fs::Permissions::from_mode(0o666));
    }

    // Create the symlink. Target is RELATIVE (`../vm{id}/dev/binder`)
    // so the kernel resolves it relative to the symlink's own location
    // — i.e. `{rootfs}/dev/` — which yields `{rootfs}/vm{id}/dev/binder`.
    // This works inside the chroot too (no leading `/`).
    #[cfg(unix)]
    {
        let target = format!("../vm{}/dev/binder", vm_id);
        std::os::unix::fs::symlink(&target, &link_path)?;
    }

    info!(
        "[KR64][binder] created socket {} (fd={}) and symlink {} → ../vm{}/dev/binder",
        sock_path,
        listener.as_raw_fd(),
        link_path,
        vm_id
    );

    // Drop the listener — the caller will re-bind via BinderProxy::new.
    // We unlink the socket file so the re-bind doesn't hit EADDRINUSE.
    drop(listener);
    let _ = fs::remove_file(&sock_path);
    Ok(sock_path)
}

// ============================================================================
// Binder proxy — owns the listener + worker pool, accepts guest
// connections, dispatches per-ioctl.
// ============================================================================

/// Owned binder proxy for one VM. Created via [`BinderProxy::new`],
/// started via [`BinderProxy::spawn`] (which consumes self and returns a
/// [`BinderProxyHandle`]).
///
/// The proxy owns:
///   * The `UnixListener` bound to `{rootfs}/vm{id}/dev/binder`.
///   * A lazily-opened file descriptor for the host's `/dev/binder`
///     (opened on the first `BC_TRANSACTION` that needs forwarding).
///   * A per-VM [`HandleTable`] (wrapped in `Arc<Mutex<…>>` so worker
///     threads can share it).
///   * A `shutdown` flag (atomic) used to ask the accept thread to exit.
pub struct BinderProxy {
    vm_id: u32,
    listener: Option<UnixListener>,
    path: String,
    /// Opened lazily on the first `BC_TRANSACTION` that targets a
    /// non-servicemanager handle. `None` means "not yet opened" (or
    /// "open failed and we're returning errors for all forwards").
    host_binder_fd: Arc<Mutex<Option<RawFd>>>,
    /// Per-VM handle table (guest handle ↔ host handle ↔ service name).
    handles: Arc<Mutex<HandleTable>>,
    /// Set to true by [`BinderProxyHandle::shutdown`] / drop to ask the
    /// accept thread to exit.
    shutdown: Arc<AtomicBool>,
}

impl BinderProxy {
    /// Construct a new binder proxy for `vm_id`, binding a Unix listener
    /// to `socket_path` (which should be the path returned by
    /// [`create_binder_device`]).
    pub fn new(vm_id: u32, socket_path: &str) -> std::io::Result<Self> {
        // Best-effort unlink of a stale socket from a previous run.
        let _ = fs::remove_file(socket_path);

        let listener = UnixListener::bind(socket_path)?;
        #[cfg(unix)]
        {
            let _ = fs::set_permissions(socket_path, fs::Permissions::from_mode(0o666));
        }

        // Make the listening socket non-blocking so the accept thread
        // can poll the shutdown flag between accept attempts.
        let fd = listener.as_raw_fd();
        let _ = unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) };

        info!(
            "[KR64][binder][vm{}] proxy bound to {} (fd={}, non-blocking)",
            vm_id,
            socket_path,
            fd
        );

        Ok(BinderProxy {
            vm_id,
            listener: Some(listener),
            path: socket_path.to_string(),
            host_binder_fd: Arc::new(Mutex::new(None)),
            handles: Arc::new(Mutex::new(HandleTable::new())),
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Spawn the accept thread + worker pool, consuming self.
    ///
    /// Returns a [`BinderProxyHandle`] that holds the shutdown flag and
    /// the accept-thread `JoinHandle`. When the handle is dropped, the
    /// shutdown flag is set and the accept thread is joined.
    pub fn spawn(mut self) -> std::io::Result<BinderProxyHandle> {
        let listener = self
            .listener
            .take()
            .expect("BinderProxy::spawn: listener already taken");
        let host_fd = Arc::clone(&self.host_binder_fd);
        let handles = Arc::clone(&self.handles);
        // Clone the shutdown Arc twice: one for the accept thread, one
        // for the returned handle. Both share the same AtomicBool.
        let shutdown_for_thread = Arc::clone(&self.shutdown);
        let shutdown_for_handle = Arc::clone(&self.shutdown);
        let vm_id = self.vm_id;
        let path = self.path.clone();

        let accept_thread = thread::Builder::new()
            .name(format!("kr64-binder-accept-{}", vm_id))
            .spawn(move || {
                // The pool lives inside the accept thread so its Drop
                // (which joins workers) runs when the accept thread
                // exits. This ensures workers are joined BEFORE the
                // accept thread returns.
                let pool = ThreadPool::new(BINDER_THREAD_POOL_SIZE);
                info!(
                    "[KR64][binder][vm{}] accept loop started (pool_size={})",
                    vm_id, BINDER_THREAD_POOL_SIZE
                );

                while !shutdown_for_thread.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, _addr)) => {
                            info!("[KR64][binder][vm{}] client connected", vm_id);
                            let host_fd = Arc::clone(&host_fd);
                            let handles = Arc::clone(&handles);
                            pool.execute(move || {
                                if let Err(e) =
                                    handle_connection(stream, vm_id, &host_fd, &handles)
                                {
                                    warning!(
                                        "[KR64][binder][vm{}] connection handler ended: {}",
                                        vm_id, e
                                    );
                                }
                            });
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // No pending connection — sleep briefly so
                            // we don't burn CPU. The accept thread's
                            // main job is to wait for the next
                            // connection or for shutdown.
                            std::thread::sleep(std::time::Duration::from_millis(25));
                        }
                        Err(e) => {
                            warning!("[KR64][binder][vm{}] accept error: {}", vm_id, e);
                            std::thread::sleep(std::time::Duration::from_millis(50));
                        }
                    }
                }
                info!("[KR64][binder][vm{}] accept loop exiting", vm_id);
                // pool drops here → workers receive Terminate and join.
            })?;

        Ok(BinderProxyHandle {
            shutdown: shutdown_for_handle,
            accept_thread: Some(accept_thread),
            path,
        })
    }
}

/// Handle to a running binder proxy. Dropping this sets the shutdown
/// flag and joins the accept thread.
pub struct BinderProxyHandle {
    shutdown: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
    path: String,
}

impl BinderProxyHandle {
    /// Ask the accept thread to shut down. (Does not join — that
    /// happens on drop.)
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }

    /// The socket path the proxy is listening on.
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl Drop for BinderProxyHandle {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        if let Some(t) = self.accept_thread.take() {
            let _ = t.join();
        }
        // Best-effort unlink the socket file so a re-run of the daemon
        // doesn't fail with EADDRINUSE.
        let _ = fs::remove_file(&self.path);
    }
}

// ============================================================================
// Minimal thread pool — fixed-size, MPMC via std::sync::mpsc.
//
// We can't add `rayon` / `crossbeam` / etc. (the crate is std + libc
// only), so we roll our own. This is the classic Rust-book ThreadPool
// with a Terminate control message added for clean shutdown.
// ============================================================================

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    Job(Job),
    Terminate,
}

struct Worker {
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Self {
        let thread = thread::spawn(move || loop {
            let msg = receiver.lock().unwrap().recv();
            match msg {
                Ok(Message::Job(job)) => job(),
                Ok(Message::Terminate) | Err(_) => break,
            }
        });
        Worker {
            thread: Some(thread),
        }
    }
}

/// A fixed-size thread pool. Used by [`BinderProxy`] to handle multiple
/// concurrent guest connections.
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Message>>,
}

impl ThreadPool {
    /// Create a pool with `size` worker threads. Panics if `size == 0`.
    pub fn new(size: usize) -> Self {
        assert!(size > 0, "ThreadPool::new: size must be > 0");
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);
        for _ in 0..size {
            workers.push(Worker::new(Arc::clone(&receiver)));
        }
        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    /// Submit a job to the pool. If all workers are busy, the job is
    /// queued until one becomes free.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if let Some(s) = &self.sender {
            if s.send(Message::Job(Box::new(f))).is_err() {
                warning!("[KR64][binder] thread pool: sender closed, job dropped");
            }
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Close the sender so workers' recv() returns Err and they exit.
        if let Some(s) = self.sender.take() {
            for _ in &self.workers {
                let _ = s.send(Message::Terminate);
            }
        }
        // Join each worker.
        for w in &mut self.workers {
            if let Some(t) = w.thread.take() {
                let _ = t.join();
            }
        }
    }
}

// ============================================================================
// Per-connection handler.
// ============================================================================

/// Handle one guest connection: read frames, dispatch, write responses.
/// Returns when the guest disconnects (EOF) or an unrecoverable I/O
/// error occurs.
fn handle_connection(
    mut stream: UnixStream,
    vm_id: u32,
    host_fd: &Arc<Mutex<Option<RawFd>>>,
    handles: &Arc<Mutex<HandleTable>>,
) -> io::Result<()> {
    info!("[KR64][binder][vm{}] handling new connection", vm_id);
    loop {
        let req = match read_frame(&mut stream) {
            Ok(r) => r,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                info!("[KR64][binder][vm{}] client disconnected", vm_id);
                return Ok(());
            }
            Err(e) => return Err(e),
        };
        let resp = dispatch_request(&req, vm_id, host_fd, handles);
        write_frame(&mut stream, &resp)?;
    }
}

// ============================================================================
// Ioctl dispatcher.
// ============================================================================

/// Dispatch one parsed request frame to the appropriate handler.
fn dispatch_request(
    req: &Frame,
    vm_id: u32,
    host_fd: &Arc<Mutex<Option<RawFd>>>,
    handles: &Arc<Mutex<HandleTable>>,
) -> Resp {
    match req.cmd {
        BINDER_VERSION => handle_version(vm_id),

        BINDER_SET_MAX_THREADS => {
            let n = if req.payload.len() >= 4 {
                u32::from_ne_bytes(req.payload[0..4].try_into().unwrap_or([0; 4]))
            } else {
                0
            };
            info!(
                "[KR64][binder][vm{}] SET_MAX_THREADS = {} (acknowledged)",
                vm_id, n
            );
            Resp {
                ret: 0,
                payload: Vec::new(),
            }
        }

        BINDER_SET_CONTEXT_MGR => {
            info!(
                "[KR64][binder][vm{}] SET_CONTEXT_MGR — guest is now the servicemanager",
                vm_id
            );
            // Accept: the guest's servicemanager has registered with us.
            // We do NOT proxy this to the host — the host already has
            // its own servicemanager on /dev/binder.
            Resp {
                ret: 0,
                payload: Vec::new(),
            }
        }

        BINDER_THREAD_EXIT => {
            info!("[KR64][binder][vm{}] THREAD_EXIT", vm_id);
            Resp {
                ret: 0,
                payload: Vec::new(),
            }
        }

        BINDER_WRITE_READ => handle_write_read(&req.payload, vm_id, host_fd, handles),

        other => {
            warning!(
                "[KR64][binder][vm{}] unknown ioctl 0x{:08x} ({} bytes payload)",
                vm_id,
                other,
                req.payload.len()
            );
            Resp {
                ret: -(libc::EINVAL),
                payload: Vec::new(),
            }
        }
    }
}

/// `BINDER_VERSION` handler — return the protocol version.
fn handle_version(vm_id: u32) -> Resp {
    info!(
        "[KR64][binder][vm{}] VERSION → {}",
        vm_id, BINDER_CURRENT_PROTOCOL_VERSION
    );
    Resp {
        ret: 0,
        payload: BINDER_CURRENT_PROTOCOL_VERSION.to_ne_bytes().to_vec(),
    }
}

// ============================================================================
// BINDER_WRITE_READ handler — the workhorse.
// ============================================================================

/// Handle a `BINDER_WRITE_READ` request.
///
/// The wire payload is [`WireBinderWriteRead`]: `[u32 write_size]
/// [u32 read_capacity][write_size bytes]`. We parse the write_buffer
/// into individual BC_* commands, dispatch each one, and build a
/// read_buffer of BR_* commands to return.
fn handle_write_read(
    payload: &[u8],
    vm_id: u32,
    host_fd: &Arc<Mutex<Option<RawFd>>>,
    handles: &Arc<Mutex<HandleTable>>,
) -> Resp {
    // Parse the wire header.
    if payload.len() < 8 {
        return Resp {
            ret: -(libc::EINVAL),
            payload: Vec::new(),
        };
    }
    let write_size = u32::from_ne_bytes(payload[0..4].try_into().unwrap()) as usize;
    let read_capacity = u32::from_ne_bytes(payload[4..8].try_into().unwrap());
    if payload.len() < 8 + write_size {
        warning!(
            "[KR64][binder][vm{}] BINDER_WRITE_READ: truncated payload (write_size={}, have {})",
            vm_id,
            write_size,
            payload.len() - 8
        );
        return Resp {
            ret: -(libc::EINVAL),
            payload: Vec::new(),
        };
    }
    let write_buf = &payload[8..8 + write_size];

    // Process each BC_* command in the write buffer.
    let mut read_buf: Vec<u8> = Vec::new();
    let mut consumed = 0usize;
    while consumed + 4 <= write_buf.len() {
        let cmd = u32::from_ne_bytes(write_buf[consumed..consumed + 4].try_into().unwrap());
        consumed += 4;
        let psize = bc_payload_size(cmd) as usize;
        if consumed + psize > write_buf.len() {
            warning!(
                "[KR64][binder][vm{}] truncated BC_* cmd 0x{:08x} (need {}, have {})",
                vm_id,
                cmd,
                psize,
                write_buf.len() - consumed
            );
            break;
        }
        let cmd_payload = &write_buf[consumed..consumed + psize];
        consumed += psize;

        match cmd {
            BC_TRANSACTION | BC_TRANSACTION_SG => {
                let result = handle_transaction(cmd_payload, vm_id, host_fd, handles);
                match result {
                    TransactionResult::Failed => {
                        push_br_failed_reply(&mut read_buf);
                    }
                    TransactionResult::Reply(data) => {
                        push_br_reply(&mut read_buf, &data);
                    }
                }
            }
            BC_REPLY | BC_REPLY_SG => {
                // We're the server side, so we shouldn't receive
                // BC_REPLY from the guest (the guest is the client).
                // Log and ignore.
                info!(
                    "[KR64][binder][vm{}] BC_REPLY from guest — ignored (skeleton)",
                    vm_id
                );
            }
            BC_ACQUIRE | BC_RELEASE | BC_INCREFS => {
                // Strong/weak refcount changes on remote handles. For
                // the skeleton we accept and ignore — a real impl would
                // forward these to the host's /dev/binder so the host's
                // binder driver can manage refcounts on the host's
                // servicemanager / ActivityManager etc.
            }
            BC_ACQUIRE_DONE | BC_INCREFS_DONE | BC_ACQUIRE_FAILED | BC_INCREFS_FAILED => {
                // Acknowledgements of refcount operations on local
                // binders. The skeleton doesn't host any local binders
                // (no guest-side services), so these are no-ops.
            }
            BC_FREE_BUFFER | BC_DEAD_BINDER_DONE => {
                // Return a transaction-data buffer to the kernel, or
                // acknowledge a death notification. Skeleton: no-op.
            }
            BC_ENTER_LOOPER | BC_REGISTER_LOOPER | BC_EXIT_LOOPER => {
                info!(
                    "[KR64][binder][vm{}] looper state change: 0x{:08x}",
                    vm_id, cmd
                );
            }
            BC_REQUEST_DEATH_NOTIFICATION | BC_CLEAR_DEATH_NOTIFICATION => {
                info!(
                    "[KR64][binder][vm{}] death-notification request: 0x{:08x}",
                    vm_id, cmd
                );
            }
            _ => {
                warning!(
                    "[KR64][binder][vm{}] unhandled BC_* 0x{:08x} ({} bytes)",
                    vm_id,
                    cmd,
                    psize
                );
            }
        }
    }

    // If we have read capacity left and produced nothing, push a BR_NOOP
    // so the guest's looper doesn't busy-spin on empty BINDER_WRITE_READ
    // returns.
    if read_buf.is_empty() && read_capacity > 0 {
        push_br_noop(&mut read_buf);
    }

    // Build the wire response: [u32 read_size][read_size bytes].
    let mut resp_payload = Vec::with_capacity(4 + read_buf.len());
    resp_payload.extend_from_slice(&(read_buf.len() as u32).to_ne_bytes());
    resp_payload.extend_from_slice(&read_buf);

    Resp {
        ret: 0,
        payload: resp_payload,
    }
}

// ============================================================================
// Transaction dispatch — servicemanager vs forward-to-host.
// ============================================================================

/// Result of handling a `BC_TRANSACTION`.
///
/// Note: a previous `Noop` variant (meaning "no reply pushed") was removed —
/// returning `Noop` caused the guest to busy-spin on `BR_NOOP` forever
/// (see the comment on `servicemanager_proxy`'s `SVC_MGR_ADD_SERVICE` arm).
/// Every handler MUST push either a `BR_REPLY` or `BR_FAILED_REPLY` so the
/// guest's `BINDER_WRITE_READ` loop terminates.
enum TransactionResult {
    /// Push a `BR_FAILED_REPLY` into the read buffer.
    Failed,
    /// Push a `BR_REPLY` with the given reply data bytes.
    Reply(Vec<u8>),
}

/// Handle a `BC_TRANSACTION` (or `BC_TRANSACTION_SG`) command.
///
/// If the target handle is [`SVC_MGR_HANDLE`] (0), this is a
/// servicemanager transaction and we dispatch to
/// [`servicemanager_proxy`]. Otherwise we forward to the host's
/// `/dev/binder` via [`forward_transaction_to_host`].
fn handle_transaction(
    cmd_payload: &[u8],
    vm_id: u32,
    host_fd: &Arc<Mutex<Option<RawFd>>>,
    handles: &Arc<Mutex<HandleTable>>,
) -> TransactionResult {
    if cmd_payload.len() < std::mem::size_of::<BinderTransactionData>() {
        warning!(
            "[KR64][binder][vm{}] BC_TRANSACTION: payload too small ({} < {})",
            vm_id,
            cmd_payload.len(),
            std::mem::size_of::<BinderTransactionData>()
        );
        return TransactionResult::Failed;
    }

    // Parse the fields we care about.
    let target_handle = u32::from_ne_bytes(cmd_payload[0..4].try_into().unwrap());
    let code = u32::from_ne_bytes(cmd_payload[16..20].try_into().unwrap());
    let flags = u32::from_ne_bytes(cmd_payload[20..24].try_into().unwrap());

    if target_handle == SVC_MGR_HANDLE {
        info!(
            "[KR64][binder][vm{}] servicemanager transaction: code={} flags=0x{:02x}",
            vm_id, code, flags
        );
        return servicemanager_proxy(cmd_payload, code, handles);
    }

    info!(
        "[KR64][binder][vm{}] transaction to handle {}: code={} flags=0x{:02x} — forwarding to host (skeleton)",
        vm_id, target_handle, code, flags
    );

    // Skeleton: forward to host. The full implementation would
    // translate the guest handle to the host handle, patch the
    // flat_binder_object array, and copy in the data buffer before
    // issuing the ioctl. This skeleton just issues the raw ioctl with
    // the unmodified transaction data — which will fail on the host
    // side (the host's /dev/binder doesn't know about our guest
    // handles), but it has the right structure.
    match forward_transaction_to_host(cmd_payload, host_fd) {
        Ok(reply_bytes) => TransactionResult::Reply(reply_bytes),
        Err(e) => {
            warning!(
                "[KR64][binder][vm{}] forward_transaction_to_host failed: {}",
                vm_id, e
            );
            TransactionResult::Failed
        }
    }
}

/// Intercept servicemanager transactions (target handle 0).
///
/// # Parcel layout (AOSP `frameworks/native/libs/binder/IServiceManager.cpp`)
///
/// The transaction's data buffer is a parcel:
///
/// ```text
///   i32  strict_mode_policy
///   i32  work_source
///   u16[] interface_descriptor_string  (length-prefixed)
///   …    per-code arguments
/// ```
///
/// For `SVC_MGR_GET_SERVICE` the per-code argument is a length-prefixed
/// UTF-16 service name string. For `SVC_MGR_ADD_SERVICE` it's the
/// service name + a strong binder flat object.
///
/// # Skeleton behaviour
///
/// The skeleton does NOT parse the parcel — it would need to follow the
/// `binder_transaction_data.data.ptr` pointer into the guest's address
/// space (which requires either `process_vm_readv` or a shared memory
/// mapping negotiated via the wire protocol). Instead we log the
/// request code and return:
///
/// * `SVC_MGR_GET_SERVICE` / `SVC_MGR_CHECK_SERVICE` → `Failed` (the
///   guest will see `BR_FAILED_REPLY` and retry, eventually giving up).
/// * `SVC_MGR_ADD_SERVICE` → `Reply(0)` (we accept the registration but
///   don't record anything; in a real impl we'd store the name + handle
///   in the [`HandleTable`]). Returning `Noop` would livelock the guest.
/// * `SVC_MGR_LIST_SERVICES` → `Failed` (no services to enumerate).
/// * anything else → `Failed`.
fn servicemanager_proxy(
    _cmd_payload: &[u8],
    code: u32,
    _handles: &Arc<Mutex<HandleTable>>,
) -> TransactionResult {
    match code {
        SVC_MGR_GET_SERVICE | SVC_MGR_CHECK_SERVICE => {
            warning!(
                "[KR64][binder][svc] SVC_MGR_GET_SERVICE/CHECK_SERVICE: skeleton cannot parse parcel"
            );
            TransactionResult::Failed
        }
        SVC_MGR_ADD_SERVICE => {
            // SVC_MGR_ADD_SERVICE is a synchronous transaction — the guest's
            // IServiceManager::addService calls waitForResponse, which loops
            // on BINDER_WRITE_READ until it sees BR_REPLY.
            //
            // Returning Noop causes a livelock: the guest receives BR_NOOP
            // and busy-spins forever. Instead, return a Reply with status 0
            // (success) so the guest proceeds.
            info!("[KR64][binder][svc] SVC_MGR_ADD_SERVICE accepted (skeleton: not recorded, returning success)");
            // The reply parcel for IServiceManager is a single i32 status (0 = OK)
            let status_reply: [u8; 4] = 0i32.to_ne_bytes();
            TransactionResult::Reply(status_reply.to_vec())
        }
        SVC_MGR_LIST_SERVICES => {
            info!("[KR64][binder][svc] SVC_MGR_LIST_SERVICES — returning empty (skeleton)");
            TransactionResult::Failed
        }
        _ => {
            warning!("[KR64][binder][svc] unhandled servicemanager code {}", code);
            TransactionResult::Failed
        }
    }
}

/// Forward a `BC_TRANSACTION` to the host's `/dev/binder` via a real
/// `BINDER_WRITE_READ` ioctl.
///
/// # What this does (skeleton)
///
/// 1. Lazily opens `/dev/binder` on the host (best-effort; if the open
///    fails — e.g. on a Linux dev host without the binder module —
///    returns an error and the caller emits `BR_FAILED_REPLY`).
/// 2. Builds a `binder_write_read` struct with a write_buffer containing
///    `[BC_TRANSACTION ioctl number][the guest's transaction_data bytes]`
///    and an empty read_buffer.
/// 3. Issues `ioctl(fd, BINDER_WRITE_READ, &bwr)`.
/// 4. Returns the read_buffer contents (which should start with
///    `BR_REPLY` + the reply transaction_data) as the reply bytes.
///
/// # What this does NOT do (TODO for `BINDER-3`)
///
/// * **Handle translation.** The guest's `target_handle` is a number
///   allocated by *us* (the proxy), not by the host's binder driver.
///   Before issuing the ioctl, we need to look up the host handle in
///   the [`HandleTable`] and patch `transaction_data.target.handle`.
/// * **Flat-binder-object patching.** If the transaction's offsets array
///   contains any `BINDER_TYPE_HANDLE` / `BINDER_TYPE_WEAK_HANDLE`
///   objects, those handles ALSO need to be translated. Conversely, any
///   `BINDER_TYPE_BINDER` / `BINDER_TYPE_WEAK_BINDER` local-binder
///   pointers need to be translated back in the reply.
/// * **Data buffer copy-in.** The guest's `data_ptr` is a user pointer
///   in the guest's address space — we need to copy the data into our
///   own buffer (or share memory via ashmem) before the kernel can read
///   it. The skeleton just passes the raw pointer through, which will
///   cause the kernel to return EFAULT.
/// * **Reply unparceling.** The reply read_buffer contains a
///   `BR_REPLY` + `binder_transaction_data` + reply data. We need to
///   extract the reply data and re-wrap it in our wire format for the
///   guest.
fn forward_transaction_to_host(
    tx_data: &[u8],
    host_fd: &Arc<Mutex<Option<RawFd>>>,
) -> io::Result<Vec<u8>> {
    // Lazily open /dev/binder on the host.
    let mut guard = host_fd.lock().map_err(|e| {
        io::Error::other(format!("host_binder_fd mutex poisoned: {}", e))
    })?;
    if guard.is_none() {
        match open_host_binder() {
            Ok(fd) => *guard = Some(fd),
            Err(e) => {
                return Err(io::Error::other(format!(
                    "could not open host /dev/binder: {}",
                    e
                )));
            }
        }
    }
    let fd = guard.unwrap();

    // Build the write_buffer: [BC_TRANSACTION][tx_data].
    let mut write_buf: Vec<u8> = Vec::with_capacity(4 + tx_data.len());
    write_buf.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
    write_buf.extend_from_slice(tx_data);

    // Allocate a read_buffer for the reply. 4 KiB matches what
    // libbinder.so uses by default.
    let mut read_buf = vec![0u8; 4096];

    // Construct the binder_write_read struct. The pointers here are
    // OUR process's pointers (the daemon's), NOT the guest's — this is
    // the host-side ioctl. The kernel writes back the `*_consumed`
    // fields, so bwr must be passed mutably.
    let mut bwr = BinderWriteRead {
        write_size: write_buf.len() as u64,
        write_consumed: 0,
        write_buffer: write_buf.as_ptr() as u64,
        read_size: read_buf.len() as u64,
        read_consumed: 0,
        read_buffer: read_buf.as_mut_ptr() as u64,
    };

    // Issue the ioctl. The `request` argument type differs between
    // libc flavours: bionic (Android) declares `ioctl(fd, int request, ...)`
    // while glibc declares `ioctl(fd, unsigned long request, ...)`. Casting
    // with `as _` lets the compiler pick the right width per target so the
    // same source compiles for aarch64-linux-android, x86_64-linux-android,
    // and the host (glibc) `cargo check`. The kernel reads/writes through
    // the raw third pointer.
    let rc = unsafe {
        libc::ioctl(
            fd,
            BINDER_WRITE_READ as _,
            &mut bwr as *mut BinderWriteRead as *mut c_void,
        )
    };
    if rc < 0 {
        let e = io::Error::last_os_error();
        return Err(e);
    }

    // The reply is in read_buf[0..bwr.read_consumed]. For the skeleton
    // we return the WHOLE read_buffer — a real impl would slice it to
    // `bwr.read_consumed as usize` and parse out the leading BR_REPLY
    // + binder_transaction_data + reply-data bytes.
    let _ = bwr.read_consumed; // TODO(BINDER-3): use this to slice read_buf.
    Ok(read_buf)
}

/// Open the host's `/dev/binder` (real char device). Returns the raw FD
/// on success. The FD is `O_RDWR | O_CLOEXEC`.
///
/// On Android this will succeed (the binder module is always loaded).
/// On a Linux dev host it'll typically fail with ENOENT (no /dev/binder)
/// or ENODEV (binder module not loaded) — that's fine for `cargo test`.
fn open_host_binder() -> io::Result<RawFd> {
    let path = std::ffi::CString::new("/dev/binder").unwrap();
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDWR | libc::O_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    info!("[KR64][binder] opened host /dev/binder (fd={})", fd);
    Ok(fd)
}

// ============================================================================
// Wire-framing I/O helpers.
// ============================================================================

/// Read one [`Frame`] from the stream. Blocks until a full frame is
/// available. Returns `UnexpectedEof` if the stream closes mid-frame.
fn read_frame(stream: &mut UnixStream) -> io::Result<Frame> {
    let mut hdr = [0u8; 8]; // [u32 cmd][u32 arg_len]
    stream.read_exact(&mut hdr)?;
    let cmd = u32::from_ne_bytes(hdr[0..4].try_into().unwrap());
    let arg_len = u32::from_ne_bytes(hdr[4..8].try_into().unwrap()) as usize;
    // Cap payload size to prevent DoS — a malicious guest could send
    // arg_len = u32::MAX (4 GiB) to OOM the daemon. 1 MiB is more than
    // enough for any legitimate binder transaction.
    const MAX_PAYLOAD: usize = 1 << 20; // 1 MiB
    if arg_len > MAX_PAYLOAD {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("read_frame: payload too large ({} > {})", arg_len, MAX_PAYLOAD),
        ));
    }
    let mut payload = vec![0u8; arg_len];
    stream.read_exact(&mut payload)?;
    Ok(Frame { cmd, payload })
}

/// Write one [`Resp`] to the stream.
fn write_frame(stream: &mut UnixStream, resp: &Resp) -> io::Result<()> {
    let mut buf = Vec::with_capacity(8 + resp.payload.len());
    buf.extend_from_slice(&resp.ret.to_ne_bytes());
    buf.extend_from_slice(&(resp.payload.len() as u32).to_ne_bytes());
    buf.extend_from_slice(&resp.payload);
    stream.write_all(&buf)
}

// ============================================================================
// BR_* push helpers (build the read_buffer).
// ============================================================================

/// Push `[BR_NOOP]` (4 bytes, no payload).
fn push_br_noop(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&BR_NOOP.to_ne_bytes());
}

/// Push `[BR_FAILED_REPLY]` (4 bytes, no payload).
fn push_br_failed_reply(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&BR_FAILED_REPLY.to_ne_bytes());
}

/// Push `[BR_REPLY][binder_transaction_data]` followed by the reply's
/// data buffer. The `binder_transaction_data` is synthesised with all
/// fields zeroed except `data_size` / `data_ptr` (which point at the
/// reply data buffer we're embedding).
///
/// NOTE: the guest's libbinder.so expects `data_ptr` to be a pointer
/// into ITS OWN address space, not ours. The skeleton here just passes
/// a placeholder pointer — the guest will get garbage if it tries to
/// dereference it. A real impl would either (a) allocate a buffer in
/// the guest's address space (via the wire protocol's "shared buffer"
/// extension) and put the reply data there, or (b) inline the reply
/// data into the `binder_transaction_data.data.buf[8]` inline array.
fn push_br_reply(buf: &mut Vec<u8>, reply_data: &[u8]) {
    buf.extend_from_slice(&BR_REPLY.to_ne_bytes());
    let tx = BinderTransactionData {
        data_size: reply_data.len() as u64,
        ..Default::default()
    };
    // Serialize the struct as little-endian bytes. Since the struct is
    // #[repr(C)] and we're on a little-endian platform (aarch64 /
    // x86_64), `to_ne_bytes` via raw byte copy works.
    let tx_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            &tx as *const BinderTransactionData as *const u8,
            std::mem::size_of::<BinderTransactionData>(),
        )
    };
    buf.extend_from_slice(tx_bytes);
    buf.extend_from_slice(reply_data);
}

// ============================================================================
// BC_* payload-size extraction.
// ============================================================================

/// Extract the payload size of a BC_* / BR_* command from its ioctl
/// number. The ioctl number encodes the arg size in bits 16..29
/// (the `size` field of `_IOC(dir, type, nr, size)`).
///
/// For example, `BC_TRANSACTION` = `_IOW('b', 0, sizeof(binder_transaction_data))`
/// = `(1<<30) | (64<<16) | ('b'<<8) | 0`, so `bc_payload_size(BC_TRANSACTION)`
/// returns 64.
fn bc_payload_size(cmd: u32) -> u32 {
    (cmd >> 16) & 0x3fff
}

// ============================================================================
// Tests — pure-Rust, no Android deps, so they run on the host too.
// (cargo test --lib)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::os::unix::net::UnixStream;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Each test gets a UNIQUE tmpdir so parallel tests don't collide
    /// on the same socket path (which would cause EADDRINUSE on bind).
    fn tmpdir() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut p = env::temp_dir();
        p.push(format!("kr64-binder-test-{}-{}", std::process::id(), n));
        fs::create_dir_all(&p).unwrap();
        p.to_string_lossy().to_string()
    }

    // -------- ioctl number correctness --------------------------------

    #[test]
    fn ioctl_macros_match_kernel_values() {
        // These are the canonical values from <uapi/linux/android/binder.h>
        // on aarch64 / x86_64. If any of these change, the guest's
        // libbinder.so (which uses the literal numbers) won't recognise
        // our ioctls.
        assert_eq!(BINDER_WRITE_READ, 0xC0306201, "BINDER_WRITE_READ");
        assert_eq!(BINDER_SET_MAX_THREADS, 0x40046205, "BINDER_SET_MAX_THREADS");
        assert_eq!(BINDER_SET_CONTEXT_MGR, 0x00006207, "BINDER_SET_CONTEXT_MGR");
        assert_eq!(BINDER_THREAD_EXIT, 0x40046208, "BINDER_THREAD_EXIT");
        assert_eq!(BINDER_VERSION, 0xC0046209, "BINDER_VERSION");
    }

    #[test]
    fn bc_br_constants_match_kernel_values() {
        // These are the canonical values from <uapi/linux/android/binder.h>
        // on aarch64 / x86_64. The kernel enum uses nr=1 for BC_TRANSACTION
        // (NOT nr=0), and nr=1 for BR_OK (NOT BR_TRANSACTION). We match the
        // kernel exactly so the guest's libbinder.so (which uses these
        // literal numbers) recognises our commands.
        assert_eq!(BC_TRANSACTION, 0x40406201, "BC_TRANSACTION");
        assert_eq!(BC_REPLY, 0x40406202, "BC_REPLY");
        assert_eq!(BC_ACQUIRE, 0x40046203, "BC_ACQUIRE");
        assert_eq!(BC_RELEASE, 0x40046204, "BC_RELEASE");
        assert_eq!(BC_INCREFS, 0x40046205, "BC_INCREFS");
        assert_eq!(BC_ACQUIRE_DONE, 0x40106206, "BC_ACQUIRE_DONE");
        assert_eq!(BC_INCREFS_DONE, 0x40106208, "BC_INCREFS_DONE");
        assert_eq!(BC_FREE_BUFFER, 0x4008620a, "BC_FREE_BUFFER");
        // BC_TRANSACTION_SG / BC_REPLY_SG use struct binder_transaction_data_sg
        // (64-byte binder_transaction_data + 8-byte buffers_size = 72 bytes),
        // so the _IOW size field is 0x48, not 0x40. The struct size MUST
        // match the kernel's or the guest's libbinder.so (which uses the
        // kernel literal) silently drops every scatter-gather transaction.
        assert_eq!(BC_TRANSACTION_SG, 0x4048620b, "BC_TRANSACTION_SG");
        assert_eq!(BC_REPLY_SG, 0x4048620c, "BC_REPLY_SG");
        assert_eq!(BC_ENTER_LOOPER, 0x0000620d, "BC_ENTER_LOOPER");
        assert_eq!(BC_REGISTER_LOOPER, 0x0000620e, "BC_REGISTER_LOOPER");
        assert_eq!(BC_EXIT_LOOPER, 0x0000620f, "BC_EXIT_LOOPER");
        assert_eq!(BC_REQUEST_DEATH_NOTIFICATION, 0x40106210);
        assert_eq!(BC_CLEAR_DEATH_NOTIFICATION, 0x40106211);
        assert_eq!(BC_DEAD_BINDER_DONE, 0x40086212);

        assert_eq!(BR_ERROR, 0x80046200, "BR_ERROR");
        assert_eq!(BR_OK, 0x00006201, "BR_OK");
        assert_eq!(BR_TRANSACTION, 0x80406202, "BR_TRANSACTION");
        assert_eq!(BR_REPLY, 0x80406203, "BR_REPLY");
        assert_eq!(BR_NOOP, 0x0000620c, "BR_NOOP");
        assert_eq!(BR_SPAWN_LOOPER, 0x0000620d, "BR_SPAWN_LOOPER");
        assert_eq!(BR_FINISHED, 0x0000620e, "BR_FINISHED");
        assert_eq!(BR_FAILED_REPLY, 0x00006211, "BR_FAILED_REPLY");
        assert_eq!(BR_DEAD_BINDER, 0x8008620f, "BR_DEAD_BINDER");
        assert_eq!(BR_CLEAR_DEATH_NOTIFICATION_DONE, 0x80086210);
    }

    #[test]
    fn bc_payload_size_extracts_size_from_ioctl_number() {
        assert_eq!(bc_payload_size(BC_TRANSACTION), 64);
        assert_eq!(bc_payload_size(BC_ACQUIRE), 4);
        assert_eq!(bc_payload_size(BC_ENTER_LOOPER), 0);
        assert_eq!(bc_payload_size(BC_FREE_BUFFER), 8);
    }

    // -------- struct sizes --------------------------------------------

    #[test]
    fn binder_write_read_size_is_48_bytes() {
        // Must match the kernel struct on aarch64 / x86_64.
        assert_eq!(std::mem::size_of::<BinderWriteRead>(), 48);
    }

    #[test]
    fn binder_transaction_data_size_is_64_bytes() {
        assert_eq!(std::mem::size_of::<BinderTransactionData>(), 64);
    }

    #[test]
    fn flat_binder_object_size_is_24_bytes() {
        assert_eq!(std::mem::size_of::<FlatBinderObject>(), 24);
    }

    // -------- HandleTable ---------------------------------------------

    #[test]
    fn handle_table_allocate_and_lookup() {
        let mut t = HandleTable::new();
        let g1 = t.allocate(100);
        let g2 = t.allocate(200);
        assert_ne!(g1, g2);
        assert_eq!(t.lookup_host(g1), Some(100));
        assert_eq!(t.lookup_host(g2), Some(200));
        assert_eq!(t.lookup_host(999), None);
    }

    #[test]
    fn handle_table_register_and_lookup_by_name() {
        let mut t = HandleTable::new();
        let g = t.allocate(42);
        t.register("activity", g);
        assert_eq!(t.lookup_by_name("activity"), Some(g));
        assert_eq!(t.lookup_by_name("package"), None);
    }

    // -------- create_binder_device ------------------------------------

    #[test]
    fn create_binder_device_creates_socket_and_symlink() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 7).expect("create_binder_device");

        // The actual socket file was unlinked by create_binder_device
        // (it drops the listener and returns the path for the caller to
        // re-bind), so we just check the path is what we expect.
        assert!(path.ends_with("vm7/dev/binder"));
        assert!(path.starts_with(&rootfs));

        // The symlink at {rootfs}/dev/binder should still exist.
        let link = format!("{}/dev/binder", rootfs);
        let meta = fs::symlink_metadata(&link).expect("symlink metadata");
        assert!(
            meta.file_type().is_symlink(),
            "{} should be a symlink",
            link
        );
        // And it should point to ../vm7/dev/binder.
        let target = fs::read_link(&link).expect("read_link");
        assert_eq!(target.to_string_lossy(), "../vm7/dev/binder");

        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- BinderProxy end-to-end (BINDER_VERSION) -----------------

    #[test]
    fn binder_proxy_responds_to_version_ioctl() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let handle = proxy.spawn().expect("BinderProxy::spawn");

        // Give the accept thread a moment to start.
        std::thread::sleep(Duration::from_millis(50));

        // Connect as a client and send a BINDER_VERSION request.
        let mut stream = UnixStream::connect(&path).expect("connect");
        let mut req = Vec::new();
        req.extend_from_slice(&BINDER_VERSION.to_ne_bytes());
        req.extend_from_slice(&0u32.to_ne_bytes()); // arg_len = 0
        stream.write_all(&req).expect("write request");

        // Read the response: [i32 ret][u32 arg_len][arg_len bytes].
        let mut hdr = [0u8; 8];
        stream.read_exact(&mut hdr).expect("read response header");
        let ret = i32::from_ne_bytes(hdr[0..4].try_into().unwrap());
        let arg_len = u32::from_ne_bytes(hdr[4..8].try_into().unwrap()) as usize;
        assert_eq!(ret, 0, "BINDER_VERSION should succeed");
        assert_eq!(arg_len, 4, "BINDER_VERSION returns a u32");

        let mut payload = vec![0u8; arg_len];
        stream.read_exact(&mut payload).expect("read response payload");
        let version = u32::from_ne_bytes(payload[0..4].try_into().unwrap());
        assert_eq!(
            version, BINDER_CURRENT_PROTOCOL_VERSION,
            "BINDER_VERSION should return the current protocol version"
        );

        drop(stream);
        drop(handle); // triggers shutdown + unlink
        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- BinderProxy end-to-end (BINDER_WRITE_READ with NOOP) ----

    #[test]
    fn binder_proxy_write_read_returns_noop_when_idle() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));

        let mut stream = UnixStream::connect(&path).expect("connect");

        // Build a BINDER_WRITE_READ payload: write_size=0, read_capacity=64.
        let mut payload = Vec::new();
        payload.extend_from_slice(&0u32.to_ne_bytes()); // write_size
        payload.extend_from_slice(&64u32.to_ne_bytes()); // read_capacity
        // (no write_buffer bytes)

        let mut req = Vec::new();
        req.extend_from_slice(&BINDER_WRITE_READ.to_ne_bytes());
        req.extend_from_slice(&(payload.len() as u32).to_ne_bytes());
        req.extend_from_slice(&payload);
        stream.write_all(&req).expect("write request");

        // Read response: [i32 ret][u32 arg_len][u32 read_size][read_size bytes].
        let mut hdr = [0u8; 8];
        stream.read_exact(&mut hdr).expect("read response header");
        let ret = i32::from_ne_bytes(hdr[0..4].try_into().unwrap());
        let arg_len = u32::from_ne_bytes(hdr[4..8].try_into().unwrap()) as usize;
        assert_eq!(ret, 0);
        assert!(arg_len >= 4, "BINDER_WRITE_READ response should have a read_size header");

        let mut resp = vec![0u8; arg_len];
        stream.read_exact(&mut resp).expect("read response payload");
        let read_size = u32::from_ne_bytes(resp[0..4].try_into().unwrap()) as usize;
        assert_eq!(read_size, 4, "idle BINDER_WRITE_READ should return exactly one BR_NOOP");

        let br_cmd = u32::from_ne_bytes(resp[4..8].try_into().unwrap());
        assert_eq!(br_cmd, BR_NOOP, "expected BR_NOOP");

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- ThreadPool ----------------------------------------------

    #[test]
    fn thread_pool_executes_jobs() {
        let pool = ThreadPool::new(2);
        let counter = Arc::new(AtomicU64::new(0));
        for _ in 0..10 {
            let c = Arc::clone(&counter);
            pool.execute(move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        // Drop the pool — this sends Terminate to all workers and
        // joins them, so by the time drop returns all 10 jobs have run.
        drop(pool);
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }
}
