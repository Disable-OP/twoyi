// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://www.mozilla.org/MPL/2.0/.

//! Binder virtualisation — per-VM `/vm%d/dev/binder` Unix socket plus a
//! userspace proxy that acts as the guest's binder driver **and** its
//! servicemanager (6-Z114 / strategy S1b of the 6-Z112 design).
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
//! # What this module implements (6-Z114 / S1b)
//!
//! * **Kernel-truth protocol constants** — every `BINDER_*` ioctl uses
//!   ioctl type `'b'`, every `BC_*` command uses type `'c'`, every `BR_*`
//!   return uses type `'r'` (verified against
//!   `/usr/include/linux/android/binder.h` AND bionic's
//!   `android-11.0.0_r1` mirror — zero differences across all 38 shared
//!   definitions; see the `bc_br_constants_match_kernel_values` test for
//!   the full locked table and 6-Z114 §CHANGES for the audit trail — the
//!   pre-Z114 table used `'b'` for everything and was wrong in every
//!   BC_/BR_ entry).
//! * **A proxy-side servicemanager** — `BC_TRANSACTION` to handle 0 is
//!   answered by the proxy itself, speaking the exact wire protocol of
//!   AOSP-11's `frameworks/native/cmds/servicemanager` (the AIDL
//!   `android.os.IServiceManager`: descriptor token
//!   `[i32 strict][i32 worksource][i32 'SYST'][string16]`, transaction
//!   codes getService=1/checkService=2/addService=3/listServices=4,
//!   replies `[i32 exception=0] + payload` — all verified from the
//!   fetched android-11 sources; see `servicemanager_transaction`).
//! * **A real name→handle registry with OWNER routing (6-Z271)** —
//!   addService stores the owning connection + its local ptr/cookie;
//!   getService answers with a `flat_binder_object{BINDER_TYPE_HANDLE}`
//!   carrying a kernel-true dense handle (`6-Z271v`: `PROXY_HANDLE_BASE 0`
//!   — real libbinder's `lookupHandleLocked` inserts `handle+1` entries
//!   into its handle Vector, so sparse `0xF0000000-range` handles aborted
//!   every client with the libutils "new_capacity overflow" fatal); a miss
//!   answers with a null binder, exactly like the native servicemanager's
//!   reply shape. Every reply that carries a binder object also carries
//!   the android-12+ binder STABILITY ANNOTATION (`6-Z271x`):
//!   `Parcel::finishFlattenBinder` writes `[flat][i32 Stability::Level]`
//!   and `finishUnflattenBinder` reads the i32 back — an annotation-less
//!   parcel makes the client's readInt32 run past the parcel end → null
//!   binder → the honest-but-fatal NAME_NOT_FOUND chain of 6-Z271w.
//! * **`BR_TRANSACTION` delivery + `BC_REPLY` correlation (6-Z271 bus)** —
//!   transactions aimed at a registered handle are queued on the owning
//!   guest connection and delivered (with the owner's ptr/cookie and the
//!   sender's announced pid/euid stamped in) on its next
//!   `BINDER_WRITE_READ` with read capacity; the server's `BC_REPLY` is
//!   routed back to the requester as `BR_REPLY` (8 s deadline →
//!   `BR_FAILED_REPLY`). One-way transactions get only
//!   `BR_TRANSACTION_COMPLETE`. Connection death unregisters its services
//!   (→ `BR_DEAD_BINDER` to watchers) and resolves queued work as dead.
//! * **In-proxy virtual services (6-Z271)** — semantically-correct minimal
//!   AIDL implementations registered at proxy start:
//!   `android.hardware.vibrator.IVibrator/default` (kills the ~5 s per-tap
//!   haptics wait; `on(ms)` is forwarded to the host app for a REAL
//!   vibration), `android.hardware.security.keymint.IKeyMintDevice/default`
//!   (lets keystore2 obtain its backend and register IKeystoreSecurity —
//!   kills the ~20 s recovery wait; key ops return honest
//!   `HARDWARE_TYPE_UNAVAILABLE` errors, no fake crypto), and
//!   `android.hardware.security.sharedsecret.ISharedSecret/default`.
//! * **HIDL-aware servicemanager (6-Z271)** — libhwbinder parcels (no SYST
//!   header tag) are parsed as `android.hidl.manager.V1_0.IServiceManager`
//!   get/add; `IBase::PING` is answered for every handle.
//! * **v2 request blobs from the loader (6-Z271)** — the shlib inlines the
//!   parcel bytes behind every BC_TRANSACTION/BC_REPLY so REAL libbinder
//!   clients (keystore2, recovery) hit the parsed registry instead of the
//!   name-less legacy path (the root cause of the inert registry).
//! * **Blocking idle** — a pure-read `BINDER_WRITE_READ` blocks on the
//!   connection's queue (250 ms tick, then `BR_NOOP`) instead of
//!   busy-answering, mirroring the kernel's blocking read.
//!
//! # What is still NOT here (the honest list)
//!
//! * **No fd passing** (BINDER_TYPE_FD objects are not translated); no
//!   refcount forwarding to a host driver; sender identity comes from the
//!   loader's WIRE_CMD_IDENT announcement (one gid is ignored).
//! * **Guest servicemanager/hwservicemanager are still zombies** — the
//!   proxy acks their BINDER_SET_CONTEXT_MGR and remains the context
//!   manager; with the 6-Z271 bus this is now harmless (their clients'
//!   lookups are served by the proxy registry directly).
//!
//! # Registration callbacks (6-Z276 — WAS the top of this list)
//!
//! `REGISTER_FOR_NOTIFICATIONS` (AIDL SM code 5) and HIDL SM code 4 store
//! the watcher (connection + LOCAL callback ptr/cookie + dialect). When
//! the watched service later registers — guest `addService`, HIDL `add`
//! — every watcher gets a ONE-WAY `onRegistration` `BR_TRANSACTION`
//! queued on its mailbox, targeted at its own callback object:
//! * AIDL: `android.os.IServiceCallback.onRegistration(name, binder)` —
//!   `[strict][worksource]['SYST'][string16 descriptor][string16 name]
//!   [flat HANDLE][i32 stability]`.
//! * HIDL: `IServiceNotification.onRegistration(fqName, instance,
//!   preexisting)` — `[hidl_string][hidl_string][i32]`.
//! An already-registered service fires the callback IMMEDIATELY
//! (`preexisting=true`) — the real servicemanagers' behaviour that
//! `waitForService`-style clients depend on. Dying connections drop their
//! watchers.
//!
//! # Wire framing
//!
//! The guest's `libbinder.so` cannot call `ioctl()` on a Unix socket —
//! `ioctl` on a `SOCK_STREAM` returns ENOTTY for binder ioctls. The
//! 6-Z113 loader hooks `ioctl` in-process and speaks the frame protocol
//! below over the socket (`twoyi_loader_shlib.c`, binder-proxy block).
//!
//! ## Frames
//!
//! ```text
//! guest → host : [u32 cmd][u32 arg_len][arg_len bytes]
//! host → guest : [i32 ret][u32 arg_len][arg_len bytes]
//! ```
//!
//! `cmd` is the binder ioctl number (the 6-Z113 loader normalises
//! `BINDER_SET_CONTEXT_MGR` from the kernel's `_IOW('b',7,__s32)`
//! spelling to the legacy `_IO('b',7)` one; the dispatcher here accepts
//! both). `arg_len` is capped at 1 MiB.
//!
//! ## BINDER_WRITE_READ payloads — v1 (6-Z113) and v2 (parcels)
//!
//! The kernel `binder_write_read` struct carries user pointers that are
//! meaningless across a socket, so the wire form is our own:
//!
//! ```text
//! v1 request : [u32 write_size][u32 read_capacity][write_size BC_* bytes]
//! v1 response: [u32 read_size][read_size BR_* bytes]
//!
//! v2 request : [u32 write_size][u32 read_capacity][write_size BC_* bytes]
//!              [u32 WIRE_V2_MAGIC][u32 blob_count]
//!              (blob_count ×) [u32 data_len][u32 offsets_len][data][offsets]
//! v2 response: [u32 read_size][read_size BR_* bytes]
//!              [u32 WIRE_V2_MAGIC][u32 blob_count]
//!              (blob_count ×) [u32 data_len][u32 offsets_len][data][offsets]
//! ```
//!
//! * A payload that ends exactly after the BC stream is v1 (6-Z113
//!   clients — byte-compatible, and the extra v2 response tail is only
//!   ever appended when the request was v2).
//! * The i-th blob belongs to the i-th `BC_TRANSACTION`/`BC_REPLY`/`*_SG`
//!   command in the BC stream, in order — the client walks the same
//!   stream to collect `data.ptr`/`offsets.ptr`, so both sides agree
//!   without offsets bookkeeping on the wire.
//! * In the response, blobs pair in order with the `BR_REPLY`/
//!   `BR_TRANSACTION` records in the BR stream. The v2 client stashes
//!   each blob in guest-addressable memory, patches the corresponding
//!   `binder_transaction_data.data_ptr`/`offsets_ptr` (0 on the wire)
//!   inside the BR bytes **before** copying them into the guest's
//!   read_buffer, and later answers `BC_FREE_BUFFER` for those blocks.
//! * v1 requests are answered without a trailer, so the 6-Z113 client
//!   (which ignores everything past `[u32 read_size][read_size bytes]`)
//!   and its mock stay byte-compatible.
//!
//! # Module layout
//!
//! * [`create_binder_device`] — creates `{rootfs}/vm{id}/dev/binder` as
//!   a Unix socket plus a `{rootfs}/dev/binder` symlink to it.
//! * [`BinderProxy`] / [`BinderProxyHandle`] — owns the listener and
//!   spawns one thread per guest connection (bounded by
//!   [`MAX_PROXY_CONNECTIONS`]).
//! * `ProxyShared` / `ConnState` — names for the shapes that hold the
//!   servicemanager state (name registry + per-connection context).
//!   NOTE: today the live state is the simpler `ServiceRegistry` +
//!   `HandleTable`; the richer per-connection delivery-queue design
//!   described by those names is NOT implemented — see "What is still
//!   NOT here" above for the honest list.
//! * `ParcelReader` / `ParcelWriter` — the libbinder Parcel codec
//!   (interface token, string16, flat_binder_object).
//! * `dispatch_request` / `handle_*` — per-ioctls handlers.
//! * `servicemanager_transaction` / `route_transaction` /
//!   `forward_transaction_to_host` — the three transaction dispatch
//!   paths (handle 0 → proxy servicemanager, fake handle → guest owner
//!   connection, anything else → the host's real `/dev/binder`).
//! * [`ThreadPool`] — kept from the skeleton era, exercised only by its
//!   own unit test (the live proxy above spawns one bounded thread per
//!   connection).
//! * [`HandleTable`] — guest↔host handle translation table for the
//!   future host-forwarding path (BINDER-3).
//! * Protocol constants (`BINDER_*`, `BC_*`, `BR_*`, `SVC_MGR_*`) —
//!   exact matches of the kernel `<uapi/linux/android/binder.h>` and
//!   AOSP-11 `frameworks/native` (IServiceManager / servicemanager).

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

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
    pub write_size: u64,     // binder_size_t
    pub write_consumed: u64, // binder_size_t
    pub write_buffer: u64,   // binder_uintptr_t (user pointer)
    pub read_size: u64,      // binder_size_t
    pub read_consumed: u64,  // binder_size_t
    pub read_buffer: u64,    // binder_uintptr_t (user pointer)
}

/// `struct binder_ptr_cookie` — payload of `BC_ACQUIRE_DONE`,
/// `BC_INCREFS_DONE`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct BinderPtrCookie {
    pub ptr: u64,
    pub cookie: u64,
}

/// `struct binder_handle_cookie` — payload of
/// `BC_REQUEST_DEATH_NOTIFICATION`, `BC_CLEAR_DEATH_NOTIFICATION`.
///
/// The kernel declares it `__attribute__((packed))` — `__u32 handle`
/// immediately followed by `binder_uintptr_t cookie` with NO padding,
/// 12 bytes on 64-bit ABIs (which is why those BC_* ioctls carry a size
/// field of 12, not 16).
#[repr(C, packed)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct BinderHandleCookie {
    pub handle: u32,
    pub cookie: u64,
}

/// `struct binder_pri_desc` — payload of `BC_ATTEMPT_ACQUIRE`.
/// 8 bytes: two 32-bit fields, no padding.
#[repr(C)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct BinderPriDesc {
    pub priority: i32,
    pub desc: u32,
}

/// `struct binder_pri_ptr_cookie` — payload of `BR_ATTEMPT_ACQUIRE`.
/// 24 bytes: `__s32 priority` + natural padding + two pointers.
#[repr(C)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct BinderPriPtrCookie {
    pub priority: i32,
    pub pad: u32,
    pub ptr: u64,
    pub cookie: u64,
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
    pub target_pad: u32,    // [ 4.. 8] — padding (or low 4 bytes of ptr)
    pub target_cookie: u64, // [ 8..16] — cookie (or high 4 bytes of ptr)
    pub code: u32,          // [16..20] — transaction code (method id)
    pub flags: u32,         // [20..24] — TF_* flags
    pub sender_pid: i32,    // [24..28] — pid_t (signed)
    pub sender_euid: u32,   // [28..32] — uid_t (unsigned)
    pub data_size: u64,     // [32..40] — size of data buffer
    pub offsets_size: u64,  // [40..48] — size of offsets array
    pub data_ptr: u64,      // [48..56] — user pointer to data buffer
    pub offsets_ptr: u64,   // [56..64] — user pointer to offsets array
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
    pub flags: u32,
    pub binder: u64, // union: handle (u32) or ptr (binder_uintptr_t)
    pub cookie: u64,
}

/// `struct binder_transaction_data_sg` — payload of
/// `BC_TRANSACTION_SG` / `BC_REPLY_SG`: the 64-byte
/// `binder_transaction_data` + `binder_size_t buffers_size` = 72 bytes.
#[repr(C)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct BinderTransactionDataSg {
    pub transaction_data: BinderTransactionData,
    pub buffers_size: u64,
}

/// `struct binder_transaction_data_secctx` — payload of
/// `BR_TRANSACTION_SEC_CTX` (same ioctl nr as `BR_TRANSACTION`; the
/// 72-byte size field distinguishes them). We never send it — AOSP-11
/// libbinder accepts the plain form too (`IPCThreadState::waitForResponse`
/// handles both) — defined for completeness.
#[repr(C)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct BinderTransactionDataSecctx {
    pub transaction_data: BinderTransactionData,
    pub secctx: u64,
}

/// `struct binder_node_debug_info` — argument of
/// `BINDER_GET_NODE_DEBUG_INFO`. 24 bytes.
#[repr(C)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct BinderNodeDebugInfo {
    pub debug_id: u32,
    pub pad: u32,
    pub ptr: u64,
    pub cookie: u64,
}

// ============================================================================
// Binder ioctl numbers (drivers/android/binder.h — `enum` with _IOWR('b', …)).
//
// These MUST match the kernel exactly — the guest's libbinder.so uses
// these literal numbers in `ioctl()` calls, and any translation layer
// has to recognise them. Only the TOP-LEVEL ioctls use type 'b'; the
// BC_* stream commands use type 'c' and the BR_* returns use type 'r'
// (see below) — that split is the kernel's own, and it is ABI-frozen:
// verified 2026-08-24 against BOTH /usr/include/linux/android/binder.h
// (this build host's kernel UAPI) AND bionic's android-11.0.0_r1 mirror
// of the same header (the one the ROM's userspace was actually built
// against) — zero differences across all 38 shared definitions.
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

/// `BINDER_ENABLE_ONEWAY_SPAM_DETECTION` — Android 11+ libbinder arms
/// oneway spam detection right after the handshake (run 33334415274:
/// OrangeFox R12 lavender's recovery + keystore2 both issued ioctl
/// 0x40046210; the proxy's "unknown ioctl → -EINVAL" answer turned each
/// process start into an error path — 6-Z265).
pub const BINDER_ENABLE_ONEWAY_SPAM_DETECTION: u32 = _IOW(BINDER_IOC_TYPE, 16, 4);

/// `BINDER_SET_IDLE_PRIORITY` — historical, no longer used.
#[allow(dead_code)]
pub const BINDER_SET_IDLE_PRIORITY: u32 = _IOW(BINDER_IOC_TYPE, 6, 4);

/// `BINDER_SET_CONTEXT_MGR` — become the servicemanager for this binder
/// context (kernel: `_IOW('b', 7, __s32)` = 0x40046207 in BOTH the
/// modern kernel header and bionic-11).
///
/// **Wire note (the 6-Z113 pairing):** this constant keeps the LEGACY
/// `_IO('b', 7)` = 0x6207 spelling because that is what the 6-Z113
/// loader translates the kernel spelling DOWN to before putting it on
/// the wire (`BP_IOC_SET_CTX_MGR_WIRE` in twoyi_loader_shlib.c). The
/// dispatcher accepts BOTH — see [`BINDER_SET_CONTEXT_MGR_KERNEL`] — so
/// either client spelling works and neither side of the loader/proxy
/// pair needs a forced change (resolves the 6-Z113 "do not fix one side
/// alone" warning).
pub const BINDER_SET_CONTEXT_MGR: u32 = _IO(BINDER_IOC_TYPE, 7);

/// The kernel/bionic spelling of `BINDER_SET_CONTEXT_MGR`
/// (`_IOW('b', 7, __s32)`). Accepted by [`dispatch_request`] alongside
/// the legacy wire spelling above.
pub const BINDER_SET_CONTEXT_MGR_KERNEL: u32 = _IOW(BINDER_IOC_TYPE, 7, 4);

/// `BINDER_THREAD_EXIT` — tell the kernel a binder thread is exiting
/// (so it can clean up its per-thread state).
pub const BINDER_THREAD_EXIT: u32 = _IOW(BINDER_IOC_TYPE, 8, 4);

/// `BINDER_VERSION` — query the binder protocol version. Returns a
/// `struct binder_version { __s32 protocol_version; }`.
pub const BINDER_VERSION: u32 = _IOWR(BINDER_IOC_TYPE, 9, 4);

/// `BINDER_GET_NODE_DEBUG_INFO` — for debuggerd / dumpstate.
#[allow(dead_code)]
pub const BINDER_GET_NODE_DEBUG_INFO: u32 = _IOWR(
    BINDER_IOC_TYPE,
    11,
    std::mem::size_of::<BinderNodeDebugInfo>() as u32,
);

/// `BINDER_SET_CONTEXT_MGR_EXT` — modern form of SET_CONTEXT_MGR, takes
/// a `flat_binder_object` to specify the priority and policy of the
/// manager.
#[allow(dead_code)]
pub const BINDER_SET_CONTEXT_MGR_EXT: u32 = _IOW(
    BINDER_IOC_TYPE,
    13,
    std::mem::size_of::<FlatBinderObject>() as u32,
);

// ============================================================================
// BC_* — binder commands (written by userspace into the write_buffer of
// BINDER_WRITE_READ). Kernel: `enum binder_driver_command_protocol`.
//
// Each BC_* is encoded in the write_buffer as a [u32 cmd][cmd-specific
// payload] pair. The cmd u32 IS an ioctl-style number (so the payload
// size can be extracted from bits 16..29 via [`bc_payload_size`]).
//
// KERNEL TRUTH (resolved 6-Z114 after two contradictory wrong guesses —
// 6-Z113 flagged BC_ENTER_LOOPER as `_IO('b',16)`, the 6-Z114 task
// brief guessed `_IO('b',13)`; BOTH were wrong): the whole BC_* family
// uses ioctl type char **'c'** (0x63), NOT 'b', and the enum starts at
// nr=0 with BC_TRANSACTION. Verified against
// /usr/include/linux/android/binder.h and bionic-11's byte-identical
// mirror:
//
//   BC_TRANSACTION        = _IOW('c',  0, binder_transaction_data)     0x40406300
//   BC_REPLY              = _IOW('c',  1, binder_transaction_data)     0x40406301
//   BC_ACQUIRE_RESULT     = _IOW('c',  2, __s32)                      0x40046302
//   BC_FREE_BUFFER        = _IOW('c',  3, binder_uintptr_t)           0x40086303
//   BC_INCREFS            = _IOW('c',  4, __u32)                      0x40046304
//   BC_ACQUIRE            = _IOW('c',  5, __u32)                      0x40046305
//   BC_RELEASE            = _IOW('c',  6, __u32)                      0x40046306
//   BC_DECREFS            = _IOW('c',  7, __u32)                      0x40046307
//   BC_INCREFS_DONE       = _IOW('c',  8, binder_ptr_cookie)          0x40106308
//   BC_ACQUIRE_DONE       = _IOW('c',  9, binder_ptr_cookie)          0x40106309
//   BC_ATTEMPT_ACQUIRE    = _IOW('c', 10, binder_pri_desc)            0x4008630a
//   BC_REGISTER_LOOPER    = _IO ('c', 11)                             0x0000630b
//   BC_ENTER_LOOPER       = _IO ('c', 12)                             0x0000630c
//   BC_EXIT_LOOPER        = _IO ('c', 13)                             0x0000630d
//   BC_REQUEST_DEATH_NOTIFICATION  = _IOW('c', 14, binder_handle_cookie[packed, 12])
//   BC_CLEAR_DEATH_NOTIFICATION    = _IOW('c', 15, binder_handle_cookie)
//   BC_DEAD_BINDER_DONE   = _IOW('c', 16, binder_uintptr_t)           0x40086310
//   BC_TRANSACTION_SG     = _IOW('c', 17, binder_transaction_data_sg) 0x40486311
//   BC_REPLY_SG           = _IOW('c', 18, binder_transaction_data_sg) 0x40486312
// ============================================================================

/// Binder command ioctl type char — `'c'` = 0x63 (kernel
/// `enum binder_driver_command_protocol`).
pub const BC_IOC_TYPE: u32 = b'c' as u32;

/// `BC_TRANSACTION` — send a synchronous (or one-way, if TF_ONE_WAY)
/// transaction to a remote binder. Kernel nr is **0** (`_IOW('c', 0, …)`).
pub const BC_TRANSACTION: u32 = _IOW(
    BC_IOC_TYPE,
    0,
    std::mem::size_of::<BinderTransactionData>() as u32,
);

/// `BC_REPLY` — send the reply to a received `BR_TRANSACTION`.
pub const BC_REPLY: u32 = _IOW(
    BC_IOC_TYPE,
    1,
    std::mem::size_of::<BinderTransactionData>() as u32,
);

/// `BC_ACQUIRE_RESULT` — legacy, not supported by the kernel.
#[allow(dead_code)]
pub const BC_ACQUIRE_RESULT: u32 = _IOW(BC_IOC_TYPE, 2, 4);

/// `BC_FREE_BUFFER` — return a transaction-data buffer received via
/// `BR_TRANSACTION` / `BR_REPLY` to the driver for reuse.
pub const BC_FREE_BUFFER: u32 = _IOW(BC_IOC_TYPE, 3, 8);

/// `BC_INCREFS` — acquire a weak reference on a remote handle.
pub const BC_INCREFS: u32 = _IOW(BC_IOC_TYPE, 4, 4);

/// `BC_ACQUIRE` — acquire a strong reference on a remote handle.
pub const BC_ACQUIRE: u32 = _IOW(BC_IOC_TYPE, 5, 4);

/// `BC_RELEASE` — release a strong reference on a remote handle.
pub const BC_RELEASE: u32 = _IOW(BC_IOC_TYPE, 6, 4);

/// `BC_DECREFS` — release a weak reference on a remote handle.
pub const BC_DECREFS: u32 = _IOW(BC_IOC_TYPE, 7, 4);

/// `BC_INCREFS_DONE` — acknowledge completion of a `BR_INCREFS`.
pub const BC_INCREFS_DONE: u32 = _IOW(
    BC_IOC_TYPE,
    8,
    std::mem::size_of::<BinderPtrCookie>() as u32,
);

/// `BC_ACQUIRE_DONE` — acknowledge completion of a `BR_ACQUIRE`.
pub const BC_ACQUIRE_DONE: u32 = _IOW(
    BC_IOC_TYPE,
    9,
    std::mem::size_of::<BinderPtrCookie>() as u32,
);

/// `BC_ATTEMPT_ACQUIRE` — try-acquire; rejected by the kernel (-EINVAL).
#[allow(dead_code)]
pub const BC_ATTEMPT_ACQUIRE: u32 =
    _IOW(BC_IOC_TYPE, 10, std::mem::size_of::<BinderPriDesc>() as u32);

/// `BC_REGISTER_LOOPER` — declare this thread was spawned by the
/// application in response to `BR_SPAWN_LOOPER`.
pub const BC_REGISTER_LOOPER: u32 = _IO(BC_IOC_TYPE, 11);

/// `BC_ENTER_LOOPER` — declare this thread a binder looper (it'll call
/// `BINDER_WRITE_READ` with `read_size > 0` to block waiting for work).
/// Kernel truth: `_IO('c', 12)` = 0x630c (NOT `_IO('b',13)` as the
/// pre-Z114 table had it, and NOT `_IO('b',16)` as 6-Z113 guessed).
pub const BC_ENTER_LOOPER: u32 = _IO(BC_IOC_TYPE, 12);

/// `BC_EXIT_LOOPER` — declare this thread is exiting the looper.
/// Kernel truth: `_IO('c', 13)` = 0x630d.
pub const BC_EXIT_LOOPER: u32 = _IO(BC_IOC_TYPE, 13);

/// `BC_REQUEST_DEATH_NOTIFICATION` — ask the driver to send
/// `BR_DEAD_BINDER` when the referenced handle dies. Payload is the
/// kernel's PACKED 12-byte `binder_handle_cookie`.
pub const BC_REQUEST_DEATH_NOTIFICATION: u32 = _IOW(
    BC_IOC_TYPE,
    14,
    std::mem::size_of::<BinderHandleCookie>() as u32,
);

/// `BC_CLEAR_DEATH_NOTIFICATION` — cancel a previous
/// `BC_REQUEST_DEATH_NOTIFICATION`.
pub const BC_CLEAR_DEATH_NOTIFICATION: u32 = _IOW(
    BC_IOC_TYPE,
    15,
    std::mem::size_of::<BinderHandleCookie>() as u32,
);

/// `BC_DEAD_BINDER_DONE` — acknowledge receipt of a `BR_DEAD_BINDER`.
pub const BC_DEAD_BINDER_DONE: u32 = _IOW(BC_IOC_TYPE, 16, 8);

/// `BC_TRANSACTION_SG` — scatter-gather variant of `BC_TRANSACTION`.
/// Payload is `binder_transaction_data_sg` = `binder_transaction_data`
/// (64 bytes) + `binder_size_t buffers_size` (8 bytes) = 72 bytes total
/// (`_IOW('c', 17, …)`).
pub const BC_TRANSACTION_SG: u32 = _IOW(
    BC_IOC_TYPE,
    17,
    (std::mem::size_of::<BinderTransactionData>() + 8) as u32,
);

/// `BC_REPLY_SG` — scatter-gather variant of `BC_REPLY`.
/// Same 72-byte payload as `BC_TRANSACTION_SG` (`_IOW('c', 18, …)`).
pub const BC_REPLY_SG: u32 = _IOW(
    BC_IOC_TYPE,
    18,
    (std::mem::size_of::<BinderTransactionData>() + 8) as u32,
);

// ============================================================================
// BR_* — binder returns (written by the driver into the read_buffer of
// BINDER_WRITE_READ, and by our proxy into the read side of our wire
// BINDER_WRITE_READ response). Kernel: `enum binder_driver_return_protocol`.
//
// KERNEL TRUTH: the whole BR_* family uses ioctl type char **'r'**
// (0x72). The pre-Z114 table used 'b' and was wrong in every entry —
// which meant the ROM's libbinder (matching 'r' constants) hit the
// `*** BAD COMMAND ***` default arm in `IPCThreadState::executeCommand`
// for every BR we emitted. Same sources as the BC_* audit above:
//
//   BR_ERROR        = _IOR('r', 0, __s32)   0x80047200
//   BR_OK           = _IO ('r', 1)          0x00007201
//   BR_TRANSACTION  = _IOR('r', 2, binder_transaction_data)       0x80407202
//   BR_REPLY        = _IOR('r', 3, binder_transaction_data)       0x80407203
//   BR_DEAD_REPLY   = _IO ('r', 5)          0x00007205
//   BR_TRANSACTION_COMPLETE = _IO ('r', 6)  0x00007206
//   BR_NOOP         = _IO ('r', 12)         0x0000720c
//   BR_SPAWN_LOOPER = _IO ('r', 13)         0x0000720d
//   BR_FAILED_REPLY = _IO ('r', 17)         0x00007211
// ============================================================================

/// Binder return ioctl type char — `'r'` = 0x72 (kernel
/// `enum binder_driver_return_protocol`).
pub const BR_IOC_TYPE: u32 = b'r' as u32;

/// `BR_ERROR` — driver returned an error (payload: i32 error code).
#[allow(dead_code)]
pub const BR_ERROR: u32 = _IOR(BR_IOC_TYPE, 0, 4);

/// `BR_OK` — success (driver often sends this as a heartbeat).
#[allow(dead_code)]
pub const BR_OK: u32 = _IO(BR_IOC_TYPE, 1);

/// `BR_TRANSACTION_SEC_CTX` — same nr as `BR_TRANSACTION` but carrying
/// `binder_transaction_data_secctx` (72 bytes). We never send it (kept
/// for the audit table; AOSP-11 libbinder understands the plain form).
#[allow(dead_code)]
pub const BR_TRANSACTION_SEC_CTX: u32 = _IOR(
    BR_IOC_TYPE,
    2,
    std::mem::size_of::<BinderTransactionDataSecctx>() as u32,
);

/// `BR_TRANSACTION` — incoming transaction from another process.
pub const BR_TRANSACTION: u32 = _IOR(
    BR_IOC_TYPE,
    2,
    std::mem::size_of::<BinderTransactionData>() as u32,
);

/// `BR_REPLY` — reply to a previously-sent `BC_TRANSACTION`.
pub const BR_REPLY: u32 = _IOR(
    BR_IOC_TYPE,
    3,
    std::mem::size_of::<BinderTransactionData>() as u32,
);

/// `BR_ACQUIRE_RESULT` — legacy, not supported.
#[allow(dead_code)]
pub const BR_ACQUIRE_RESULT: u32 = _IOR(BR_IOC_TYPE, 4, 4);

/// `BR_DEAD_REPLY` — the target of the last transaction is dead.
pub const BR_DEAD_REPLY: u32 = _IO(BR_IOC_TYPE, 5);

/// `BR_TRANSACTION_COMPLETE` — the last BC_TRANSACTION/BC_REPLY was
/// accepted. `IPCThreadState::waitForResponse` consumes this and keeps
/// looping for the actual `BR_REPLY` — batching COMPLETE+REPLY in one
/// response frame is legal (and what we do).
pub const BR_TRANSACTION_COMPLETE: u32 = _IO(BR_IOC_TYPE, 6);

/// `BR_INCREFS` — acquire a weak reference on a local binder
/// (payload: `binder_ptr_cookie`).
#[allow(dead_code)]
pub const BR_INCREFS: u32 = _IOR(
    BR_IOC_TYPE,
    7,
    std::mem::size_of::<BinderPtrCookie>() as u32,
);

/// `BR_ACQUIRE` — acquire a strong reference on a local binder.
#[allow(dead_code)]
pub const BR_ACQUIRE: u32 = _IOR(
    BR_IOC_TYPE,
    8,
    std::mem::size_of::<BinderPtrCookie>() as u32,
);

/// `BR_RELEASE` — release a strong reference on a local binder.
#[allow(dead_code)]
pub const BR_RELEASE: u32 = _IOR(
    BR_IOC_TYPE,
    9,
    std::mem::size_of::<BinderPtrCookie>() as u32,
);

/// `BR_DECREFS` — release a weak reference on a local binder.
#[allow(dead_code)]
pub const BR_DECREFS: u32 = _IOR(
    BR_IOC_TYPE,
    10,
    std::mem::size_of::<BinderPtrCookie>() as u32,
);

/// `BR_ATTEMPT_ACQUIRE` — try-acquire (rare; payload
/// `binder_pri_ptr_cookie`, 24 bytes).
#[allow(dead_code)]
pub const BR_ATTEMPT_ACQUIRE: u32 = _IOR(
    BR_IOC_TYPE,
    11,
    std::mem::size_of::<BinderPriPtrCookie>() as u32,
);

/// `BR_NOOP` — no-op. The looper consumes this and loops again.
pub const BR_NOOP: u32 = _IO(BR_IOC_TYPE, 12);

/// `BR_SPAWN_LOOPER` — driver wants the process to spawn another binder
/// thread (up to the `BINDER_SET_MAX_THREADS` limit).
pub const BR_SPAWN_LOOPER: u32 = _IO(BR_IOC_TYPE, 13);

/// `BR_FINISHED` — historical, no longer sent by the driver.
#[allow(dead_code)]
pub const BR_FINISHED: u32 = _IO(BR_IOC_TYPE, 14);

/// `BR_DEAD_BINDER` — a remote binder we requested death notification
/// for has died.
#[allow(dead_code)]
pub const BR_DEAD_BINDER: u32 = _IOR(BR_IOC_TYPE, 15, 8);

/// `BR_CLEAR_DEATH_NOTIFICATION_DONE` — ack of `BC_CLEAR_DEATH_NOTIFICATION`.
#[allow(dead_code)]
pub const BR_CLEAR_DEATH_NOTIFICATION_DONE: u32 = _IOR(BR_IOC_TYPE, 16, 8);

/// `BR_FAILED_REPLY` — the last `BC_TRANSACTION` failed (e.g. the
/// target handle is invalid, or the target process died).
pub const BR_FAILED_REPLY: u32 = _IO(BR_IOC_TYPE, 17);

// ============================================================================
// Service manager transaction codes.
//
// AOSP-11 truth: the ROM's servicemanager implements the AIDL
// `android.os.IServiceManager` (frameworks/native
// cmds/servicemanager/ServiceManager.cpp), and libbinder's
// `defaultServiceManager()` returns a shim over that same AIDL
// interface (libs/binder/IServiceManager.cpp, ServiceManagerShim). AIDL
// numbers transactions FIRST_CALL_TRANSACTION (=1) + method index, and
// the first four methods were ordered to match the legacy hand-written
// protocol codes exactly:
//
//   1 getService          2 checkService
//   3 addService          4 listServices
//   5 registerForNotifications   6 unregisterForNotifications
//   7 isDeclared          8 registerClientCallback
//   9 tryUnregisterService
// ============================================================================

/// `getService` — look up a service by name (legacy GET_SERVICE).
pub const SVC_MGR_GET_SERVICE: u32 = 1;

/// `checkService` — non-blocking lookup (same wire shape as GET).
pub const SVC_MGR_CHECK_SERVICE: u32 = 2;

/// `addService` — register a service by name. The transaction carries
/// the service name + a strong binder flat object + allowIsolated +
/// dumpPriority.
pub const SVC_MGR_ADD_SERVICE: u32 = 3;

/// `listServices` — enumerate registered service names.
pub const SVC_MGR_LIST_SERVICES: u32 = 4;

/// `registerForNotifications` — request a callback when a service is
/// registered (accepted + acknowledged; no callback delivery — minimal).
pub const SVC_MGR_REGISTER_FOR_NOTIFICATIONS: u32 = 5;

/// `unregisterForNotifications`.
#[allow(dead_code)]
pub const SVC_MGR_UNREGISTER_FOR_NOTIFICATIONS: u32 = 6;

/// `isDeclared`.
#[allow(dead_code)]
pub const SVC_MGR_IS_DECLARED: u32 = 7;

/// `registerClientCallback`.
#[allow(dead_code)]
pub const SVC_MGR_REGISTER_CLIENT_CALLBACK: u32 = 8;

/// `tryUnregisterService`.
#[allow(dead_code)]
pub const SVC_MGR_TRY_UNREGISTER_SERVICE: u32 = 9;

/// The well-known binder handle of the servicemanager itself.
pub const SVC_MGR_HANDLE: u32 = 0;

/// `IBinder::PING_TRANSACTION` = `B_PACK_CHARS('_','P','N','G')` —
/// big-endian char packing = 0x5F504E47 (VERIFIED on-device: run
/// 33411932921 logged hwservicemanager pings as `code=1599098439`).
/// The pre-6-Z271c `from_ne_bytes` spelling was byte-swapped, so the
/// PING fast-path never matched a real ping.
pub const PING_TRANSACTION: u32 = u32::from_be_bytes(*b"_PNG");
/// `IBinder::INTERFACE_TRANSACTION` — `_NTF` (0x5f4e5446). The descriptor
/// query every real client sends to a fresh proxy; answered with the
/// BARE interface-descriptor string16 (no exception header — see the
/// 6-Z272f branch in `handle_transaction`).
pub const INTERFACE_TRANSACTION: u32 = u32::from_be_bytes(*b"_NTF");

/// `android.hidl.manager.V1_0.IServiceManager` method codes (HIDL —
/// declaration order, FIRST_CALL_TRANSACTION = 1): get = 1, add = 2.
pub const HIDL_SM_GET: u32 = 1;
pub const HIDL_SM_ADD: u32 = 2;
/// 6-Z276: `android.hidl.manager.V1_0.IServiceManager` method codes
/// (hidl-generated order: get, add, getTransport,
/// registerForNotifications, unregisterForNotifications, …).
pub const HIDL_SM_REGISTER_FOR_NOTIFICATIONS: u32 = 4;
pub const HIDL_SM_UNREGISTER_FOR_NOTIFICATIONS: u32 = 5;

// ============================================================================
// Flat-binder-object type constants (kernel `B_PACK_CHARS(c1,c2,c3,0x85)`).
// ============================================================================

const fn b_pack_chars(c1: u8, c2: u8, c3: u8) -> u32 {
    ((c1 as u32) << 24) | ((c2 as u32) << 16) | ((c3 as u32) << 8) | 0x85
}

/// Strong local binder (ptr + cookie point at the owner's BBinder).
pub const BINDER_TYPE_BINDER: u32 = b_pack_chars(b's', b'b', b'*'); // 0x73622a85
/// Weak local binder.
#[allow(dead_code)]
pub const BINDER_TYPE_WEAK_BINDER: u32 = b_pack_chars(b'w', b'b', b'*'); // 0x77622a85
/// Strong remote reference (binder field = the remote handle).
pub const BINDER_TYPE_HANDLE: u32 = b_pack_chars(b's', b'h', b'*'); // 0x73682a85
/// Weak remote reference.
#[allow(dead_code)]
pub const BINDER_TYPE_WEAK_HANDLE: u32 = b_pack_chars(b'w', b'h', b'*'); // 0x77682a85
/// File descriptor.
#[allow(dead_code)]
pub const BINDER_TYPE_FD: u32 = b_pack_chars(b'f', b'd', b'*'); // 0x66642a85
/// File-descriptor array.
#[allow(dead_code)]
pub const BINDER_TYPE_FDA: u32 = b_pack_chars(b'f', b'd', b'a'); // 0x66646185
/// Scatter-gather pointer.
#[allow(dead_code)]
pub const BINDER_TYPE_PTR: u32 = b_pack_chars(b'p', b't', b'*'); // 0x70742a85

/// `FLAT_BINDER_FLAG_ACCEPTS_FDS`.
pub const FLAT_BINDER_FLAG_ACCEPTS_FDS: u32 = 0x100;

/// The flags value libbinder's `flattenBinder()` stamps on every binder
/// flat object it writes when background scheduling is enabled (the
/// normal case): `0x13` (MAX_NICE / priority 19) | ACCEPTS_FDS = `0x113`
/// (android-11 `Parcel.cpp:200`; the FD-object writer at `Parcel.cpp:1108`
/// uses `0x7f` instead — not a binder flag). The receiving
/// `unflattenBinder` ignores `flags`, so the value is cosmetic, but we
/// reproduce the wire truth for byte-fidelity.
pub const FLAT_FLAGS_LIBBINDER_DEFAULT: u32 = 0x13 | FLAT_BINDER_FLAG_ACCEPTS_FDS;

/// 6-Z271x: the android-12+ binder stability annotation that follows EVERY
/// `flat_binder_object` on the wire. Verified against android-13
/// `framework/native/libs/binder/Parcel.cpp` + `Stability.{h,cpp}`:
///
/// * `Parcel::flattenBinder` ends with `finishFlattenBinder(binder)` =
///   `writeInt32(Stability::getRepr(binder))` — a 4-byte annotation AFTER
///   the 24-byte flat object.
/// * `Parcel::unflattenBinder` → `finishUnflattenBinder` =
///   `readInt32(&stability)` + `Stability::setRepr(binder, stability,
///   /*log=*/true)`. `setRepr` rejects anything outside
///   `isDeclaredLevel()` = {VENDOR 0b000011, SYSTEM 0b001100,
///   VINTF 0b111111} with BAD_TYPE → the whole `readStrongBinder`
///   returns null (the 6-Z271w NAME_NOT_FOUND class).
/// * The annotation also feeds the later CALL-TIME gate
///   (`BpBinder::transact`: `Stability::check(getRepr(this),
///   getLocalLevel())`, where check is `(provided & required) ==
///   required`). VINTF `0b111111` is the only declared level that passes
///   for both system clients (`getLocalLevel()=SYSTEM`, system
///   keystore2/recovery) and vendor clients (`VENDOR`), and it is exactly
///   what real `@VintfStability` HALs (keymint/vibrator/sharedsecret)
///   put on the wire — so VIRTUALLY-REGISTERED services are annotated
///   VINTF.
const STABILITY_ANNOTATION_VINTF: i32 = 0b1111_11; // 63 = Stability::Level::VINTF

/// The annotation the real `flattenBinder(nullptr)` writes for a null
/// binder: `getRepr(nullptr) = UNDECLARED (0)`, and
/// `setRepr(nullptr, UNDECLARED)` returns OK. Used on the SM miss
/// replies (null-binder flats).
const STABILITY_ANNOTATION_NULL: i32 = 0;

/// 6-Z272e: android-12/12L wire format. The A12 libbinder wraps the
/// stability level in a `Category { u8 version; u8 reserved[2]; u8 level }`
/// (Stability.h) — `level << 24 | version` as an i32 — and
/// `Category::fromRepr(63)` decodes to version=63 / level=UNDECLARED(0)
/// → "Can only set known stability, not 0." → BAD_TYPE → null binder
/// (714 `V/Stability` lines in the R12-lavender run = every SM-reply
/// parse). A12 accepts version >= 1 (kBinderWireFormatOldest) and the
/// real A12 servicemanager writes `Category::currentFromLevel(level)` =
/// version 1 + level — so VINTF = `0x3F000001`.
const STABILITY_ANNOTATION_VINTF_A12: i32 = (0b1111_11 << 24) | 1; // 0x3F000001

/// 6-Z272e: the android-12/12L null-binder Category (version 1, level
/// UNDECLARED — setRepr(nullptr, level==UNDECLARED) returns OK).
const STABILITY_ANNOTATION_NULL_A12: i32 = 1;

// ============================================================================
// Transaction flags (kernel `enum transaction_flags`).
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
/// `TF_CLEAR_BUF` — clear buffer on txn complete.
#[allow(dead_code)]
pub const TF_CLEAR_BUF: u32 = 0x20;
/// `TF_UPDATE_TXN` — update the outdated pending async txn.
#[allow(dead_code)]
pub const TF_UPDATE_TXN: u32 = 0x40;

// ============================================================================
// Misc constants.
// ============================================================================

/// Binder protocol version returned by `BINDER_VERSION`. Matches
/// `CURRENT_PROTOCOL_VERSION` in `drivers/android/binder.c`. Android 11
/// ships protocol version 8.
pub const BINDER_CURRENT_PROTOCOL_VERSION: u32 = 8;

/// Number of worker threads in the binder proxy's thread pool. Kept
/// for the skeleton-era [`ThreadPool`] (now only exercised by its own
/// unit test; the live proxy above spawns one thread PER connection
/// bounded by [`MAX_PROXY_CONNECTIONS`]).
pub const BINDER_THREAD_POOL_SIZE: usize = 4;

/// Base of the proxy-allocated service handles. 6-Z271v: handles are
/// now KERNEL-TRUE — small dense integers from 1 (handle 0 stays the
/// context manager), exactly like the real binder driver allocates them.
///
/// WHY THE OLD `0xF0000000` BASE WAS FATAL (the last piece of the ~18 s
/// hole, runs 33496750544/33501057212/33509290359): a REAL libbinder
/// client that receives a handle builds its handle table in
/// `ProcessState::lookupHandleLocked` —
///   mHandleToObject.insertAt(e, N, handle+1-N)
/// — i.e. it inserts `handle+1` entries into an `android::Vector`. With
/// handle 0xF0000004 (negative as int32 → ~4.29 billion as size_t) that
/// is a FOUR-BILLION-ENTRY insert → libutils Vector capacity overflow →
/// LOG_ALWAYS_FATAL("new_capacity overflow", tag "Vector") → abort() →
/// the §13 park → every mutex the aborting thread holds wedges the
/// process. Every client that ever received one of our handles died
/// before its first transaction (0 routed/virtual transactions across
/// all runs since the registry went live). The old base existed to keep
/// fake handles from aliasing host handles on the retired
/// `forward_transaction_to_host` skeleton — the bus routes everything
/// locally now, so the constraint is gone.
pub const PROXY_HANDLE_BASE: u32 = 0;

/// Maximum concurrent guest connections (one thread each). The guest's
/// `libbinder.so` opens one binder fd per process (`ProcessState`),
/// and a booting GSI has dozens of processes; 64 is comfortably above
/// that and bounded so a misbehaving guest can't thread-bomb the daemon.
pub const MAX_PROXY_CONNECTIONS: usize = 64;

/// How long a pure-read `BINDER_WRITE_READ` blocks on its connection
/// queue before yielding `BR_NOOP` (the S1b blocking-idle analogue of
/// the kernel's blocking `read_buffer`). 250 ms keeps polling latency
/// reasonable while not busy-spinning the connection thread.
pub const IDLE_POLL_TICK: Duration = Duration::from_millis(250);

/// Cap on queued undelivered BR items per connection (defence against a
/// runaway server that never reads its reply queue).
pub const MAX_QUEUED_ITEMS: usize = 256;

/// The marker that announces a v2 WRITE_READ payload (the z115 loader
/// counterpart). A request that ends exactly after its BC_* stream is
/// v1 (z113) — byte-compatible; a v2 request appends
/// `[WIRE_V2_MAGIC][u32 blob_count]` followed by `blob_count` parcel
/// blobs pairing in order with the BC_TRANSACTION/BC_REPLY/`*_SG`
/// records. Responses echo the trailer only when the request was v2.
/// Bytes are `'W' 'V' '2' '0'` in native-endian word order.
pub const WIRE_V2_MAGIC: u32 = u32::from_ne_bytes(*b"WV20");

/// AIDL interface-token header tag for `/dev/binder` clients
/// (`Parcel::writeInterfaceToken`): `B_PACK_CHARS('S','Y','S','T')` —
/// big-endian char packing, so the wire u32 is 0x53595354. NOTE: this
/// must be `from_be_bytes` — the pre-6-Z271c constant was byte-swapped
/// (masked by the self-consistent test codec) so EVERY real guest parcel
/// failed the tag peek and fell into the HIDL branch, where addService
/// (code 3) is unhandled → the registry stayed inert even after the v2
/// request inlining landed (run 33425291816: v2=true everywhere, zero
/// parse-success logs, the 18.5 s wait intact).
pub const AIDL_HEADER_TAG_SYST: u32 = u32::from_be_bytes(*b"SYST");

/// AIDL interface-token header tag for `/dev/vndbinder` clients
/// (`B_PACK_CHARS('V','N','D','R')`). Accepted alongside `SYST` (the
/// proxy does not split contexts).
pub const AIDL_HEADER_TAG_VNDR: u32 = u32::from_be_bytes(*b"VNDR");

/// AIDL interface-token header tag written by a libbinder built with
/// `__ANDROID_RECOVERY__` — `B_PACK_CHARS('R','E','C','O')` (LineageOS-20
/// `libs/binder/Parcel.cpp`: `#elif defined(__ANDROID_RECOVERY__)
/// constexpr int32_t kHeader = B_PACK_CHARS('R','E','C','O');`). The
/// RECOVERY guests ARE the primary corpus (TWRP/OrangeFox) — every
/// servicemanager parcel they emit carries THIS tag, never SYST.
pub const AIDL_HEADER_TAG_RECO: u32 = u32::from_be_bytes(*b"RECO");

/// All AIDL header tags the proxy accepts at the `is_aidl` peek.
pub(crate) fn is_aidl_header_tag(tag: u32) -> bool {
    tag == AIDL_HEADER_TAG_SYST || tag == AIDL_HEADER_TAG_VNDR || tag == AIDL_HEADER_TAG_RECO
}

/// The servicemanager AIDL interface descriptor (the string16 that
/// follows the 3-i32 header in every `android.os.IServiceManager`
/// transaction parcel).
pub const SVC_MGR_IFACE_DESCRIPTOR: &str = "android.os.IServiceManager";

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
// Parcel codec — libbinder Parcel byte buffer reader/writer.
//
// Matches android-11 `frameworks/native/libs/binder/Parcel.cpp` field for
// field (verified against the fetched source per 6-Z114 §3.2). The proxy
// uses these to (a) parse the AIDL interface-token header + per-code args
// out of each `BC_TRANSACTION`'s parcel data, and (b) synthesise the
// `binder::Status`-prefixed reply parcels the AIDL stub expects back.
// ============================================================================

/// Cursor over a libbinder Parcel byte buffer. All multi-byte reads are
/// native-endian (LE on the supported aarch64/x86_64 targets). All
/// `read_*` methods return `None` if the cursor runs past the end of the
/// buffer; callers treat that as a malformed parcel and reply with
/// `BR_FAILED_REPLY`.
struct ParcelReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> ParcelReader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        ParcelReader { buf, pos: 0 }
    }

    /// Bytes remaining between the cursor and the end of the buffer.
    #[allow(dead_code)]
    fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    /// Read a little-endian i32. `None` if past end.
    fn read_i32(&mut self) -> Option<i32> {
        self.read_u32().map(|v| v as i32)
    }

    /// Read a little-endian u32. `None` if past end.
    fn read_u32(&mut self) -> Option<u32> {
        if self.pos + 4 > self.buf.len() {
            return None;
        }
        let v = u32::from_ne_bytes(self.buf[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        Some(v)
    }

    /// Read a UTF-16LE string16 (`Parcel::writeString16` android-11):
    /// `[i32 len_in_char16][len × u16][u16 0 NUL — always written]
    /// [zero-pad to 4-byte alignment]`. `len = -1` encodes null →
    /// returns `Some(None)` so callers can distinguish; `None` only on
    /// truncation. Invalid surrogate pairs are replaced with U+FFFD
    /// (lossy — matches `String::from_utf16_lossy`).
    fn read_string16(&mut self) -> Option<Option<String>> {
        let len = self.read_i32()?;
        if len < 0 {
            // Parcel::writeString16(nullptr) writes -1 and nothing else
            // (no NUL, no pad). Consumer treats as null.
            return Some(None);
        }
        let len = len as usize;
        let byte_len = len * 2;
        if self.pos + byte_len + 2 > self.buf.len() {
            return None;
        }
        let mut chars = Vec::with_capacity(len);
        for i in 0..len {
            let off = self.pos + i * 2;
            let u = u16::from_ne_bytes(self.buf[off..off + 2].try_into().unwrap());
            chars.push(u);
        }
        self.pos += byte_len;
        self.pos += 2; // always-written trailing NUL
                       // Pad to 4-byte alignment of the data buffer.
        while self.pos < self.buf.len() && self.pos % 4 != 0 {
            self.pos += 1;
        }
        Some(Some(String::from_utf16_lossy(&chars)))
    }

    /// Read a `flat_binder_object` (24 bytes). `None` if past end.
    fn read_flat_binder(&mut self) -> Option<FlatBinderObject> {
        if self.pos + 24 > self.buf.len() {
            return None;
        }
        let typ = u32::from_ne_bytes(self.buf[self.pos..self.pos + 4].try_into().unwrap());
        let flags = u32::from_ne_bytes(self.buf[self.pos + 4..self.pos + 8].try_into().unwrap());
        let binder = u64::from_ne_bytes(self.buf[self.pos + 8..self.pos + 16].try_into().unwrap());
        let cookie = u64::from_ne_bytes(self.buf[self.pos + 16..self.pos + 24].try_into().unwrap());
        self.pos += 24;
        Some(FlatBinderObject {
            r#type: typ,
            flags,
            binder,
            cookie,
        })
    }

    /// Consume the AIDL interface-token header
    /// (`Parcel::writeInterfaceToken` android-11 — verified per 6-Z114 §3.2):
    /// `[i32 strict_policy][i32 work_source][i32 header_tag][string16 descriptor]`.
    /// Returns the parsed `(strict, work, tag, descriptor)` tuple; `None`
    /// if the buffer was too short. The descriptor is `None` if the
    /// request parcel encoded it as null.
    fn read_aidl_header(&mut self) -> Option<(i32, i32, u32, Option<String>)> {
        let strict = self.read_i32()?;
        let work = self.read_i32()?;
        let tag = self.read_u32()?;
        let iface = self.read_string16()?;
        Some((strict, work, tag, iface))
    }

    /// Read a HIDL `hidl_string` (libhwbinder `writeHidlString`):
    /// `[i32 len][len bytes][NUL][zero-pad to 4-byte alignment]`.
    /// Null (`len < 0`) reads as an empty string.
    fn read_hidl_string(&mut self) -> Option<String> {
        let len = self.read_i32()?;
        if len <= 0 {
            return Some(String::new());
        }
        let len = len as usize;
        if self.pos + len + 1 > self.buf.len() {
            return None;
        }
        let s = String::from_utf8_lossy(&self.buf[self.pos..self.pos + len]).into_owned();
        self.pos += len;
        self.pos += 1; // trailing NUL
        while self.pos < self.buf.len() && self.pos % 4 != 0 {
            self.pos += 1;
        }
        Some(s)
    }
}

/// Builder for a libbinder Parcel byte buffer + its companion offsets
/// array. Writes are little-endian. `write_flat_binder` ALSO appends the
/// offset of the object being written to the offsets array — both the
/// kernel's translation table and the Parcel object bookkeeping depend
/// on every flat object being listed in `binder_transaction_data.offsets`
/// (verified 6-Z114 §3.2 / §3.3).
struct ParcelWriter {
    data: Vec<u8>,
    offsets: Vec<u8>,
}

impl ParcelWriter {
    fn new() -> Self {
        ParcelWriter {
            data: Vec::new(),
            offsets: Vec::new(),
        }
    }

    fn write_i32(&mut self, v: i32) {
        self.data.extend_from_slice(&v.to_ne_bytes());
    }

    #[allow(dead_code)]
    fn write_u32(&mut self, v: u32) {
        self.data.extend_from_slice(&v.to_ne_bytes());
    }

    /// 6-Z276: write a HIDL `hidl_string` (libhwbinder `writeHidlString`):
    /// `[i32 len][len bytes][NUL][zero-pad to 4-byte alignment]` — the
    /// exact inverse of [`ParcelReader::read_hidl_string`].
    fn write_hidl_string(&mut self, s: &str) {
        self.write_i32(s.len() as i32);
        self.data.extend_from_slice(s.as_bytes());
        self.data.push(0); // trailing NUL
        while self.data.len() % 4 != 0 {
            self.data.push(0);
        }
    }

    /// Write an AIDL "structured parcelable" region — the android-12/13
    /// wire shape a REAL AIDL client deserializes:
    ///
    /// ```text
    ///   [i32 1]      null-flag word — NON_NULL_PARCELABLE_FLAG (0 = null)
    ///   [i32 size]   self-inclusive size word (covers itself + fields)
    ///   [fields…]
    /// ```
    ///
    /// Verified against android-13.0.0_r1: the AIDL Rust backend's
    /// `impl_deserialize_for_parcelable!` → `DeserializeOption::
    /// deserialize_option_from` reads the flag word FIRST, then
    /// `Parcelable::read_from_parcel` → `sized_read` consumes the size
    /// word (generate_rust.cpp); the C++ NDK backend reads the same
    /// shape via `AParcel_readParcelable` (null_flag i32) +
    /// `_aidl_readFromParcel` (size i32). Run 33776470629 proved the
    /// flag word is real: with only the size word present keystore2
    /// read our `size` as the flag, then read versionNumber=300 as the
    /// size → bounded past the reply → NOT_ENOUGH_DATA →
    /// EX_TRANSACTION_FAILED ("Binder exception code TRANSACTION_
    /// FAILED, 0.") — the LAST keystore2 stall.
    fn write_structured_parcelable<T>(&mut self, fields: T) -> &mut Self
    where
        T: FnOnce(&mut ParcelWriter),
    {
        self.data.extend_from_slice(&1i32.to_ne_bytes()); // NON_NULL_PARCELABLE_FLAG
        let start = self.data.len();
        self.data.extend_from_slice(&0i32.to_ne_bytes()); // size placeholder
        fields(self);
        let size = (self.data.len() - start) as i32;
        self.data[start..start + 4].copy_from_slice(&size.to_ne_bytes());
        self
    }

    /// Write a UTF-16LE string16 with the always-written trailing NUL and
    /// 4-byte pad (`Parcel::writeString16` android-11).
    fn write_string16(&mut self, s: &str) {
        let u16s: Vec<u16> = s.encode_utf16().collect();
        let len = u16s.len() as i32;
        self.data.extend_from_slice(&len.to_ne_bytes());
        for u in u16s {
            self.data.extend_from_slice(&u.to_ne_bytes());
        }
        // Always-written NUL terminator
        self.data.extend_from_slice(&0u16.to_ne_bytes());
        // Pad to 4-byte alignment of the DATA buffer (Parcel pads to 4).
        while self.data.len() % 4 != 0 {
            self.data.push(0);
        }
    }

    /// Write a `flat_binder_object` (24 bytes) and append its offset to
    /// the offsets array. Returns the offset at which the object was
    /// written.
    fn write_flat_binder(&mut self, obj: &FlatBinderObject) -> u64 {
        let off = self.data.len() as u64;
        self.data.extend_from_slice(&obj.r#type.to_ne_bytes());
        self.data.extend_from_slice(&obj.flags.to_ne_bytes());
        self.data.extend_from_slice(&obj.binder.to_ne_bytes());
        self.data.extend_from_slice(&obj.cookie.to_ne_bytes());
        self.offsets.extend_from_slice(&off.to_ne_bytes());
        off
    }

    /// Write the AIDL success-status prefix (`Parcel::writeNoException`):
    /// a single i32 0 (EX_NONE). Reply parcels begin with this; the
    /// 3-word interface-token header is REQUEST-side only.
    fn write_status_ok(&mut self) {
        self.write_i32(0);
    }

    /// Write a native-endian i64 (Parcel's 64-bit integer encoding;
    /// the writer only ever emits aligned 4-byte blocks so no explicit
    /// padding is needed before/after).
    fn write_i64(&mut self, v: i64) {
        self.data.extend_from_slice(&v.to_ne_bytes());
    }

    /// Write a `@nullable String` field as NULL — the AIDL wire encoding
    /// is a single i32 -1 length word (`AParcel_writeNullableString` →
    /// `AParcel_writeString(nullptr)`; libbinder `writeString16` with a
    /// null String16 writes -1 too). The reader-side (`readNullableString`)
    /// sees a negative length and yields std::nullopt/None.
    fn write_nullable_string16_none(&mut self) {
        self.write_i32(-1);
    }

    /// The current data buffer length (for offsets bookkeeping / assertions).
    #[allow(dead_code)]
    fn data_len(&self) -> usize {
        self.data.len()
    }

    /// Consume the writer, returning `(data, offsets)`.
    fn into_parts(self) -> (Vec<u8>, Vec<u8>) {
        (self.data, self.offsets)
    }
}

// ============================================================================
// Proxy-side servicemanager registry + v2 wire blob.
// ============================================================================

/// One v2 wire blob (parcel data + offsets array) — pairs in stream order
/// with each `BC_TRANSACTION`/`BC_REPLY`/`*_SG` on the request side and
/// each `BR_REPLY`/`BR_TRANSACTION` on the response side (6-Z114 §4.4).
///
/// `offsets` is informational on the proxy side: the servicemanager
/// proxy reads the request parcel sequentially (so it doesn't NEED the
/// offsets array), but a future host-forwarding path (BINDER-3) and a
/// proper server-routing extension will need it to walk the flat
/// objects in their kernel-listed order.
struct RequestBlob {
    data: Vec<u8>,
    #[allow(dead_code)]
    offsets: Vec<u8>,
}

impl Clone for RequestBlob {
    fn clone(&self) -> Self {
        RequestBlob {
            data: self.data.clone(),
            offsets: self.offsets.clone(),
        }
    }
}

/// Proxy-side servicemanager registry: service name → proxy handle
/// (allocated from [`PROXY_HANDLE_BASE`] + 1 — 6-Z271v: dense
/// kernel-true integers). Per 6-Z114 §3.3 / §3.4 the
/// proxy stamps the proxy handle into the `BINDER_TYPE_HANDLE` flat
/// object it returns to the requester; a subsequent `BC_TRANSACTION` to
/// that handle would be routed back to the owning guest connection (the
/// route path is a v2+ extension; the registry itself only needs the
/// lookup + add).
#[derive(Default)]
pub struct ServiceRegistry {
    /// name → fake proxy handle. A `BTreeMap` so `list_services` iterates
    /// in deterministic (alphabetical) order — matches the array shape
    /// `listServices` returns and makes unit tests reproducible.
    by_name: BTreeMap<String, u32>,
    /// Monotonic counter; starts at `PROXY_HANDLE_BASE + 1` so handle 0
    /// stays reserved for the servicemanager itself.
    next_handle: u32,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        ServiceRegistry {
            by_name: BTreeMap::new(),
            next_handle: PROXY_HANDLE_BASE + 1,
        }
    }

    /// Look up a service by name. Returns the proxy handle on hit.
    pub fn get(&self, name: &str) -> Option<u32> {
        self.by_name.get(name).copied()
    }

    /// Register a service name, allocating a new fake handle if the name
    /// isn't already registered. Returns the handle to use in the reply.
    /// Re-registering the same name returns the EXISTING handle (matches
    /// the native servicemanager's "overwrite" semantics in
    /// `frameworks/native/cmds/servicemanager/ServiceManager.cpp`).
    pub fn add(&mut self, name: &str) -> u32 {
        if let Some(&h) = self.by_name.get(name) {
            return h;
        }
        let h = self.next_handle;
        self.next_handle += 1;
        self.by_name.insert(name.to_string(), h);
        h
    }

    /// Snapshot of all registered service names in alphabetical order
    /// (BTreeMap iter is sorted — matches `listServices`' array shape).
    pub fn list(&self) -> Vec<String> {
        self.by_name.keys().cloned().collect()
    }

    /// Number of services currently registered.
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// `true` iff no services are registered.
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

// ============================================================================
// 6-Z271: guest-local transaction bus.
// ============================================================================

/// Unique per-connection id. `PROXY_CONN_ID` (0) is the proxy itself —
/// the "owner" of the in-proxy virtual services.
pub type ConnId = u64;

/// The proxy's own connection id (virtual services are "owned" by it).
pub const PROXY_CONN_ID: ConnId = 0;

/// 6-Z271 wire extension: connection-identity frame command. NOT a real
/// binder ioctl — a dedicated `'b'`-type number the loader sends right
/// after connect with `[u32 pid][u32 uid][u32 gid]`, so routed
/// transactions can carry kernel-true sender identities.
pub const WIRE_CMD_IDENT: u32 = 0x4004_62FF;

/// Which in-proxy virtual service backs a handle. These are minimal but
/// SEMANTICALLY CORRECT AIDL implementations (real parcel shapes, honest
/// errors) — never fake-success: operations the container cannot satisfy
/// return service-specific errors rather than bogus data.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum VirtualService {
    /// `android.hardware.vibrator.IVibrator/default` — kills the ~5 s
    /// per-tap waitForService on the recovery input thread; `on(ms)` is
    /// forwarded to the host app for a REAL vibration.
    Vibrator,
    /// `android.hardware.security.keymint.IKeyMintDevice/default` —
    /// lets keystore2 obtain its backend HAL and register
    /// IKeystoreSecurity (kills the ~20 s recovery wait). Key operations
    /// return honest KeyMint errors (software device, no hardware
    /// backend) — TWRP already handles that outcome (unmountable-/data
    /// fallback), just 20 s sooner.
    KeyMint,
    /// `android.hardware.security.sharedsecret.ISharedSecret/default` —
    /// keystore2's shared-secret negotiation partner.
    SharedSecret,
    /// `android.hardware.health.IHealth/default` (6-Z298) — serves the
    /// AIDL battery chain of AOSP/Lineage recovery ≥ A12
    /// (`GetBatteryInfo()` → `AServiceManager_isDeclared` +
    /// `waitForService` + `getCapacity`/`getChargeStatus`/`getHealthInfo`,
    /// verified from android-15.0.0_r1 `recovery_utils/battery_utils.cpp`).
    /// Without it, recovery's battery gate falls back to the HIDL shim
    /// and — finding nothing — assumes fake defaults (capacity 100,
    /// charging=true), so a REAL phone's low-battery sideload gate is
    /// silently bypassed. Values come from the pinned battery sysfs tree
    /// (`crate::battery::read_guest_battery_values`) — the SAME tree the
    /// sysfs-reader class (BatteryMonitor) reads: one source of truth.
    /// Missing sysfs files translate to the interface's documented
    /// `EX_UNSUPPORTED_OPERATION`, never fabricated data.
    Health,
}

impl VirtualService {
    /// 6-Z272f: the AIDL interface descriptor — the reply body for
    /// `IBinder::INTERFACE_TRANSACTION` (`BpBinder::getInterface-
    /// Descriptor` reads a BARE string16 from the reply — no exception
    /// header, exactly what `BBinder::onTransact`'s default case writes).
    pub fn descriptor(self) -> &'static str {
        match self {
            VirtualService::Vibrator => "android.hardware.vibrator.IVibrator",
            VirtualService::KeyMint => "android.hardware.security.keymint.IKeyMintDevice",
            VirtualService::SharedSecret => "android.hardware.security.sharedsecret.ISharedSecret",
            VirtualService::Health => "android.hardware.health.IHealth",
        }
    }
}

/// One registered service: name → handle + owner + the owner's local
/// binder ptr/cookie (stamped into delivered `BR_TRANSACTION`s so the
/// server sees its own BBinder identity, kernel-style).
struct ServiceEntry {
    handle: u32,
    owner: ConnId,
    ptr: u64,
    cookie: u64,
    virtual_kind: Option<VirtualService>,
}

/// 6-Z276: a `registerForNotifications` watcher — the connection, the
/// LOCAL identity of its callback object (the `flat_binder_object` the
/// watcher passed: `BINDER_TYPE_BINDER` carries the watcher's own
/// ptr/cookie), and which SM dialect it registered through (AIDL
/// `android.os.IServiceManager` / HIDL
/// `android.hidl.manager.V1_0.IServiceManager` — the callback parcel
/// shapes differ).
#[derive(Clone, Copy)]
struct ServiceWatcher {
    conn: ConnId,
    ptr: u64,
    cookie: u64,
    hidl: bool,
}

/// An item queued for delivery on a server connection's mailbox.
enum InboxItem {
    /// An incoming transaction (kernel `BR_TRANSACTION` analogue).
    Tx(IncomingTx),
    /// A death notification: `[BR_DEAD_BINDER][cookie]`.
    Death(u64),
}

/// A reply (or failure) that is waiting for the requester's NEXT
/// `BINDER_WRITE_READ`. Kernel semantics: a sync `BC_TRANSACTION` returns
/// `BR_TRANSACTION_COMPLETE` from the ioctl that carried it, and the
/// `BR_REPLY` (or `BR_FAILED_REPLY` / `BR_DEAD_REPLY`) surfaces on a
/// LATER read — possibly after the SAME thread serviced the transaction
/// itself (self-transaction) or while it is mid-nested-call.
enum DeferredReply {
    /// `[BR_REPLY][binder_transaction_data]` + the blob trailer.
    Reply { data: Vec<u8>, offsets: Vec<u8> },
    /// `[BR_FAILED_REPLY]` (reply timeout / server died).
    Failed,
}

/// An incoming transaction queued for delivery to a server connection.
struct IncomingTx {
    requester: ConnId,
    txn_id: u64,
    code: u32,
    flags: u32,
    one_way: bool,
    /// Sender identity (kernel semantics — stamped from the connection's
    /// announced `WIRE_CMD_IDENT` values).
    sender_pid: i32,
    sender_euid: u32,
    /// The request parcel (v2 clients only — the bus needs real bytes to
    /// deliver; v1 transactions to routed handles fail with
    /// `BR_FAILED_REPLY` because the parcel is unreachable).
    blob: Option<RequestBlob>,
    /// The owner's local binder ptr/cookie (target of the delivered
    /// `BR_TRANSACTION`).
    ptr: u64,
    cookie: u64,
}

/// Per-connection mailbox state, held inside [`BusState`].
#[derive(Default)]
struct ConnBox {
    inbox: std::collections::VecDeque<InboxItem>,
    /// Sync transactions queued in `inbox` but not yet delivered — used
    /// to resolve their waiters as `Dead` when the connection dies.
    pending_in: Vec<u64>,
    /// Replies resolved by the bus but not yet consumed by this
    /// connection's next read (kernel: the thread's reply lands on the
    /// thread todo list — see [`DeferredReply`]).
    reply_queue: std::collections::VecDeque<DeferredReply>,
    /// Sync transactions this connection SENT and has not seen a reply
    /// for: (txn id, queued-at). Used for the bounded reply timeout and
    /// for waiter cleanup when the connection dies.
    out_sync: std::collections::VecDeque<(u64, std::time::Instant)>,
    /// This connection currently holds a delivered transaction and owes
    /// its `BC_REPLY` (the requester's pending sync transaction id).
    inflight_txn: Option<u64>,
    /// Death notifications this connection requested:
    /// handle → cookie. Delivered as `BR_DEAD_BINDER` when the owning
    /// connection unregisters.
    death_watch: HashMap<u32, u64>,
    /// Guest process identity (announced via `WIRE_CMD_IDENT`). Stamped
    /// into routed transactions' `sender_pid`/`sender_euid` — kernel
    /// semantics. Zero until the (optional) IDENT frame arrives.
    sender_pid: i32,
    sender_euid: u32,
    /// 6-Z272e: the stability-annotation wire format for THIS
    /// connection. false (default) = the android-12/12L `Category` form
    /// (level<<24 | version 1); true = the android-11/13+ plain-level
    /// form. One VM mixes client generations (the A11 rootfs keystore2
    /// AND the per-image recovery binary talk to the SAME proxy), so
    /// the format self-tunes per connection: a same-service re-get
    /// inside the window is the `waitForService` retry signature of a
    /// reply the client failed to parse → flip (sticky).
    sm_annotate_plain: bool,
    /// The previous SM GET on this connection: (service name, at).
    last_sm_get: Option<(String, std::time::Instant)>,
}

/// Shared per-VM bus state: the service registry with OWNER routing, the
/// connection mailboxes, and the reply-waiter channels.
///
/// 6-Z271 replaces the 6-Z114 inert registry (addService consumed the
/// name but never stored the owner — every real-libbinder transaction ran
/// through `servicemanager_legacy` because the loader sent no request
/// blobs). The bus delivers `BR_TRANSACTION` to the owning guest
/// connection and routes its `BC_REPLY` back to the requester, i.e. the
/// proxy is now a genuine guest-local binder bus for guest↔guest IPC.
pub struct BusState {
    services: BTreeMap<String, ServiceEntry>,
    /// handle → service name (routing lookup for `BC_TRANSACTION`).
    by_handle: HashMap<u32, String>,
    /// Monotonic handle allocator (starts at `PROXY_HANDLE_BASE + 1`;
    /// handle 0 stays reserved for the context manager).
    next_handle: u32,
    /// Active connection mailboxes.
    conns: HashMap<ConnId, ConnBox>,
    /// Pending sync transactions: txn id → the requester connection. The
    /// reply is pushed onto that connection's `reply_queue` (kernel
    /// semantics — the requester picks it up on a later ioctl).
    waiters: HashMap<u64, ConnId>,
    /// 6-Z276: `registerForNotifications` watchers per service name.
    /// When a service registers, every watcher gets a one-way
    /// `onRegistration` callback transaction queued on its connection
    /// (the real servicemanagers' `ServiceCallback::onRegistration` /
    /// HIDL `IServiceNotification::onRegistration` analogue).
    watchers: HashMap<String, Vec<ServiceWatcher>>,
    next_conn: u64,
    next_txn: u64,
}

impl BusState {
    fn new() -> Self {
        let mut bus = BusState {
            services: BTreeMap::new(),
            by_handle: HashMap::new(),
            next_handle: PROXY_HANDLE_BASE + 1,
            conns: HashMap::new(),
            waiters: HashMap::new(),
            watchers: HashMap::new(),
            next_conn: PROXY_CONN_ID + 1,
            next_txn: 1,
        };
        bus.ensure_virtual_services();
        bus
    }

    /// Register the in-proxy virtual services (idempotent). Virtual
    /// handles allocate FIRST so they are stable across boots.
    fn ensure_virtual_services(&mut self) {
        const VIRTUALS: &[(&str, VirtualService)] = &[
            (
                "android.hardware.vibrator.IVibrator/default",
                VirtualService::Vibrator,
            ),
            (
                "android.hardware.security.keymint.IKeyMintDevice/default",
                VirtualService::KeyMint,
            ),
            (
                "android.hardware.security.sharedsecret.ISharedSecret/default",
                VirtualService::SharedSecret,
            ),
            (
                // 6-Z298: makes AServiceManager_isDeclared("android.
                // hardware.health.IHealth/default") → true and
                // waitForService resolve immediately for every AIDL
                // battery client (lineage recovery's GetBatteryInfo).
                "android.hardware.health.IHealth/default",
                VirtualService::Health,
            ),
        ];
        for (name, kind) in VIRTUALS {
            if self.services.contains_key(*name) {
                continue;
            }
            let h = self.next_handle;
            self.next_handle += 1;
            self.services.insert(
                name.to_string(),
                ServiceEntry {
                    handle: h,
                    owner: PROXY_CONN_ID,
                    ptr: 0,
                    cookie: 0,
                    virtual_kind: Some(*kind),
                },
            );
            self.by_handle.insert(h, name.to_string());
            info!(
                "[KR64][binder][svc] virtual service registered: {} → handle 0x{:08x}",
                name, h
            );
        }
    }

    /// Register (or overwrite) a guest-owned service. Returns the handle.
    fn add_guest_service(&mut self, name: &str, owner: ConnId, ptr: u64, cookie: u64) -> u32 {
        if let Some(entry) = self.services.get_mut(name) {
            // 6-Z298: a guest addService OVER a virtual-service name
            // takes ownership (native servicemanager "overwrite"
            // semantics: same name → same handle, new owner). The
            // in-proxy handler must stop answering transactions for the
            // name — from now on they are delivered to the guest owner
            // as BR_TRANSACTION (clearing virtual_kind routes the
            // dispatch into the guest-delivery path below).
            if entry.virtual_kind.is_some() {
                info!(
                    "[KR64][binder][svc] guest addService({}) overrides the in-proxy virtual \
                     implementation — routing now goes to the guest owner (conn={})",
                    name, owner
                );
                entry.virtual_kind = None;
            }
            // Native servicemanager "overwrite" semantics: same name →
            // same handle, new owner.
            entry.owner = owner;
            entry.ptr = ptr;
            entry.cookie = cookie;
            return entry.handle;
        }
        let h = self.next_handle;
        self.next_handle += 1;
        self.services.insert(
            name.to_string(),
            ServiceEntry {
                handle: h,
                owner,
                ptr,
                cookie,
                virtual_kind: None,
            },
        );
        self.by_handle.insert(h, name.to_string());
        h
    }

    /// 6-Z276: record a `registerForNotifications` watcher for `name`.
    /// Duplicate (conn, ptr) registrations are ignored (libbinder dedupes
    /// too — one callback object per service).
    fn add_watcher(&mut self, name: &str, w: ServiceWatcher) {
        let list = self.watchers.entry(name.to_string()).or_default();
        if !list.iter().any(|x| x.conn == w.conn && x.ptr == w.ptr) {
            list.push(w);
        }
    }

    /// 6-Z276: drop a watcher (libbinder `unregisterForNotifications`).
    fn remove_watcher(&mut self, name: &str, conn: ConnId, ptr: u64) {
        if let Some(list) = self.watchers.get_mut(name) {
            list.retain(|x| !(x.conn == conn && x.ptr == ptr));
            if list.is_empty() {
                self.watchers.remove(name);
            }
        }
    }

    /// 6-Z276: drop every watcher registered by a dying connection.
    fn remove_watchers_of_conn(&mut self, conn: ConnId) {
        self.watchers.retain(|_, list| {
            list.retain(|w| w.conn != conn);
            !list.is_empty()
        });
    }

    /// 6-Z276: deliver one-way `onRegistration` callbacks for `name` to
    /// every watcher, then drop the watcher list (real servicemanagers
    /// fire each registration once; libbinder re-registers on demand).
    ///
    /// The callback rides the same `BR_TRANSACTION` mailbox a routed
    /// guest→guest transaction uses, targeted at the watcher's LOCAL
    /// callback ptr/cookie, one-way (both `IServiceCallback.onRegistration`
    /// and HIDL `IServiceNotification.onRegistration` are `oneway`).
    fn fire_registration_callbacks(&mut self, name: &str, handle: u32, preexisting: bool) {
        let Some(watchers) = self.watchers.remove(name) else {
            return;
        };
        for w in watchers {
            // Build the callback parcel in the watcher's own dialect.
            let mut writer = ParcelWriter::new();
            if w.hidl {
                // HIDL `IServiceNotification.onRegistration(fqName,
                // instance, preexisting)` — NO interface-token header
                // (libhwbinder parcels carry none), args are hidl_strings.
                // fqName = the name before the '/' split the guest used at
                // register time; our registry key is the FULL
                // "fqName/instance" string, so send it as both halves of
                // what we have (libhwbinder clients match on the
                // descriptor + instance pair they registered).
                let (fq, inst) = match name.rfind('/') {
                    Some(i) => (&name[..i], &name[i + 1..]),
                    None => (name, "default"),
                };
                writer.write_hidl_string(fq);
                writer.write_hidl_string(inst);
                writer.write_i32(preexisting as i32);
            } else {
                // AIDL `android.os.IServiceCallback.onRegistration(name,
                // binder)` — standard writeInterfaceToken header + the
                // service name + the HANDLE flat + the stability i32
                // (6-Z271x: finishUnflattenBinder reads it back).
                writer.write_i32(0); // strict-mode policy
                writer.write_i32(-1); // kUnsetWorkSource
                writer.write_u32(AIDL_HEADER_TAG_SYST);
                writer.write_string16("android.os.IServiceCallback");
                writer.write_string16(name);
                writer.write_flat_binder(&FlatBinderObject {
                    r#type: BINDER_TYPE_HANDLE,
                    flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                    binder: handle as u64,
                    cookie: 0,
                });
                writer.write_i32(STABILITY_ANNOTATION_VINTF);
            }
            let (data, offsets) = writer.into_parts();
            let tx = IncomingTx {
                // The proxy itself is the "sender" (kernel semantics: the
                // context manager initiated this callback).
                requester: PROXY_CONN_ID,
                // One-way: txn_id 0 = no inflight/reply bookkeeping (both
                // callback interfaces are declared oneway).
                txn_id: 0,
                code: 1, // onRegistration
                flags: TF_ONE_WAY,
                one_way: true,
                sender_pid: 0,
                sender_euid: 0,
                blob: Some(RequestBlob { data, offsets }),
                ptr: w.ptr,
                cookie: w.cookie,
            };
            if !self.queue_transaction(tx, w.conn) {
                warning!(
                    "[KR64][binder][svc] 6-Z276: onRegistration({}) callback dropped (conn={} mailbox full/gone)",
                    name, w.conn
                );
            } else {
                info!(
                    "[KR64][binder][svc] 6-Z276: onRegistration({}) queued for conn={} ({} dialect)",
                    name,
                    w.conn,
                    if w.hidl { "HIDL" } else { "AIDL" }
                );
            }
        }
    }

    /// Allocate the next connection id and create its mailbox.
    fn register_conn(&mut self) -> ConnId {
        let id = self.next_conn;
        self.next_conn += 1;
        self.conns.insert(id, ConnBox::default());
        id
    }

    /// Tear down a connection: unregister owned services, resolve its
    /// undelivered incoming transactions as `DEAD`, and deliver
    /// `BR_DEAD_BINDER` to death watchers.
    fn unregister_conn(&mut self, conn: ConnId) {
        // Resolve waiters whose transaction was queued on this connection
        // (server died before the guest even saw the work) — AND the one
        // transaction this conn had DELIVERED but not yet answered with
        // BC_REPLY (kernel semantics: the dying thread's transaction stack
        // dies, the requester gets BR_DEAD_REPLY instead of hanging out
        // its full REPLY_TIMEOUT). The resolution now lands on the
        // REQUESTER's reply_queue — unless the requester IS the dying
        // conn (its mailbox is gone; nobody to tell).
        let (mut dead_txns, out_sync) = match self.conns.remove(&conn) {
            Some(bx) => {
                let mut v = bx.pending_in;
                if let Some(t) = bx.inflight_txn {
                    v.push(t);
                }
                (v, bx.out_sync)
            }
            None => (Vec::new(), Default::default()),
        };
        for txn_id in dead_txns.drain(..) {
            if let Some(requester) = self.waiters.remove(&txn_id) {
                if let Some(rb) = self.conns.get_mut(&requester) {
                    rb.reply_queue.push_back(DeferredReply::Failed);
                }
            }
        }
        // The dying conn's own outstanding sync calls: nobody is left to
        // receive a resolution — just drop their waiters.
        for (txn_id, _) in out_sync {
            self.waiters.remove(&txn_id);
        }
        let dead: Vec<(String, u32)> = self
            .services
            .iter()
            .filter(|(_, e)| e.owner == conn)
            .map(|(n, e)| (n.clone(), e.handle))
            .collect();
        for (name, handle) in &dead {
            self.services.remove(name);
            self.by_handle.remove(handle);
            let watchers: Vec<(ConnId, u64)> = self
                .conns
                .iter()
                .filter_map(|(id, b)| b.death_watch.get(handle).map(|c| (*id, *c)))
                .collect();
            for (watcher, cookie) in watchers {
                if let Some(wb) = self.conns.get_mut(&watcher) {
                    wb.inbox.push_back(InboxItem::Death(cookie));
                }
            }
        }
        // 6-Z276: a dying connection's registerForNotifications watchers
        // are gone with it — no callback may target a dead conn's mailbox.
        self.remove_watchers_of_conn(conn);
    }

    /// Route a transaction to its owner's mailbox. Returns false when the
    /// owner is gone or its mailbox is full.
    fn queue_transaction(&mut self, tx: IncomingTx, owner: ConnId) -> bool {
        match self.conns.get_mut(&owner) {
            Some(b) if b.inbox.len() < MAX_QUEUED_ITEMS => {
                if tx.txn_id != 0 {
                    b.pending_in.push(tx.txn_id);
                }
                b.inbox.push_back(InboxItem::Tx(tx));
                true
            }
            _ => false,
        }
    }
}

/// How long a sync transaction waits for the server's `BC_REPLY` before
/// the proxy resolves it as `BR_FAILED_REPLY` (the kernel has no timeout,
/// but a hung server would otherwise hang the requester forever; 8 s
/// matches the class of client-side waits this wave is eliminating).
pub const REPLY_TIMEOUT: Duration = Duration::from_secs(8);

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

    // 6-Z151: ALL THREE binder contexts (/dev/binder, /dev/hwbinder,
    // /dev/vndbinder) must be exposed as symlinks to the single proxy
    // socket at {rootfs}/vm{id}/dev/binder. The single-socket design
    // (G5: see the loader's binder_open_fallback block comment in
    // twoyi_loader_shlib.c) routes all three contexts to the same
    // kr64 binder proxy — but until 6-Z151 only /dev/binder was
    // symlinked; /dev/hwbinder and /dev/vndbinder were MISSING from
    // the rootfs.
    //
    // ROOT CAUSE (run 32863013472, head e7a16e0 = 6-Z150): once Z150
    // cured the ComputeLastValidCap prctl spin, init finally reached
    // service-start. Its FIRST early service — wait_for_keymaster —
    // calls libhidlbase's defaultServiceManager(), which does
    // `access("/dev/hwbinder", F_OK)` BEFORE open(). The loader's
    // access() PLT hook (twoyi_loader_shlib.c line ~2178) calls
    // should_translate(), which logs "should_translate: /dev/hwbinder
    // -> YES (binder)" and returns 1, then translate() prepends the
    // rootfs, then real_access() issues faccessat on
    // {rootfs}/dev/hwbinder → ENOENT (the path didn't exist — no
    // symlink, no file). The guest's libhidlbase treats this as
    // "device absent" → defaultServiceManager() returns null →
    // `CHECK(serviceManager != nullptr) << "Could not retrieve
    // ServiceManager"` (Keymaster.cpp:125) → abort() → init's
    // InitFatalReboot handler (signal 6) → reboot loop ~every 90s.
    // BOOT_COMPLETED = 0.
    //
    // The fix mirrors what /dev/binder already does: create /dev/hwbinder
    // and /dev/vndbinder as relative symlinks to ../vm{id}/dev/binder.
    // access() now resolves the symlink target (the bound socket node
    // EXISTS) → returns 0 → libhidlbase proceeds to open() → the
    // openat PLT hook's real_openat on the symlink target returns ENXIO
    // (can't open() a bound Unix socket) → is_binder_device_path →
    // binder_open_fallback → binder_proxy_connect → CONNECTED to the
    // kr64 proxy. Both access() and open() are satisfied.
    let link_paths: [&str; 3] = ["/dev/binder", "/dev/hwbinder", "/dev/vndbinder"];

    // Make sure /vm{id}/dev and /dev exist.
    fs::create_dir_all(&vm_dev)?;
    fs::create_dir_all(format!("{}/dev", rootfs))?;

    #[cfg(unix)]
    {
        let _ = fs::set_permissions(&vm_dev, fs::Permissions::from_mode(0o755));
        let _ = fs::set_permissions(format!("{}/dev", rootfs), fs::Permissions::from_mode(0o755));
    }

    // Remove stale socket / symlinks from a previous run.
    match fs::remove_file(&sock_path) {
        Ok(()) => info!("[KR64][binder] removed stale socket: {}", sock_path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warning!("[KR64][binder] could not remove {}: {}", sock_path, e),
    }
    for link in link_paths {
        let link_path = format!("{}{}", rootfs, link);
        match fs::remove_file(&link_path) {
            Ok(()) => info!("[KR64][binder] removed stale symlink: {}", link_path),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => warning!("[KR64][binder] could not remove {}: {}", link_path, e),
        }
    }

    // NOTE: we deliberately do NOT bind the socket here. The only
    // caller chain is create_binder_device() -> BinderProxy::new()
    // (lib.rs:3385), and BinderProxy::new does its own unlink+bind+chmod
    // of this exact path. Binding here as well would be dead work whose
    // listener is immediately dropped and unlinked again. The symlinks
    // below therefore dangle for a few instructions until the proxy
    // binds — harmless, because the guest has not been exec'd yet.

    // Create the symlinks. Target is RELATIVE (`../vm{id}/dev/binder`)
    // so the kernel resolves it relative to the symlink's own location
    // — i.e. `{rootfs}/dev/` — which yields `{rootfs}/vm{id}/dev/binder`.
    // This works inside the chroot too (no leading `/`).
    #[cfg(unix)]
    {
        let target = format!("../vm{}/dev/binder", vm_id);
        for link in link_paths {
            let link_path = format!("{}{}", rootfs, link);
            std::os::unix::fs::symlink(&target, &link_path)?;
        }
    }

    info!(
        "[KR64][binder] prepared socket path {} and 3 symlinks {{/dev/binder, /dev/hwbinder, /dev/vndbinder}} -> ../vm{}/dev/binder (proxy binds next)",
        sock_path, vm_id
    );

    Ok(sock_path)
}

// ============================================================================
// Binder proxy — owns the listener, accepts guest connections,
// dispatches per-ioctl (one bounded thread per connection).
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
    /// 6-Z271 guest-local bus: service registry with owner routing, per-
    /// connection mailboxes, and reply-waiter channels. Replaces the
    /// 6-Z114 inert registry + the untested forward-to-host skeleton.
    bus: Arc<Mutex<BusState>>,
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
        // Read-modify-write: OR O_NONBLOCK into the existing flags instead
        // of clobbering them (F_SETFL replaces the whole status word).
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags >= 0 {
            let _ = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        }

        info!(
            "[KR64][binder][vm{}] proxy bound to {} (fd={}, non-blocking)",
            vm_id, socket_path, fd
        );

        Ok(BinderProxy {
            vm_id,
            listener: Some(listener),
            path: socket_path.to_string(),
            bus: Arc::new(Mutex::new(BusState::new())),
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
        let bus = Arc::clone(&self.bus);
        // Clone the shutdown Arc twice: one for the accept thread, one
        // for the returned handle. Both share the same AtomicBool.
        let shutdown_for_thread = Arc::clone(&self.shutdown);
        let shutdown_for_handle = Arc::clone(&self.shutdown);
        let vm_id = self.vm_id;
        let path = self.path.clone();

        let accept_thread = thread::Builder::new()
            .name(format!("kr64-binder-accept-{}", vm_id))
            .spawn(move || {
                // One thread PER CONNECTION, bounded by MAX_PROXY_CONNECTIONS.
                // (An earlier revision used a fixed BINDER_THREAD_POOL_SIZE
                // pool here — but handle_connection() serves a connection
                // for its entire lifetime with blocking reads, so a fixed
                // pool caps the number of SIMULTANEOUS guest binder clients
                // at 4; the moment a 5th guest process connected, its first
                // ioctl sat in the pool queue forever and the guest hung.)
                // Connection threads are detached: they exit when the guest
                // closes the socket (read_frame EOF) or the process dies —
                // the counter is what enforces the bound, not joining.
                let active = Arc::new(AtomicUsize::new(0));
                info!(
                    "[KR64][binder][vm{}] accept loop started (max_conns={})",
                    vm_id, MAX_PROXY_CONNECTIONS
                );

                while !shutdown_for_thread.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, _addr)) => {
                            // Bound check BEFORE spawning: a connection over
                            // the cap is dropped immediately (the guest sees
                            // EOF on its next ioctl and may retry later).
                            if active.load(Ordering::Acquire) >= MAX_PROXY_CONNECTIONS {
                                warning!(
                                    "[KR64][binder][vm{}] connection over cap ({}) dropped",
                                    vm_id,
                                    MAX_PROXY_CONNECTIONS
                                );
                                drop(stream);
                                std::thread::sleep(std::time::Duration::from_millis(25));
                                continue;
                            }
                            info!("[KR64][binder][vm{}] client connected", vm_id);
                            let bus = Arc::clone(&bus);
                            let active_conn = Arc::clone(&active);
                            active.fetch_add(1, Ordering::AcqRel);
                            let spawned = thread::Builder::new()
                                .name(format!("kr64-binder-conn-{}", vm_id))
                                .spawn(move || {
                                    let result = handle_connection(stream, vm_id, &bus);
                                    if let Err(e) = result {
                                        warning!(
                                            "[KR64][binder][vm{}] connection handler ended: {}",
                                            vm_id,
                                            e
                                        );
                                    }
                                    active_conn.fetch_sub(1, Ordering::AcqRel);
                                });
                            if spawned.is_err() {
                                // Spawn failed: undo the counter bump so the
                                // slot stays available to later connects.
                                active.fetch_sub(1, Ordering::AcqRel);
                                warning!("[KR64][binder][vm{}] conn-thread spawn failed", vm_id);
                                std::thread::sleep(std::time::Duration::from_millis(50));
                            }
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

/// Real peer credentials of an accepted unix-socket connection, via
/// SO_PEERCRED (kernel truth — no guest cooperation involved).
///
/// 6-Z271f: the 6-Z271e IDENT announcement turned out to be blind —
/// the tracer fakes every guest getpid() to 1 (load-bearing illusion),
/// so ALL FOUR conns in run 33431538542 announced `pid=1` and conn
/// ownership stayed unattributable exactly when it mattered (the futex
/// stall analysis). SO_PEERCRED reads the credentials the kernel stored
/// on the socket at connect time: the REAL host pid/uid/gid of the
/// connecting guest process (`use_namespaces=false` ⇒ host pid == guest
/// pid). This is what the real binder driver would stamp too. A local
/// repr(C) struct + raw constants keep this independent of the libc
/// crate's per-target feature surface (SO_PEERCRED=1 on Linux; `ucred`
/// is 3×u32 on every Linux ABI).
#[repr(C)]
struct Ucred {
    pid: i32,
    uid: u32,
    gid: u32,
}
const SO_PEERCRED_RAW: i32 = 1;

fn peer_credentials(stream: &UnixStream) -> (i32, u32, u32) {
    let mut cred = Ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = std::mem::size_of::<Ucred>() as u32;
    let rc = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            SO_PEERCRED_RAW,
            &mut cred as *mut Ucred as *mut libc::c_void,
            &mut len,
        )
    };
    if rc == 0 {
        (cred.pid, cred.uid, cred.gid)
    } else {
        (0, 0, 0)
    }
}

/// Handle one guest connection: read frames, dispatch, write responses.
/// Returns when the guest disconnects (EOF) or an unrecoverable I/O
/// error occurs.
///
/// 6-Z271: the connection registers on the per-VM bus (mailbox + identity)
/// and tears down on exit — owned services are unregistered, death
/// watchers get `BR_DEAD_BINDER`, and pending routed transactions fail.
fn handle_connection(
    mut stream: UnixStream,
    vm_id: u32,
    bus: &Arc<Mutex<BusState>>,
) -> io::Result<()> {
    let conn_id = {
        let mut b = bus.lock().expect("binder bus poisoned");
        b.register_conn()
    };
    info!(
        "[KR64][binder][vm{}] handling new connection (conn={})",
        vm_id, conn_id
    );
    // 6-Z271f: authoritative identity at accept time (see peer_credentials).
    // Stamped BEFORE the IDENT frame arrives so the very first routed
    // transaction already carries the real sender pid; the guest's
    // IDENT announcement (getpid-faked) stays a fallback + cross-check.
    let (peer_pid, peer_uid, peer_gid) = peer_credentials(&stream);
    if peer_pid != 0 {
        if let Ok(mut b) = bus.lock() {
            if let Some(box_) = b.conns.get_mut(&conn_id) {
                box_.sender_pid = peer_pid;
                box_.sender_euid = peer_uid;
            }
        }
    }
    info!(
        "[KR64][binder][vm{}] conn={} identity: SO_PEERCRED pid={} uid={} gid={}",
        vm_id, conn_id, peer_pid, peer_uid, peer_gid
    );
    let result = connection_loop(&mut stream, vm_id, bus, conn_id);
    bus.lock()
        .expect("binder bus poisoned")
        .unregister_conn(conn_id);
    result
}

/// The per-connection frame loop (split out so the bus teardown runs for
/// every exit path).
fn connection_loop(
    stream: &mut UnixStream,
    vm_id: u32,
    bus: &Arc<Mutex<BusState>>,
    conn_id: ConnId,
) -> io::Result<()> {
    // 6-Z271e: bounded per-frame DIAG — the first 12 WRITE_READ exchanges
    // per connection with their shape, plus every response. Run
    // 33430336853 left a guest binder thread blocked in recvfrom with no
    // proxy-side trace; this closes the observability gap.
    // 6-Z272k: 12 → 200 — the keystore2 compat chain (self-routed _NTF +
    // steal + nested BC_REPLY reentrancy) burns the 12-frame budget before
    // the deadlock window, leaving the exact stopping frame invisible.
    // 200/con × a handful of conns stays bounded (~2 KB).
    let mut wr_diag_budget: u32 = 200;
    loop {
        let req = match read_frame(stream) {
            Ok(r) => r,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                info!(
                    "[KR64][binder][vm{}] client disconnected (conn={})",
                    vm_id, conn_id
                );
                return Ok(());
            }
            Err(e) => return Err(e),
        };
        if req.cmd == BINDER_WRITE_READ && wr_diag_budget > 0 {
            wr_diag_budget -= 1;
            let (ws, rc) = if req.payload.len() >= 8 {
                (
                    u32::from_ne_bytes(req.payload[0..4].try_into().unwrap()),
                    u32::from_ne_bytes(req.payload[4..8].try_into().unwrap()),
                )
            } else {
                (0, 0)
            };
            info!(
                "[KR64][binder][vm{}] conn={} WRITE_READ ws={} rc={}",
                vm_id, conn_id, ws, rc
            );
        }
        let resp = dispatch_request(&req, vm_id, bus, conn_id);
        if req.cmd == BINDER_WRITE_READ && wr_diag_budget > 0 {
            let (rs, blobs) = if resp.payload.len() >= 4 {
                let rs = u32::from_ne_bytes(resp.payload[0..4].try_into().unwrap());
                let tail = resp.payload.len() - 4 - rs as usize;
                (rs, tail)
            } else {
                (0, resp.payload.len())
            };
            info!(
                "[KR64][binder][vm{}] conn={} -> ret={} read_size={} trailer={}B",
                vm_id, conn_id, resp.ret, rs, blobs
            );
        }
        write_frame(stream, &resp)?;
    }
}

// ============================================================================
// Ioctl dispatcher.
// ============================================================================

/// Dispatch one parsed request frame to the appropriate handler.
fn dispatch_request(req: &Frame, vm_id: u32, bus: &Arc<Mutex<BusState>>, conn_id: ConnId) -> Resp {
    match req.cmd {
        BINDER_VERSION => handle_version(vm_id),

        // 6-Z271 wire extension (twoyi_loader_shlib.c sends it right
        // after connect): [u32 pid][u32 uid][u32 gid]. The kernel stamps
        // real sender identities into transactions; the wire cannot, so
        // the guest announces them once per connection.
        //
        // 6-Z271f: DEMOTED to fallback + cross-check. The announced pid
        // comes from getpid(), which the tracer fakes to 1 — run
        // 33431538542 showed all conns announcing pid=1. The connection
        // is now stamped with kernel truth (SO_PEERCRED, see
        // handle_connection) BEFORE this frame arrives; the announced
        // values only fill the gap when SO_PEERCRED was unavailable,
        // and the two are logged side by side to catch disagreements.
        WIRE_CMD_IDENT => {
            let (pid, uid) = if req.payload.len() >= 8 {
                (
                    i32::from_ne_bytes(req.payload[0..4].try_into().unwrap()),
                    u32::from_ne_bytes(req.payload[4..8].try_into().unwrap()),
                )
            } else {
                (0, 0)
            };
            let mut stamped = false;
            if let Ok(mut b) = bus.lock() {
                if let Some(box_) = b.conns.get_mut(&conn_id) {
                    if box_.sender_pid == 0 && pid != 0 {
                        box_.sender_pid = pid;
                        box_.sender_euid = uid;
                        stamped = true;
                    }
                }
            }
            info!(
                "[KR64][binder][vm{}] conn={} IDENT announced pid={} uid={} (getpid-faked) — {}",
                vm_id,
                conn_id,
                pid,
                uid,
                if stamped {
                    "stamped (no SO_PEERCRED available)"
                } else {
                    "ignored — SO_PEERCRED already stamped real pid"
                }
            );
            Resp {
                ret: 0,
                payload: Vec::new(),
            }
        }

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

        BINDER_ENABLE_ONEWAY_SPAM_DETECTION => {
            // 6-Z265: the real kernel accepts this unconditionally (it
            // only arms an internal flood counter) — ACK with 0 so real
            // libbinder clients don't log/handle EINVAL on handshake.
            info!(
                "[KR64][binder][vm{}] ENABLE_ONEWAY_SPAM_DETECTION (acknowledged)",
                vm_id
            );
            Resp {
                ret: 0,
                payload: Vec::new(),
            }
        }

        BINDER_SET_CONTEXT_MGR | BINDER_SET_CONTEXT_MGR_KERNEL => {
            // Both spellings accepted: the kernel/bionic header spells it
            // `_IOW('b',7,__s32)` = 0x40046207 (BINDER_SET_CONTEXT_MGR_KERNEL);
            // the 6-Z113 loader puts the legacy `_IO('b',7)` = 0x6207 on the
            // wire (BINDER_SET_CONTEXT_MGR). The proxy is the servicemanager
            // for this VM, so either way we just ack — there is nothing for
            // the host's /dev/binder to do here.
            info!(
                "[KR64][binder][vm{}] SET_CONTEXT_MGR (0x{:08x}) — proxy is the servicemanager",
                vm_id, req.cmd
            );
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

        BINDER_WRITE_READ => handle_write_read(&req.payload, vm_id, bus, conn_id),

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
/// The wire payload is `[u32 write_size][u32 read_capacity][write_size
/// bytes]` (+ the v2 request trailer when the loader inlines parcel
/// blobs). We parse the write_buffer into individual BC_* commands,
/// dispatch each one, and build a read_buffer of BR_* commands to return.
///
/// 6-Z271 additions on top of the 6-Z114 shape:
/// * `BC_TRANSACTION` to a registered (guest or virtual) handle is routed
///   to the owning connection's mailbox; the caller gets
///   `BR_TRANSACTION_COMPLETE` from the same ioctl and the server's
///   `BC_REPLY` surfaces on the requester's LATER read (kernel semantics
///   — 6-Z271i; one-way likewise gets only `BR_TRANSACTION_COMPLETE`).
/// * `BC_REPLY` from a server connection is correlated to the delivered
///   transaction and pushed onto the requester's reply queue.
/// * Self-transactions (a process transacting on its own service — the
///   keystore2/km_compat chain) and nested transactions work: the same
///   connection pops its own `BR_TRANSACTION`, services it, and its
///   `BC_REPLY` resolves the original call.
/// * A read-only ioctl first resolves timed-out sync calls and drained
///   replies, then drains the connection's mailbox (incoming
///   transactions / death notifications) before falling back to the
///   250 ms `BR_NOOP` idle tick.
/// * `BC_REQUEST_DEATH_NOTIFICATION` / `BC_CLEAR_DEATH_NOTIFICATION` are
///   recorded; owner death pushes `BR_DEAD_BINDER`.
fn handle_write_read(
    payload: &[u8],
    vm_id: u32,
    bus: &Arc<Mutex<BusState>>,
    conn_id: ConnId,
) -> Resp {
    // Parse the v1 wire header: [u32 write_size][u32 read_capacity][write_size BC_* bytes].
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
            payload.len().saturating_sub(8)
        );
        return Resp {
            ret: -(libc::EINVAL),
            payload: Vec::new(),
        };
    }
    let write_buf = &payload[8..8 + write_size];

    // Parse the optional v2 trailer (6-Z114 §4.4):
    //   [u32 WIRE_V2_MAGIC][u32 blob_count]
    //   (blob_count ×) [u32 data_len][u32 offsets_len][data_len bytes][offsets_len bytes]
    // A request that ends exactly after the BC stream is v1 (z113 client —
    // byte-compatible, no parcel blobs). A v2 request inlines the parcel
    // bytes the proxy needs to actually parse BC_TRANSACTION data.
    let mut off = 8 + write_size;
    let mut req_blobs: Vec<RequestBlob> = Vec::new();
    let mut is_v2 = false;
    if off + 8 <= payload.len() {
        let magic = u32::from_ne_bytes(payload[off..off + 4].try_into().unwrap());
        if magic == WIRE_V2_MAGIC {
            is_v2 = true;
            off += 4;
            let count = u32::from_ne_bytes(payload[off..off + 4].try_into().unwrap()) as usize;
            off += 4;
            for _ in 0..count {
                if off + 8 > payload.len() {
                    break;
                }
                let data_len =
                    u32::from_ne_bytes(payload[off..off + 4].try_into().unwrap()) as usize;
                let offsets_len =
                    u32::from_ne_bytes(payload[off + 4..off + 8].try_into().unwrap()) as usize;
                off += 8;
                if off + data_len + offsets_len > payload.len() {
                    break;
                }
                let data = payload[off..off + data_len].to_vec();
                off += data_len;
                let offsets = payload[off..off + offsets_len].to_vec();
                off += offsets_len;
                req_blobs.push(RequestBlob { data, offsets });
            }
        }
    }

    // Walk the BC_* stream. The i-th v2 blob pairs with the i-th
    // BC_TRANSACTION/BC_REPLY/`*_SG` command in stream order.
    let mut read_buf: Vec<u8> = Vec::new();
    let mut resp_blobs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    let mut blob_idx = 0usize;
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
                write_buf.len().saturating_sub(consumed)
            );
            break;
        }
        let cmd_payload = &write_buf[consumed..consumed + psize];
        consumed += psize;

        match cmd {
            BC_TRANSACTION | BC_TRANSACTION_SG => {
                // Pull the next v2 blob as this transaction's parcel.
                let req_blob = if is_v2 && blob_idx < req_blobs.len() {
                    let b = &req_blobs[blob_idx];
                    blob_idx += 1;
                    Some(b.clone())
                } else {
                    None
                };
                let result = handle_transaction(cmd_payload, vm_id, bus, conn_id, req_blob);
                match result {
                    TransactionResult::Failed => {
                        push_br_failed_reply(&mut read_buf);
                    }
                    TransactionResult::CompleteOnly => {
                        // One-way, or a routed sync transaction (6-Z271i
                        // deferred reply): the kernel returns only
                        // BR_TRANSACTION_COMPLETE for the write half; the
                        // reply/failure surfaces on a later read.
                        push_br_transaction_complete(&mut read_buf);
                    }
                    TransactionResult::Reply { data, offsets } => {
                        // Kernel-true batch (6-Z114 §4.5): the client's
                        // `waitForResponse` consumes BR_TRANSACTION_COMPLETE
                        // then loops to read BR_REPLY.
                        push_br_transaction_complete(&mut read_buf);
                        push_br_reply(&mut read_buf, data.len() as u64, offsets.len() as u64);
                        resp_blobs.push((data, offsets));
                    }
                }
            }
            BC_REPLY | BC_REPLY_SG => {
                // 6-Z271: the guest is answering a transaction the bus
                // delivered to it. Correlate via the connection's inflight
                // txn and route the reply to the original requester.
                let reply_blob = if is_v2 && blob_idx < req_blobs.len() {
                    let b = &req_blobs[blob_idx];
                    blob_idx += 1;
                    Some(b.clone())
                } else {
                    None
                };
                let inflight = {
                    let mut b = bus.lock().expect("binder bus poisoned");
                    b.conns
                        .get_mut(&conn_id)
                        .and_then(|bx| bx.inflight_txn.take())
                };
                match inflight {
                    Some(txn_id) => {
                        let (data, offsets) = match reply_blob {
                            Some(rb) => (rb.data, rb.offsets),
                            None => (Vec::new(), Vec::new()),
                        };
                        // 6-Z271i: kernel-true deferred resolution — the
                        // requester is no longer blocked inside its ioctl;
                        // the reply lands on ITS reply_queue and its next
                        // BINDER_WRITE_READ returns [BR_REPLY].
                        let requester = {
                            let mut b = bus.lock().expect("binder bus poisoned");
                            let requester = b.waiters.remove(&txn_id);
                            if let Some(rc) = requester {
                                if let Some(rbx) = b.conns.get_mut(&rc) {
                                    rbx.out_sync.retain(|(t, _)| *t != txn_id);
                                }
                            }
                            requester
                        };
                        match requester {
                            Some(rc) => {
                                let mut b = bus.lock().expect("binder bus poisoned");
                                match b.conns.get_mut(&rc) {
                                    Some(rbx) => {
                                        rbx.reply_queue
                                            .push_back(DeferredReply::Reply { data, offsets });
                                    }
                                    None => {
                                        warning!(
                                            "[KR64][binder][vm{}] BC_REPLY for txn {} — requester conn {} gone",
                                            vm_id, txn_id, rc
                                        );
                                    }
                                }
                            }
                            None => {
                                // Requester timed out and left. Drop the reply.
                                warning!(
                                    "[KR64][binder][vm{}] BC_REPLY for txn {} — requester gone",
                                    vm_id,
                                    txn_id
                                );
                            }
                        }
                    }
                    None => {
                        warning!(
                            "[KR64][binder][vm{}] BC_REPLY with no delivered transaction (conn={}) — ignored",
                            vm_id, conn_id
                        );
                    }
                }
            }
            BC_ACQUIRE | BC_RELEASE | BC_INCREFS | BC_DECREFS => {
                // Strong/weak refcount changes on remote handles. The bus
                // keeps no refcounts (guest-owned nodes live as long as
                // their owning connection).
            }
            BC_ACQUIRE_DONE | BC_INCREFS_DONE => {
                // Acknowledgements of refcount operations on local binders.
            }
            BC_FREE_BUFFER | BC_DEAD_BINDER_DONE => {
                // Return a transaction-data buffer to the kernel, or
                // acknowledge a death notification. With v2 blobs the
                // client frees its own stash; v1 has no buffers to free.
                // Either way: no-op.
            }
            BC_ENTER_LOOPER | BC_REGISTER_LOOPER | BC_EXIT_LOOPER => {
                info!(
                    "[KR64][binder][vm{}] looper state change: 0x{:08x}",
                    vm_id, cmd
                );
            }
            BC_REQUEST_DEATH_NOTIFICATION => {
                // Payload: packed `binder_handle_cookie` (u32 handle + u64
                // cookie, 12 bytes).
                if cmd_payload.len() >= 12 {
                    let handle = u32::from_ne_bytes(cmd_payload[0..4].try_into().unwrap());
                    let cookie = u64::from_ne_bytes(cmd_payload[4..12].try_into().unwrap());
                    let mut b = bus.lock().expect("binder bus poisoned");
                    if let Some(bx) = b.conns.get_mut(&conn_id) {
                        bx.death_watch.insert(handle, cookie);
                    }
                }
            }
            BC_CLEAR_DEATH_NOTIFICATION => {
                if cmd_payload.len() >= 12 {
                    let handle = u32::from_ne_bytes(cmd_payload[0..4].try_into().unwrap());
                    let mut b = bus.lock().expect("binder bus poisoned");
                    if let Some(bx) = b.conns.get_mut(&conn_id) {
                        bx.death_watch.remove(&handle);
                    }
                }
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

    // Read half. A guest that offered read capacity gets, in order of
    // preference: a resolved reply for one of its earlier sync calls, an
    // incoming transaction, a queued death notification, or (after the
    // idle tick) BR_NOOP — mirroring the kernel's blocking `read_buffer`.
    if read_buf.is_empty() && read_capacity > 0 {
        // 6-Z271i: resolve sync calls that exceeded the bounded reply
        // budget — kernel has no timeout, but a hung server would
        // otherwise wedge the requester forever (the budget this wave is
        // eliminating). The failure is delivered as the requester's own
        // [BR_FAILED_REPLY] on THIS ioctl; a late BC_REPLY finds no
        // waiter and is dropped with a warning.
        {
            let mut b = bus.lock().expect("binder bus poisoned");
            let now = std::time::Instant::now();
            let expired: Vec<u64> = match b.conns.get_mut(&conn_id) {
                Some(bx) => {
                    let mut v = Vec::new();
                    while let Some((_t, at)) = bx.out_sync.front() {
                        if now.duration_since(*at) < REPLY_TIMEOUT {
                            break;
                        }
                        let (t, _) = bx.out_sync.pop_front().expect("front checked");
                        v.push(t);
                    }
                    v
                }
                None => Vec::new(),
            };
            for t in expired {
                b.waiters.remove(&t);
                if let Some(bx) = b.conns.get_mut(&conn_id) {
                    bx.reply_queue.push_back(DeferredReply::Failed);
                }
            }
        }
        // Deliver a resolved reply (BR_REPLY) if one is waiting. This is
        // what completes a routed sync transaction: the reply was pushed
        // here by the server's BC_REPLY — possibly by the SAME connection
        // servicing its own request (self-transaction). When a reply was
        // delivered the mailbox walk below is skipped: the guest's
        // waitForResponse consumes ONE reply per ioctl (kernel order —
        // thread todo before proc todo).
        let mut reply_delivered = false;
        if let Some(dr) = {
            let mut b = bus.lock().expect("binder bus poisoned");
            b.conns
                .get_mut(&conn_id)
                .and_then(|bx| bx.reply_queue.pop_front())
        } {
            match dr {
                DeferredReply::Reply { data, offsets } => {
                    push_br_reply(&mut read_buf, data.len() as u64, offsets.len() as u64);
                    resp_blobs.push((data, offsets));
                }
                DeferredReply::Failed => {
                    push_br_failed_reply(&mut read_buf);
                }
            }
            reply_delivered = true;
        }
        if !reply_delivered {
            enum Delivery {
                None,
                Tx(IncomingTx),
                Death(u64),
            }
            let mut delivery = {
                let mut b = bus.lock().expect("binder bus poisoned");
                match b.conns.get_mut(&conn_id) {
                    Some(bx) => match bx.inbox.pop_front() {
                        Some(InboxItem::Tx(tx)) => {
                            // Mark the delivered transaction as inflight so the
                            // guest's BC_REPLY correlates (sync only — one-way
                            // has txn_id 0 and expects no reply). Remove it
                            // from pending_in: it's no longer "queued".
                            if tx.txn_id != 0 {
                                bx.inflight_txn = Some(tx.txn_id);
                                bx.pending_in.retain(|id| *id != tx.txn_id);
                            }
                            Delivery::Tx(tx)
                        }
                        Some(InboxItem::Death(cookie)) => Delivery::Death(cookie),
                        None => Delivery::None,
                    },
                    None => Delivery::None,
                }
            };
            // 6-Z271g: PROCESS-POOL WORK STEALING — real binder queues incoming
            // transactions on the PROCESS's todo list and any ready pool
            // thread pops the next item, not only the thread that registered
            // the node. With per-thread proxy conns (the shlib now opens one
            // conn per guest thread) the REGISTERING conn may be busy — mid
            // outgoing call, its next WRITE_READ parked in the proxy's reply
            // wait — while a sibling thread idles. A parked sibling conn of
            // the same guest PROCESS (same real sender_pid, thanks to
            // SO_PEERCRED/procfs IDENT) steals the queued node work here.
            // Death notifications are NOT stealable: they belong to the conn
            // that requested them (handle+cookie watcher pairs).
            if matches!(delivery, Delivery::None) {
                let stolen = {
                    let mut b = bus.lock().expect("binder bus poisoned");
                    let my_pid = b.conns.get(&conn_id).map(|bx| bx.sender_pid).unwrap_or(0);
                    if my_pid == 0 {
                        None
                    } else {
                        let mut sibs: Vec<ConnId> = b
                            .conns
                            .iter()
                            .filter(|(cid, bx)| **cid != conn_id && bx.sender_pid == my_pid)
                            .map(|(cid, _)| *cid)
                            .collect();
                        sibs.sort();
                        let mut item: Option<IncomingTx> = None;
                        for sib in sibs {
                            let has_tx = matches!(
                                b.conns.get(&sib).and_then(|sbx| sbx.inbox.front()),
                                Some(InboxItem::Tx(_))
                            );
                            if !has_tx {
                                continue;
                            }
                            let sbx = b.conns.get_mut(&sib).expect("sibling vanished");
                            match sbx.inbox.pop_front() {
                                Some(InboxItem::Tx(tx)) => {
                                    if tx.txn_id != 0 {
                                        sbx.pending_in.retain(|id| *id != tx.txn_id);
                                    }
                                    item = Some(tx);
                                    break;
                                }
                                _ => continue,
                            }
                        }
                        if let Some(tx) = &item {
                            if tx.txn_id != 0 {
                                if let Some(bx) = b.conns.get_mut(&conn_id) {
                                    bx.inflight_txn = Some(tx.txn_id);
                                    bx.pending_in.push(tx.txn_id);
                                }
                            }
                        }
                        item
                    }
                };
                if let Some(tx) = stolen {
                    info!(
                    "[KR64][binder][vm{}] process-pool steal: conn={} takes tx #{} queued for a sibling (code={})",
                    vm_id, conn_id, tx.txn_id, tx.code
                );
                    delivery = Delivery::Tx(tx);
                }
            }
            match delivery {
                Delivery::Tx(tx) => {
                    let (ds, os) = match &tx.blob {
                        Some(b) => (b.data.len() as u64, b.offsets.len() as u64),
                        None => (0, 0),
                    };
                    push_br_transaction(
                        &mut read_buf,
                        tx.code,
                        tx.flags,
                        tx.sender_pid,
                        tx.sender_euid,
                        tx.ptr,
                        tx.cookie,
                        ds,
                        os,
                    );
                    if let Some(blob) = tx.blob {
                        resp_blobs.push((blob.data, blob.offsets));
                    }
                    info!(
                    "[KR64][binder][vm{}] delivered transaction conn={} <- conn={} code={} oneway={} (tx #{})",
                    vm_id, conn_id, tx.requester, tx.code, tx.one_way, tx.txn_id
                );
                }
                Delivery::Death(cookie) => {
                    push_br_dead_binder(&mut read_buf, cookie);
                }
                Delivery::None => {
                    // 6-Z152: BLOCKING idle — sleep before BR_NOOP so a
                    // guest poll loop can't pin the tracer (see the 6-Z268
                    // analysis in the original comment history).
                    std::thread::sleep(IDLE_POLL_TICK);
                    push_br_noop(&mut read_buf);
                }
            }
        }
    }

    // Build the wire response: [u32 read_size][read_size BR_* bytes] plus
    // the trailer when the request was v2 or the stream produced blobs
    // (6-Z265 — v1 real-libbinder clients dereference tr.data_ptr).
    let mut resp_payload = Vec::with_capacity(4 + read_buf.len() + 8);
    resp_payload.extend_from_slice(&(read_buf.len() as u32).to_ne_bytes());
    resp_payload.extend_from_slice(&read_buf);
    if is_v2 || !resp_blobs.is_empty() {
        resp_payload.extend_from_slice(&WIRE_V2_MAGIC.to_ne_bytes());
        resp_payload.extend_from_slice(&(resp_blobs.len() as u32).to_ne_bytes());
        for (data, offsets) in &resp_blobs {
            resp_payload.extend_from_slice(&(data.len() as u32).to_ne_bytes());
            resp_payload.extend_from_slice(&(offsets.len() as u32).to_ne_bytes());
            resp_payload.extend_from_slice(data);
            resp_payload.extend_from_slice(offsets);
        }
    }

    Resp {
        ret: 0,
        payload: resp_payload,
    }
}

// ============================================================================
// Transaction dispatch — servicemanager / routed bus / virtual services.
// ============================================================================

/// Result of handling a `BC_TRANSACTION`. Every handler path MUST push
/// either a `BR_REPLY` (with its reply parcel bytes) or a `BR_FAILED_REPLY`
/// into the read buffer so the guest's `BINDER_WRITE_READ` loop terminates
/// (a previous `Noop` variant livelocked the guest on `BR_NOOP` forever —
/// see 6-Z114 §5.1 / the comment on `servicemanager_proxy`).
enum TransactionResult {
    /// Push `[BR_FAILED_REPLY]` (no payload). Used when the transaction
    /// parcel is malformed or the target handle is invalid.
    Failed,
    /// Push `[BR_TRANSACTION_COMPLETE][BR_REPLY][binder_transaction_data]`
    /// with `tr.data_size = data.len()` / `tr.offsets_size = offsets.len()`.
    /// The `data`/`offsets` bytes ride the response trailer (both v2 and
    /// v1 real-libbinder requests — the hook backs tr.data_ptr with them).
    /// Used only by the IN-IOCTL handlers (servicemanager, virtual
    /// services, PING) — routed guest-owned transactions are deferred.
    Reply { data: Vec<u8>, offsets: Vec<u8> },
    /// Transaction accepted with no in-ioctl reply: one-way, or a routed
    /// sync call whose `BC_REPLY` resolves on the requester's LATER read
    /// (kernel semantics — 6-Z271i deferred resolution).
    CompleteOnly,
}

/// Handle a `BC_TRANSACTION` (or `BC_TRANSACTION_SG`) command.
///
/// Dispatch order: `PING_TRANSACTION` (every binder answers), handle 0 →
/// [`servicemanager_proxy`], registered handle → in-proxy virtual service
/// handler or routed to the owning guest connection (kernel `BR_TRANSACTION`
/// delivery + `BC_REPLY` correlation), anything else → `BR_FAILED_REPLY`
/// (the old forward-to-host skeleton never worked — untranslated handles
/// and raw guest pointers — and is retired).
fn handle_transaction(
    cmd_payload: &[u8],
    vm_id: u32,
    bus: &Arc<Mutex<BusState>>,
    conn_id: ConnId,
    req_blob: Option<RequestBlob>,
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

    // Parse the fields we care about (offsets per BinderTransactionData).
    let target_handle = u32::from_ne_bytes(cmd_payload[0..4].try_into().unwrap());
    let code = u32::from_ne_bytes(cmd_payload[16..20].try_into().unwrap());
    let flags = u32::from_ne_bytes(cmd_payload[20..24].try_into().unwrap());
    let one_way = flags & TF_ONE_WAY != 0;

    // PING_TRANSACTION (0x5F504E47 — IBinder::PING) is answered by every
    // binder object; the reply is the empty AIDL status (HIDL void
    // clients read nothing from it either). hwservicemanager pings the
    // context manager on startup — run 33411932921 showed our BR_FAILED_
    // REPLY answer before this wave.
    if code == PING_TRANSACTION {
        let mut w = ParcelWriter::new();
        w.write_status_ok();
        let (data, offsets) = w.into_parts();
        return TransactionResult::Reply { data, offsets };
    }

    // 6-Z272f: `IBinder::INTERFACE_TRANSACTION` (0x5f4e5446, "_NTF") —
    // the first transaction EVERY real client sends to a freshly-created
    // proxy (`BpBinder::getInterfaceDescriptor` — the AIDL
    // fromBinder/asInterface machinery reads the descriptor via
    // INTERFACE_TRANSACTION when it is not cached). The R12-lavender run
    // proved clients DO reach the services now — and got
    // EX_UNSUPPORTED_OPERATION from the virtual catch-all, so the
    // descriptor query failed, fromBinder failed, and keystore2's
    // connect_keymint panicked one level deeper ("Failed to create
    // service android.system.keystore2.IKeystoreService/default ←
    // connect_keymint ..."). Kernel semantics: `BBinder::onTransact`'s
    // default case answers with the BARE descriptor string16 (NO
    // exception header). Guest-owned services ROUTE as usual (their own
    // BBinder answers).
    if code == INTERFACE_TRANSACTION {
        if target_handle == SVC_MGR_HANDLE {
            let mut w = ParcelWriter::new();
            w.write_string16(SVC_MGR_IFACE_DESCRIPTOR);
            let (data, offsets) = w.into_parts();
            return TransactionResult::Reply { data, offsets };
        }
        let virtual_kind = {
            let b = bus.lock().expect("binder bus poisoned");
            b.by_handle
                .get(&target_handle)
                .and_then(|name| b.services.get(name))
                .and_then(|e| e.virtual_kind)
        };
        if let Some(kind) = virtual_kind {
            let mut w = ParcelWriter::new();
            w.write_string16(kind.descriptor());
            let (data, offsets) = w.into_parts();
            info!(
                "[KR64][binder][svc] INTERFACE_TRANSACTION → {} descriptor",
                kind.descriptor()
            );
            return TransactionResult::Reply { data, offsets };
        }
        // Guest-owned service: fall through to the routing below — the
        // owner's BBinder answers like any other transaction.
    }

    if target_handle == SVC_MGR_HANDLE {
        // 6-Z266: RATE-LIMITED — the user's lavender boot polled
        // checkService (code=2) every ~100 ms for a service that never
        // registered, which made this per-transaction INFO line a
        // 10-lines/sec-forever flood in the phone log pack. Keep the
        // first 4 lines per (vm, code) shape for evidence, then one
        // sampled line per 200th transaction carrying the running
        // count (the shape stays provably alive without the flood).
        static SVC_MGR_TX_SEEN: std::sync::OnceLock<
            std::sync::Mutex<std::collections::HashMap<(u32, u32), u64>>,
        > = std::sync::OnceLock::new();
        let seen = match SVC_MGR_TX_SEEN
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
            .lock()
        {
            Ok(mut m) => *m.entry((vm_id, code)).and_modify(|c| *c += 1).or_insert(1),
            Err(_) => 0,
        };
        if seen <= 4 || seen % 200 == 0 {
            info!(
                "[KR64][binder][vm{}] servicemanager transaction: code={} flags=0x{:02x} v2={} [tx #{}{}]",
                vm_id,
                code,
                flags,
                req_blob.is_some(),
                seen,
                if seen <= 4 { "" } else { " sampled" }
            );
        }
        return servicemanager_proxy(code, bus, req_blob.as_ref(), conn_id);
    }

    // Route to a registered service (guest-owned or in-proxy virtual).
    let route = {
        let b = bus.lock().expect("binder bus poisoned");
        b.by_handle.get(&target_handle).and_then(|name| {
            b.services
                .get(name)
                .map(|e| (e.owner, e.ptr, e.cookie, e.virtual_kind))
        })
    };
    let (owner, ptr, cookie, virtual_kind) = match route {
        Some(r) => r,
        None => {
            // Unknown handle — the kernel answers BR_FAILED_REPLY for a
            // transact to an invalid handle; so do we. (The retired
            // forward-to-host skeleton could never work: untranslated
            // handles + guest pointers → host EFAULT.)
            return TransactionResult::Failed;
        }
    };

    if let Some(kind) = virtual_kind {
        // 6-Z271n: bounded virtual-service transaction DIAG — this path
        // was COMPLETELY silent, so a guest spinning on getHardwareInfo /
        // the interface-version handshake was invisible in artifacts (the
        // run-33486586515 keystore2 stall). First 16 per (vm, kind).
        static VIRTUAL_TX_SEEN: std::sync::OnceLock<
            std::sync::Mutex<std::collections::HashMap<(u32, VirtualService), u64>>,
        > = std::sync::OnceLock::new();
        let seen = match VIRTUAL_TX_SEEN
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
            .lock()
        {
            Ok(mut m) => *m.entry((vm_id, kind)).and_modify(|c| *c += 1).or_insert(1),
            Err(_) => 0,
        };
        if seen <= 16 || seen % 500 == 0 {
            info!(
                "[KR64][binder][vm{}] virtual {:?} transaction conn={} code=0x{:x} [tx #{}{}]",
                vm_id,
                kind,
                conn_id,
                code,
                seen,
                if seen <= 16 { "" } else { " sampled" }
            );
        }
        return virtual_service_transaction(kind, code, req_blob.as_ref());
    }

    // Guest-owned service: the request parcel must be deliverable.
    let Some(blob) = req_blob else {
        warning!(
            "[KR64][binder][vm{}] transaction to handle 0x{:08x}: v1 request has no parcel bytes — failing (v2 loader required)",
            vm_id, target_handle
        );
        return TransactionResult::Failed;
    };

    // 6-Z271i: SELF-TRANSACTIONS ARE LEGAL (kernel semantics). Real binder
    // queues the request on the target node's process todo list even when
    // that process is the caller's own: the requesting ioctl completes
    // with BR_TRANSACTION_COMPLETE, the same thread (or a pool sibling —
    // 6-Z271g work stealing) pops the BR_TRANSACTION on its NEXT ioctl,
    // services it, and its BC_REPLY resolves the original call. This is
    // exactly what keystore2's in-process keymaster-compat chain does
    // (km_compat registers android.security.compat inside keystore2 and
    // the negotiation thread then transacts on it — run 33428365193).
    // The old hard-FAIL here deadlocked that class until the 8 s budget
    // burned.
    //
    // Stamp the sender identity (announced via WIRE_CMD_IDENT — kernel
    // would do this from the socket credentials).
    let (sender_pid, sender_euid) = {
        let b = bus.lock().expect("binder bus poisoned");
        b.conns
            .get(&conn_id)
            .map(|bx| (bx.sender_pid, bx.sender_euid))
            .unwrap_or((0, 0))
    };

    let txn_id;
    {
        let mut b = bus.lock().expect("binder bus poisoned");
        txn_id = b.next_txn;
        b.next_txn += 1;
        if !one_way {
            b.waiters.insert(txn_id, conn_id);
        }
        let queued = b.queue_transaction(
            IncomingTx {
                requester: conn_id,
                txn_id: if one_way { 0 } else { txn_id },
                code,
                flags,
                one_way,
                sender_pid,
                sender_euid,
                blob: Some(blob),
                ptr,
                cookie,
            },
            owner,
        );
        if !queued {
            if !one_way {
                b.waiters.remove(&txn_id);
            }
            drop(b);
            warning!(
                "[KR64][binder][vm{}] transaction to handle 0x{:08x}: owner mailbox full or gone",
                vm_id,
                target_handle
            );
            return TransactionResult::Failed;
        }
        if !one_way {
            // Track the outstanding sync call on the requester's conn for
            // the bounded reply timeout + teardown cleanup.
            if let Some(rbx) = b.conns.get_mut(&conn_id) {
                rbx.out_sync.push_back((txn_id, std::time::Instant::now()));
            }
        }
        // 6-Z271n: bounded routed-transaction DIAG — the queue side was
        // completely silent (only the delivery side logged), which made a
        // spinning requester invisible in artifacts. First 16 per (vm,
        // owner) prove the routing shape without flooding.
        static ROUTED_TX_SEEN: std::sync::OnceLock<
            std::sync::Mutex<std::collections::HashMap<(u32, ConnId), u64>>,
        > = std::sync::OnceLock::new();
        let seen = match ROUTED_TX_SEEN
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
            .lock()
        {
            Ok(mut m) => *m.entry((vm_id, owner)).and_modify(|c| *c += 1).or_insert(1),
            Err(_) => 0,
        };
        if seen <= 16 || seen % 500 == 0 {
            info!(
                "[KR64][binder][vm{}] routed transaction conn={} -> conn={} handle=0x{:08x} code={} oneway={} self={} [tx #{}{}]",
                vm_id,
                conn_id,
                owner,
                target_handle,
                code,
                one_way,
                owner == conn_id,
                seen,
                if seen <= 16 { "" } else { " sampled" }
            );
        }
    }

    // Both one-way and (now) sync transactions return
    // BR_TRANSACTION_COMPLETE from THIS ioctl; the sync reply surfaces on
    // a later read (kernel semantics — no blocking inside the proxy).
    TransactionResult::CompleteOnly
}

/// Intercept servicemanager transactions (target handle 0).
///
/// AIDL (libbinder, `/dev/binder` + `/dev/vndbinder`) request parcels begin
/// with the interface-token header `writeInterfaceToken(descriptor)`:
///
/// ```text
///   i32  strict_mode_policy
///   i32  work_source_uid      (kUnsetWorkSource = -1 unless propagated)
///   i32  header_tag           'SYST' (system) or 'VNDR' (vendor)
///   string16 descriptor       "android.os.IServiceManager"
///   …    per-code arguments
/// ```
///
/// HIDL (libhwbinder, `/dev/hwbinder`) writes NO header tag — the parcel is
/// `[i32 strict][i32 work][string16 descriptor]`. The proxy distinguishes
/// the two by peeking word 2: `SYST`/`VNDR` → AIDL, anything else → HIDL.
/// (Run 33411932921: the guest's real hwservicemanager spoke HIDL through
/// the proxy — its `IBase::PING` was the `code=1599098439` line.)
///
/// # v2 vs legacy v1 behaviour
///
/// With a v2 wire blob the proxy parses the real request parcel and
/// synthesises a real reply parcel — both ride the v2 trailer. With v1
/// (`req_blob = None`) the loader could not inline parcel bytes, so the
/// proxy answers the legacy synthetic shapes (GET → null binder, ADD →
/// status 0): the registry cannot work name-less, which is exactly the
/// 6-Z271 keystore2 20 s root cause.
fn servicemanager_proxy(
    code: u32,
    bus: &Arc<Mutex<BusState>>,
    req_blob: Option<&RequestBlob>,
    conn_id: ConnId,
) -> TransactionResult {
    // No v2 blob → legacy v1 path (names unknowable).
    let blob = match req_blob {
        Some(b) => b,
        None => return servicemanager_legacy(code),
    };
    let parcel = &blob.data;
    // Peek word 2 to pick the header shape (SYST / VNDR / RECO → AIDL;
    // anything else is a HIDL parcel — libhwbinder writes no tag).
    let is_aidl = parcel.len() >= 12 && {
        let tag = u32::from_ne_bytes(parcel[8..12].try_into().unwrap());
        is_aidl_header_tag(tag)
    };

    if !is_aidl {
        return servicemanager_hidl(code, parcel, bus, conn_id);
    }

    let mut reader = ParcelReader::new(parcel);
    // Consume the AIDL interface-token header.
    let (_strict, _work, tag, iface) = match reader.read_aidl_header() {
        Some(v) => v,
        None => {
            warning!("[KR64][binder][svc] malformed AIDL header (code={})", code);
            return TransactionResult::Failed;
        }
    };
    // 6-Z271c DIAG (bounded): the first SM parcels of a boot, hex-dumped —
    // this is the ground truth of what the guest's libbinder actually
    // writes (the RECO-tag discovery came from the source; this proves it
    // on-device).
    static SM_PARCEL_DUMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    if SM_PARCEL_DUMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 8 {
        let mut hex = String::new();
        for b in parcel.iter().take(24) {
            hex.push_str(&format!("{:02x} ", b));
        }
        info!(
            "[KR64][binder][svc] SM parcel head (code={}, {} bytes): {}",
            code,
            parcel.len(),
            hex
        );
    }
    // Lenient hygiene: log mismatches but don't reject — the proxy parses
    // leniently per 6-Z114 §3.2.
    if !is_aidl_header_tag(tag) {
        warning!(
            "[KR64][binder][svc] unexpected AIDL header tag 0x{:08x} (expected SYST/VNDR/RECO)",
            tag
        );
    }
    if let Some(ref iface_str) = iface {
        if iface_str != SVC_MGR_IFACE_DESCRIPTOR {
            warning!(
                "[KR64][binder][svc] unexpected AIDL descriptor {:?} (expected {:?})",
                iface_str,
                SVC_MGR_IFACE_DESCRIPTOR
            );
        }
    }

    let mut writer = ParcelWriter::new();
    // Reply prefix: AIDL Status::ok() = EX_NONE (i32 0).
    writer.write_status_ok();

    match code {
        SVC_MGR_GET_SERVICE | SVC_MGR_CHECK_SERVICE => {
            // Arg: string16 name. Reply hit: BINDER_TYPE_HANDLE with proxy
            // handle in low 32 bits. Reply miss: AIDL null binder =
            // BINDER_TYPE_BINDER with cookie 0 (the client's
            // `readStrongBinder` sees that as nullptr — 6-Z114 §3.3).
            let name = match reader.read_string16() {
                Some(Some(s)) => s,
                _ => String::new(),
            };
            let mut b = bus.lock().expect("binder bus poisoned");
            // 6-Z272e: advance the per-connection annotation format BEFORE
            // building the reply. A same-service re-get inside the window
            // means the client's waitForService retry loop is re-asking
            // for a service whose reply it FAILED to parse — flip to the
            // plain form (sticky). A different name (or no retry) keeps
            // the current format, so a working A12 client never breaks.
            let (ann_hit, ann_null) = {
                let now = std::time::Instant::now();
                match b.conns.get_mut(&conn_id) {
                    Some(bx) => {
                        if !bx.sm_annotate_plain {
                            if let Some((last, ts)) = &bx.last_sm_get {
                                if *last == name
                                    && now.duration_since(*ts) < std::time::Duration::from_secs(2)
                                {
                                    bx.sm_annotate_plain = true;
                                    info!(
                                        "[KR64][binder][svc] 6-Z272e: conn{} stability annotation → plain level (same-name retry observed)",
                                        conn_id
                                    );
                                }
                            }
                        }
                        bx.last_sm_get = Some((name.clone(), now));
                        if bx.sm_annotate_plain {
                            (STABILITY_ANNOTATION_VINTF, STABILITY_ANNOTATION_NULL)
                        } else {
                            (
                                STABILITY_ANNOTATION_VINTF_A12,
                                STABILITY_ANNOTATION_NULL_A12,
                            )
                        }
                    }
                    None => (STABILITY_ANNOTATION_VINTF, STABILITY_ANNOTATION_NULL),
                }
            };
            match b.services.get(&name).map(|e| e.handle) {
                Some(handle) => {
                    let obj = FlatBinderObject {
                        r#type: BINDER_TYPE_HANDLE,
                        flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                        binder: handle as u64,
                        cookie: 0,
                    };
                    writer.write_flat_binder(&obj);
                    // 6-Z271x: android-12+ stability annotation follows the
                    // flat; format per 6-Z272e.
                    writer.write_i32(ann_hit);
                    info!(
                        "[KR64][binder][svc] getService({}) hit → handle 0x{:08x}",
                        name, handle
                    );
                }
                None => {
                    let obj = FlatBinderObject {
                        r#type: BINDER_TYPE_BINDER,
                        flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                        binder: 0,
                        cookie: 0,
                    };
                    writer.write_flat_binder(&obj);
                    // 6-Z271x: real flattenBinder(nullptr) annotates the
                    // null flat; the client's finishUnflattenBinder still
                    // reads the i32. Format per 6-Z272e.
                    writer.write_i32(ann_null);
                    info!(
                        "[KR64][binder][svc] getService({}) miss → null binder",
                        name
                    );
                }
            }
        }
        SVC_MGR_ADD_SERVICE => {
            // Arg: string16 name + flat_binder_object (strong) + i32
            // allowIsolated + i32 dumpPriority. 6-Z271: the owner
            // connection and its local ptr/cookie ARE stored now —
            // transactions to the returned handle are delivered to the
            // owner as BR_TRANSACTION (kernel semantics).
            let name = match reader.read_string16() {
                Some(Some(s)) => s,
                _ => String::new(),
            };
            let flat = reader.read_flat_binder();
            let _allow_isolated = reader.read_i32();
            let _dump_priority = reader.read_i32();
            let (ptr, cookie) = match &flat {
                Some(f) => (f.binder, f.cookie),
                None => (0, 0),
            };
            let mut b = bus.lock().expect("binder bus poisoned");
            let handle = b.add_guest_service(&name, conn_id, ptr, cookie);
            info!(
                "[KR64][binder][svc] addService({}) → handle 0x{:08x} (conn={}, ptr=0x{:x})",
                name, handle, conn_id, ptr
            );
            // 6-Z276: a registered watcher now gets its one-way
            // `onRegistration` callback (the real servicemanager fires
            // IServiceCallback.onRegistration on every later addService).
            b.fire_registration_callbacks(&name, handle, false);
            // Reply body: void (header only) per 6-Z114 §3.3.
        }
        SVC_MGR_LIST_SERVICES => {
            // Arg: i32 dumpPriority (ignored — we don't filter).
            let _ = reader.read_i32();
            let b = bus.lock().expect("binder bus poisoned");
            let names = b.services.keys().cloned().collect::<Vec<_>>();
            // Reply body: [i32 count][count × string16].
            writer.write_i32(names.len() as i32);
            for n in &names {
                writer.write_string16(n);
            }
            info!("[KR64][binder][svc] listServices → {} entries", names.len());
        }
        SVC_MGR_REGISTER_FOR_NOTIFICATIONS => {
            // 6-Z276: args = [string16 name][flat_binder callback][i32
            // stability]. The callback flat is the WATCHER's local
            // IServiceCallback (BINDER_TYPE_BINDER, ptr/cookie as the
            // guest wrote them) — a later addService(name) fires a one-way
            // `onRegistration` BR_TRANSACTION targeted at that local
            // object. If the service is ALREADY registered, the real SM
            // fires the callback immediately (preexisting=true) — mirror
            // that by queueing it right away.
            let name = match reader.read_string16() {
                Some(Some(s)) => s,
                _ => String::new(),
            };
            let flat = reader.read_flat_binder();
            if let Some(f) = flat {
                let w = ServiceWatcher {
                    conn: conn_id,
                    ptr: f.binder,
                    cookie: f.cookie,
                    hidl: false,
                };
                let already = {
                    let mut b = bus.lock().expect("binder bus poisoned");
                    let handle = b.services.get(&name).map(|e| e.handle);
                    match handle {
                        Some(h) => {
                            drop(b);
                            // Already registered: immediate preexisting callback.
                            let mut b2 = bus.lock().expect("binder bus poisoned");
                            b2.fire_registration_callbacks(&name, h, true);
                            true
                        }
                        None => {
                            b.add_watcher(&name, w);
                            false
                        }
                    }
                };
                info!(
                    "[KR64][binder][svc] 6-Z276: registerForNotifications({}) conn={} — {}",
                    name,
                    conn_id,
                    if already {
                        "already registered → immediate callback"
                    } else {
                        "watching"
                    }
                );
            }
        }
        SVC_MGR_UNREGISTER_FOR_NOTIFICATIONS => {
            // 6-Z276: args = [string16 name][flat_binder callback]. Drop
            // the watcher (match on conn + local ptr).
            let name = match reader.read_string16() {
                Some(Some(s)) => s,
                _ => String::new(),
            };
            let flat = reader.read_flat_binder();
            if let Some(f) = flat {
                let mut b = bus.lock().expect("binder bus poisoned");
                b.remove_watcher(&name, conn_id, f.binder);
                info!(
                    "[KR64][binder][svc] 6-Z276: unregisterForNotifications({}) conn={} — dropped",
                    name, conn_id
                );
            }
        }
        SVC_MGR_IS_DECLARED => {
            // Arg: string16 name. Reply body: i32 0|1.
            let name = match reader.read_string16() {
                Some(Some(s)) => s,
                _ => String::new(),
            };
            let b = bus.lock().expect("binder bus poisoned");
            let declared = b.services.contains_key(&name) as i32;
            writer.write_i32(declared);
        }
        _ => {
            warning!("[KR64][binder][svc] unhandled servicemanager code {}", code);
            return TransactionResult::Failed;
        }
    }

    let (data, offsets) = writer.into_parts();
    TransactionResult::Reply { data, offsets }
}

/// HIDL `android.hidl.manager.V1_0.IServiceManager` transactions (libhwbinder
/// parcels — no SYST header tag, hidl_string args).
///
/// Only the subset the guest actually uses is implemented:
/// * `get` (code 1): `get(fqName, name)` — registry lookup of
///   `"fqName/name"`; hit → flat handle, miss → null binder (HIDL reads the
///   object at offsets[0], so the AIDL status prefix is skipped naturally).
/// * `add` (code 2): register a HIDL service name with the caller as owner
///   (no HIDL HAL processes exist in the recovery guest today, but the
///   path keeps hwservicemanager-shaped traffic coherent).
/// * everything else → `BR_FAILED_REPLY` (unchanged from the pre-bus
///   behaviour, minus the header mangling).
fn servicemanager_hidl(
    code: u32,
    parcel: &[u8],
    bus: &Arc<Mutex<BusState>>,
    conn_id: ConnId,
) -> TransactionResult {
    let mut reader = ParcelReader::new(parcel);
    // HIDL header: [i32 strict][i32 work][string16 descriptor].
    let _strict = match reader.read_i32() {
        Some(v) => v,
        None => return TransactionResult::Failed,
    };
    let _work = match reader.read_i32() {
        Some(v) => v,
        None => return TransactionResult::Failed,
    };
    let _iface = reader.read_string16();

    let mut writer = ParcelWriter::new();
    // HIDL replies carry no AIDL exception prefix; the object (if any)
    // lands at offsets[0] and HIDL reads it there. The status prefix is
    // harmless for HIDL and correct for any libbinder-side reader.
    writer.write_status_ok();

    match code {
        HIDL_SM_GET => {
            let fq = match reader.read_hidl_string() {
                Some(s) => s,
                None => return TransactionResult::Failed,
            };
            let name = match reader.read_hidl_string() {
                Some(s) => s,
                None => return TransactionResult::Failed,
            };
            let key = format!("{}/{}", fq, name);
            let b = bus.lock().expect("binder bus poisoned");
            match b.services.get(&key).map(|e| e.handle) {
                Some(handle) => {
                    let obj = FlatBinderObject {
                        r#type: BINDER_TYPE_HANDLE,
                        flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                        binder: handle as u64,
                        cookie: 0,
                    };
                    writer.write_flat_binder(&obj);
                    info!(
                        "[KR64][binder][svc] HIDL get({}) hit → handle 0x{:08x}",
                        key, handle
                    );
                }
                None => {
                    let obj = FlatBinderObject {
                        r#type: BINDER_TYPE_BINDER,
                        flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                        binder: 0,
                        cookie: 0,
                    };
                    writer.write_flat_binder(&obj);
                    info!("[KR64][binder][svc] HIDL get({}) miss → null binder", key);
                }
            }
        }
        HIDL_SM_ADD => {
            let name = match reader.read_hidl_string() {
                Some(s) => s,
                None => return TransactionResult::Failed,
            };
            let flat = reader.read_flat_binder();
            let (ptr, cookie) = match &flat {
                Some(f) => (f.binder, f.cookie),
                None => (0, 0),
            };
            let handle = {
                let mut b = bus.lock().expect("binder bus poisoned");
                let h = b.add_guest_service(&name, conn_id, ptr, cookie);
                info!(
                    "[KR64][binder][svc] HIDL add({}) → handle 0x{:08x} (conn={})",
                    name, h, conn_id
                );
                // 6-Z276: fire the HIDL IServiceNotification.onRegistration
                // callbacks BEFORE releasing the bus lock (the fire helper
                // re-locks internally in the AIDL path — here we hold the
                // lock, so call the same method on the guard's target).
                b.fire_registration_callbacks(&name, h, false);
                h
            };
            // Reply: bool success = true.
            writer.write_i32(1);
            let _ = handle;
        }
        HIDL_SM_REGISTER_FOR_NOTIFICATIONS => {
            // 6-Z276: args = [hidl_string fqName][hidl_string name][flat
            // callback]. The HIDL registry key is "fqName/instance". The
            // flat is the watcher's local IServiceNotification object.
            let fq = match reader.read_hidl_string() {
                Some(s) => s,
                None => return TransactionResult::Failed,
            };
            let inst = match reader.read_hidl_string() {
                Some(s) => s,
                None => return TransactionResult::Failed,
            };
            let flat = reader.read_flat_binder();
            let key = format!("{}/{}", fq, inst);
            let registered = if let Some(f) = flat {
                let mut b = bus.lock().expect("binder bus poisoned");
                match b.services.get(&key).map(|e| e.handle) {
                    Some(h) => {
                        drop(b);
                        // Already registered: immediate preexisting callback
                        // (the real hwservicemanager behaviour).
                        let mut b2 = bus.lock().expect("binder bus poisoned");
                        b2.fire_registration_callbacks(&key, h, true);
                        true
                    }
                    None => {
                        b.add_watcher(
                            &key,
                            ServiceWatcher {
                                conn: conn_id,
                                ptr: f.binder,
                                cookie: f.cookie,
                                hidl: true,
                            },
                        );
                        false
                    }
                }
            } else {
                false
            };
            // Reply: bool registered = true (the registration itself took).
            writer.write_i32(1);
            info!(
                "[KR64][binder][svc] 6-Z276: HIDL registerForNotifications({}) conn={} — {}",
                key,
                conn_id,
                if registered {
                    "already registered → immediate callback"
                } else {
                    "watching"
                }
            );
        }
        HIDL_SM_UNREGISTER_FOR_NOTIFICATIONS => {
            let fq = match reader.read_hidl_string() {
                Some(s) => s,
                None => return TransactionResult::Failed,
            };
            let inst = match reader.read_hidl_string() {
                Some(s) => s,
                None => return TransactionResult::Failed,
            };
            let flat = reader.read_flat_binder();
            if let Some(f) = flat {
                let key = format!("{}/{}", fq, inst);
                let mut b = bus.lock().expect("binder bus poisoned");
                b.remove_watcher(&key, conn_id, f.binder);
                info!(
                    "[KR64][binder][svc] 6-Z276: HIDL unregisterForNotifications({}) conn={} — dropped",
                    key, conn_id
                );
            }
            // Reply: bool success = true.
            writer.write_i32(1);
        }
        _ => {
            return TransactionResult::Failed;
        }
    }

    let (data, offsets) = writer.into_parts();
    TransactionResult::Reply { data, offsets }
}

/// Legacy v1 path (no parcel blob): the loader could not inline the
/// guest's parcel bytes, so the proxy answers the *synthetic* shapes per
/// 6-Z114 §2.4 — GET/CHECK → AIDL null binder (status 0 + flat
/// `{BINDER_TYPE_BINDER, 0, 0, 0}`), ADD → status 0 (header only). The
/// registry cannot work name-less — this path exists to keep the ROM's
/// libbinder loops terminating until a v2-capable loader attaches
/// (6-Z271 inlined request blobs for ALL real-libbinder clients).
fn servicemanager_legacy(code: u32) -> TransactionResult {
    let mut writer = ParcelWriter::new();
    writer.write_status_ok(); // EX_NONE
    match code {
        SVC_MGR_GET_SERVICE | SVC_MGR_CHECK_SERVICE => {
            // AIDL null binder — the client's readStrongBinder sees
            // BINDER_TYPE_BINDER with cookie 0 → nullptr (6-Z114 §3.3).
            writer.write_flat_binder(&FlatBinderObject {
                r#type: BINDER_TYPE_BINDER,
                flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                binder: 0,
                cookie: 0,
            });
            // 6-Z271x: the client's finishUnflattenBinder reads the
            // stability i32 after the flat even for a null binder.
            writer.write_i32(STABILITY_ANNOTATION_NULL);
        }
        SVC_MGR_ADD_SERVICE
        | SVC_MGR_LIST_SERVICES
        | SVC_MGR_REGISTER_FOR_NOTIFICATIONS
        | SVC_MGR_IS_DECLARED => {
            // Header-only "accepted" reply (no payload after exception=0).
            // For LIST this is the wrong shape (should be [i32 0] for an
            // empty array) but the v1 client can't dereference the parcel
            // anyway — keeping the loop terminating is the best we can do.
        }
        _ => return TransactionResult::Failed,
    }
    let (data, offsets) = writer.into_parts();
    TransactionResult::Reply { data, offsets }
}

// ============================================================================
// 6-Z271: in-proxy virtual services — minimal, semantically-correct AIDL
// implementations. Reply parcels use REAL AIDL shapes (Status prefix +
// typed payloads); operations the container cannot satisfy return honest
// binder exceptions instead of fabricated data.
// ============================================================================

/// AIDL exception codes — VERIFIED against android-13
/// `frameworks/native/libs/binder/include/binder/Status.h` (the wire
/// shape libbinder C++/NDK and libbinder_rs all read):
/// EX_UNSUPPORTED_OPERATION = -7, EX_SERVICE_SPECIFIC = -8.
const EX_NONE: i32 = 0;
const EX_UNSUPPORTED_OPERATION: i32 = -7;
const EX_SERVICE_SPECIFIC: i32 = -8;

/// `android.hardware.security.keymint.ErrorCode.HARDWARE_TYPE_UNAVAILABLE`
/// (measured empirically in run 33411932921's host-side vold log: "service
/// specific error: -68").
const KM_ERROR_HARDWARE_TYPE_UNAVAILABLE: i32 = -68;

/// `android.hardware.security.keymint.SecurityLevel` (android-13.0.0_r1
/// SecurityLevel.aidl): SOFTWARE=0, TRUSTED_ENVIRONMENT=1, STRONGBOX=2,
/// KEYSTORE=100. Our virtual device claims TRUSTED_ENVIRONMENT because
/// keystore2 constructs the MANDATORY TEE level and would refuse the
/// software level before ever registering IKeystoreSecurity; honesty is
/// preserved at the operation level (key ops fail
/// KM_ERROR_HARDWARE_TYPE_UNAVAILABLE).
const SECURITY_LEVEL_TRUSTED_ENVIRONMENT: i32 = 1;

/// Reply with an AIDL exception (no payload).
///
/// 6-Z272h: the REAL Status wire (android-13.0.0_r1 Status.cpp
/// `writeToParcel`/`readFromParcel` — identical for the C++ and Rust
/// clients) is:
/// ```text
///   [i32 exception]
///   [string16 message]              — for every exception != EX_NONE
///   [i32 0 (remote stack trace)]    — for every exception != EX_NONE
///   [i32 service code]              — EX_SERVICE_SPECIFIC only
/// ```
/// The previous shape wrote the service code directly after the
/// exception word, so the client's `readString16` consumed the CODE as
/// the message length (negative → UNEXPECTED_NULL → the whole status
/// mangled). EX_NONE stays a bare 4-byte word.
/// NOTE: EX_TRANSACTION_FAILED must never be written as parcel content —
/// real `Status::writeToParcel` turns it into a transport error; use
/// `TransactionResult::Failed` (BR_FAILED_REPLY) for that class.
fn virtual_error_reply(exception: i32, service_code: i32) -> TransactionResult {
    let mut w = ParcelWriter::new();
    w.write_i32(exception);
    if exception != EX_NONE {
        w.write_string16(""); // empty error message (len 0 + NUL + pad = 8 B)
        w.write_i32(0); // empty remote stack trace header
        if exception == EX_SERVICE_SPECIFIC {
            w.write_i32(service_code);
        }
    }
    let (data, offsets) = w.into_parts();
    TransactionResult::Reply { data, offsets }
}

/// Dispatch a transaction to an in-proxy virtual service.
fn virtual_service_transaction(
    kind: VirtualService,
    code: u32,
    req_blob: Option<&RequestBlob>,
) -> TransactionResult {
    let parcel: &[u8] = req_blob.map(|b| b.data.as_slice()).unwrap_or(&[]);
    let mut reader = ParcelReader::new(parcel);
    // AIDL meta transactions (defensive): 0 / 0xFFFFFFFF → interface
    // version; 0xFFFFFFFE → interface hash. Codes ≤ FIRST_CALL-1 are not
    // used by any real interface method.
    match code {
        0 | 0xFFFF_FFFF => {
            let mut w = ParcelWriter::new();
            w.write_status_ok();
            w.write_i32(match kind {
                VirtualService::Vibrator => 3, // IVibrator V3 (Android 13)
                VirtualService::KeyMint => 3,  // IKeyMintDevice V3
                VirtualService::SharedSecret => 1,
                // IHealth V4 (Android 15; the V4 additions are
                // batteryHealthData / getBatteryHealthData — verified
                // from android-15.0.0_r1 IHealth.aidl; the interface
                // library ships as android.hardware.health-V4-ndk.so).
                VirtualService::Health => 4,
            });
            let (data, offsets) = w.into_parts();
            return TransactionResult::Reply { data, offsets };
        }
        0xFFFF_FFFE => {
            let mut w = ParcelWriter::new();
            w.write_status_ok();
            w.write_string16("ffffffff");
            let (data, offsets) = w.into_parts();
            return TransactionResult::Reply { data, offsets };
        }
        _ => {}
    }

    // 6-Z271z: REAL AIDL request parcels open with the interface-token
    // header `[i32 strict][i32 work][i32 tag][string16 descriptor]` —
    // method args start AFTER it. Before 6-Z271x no client ever reached
    // this dispatch (every getService reply parsed as null client-side),
    // so the header was never consumed and e.g. IVibrator.on(ms) would
    // have read the token's strict-mode word as the timeout (a real
    // client writes strict=0/-1 there → EX_UNSUPPORTED_OPERATION → the
    // vibration silently never fires). The empty legacy-v1 parcel (no
    // blob) reads None here and leaves position 0 — harmless. HIDL-
    // shaped parcels never reach virtual services (HIDL SM lookups miss
    // — the services are registered under AIDL names only, §5 deferral).
    let _ = reader.read_aidl_header();

    match kind {
        VirtualService::Vibrator => virtual_vibrator(code, &mut reader),
        VirtualService::KeyMint => virtual_keymint(code, &mut reader),
        VirtualService::SharedSecret => virtual_sharedsecret(code, &mut reader),
        VirtualService::Health => virtual_health(code, &mut reader),
    }
}

/// `android.hardware.health.IHealth/default` — method codes VERIFIED
/// against android-15.0.0_r1 `health/aidl/android/hardware/health/
/// IHealth.aidl` (declaration order, codes from FIRST_CALL_TRANSACTION=1):
///   1  registerCallback(IHealthInfoCallback) → void
///   2  unregisterCallback(IHealthInfoCallback) → void
///   3  update() → void
///   4  getChargeCounterUah → int (µAh)
///   5  getCurrentNowMicroamps → int (µA)
///   6  getCurrentAverageMicroamps → int (µA)
///   7  getCapacity → int (percent)
///   8  getEnergyCounterNwh → long (nWh)
///   9  getChargeStatus → BatteryStatus
///   10 getStorageInfo → StorageInfo[]
///   11 getDiskStats → DiskStats[]
///   12 getHealthInfo → HealthInfo
///   13 setChargingPolicy(BatteryChargingPolicy) → void
///   14 getChargingPolicy → BatteryChargingPolicy
///   15 getBatteryHealthData → BatteryHealthData
///
/// Every .aidl comment documents `EX_UNSUPPORTED_OPERATION` as the
/// response "if the file that stores this property does not exist" —
/// so a missing sysfs file maps to that exception (honest: a device
/// without that sensor reports the same). Value reads go through
/// [`crate::battery::read_guest_battery_values`] — the pinned sysfs
/// tree the sysfs-reader class sees, host-honest by construction.
fn virtual_health(code: u32, reader: &mut ParcelReader) -> TransactionResult {
    // Snapshot once per transaction: the refresh thread may rewrite the
    // files mid-transaction; a single coherent snapshot is what the
    // real HAL's own HealthInfo mutex gives clients.
    virtual_health_with_values(code, reader, &crate::battery::read_guest_battery_values())
}

/// The full IHealth dispatch, parameterised over the value snapshot so
/// tests can feed synthetic sysfs states without touching the
/// process-global battery directory (one shared test process).
fn virtual_health_with_values(
    code: u32,
    _reader: &mut ParcelReader,
    vals: &crate::battery::GuestBatteryValues,
) -> TransactionResult {
    use crate::battery::sysfs_status_to_aidl;

    let int_reply = |v: i32| {
        let mut w = ParcelWriter::new();
        w.write_status_ok();
        w.write_i32(v);
        let (data, offsets) = w.into_parts();
        TransactionResult::Reply { data, offsets }
    };

    match code {
        // register/unregisterCallback + update() → OK. We never push
        // health-info change events (no guest health HAL thread polls
        // sysfs in the proxy yet); recovery's callers re-poll every
        // IsBatteryOk/battery-header cycle anyway, and lineage's
        // BattMonitorThreadLoop polls sysfs directly, not via callbacks.
        1 | 2 | 3 => virtual_error_reply(EX_NONE, 0),
        4 => match vals.charge_counter_uah {
            Some(v) => int_reply(v),
            None => virtual_error_reply(EX_UNSUPPORTED_OPERATION, 0),
        },
        5 => match vals.current_now_ua {
            Some(v) => int_reply(v),
            None => virtual_error_reply(EX_UNSUPPORTED_OPERATION, 0),
        },
        6 => match vals.current_avg_ua {
            Some(v) => int_reply(v),
            None => virtual_error_reply(EX_UNSUPPORTED_OPERATION, 0),
        },
        // THE method IsBatteryOk gates on (sideload battery check).
        7 => match vals.capacity_pct {
            Some(v) => int_reply(v),
            None => virtual_error_reply(EX_UNSUPPORTED_OPERATION, 0),
        },
        8 => {
            // getEnergyCounterNwh → long. No energy-counter file is
            // materialised (real drivers rarely expose it) → UNSUPPORTED,
            // exactly what the .aidl documents.
            virtual_error_reply(EX_UNSUPPORTED_OPERATION, 0)
        }
        // THE method battery_utils.cpp checks FIRST (charging →
        // `+` in the header, sideload charger threshold).
        9 => match vals.status_str.as_deref().and_then(sysfs_status_to_aidl) {
            Some(v) => int_reply(v),
            None => virtual_error_reply(EX_UNSUPPORTED_OPERATION, 0),
        },
        // getStorageInfo / getDiskStats → EMPTY arrays (a recovery
        // environment has no usable storage statistics; empty is the
        // honest wire shape: [EX_NONE][i32 0]).
        10 | 11 => int_reply(0),
        // getHealthInfo → the full parcelable (field order VERIFIED
        // against android-15.0.0_r1 HealthInfo.aidl).
        12 => virtual_health_info(vals),
        // setChargingPolicy(in value) → OK, accepted and ignored (a
        // policy change is a power-user knob no recovery exercises;
        // a real HAL without long-life support returns OK too).
        13 => virtual_error_reply(EX_NONE, 0),
        14 => virtual_error_reply(EX_UNSUPPORTED_OPERATION, 0),
        15 => {
            // getBatteryHealthData → manufacturing/first-usage dates 0,
            // state-of-health 0 (documented: "must be 0 if batteryStatus
            // is UNKNOWN" — we don't know it), serial null, part status
            // UNSUPPORTED(0). All honest no-knowledge values.
            let mut w = ParcelWriter::new();
            w.write_status_ok();
            w.write_i64(0); // batteryManufacturingDateSeconds
            w.write_i64(0); // batteryFirstUsageSeconds
            w.write_i64(0); // batteryStateOfHealth
            w.write_nullable_string16_none(); // batterySerialNumber
            w.write_i32(0); // batteryPartStatus = UNSUPPORTED
            let (data, offsets) = w.into_parts();
            TransactionResult::Reply { data, offsets }
        }
        _ => virtual_error_reply(EX_UNSUPPORTED_OPERATION, 0),
    }
}

/// Build the `HealthInfo` AIDL reply parcel (android-15.0.0_r1
/// `HealthInfo.aidl` field order — enums are `int`-backed on the wire,
/// booleans are `int` 0/1, arrays are length-prefixed, the only string
/// is `batteryTechnology`):
///   1  boolean chargerAcOnline
///   2  boolean chargerUsbOnline
///   3  boolean chargerWirelessOnline
///   4  boolean chargerDockOnline
///   5  int    maxChargingCurrentMicroamps
///   6  int    maxChargingVoltageMicrovolts
///   7  BatteryStatus batteryStatus (int)
///   8  BatteryHealth batteryHealth (int)
///   9  boolean batteryPresent
///   10 int    batteryLevel
///   11 int    batteryVoltageMillivolts
///   12 int    batteryTemperatureTenthsCelsius
///   13 int    batteryCurrentMicroamps
///   14 int    batteryCycleCount
///   15 int    batteryFullChargeUah
///   16 int    batteryChargeCounterUah
///   17 String batteryTechnology
///   18 int    batteryCurrentAverageMicroamps
///   19 DiskStats[] diskStats
///   20 StorageInfo[] storageInfos
///   21 BatteryCapacityLevel batteryCapacityLevel (int; UNSUPPORTED=-1)
///   22 long   batteryChargeTimeToFullNowSeconds
///   23 int    batteryFullChargeDesignCapacityUah
///   24 BatteryChargingState chargingState (int; NORMAL=1 default)
///   25 BatteryChargingPolicy chargingPolicy (int; DEFAULT=1 default)
fn virtual_health_info(vals: &crate::battery::GuestBatteryValues) -> TransactionResult {
    use crate::battery::sysfs_status_to_aidl;
    let status = vals
        .status_str
        .as_deref()
        .and_then(sysfs_status_to_aidl)
        .unwrap_or(1); // BatteryStatus.UNKNOWN when the driver is silent
    let health = vals
        .health_str
        .as_deref()
        .and_then(crate::battery::sysfs_health_to_aidl)
        .unwrap_or(1); // BatteryHealth.UNKNOWN
    let charging = matches!(status, 2); // CHARGING
    let usb_online = if charging { 1 } else { 0 };
    let voltage_mv = vals.voltage_uv.unwrap_or(0) / 1000;
    let level = vals.capacity_pct.unwrap_or(0);

    let mut w = ParcelWriter::new();
    w.write_status_ok();
    w.write_i32(0); // chargerAcOnline (the host charges over USB — 6-Z271h)
    w.write_i32(usb_online); // chargerUsbOnline
    w.write_i32(0); // chargerWirelessOnline
    w.write_i32(0); // chargerDockOnline
    w.write_i32(0); // maxChargingCurrentMicroamps (unknown)
    w.write_i32(0); // maxChargingVoltageMicrovolts (unknown)
    w.write_i32(status); // batteryStatus
    w.write_i32(health); // batteryHealth
    w.write_i32(if vals.present { 1 } else { 0 }); // batteryPresent
    w.write_i32(level); // batteryLevel (0..100, clamped by the reader)
    w.write_i32(voltage_mv); // batteryVoltageMillivolts
    w.write_i32(vals.temp_decic.unwrap_or(0)); // batteryTemperatureTenthsCelsius
    w.write_i32(vals.current_now_ua.unwrap_or(0)); // batteryCurrentMicroamps
    w.write_i32(vals.cycle_count.unwrap_or(0)); // batteryCycleCount
    w.write_i32(0); // batteryFullChargeUah (unknown)
    w.write_i32(vals.charge_counter_uah.unwrap_or(0)); // batteryChargeCounterUah
    w.write_string16(vals.technology.as_deref().unwrap_or("")); // batteryTechnology
    w.write_i32(vals.current_avg_ua.unwrap_or(0)); // batteryCurrentAverageMicroamps
    w.write_i32(0); // diskStats: empty array
    w.write_i32(0); // storageInfos: empty array
    w.write_i32(-1); // batteryCapacityLevel = UNSUPPORTED: we report the raw
                     // percentage but no fuel-gauge classification (UNSAFE to guess —
                     // CRITICAL makes the framework schedule a shutdown).
    w.write_i64(0); // batteryChargeTimeToFullNowSeconds (unknown)
    w.write_i32(0); // batteryFullChargeDesignCapacityUah (unknown)
    w.write_i32(1); // chargingState = NORMAL
    w.write_i32(1); // chargingPolicy = DEFAULT
    let (data, offsets) = w.into_parts();
    TransactionResult::Reply { data, offsets }
}

/// `android.hardware.vibrator.IVibrator` — method codes VERIFIED against
/// android-13.0.0_r1 `IVibrator.aidl` (V1..V3 are append-only, so these
/// codes are stable across the T-base corpus):
///   1 getCapabilities → int
///   2 off() → void
///   3 on(int timeoutMs, IVibratorCallback? callback) → void
///   4 perform(Effect, EffectStrength, IVibratorCallback?) → int
///   5 getSupportedEffects → Effect[]
///   6 setAmplitude(float) → void
///   7 setExternalControl(boolean) → void
/// We advertise capabilities = 0, so well-behaved clients stick to
/// plain on(ms) / off(). Every on(ms) is FORWARDED to the host app for a
/// REAL vibration.
fn virtual_vibrator(code: u32, reader: &mut ParcelReader) -> TransactionResult {
    match code {
        1 => {
            // getCapabilities → 0 (no callbacks, no amplitude control).
            let mut w = ParcelWriter::new();
            w.write_status_ok();
            w.write_i32(0);
            let (data, offsets) = w.into_parts();
            TransactionResult::Reply { data, offsets }
        }
        2 => {
            // off() → cancel the host vibration.
            crate::hostbridge::notify_vibrator_off();
            virtual_error_reply(EX_NONE, 0)
        }
        3 => {
            // on(int timeoutMs, callback?) → forward to the host.
            let timeout_ms = reader.read_i32().unwrap_or(0);
            if timeout_ms <= 0 || timeout_ms > 60_000 {
                // Degenerate/nonsensical duration — refuse (a real HAL
                // would also reject a 0 or absurd timeout).
                return virtual_error_reply(EX_UNSUPPORTED_OPERATION, 0);
            }
            crate::hostbridge::notify_vibrate(timeout_ms);
            info!("[KR64][binder][svc] IVibrator.on({} ms) → host", timeout_ms);
            virtual_error_reply(EX_NONE, 0)
        }
        4 => {
            // perform(effect, strength, callback?) → unsupported (we
            // advertise no composite/effect support).
            virtual_error_reply(EX_UNSUPPORTED_OPERATION, 0)
        }
        5 => {
            // getSupportedEffects → empty int[].
            let mut w = ParcelWriter::new();
            w.write_status_ok();
            w.write_i32(0);
            let (data, offsets) = w.into_parts();
            TransactionResult::Reply { data, offsets }
        }
        6 | 7 => {
            // setAmplitude / setExternalControl → unsupported (caps = 0).
            virtual_error_reply(EX_UNSUPPORTED_OPERATION, 0)
        }
        _ => {
            // Upper methods (compose/pwle/alwaysOn/…) — unsupported.
            virtual_error_reply(EX_UNSUPPORTED_OPERATION, 0)
        }
    }
}

/// `android.hardware.security.keymint.IKeyMintDevice/default` — method
/// codes VERIFIED against android-13.0.0_r1 `IKeyMintDevice.aidl`
/// (declaration order):
///   1 getHardwareInfo, 2 addRngEntropy, 3 generateKey, 4 importKey,
///   5 importWrappedKey, 6 upgradeKey, 7 deleteKey, 8 deleteAllKeys,
///   9 destroyAttestationIds, 10 finish, 11 begin, 12 updateAad,
///   13 update, 14 abort, 15 deviceLocked, 16 earlyBootEnded,
///   17 convertStorageKeyToEphemeral, 18 getKeyCharacteristics,
///   19 getRootOfTrustChallenge, 20 getRootOfTrust, 21 sendRootOfTrust
///
/// The device reports itself as a SOFTWARE-level implementation so
/// keystore2 (a) obtains its backend HAL and registers IKeystoreSecurity —
/// collapsing the ~20 s recovery wait — and (b) gets honest errors for
/// key operations it cannot perform against a software device. TWRP's
/// existing unmountable-/data fallback handles those errors; it just
/// reaches them 20 s sooner.
fn virtual_keymint(code: u32, _reader: &mut ParcelReader) -> TransactionResult {
    match code {
        1 => {
            // getHardwareInfo → KeyMintHardwareInfo parcel. Field order
            // verified against android-13.0.0_r1 KeyMintHardwareInfo.aidl:
            //   int versionNumber; SecurityLevel securityLevel;
            //   @utf8InCpp String keyMintName; @utf8InCpp String keyMintAuthorName;
            //   boolean timestampTokenRequired;
            //
            // 6-Z272h: KeyMintHardwareInfo is a STRUCTURED parcelable —
            // the android-12+ wire carries a leading size i32 (see
            // `write_sized_parcelable`). Without it keystore2's
            // `sized_read` consumed versionNumber as the size and the
            // whole chain died with TRANSACTION_FAILED (run 33543923394
            // — the last stall before IKeystoreSecurity registration).
            //
            // securityLevel is TRUSTED_ENVIRONMENT (1): keystore2 builds
            // the MANDATORY TEE level (globals.rs new_native_binder —
            // "Trying to construct mandatory security level TEE") and
            // android-13 SecurityLevel = {SOFTWARE=0, TRUSTED_ENVIRONMENT=1,
            // STRONGBOX=2, KEYSTORE=100} — the previous -2 was not a valid
            // variant at all. The device stays honest where it counts:
            // key OPERATIONS fail with KM_ERROR_HARDWARE_TYPE_UNAVAILABLE.
            let mut w = ParcelWriter::new();
            w.write_status_ok();
            w.write_structured_parcelable(|w| {
                w.write_i32(300); // versionNumber: KeyMint V3 (Android 13)
                w.write_i32(SECURITY_LEVEL_TRUSTED_ENVIRONMENT);
                w.write_string16("TwoyiSoftwareKeyMint");
                w.write_string16("twoyi");
                w.write_i32(0); // timestampTokenRequired = false
            });
            let (data, offsets) = w.into_parts();
            TransactionResult::Reply { data, offsets }
        }
        2 => {
            // addRngEntropy(byte[] data) → accepted (a software
            // implementation mixes what it is given).
            virtual_error_reply(EX_NONE, 0)
        }
        3 | 4 | 5 | 6 => {
            // generateKey / importKey / importWrappedKey / upgradeKey →
            // honest failure: no hardware backend behind this device.
            virtual_error_reply(EX_SERVICE_SPECIFIC, KM_ERROR_HARDWARE_TYPE_UNAVAILABLE)
        }
        7 | 8 => {
            // deleteKey / deleteAllKeys → void ok (deleting a key that a
            // software device never stored is a no-op).
            virtual_error_reply(EX_NONE, 0)
        }
        9 => virtual_error_reply(EX_NONE, 0), // destroyAttestationIds → void ok
        10 | 11 | 12 | 13 => {
            // finish / begin / updateAad / update → cannot run operations.
            virtual_error_reply(EX_SERVICE_SPECIFIC, KM_ERROR_HARDWARE_TYPE_UNAVAILABLE)
        }
        14 => virtual_error_reply(EX_NONE, 0), // abort → void ok
        15 => virtual_error_reply(EX_NONE, 0), // deviceLocked → void ok
        16 => virtual_error_reply(EX_NONE, 0), // earlyBootEnded → void ok
        17 | 18 | 19 | 20 | 21 => {
            virtual_error_reply(EX_SERVICE_SPECIFIC, KM_ERROR_HARDWARE_TYPE_UNAVAILABLE)
        }
        _ => virtual_error_reply(EX_UNSUPPORTED_OPERATION, 0),
    }
}

/// `android.hardware.security.sharedsecret.ISharedSecret/default` —
/// method codes from android-13.0.0_r1 `ISharedSecret.aidl`:
///   1 getSharedSecretParameters → SharedSecretParameters { byte[] seed; byte[] nonce; }
///   2 computeSharedSecret(SharedSecretParameters[] params) → byte[]
/// A deterministic software implementation: the reply shape is what
/// keystore2's negotiation needs to terminate; the anti-compromise
/// property of the real protocol is meaningless for a software device.
fn virtual_sharedsecret(code: u32, _reader: &mut ParcelReader) -> TransactionResult {
    match code {
        1 => {
            // getSharedSecretParameters → SharedSecretParameters parcel
            // (android-13.0.0_r1 SharedSecretParameters.aidl:
            //   byte[] seed; byte[] nonce;
            // ). 6-Z272h: a STRUCTURED parcelable — same leading size
            // i32 as KeyMintHardwareInfo (see `write_sized_parcelable`).
            let mut w = ParcelWriter::new();
            w.write_status_ok();
            w.write_structured_parcelable(|w| {
                let seed: Vec<u8> = (0..32u32)
                    .map(|i| (i as u8).wrapping_mul(31).wrapping_add(7))
                    .collect();
                w.write_i32(seed.len() as i32);
                w.data.extend_from_slice(&seed);
                while w.data.len() % 4 != 0 {
                    w.data.push(0);
                }
                w.write_i32(0); // nonce: empty byte[]
                while w.data.len() % 4 != 0 {
                    w.data.push(0);
                }
            });
            let (data, offsets) = w.into_parts();
            TransactionResult::Reply { data, offsets }
        }
        2 => {
            // computeSharedSecret → 32 deterministic bytes (no seed
            // mixing — a fixed software secret; keystore2 only needs the
            // 32-byte shape and cross-boot stability, not secrecy).
            let mut w = ParcelWriter::new();
            w.write_status_ok();
            let out: Vec<u8> = (0..32u8)
                .map(|i| i.wrapping_mul(29).wrapping_add(11))
                .collect();
            w.write_i32(out.len() as i32);
            w.data.extend_from_slice(&out);
            while w.data.len() % 4 != 0 {
                w.data.push(0);
            }
            let (data, offsets) = w.into_parts();
            TransactionResult::Reply { data, offsets }
        }
        _ => virtual_error_reply(EX_UNSUPPORTED_OPERATION, 0),
    }
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
            format!(
                "read_frame: payload too large ({} > {})",
                arg_len, MAX_PAYLOAD
            ),
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

/// Push `[BR_TRANSACTION_COMPLETE]` (4 bytes, no payload). The client's
/// `IPCThreadState::waitForResponse` consumes this and keeps looping for
/// the actual `BR_REPLY` (6-Z114 §4.5 — sync reply may batch
/// `[BR_TRANSACTION_COMPLETE][BR_REPLY]`).
fn push_br_transaction_complete(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&BR_TRANSACTION_COMPLETE.to_ne_bytes());
}

/// Push `[BR_REPLY][binder_transaction_data]` with `tr.data_size` and
/// `tr.offsets_size` stamped from the reply parcel; `tr.data_ptr` and
/// `tr.offsets_ptr` stay 0 on the wire — the v2 client patches them from
/// the response trailer's blob index before copying the BR bytes into
/// its mIn (6-Z114 §4.4 / §4.5). The reply parcel bytes themselves ride
/// the v2 response trailer; they are NOT inlined here (the v1 client
/// can't dereference `tr.data_ptr = 0` anyway).
fn push_br_reply(buf: &mut Vec<u8>, data_size: u64, offsets_size: u64) {
    buf.extend_from_slice(&BR_REPLY.to_ne_bytes());
    let tx = BinderTransactionData {
        data_size,
        offsets_size,
        ..Default::default()
    };
    // Serialize the struct as native-endian bytes. The struct is
    // #[repr(C)] and we're on a little-endian platform (aarch64 / x86_64),
    // so a raw byte copy IS `to_ne_bytes`.
    let tx_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            &tx as *const BinderTransactionData as *const u8,
            std::mem::size_of::<BinderTransactionData>(),
        )
    };
    buf.extend_from_slice(tx_bytes);
}

/// 6-Z271: Push `[BR_TRANSACTION][binder_transaction_data]` for a
/// transaction the bus delivers to a server connection. The target union
/// uses the PTR form (the server's own local binder): `target.ptr =
/// (target_handle_field, target_pad_field)`, `cookie` — so the server's
/// `BBinder::onTransact` sees its own identity, kernel-style. Sender
/// pid/euid come from the requester's announced `WIRE_CMD_IDENT`. The
/// request parcel bytes ride the response trailer (blob pairing).
#[allow(clippy::too_many_arguments)]
fn push_br_transaction(
    buf: &mut Vec<u8>,
    code: u32,
    flags: u32,
    sender_pid: i32,
    sender_euid: u32,
    ptr: u64,
    cookie: u64,
    data_size: u64,
    offsets_size: u64,
) {
    buf.extend_from_slice(&BR_TRANSACTION.to_ne_bytes());
    let tx = BinderTransactionData {
        // The kernel's union: for the ptr form, the 8 bytes at offset 0
        // ARE the pointer (low word in `target_handle`, high in `pad`).
        target_handle: (ptr & 0xFFFF_FFFF) as u32,
        target_pad: (ptr >> 32) as u32,
        target_cookie: cookie,
        code,
        flags,
        sender_pid,
        sender_euid,
        // Parcel SIZES are stamped here; data_ptr/offsets_ptr stay 0 on
        // the wire — the v2 client patches them from the trailer blob.
        data_size,
        offsets_size,
        data_ptr: 0,
        offsets_ptr: 0,
    };
    let tx_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            &tx as *const BinderTransactionData as *const u8,
            std::mem::size_of::<BinderTransactionData>(),
        )
    };
    buf.extend_from_slice(tx_bytes);
}

/// 6-Z271: Push `[BR_DEAD_BINDER][u64 cookie]` — the kernel delivers this
/// when a binder node a connection holds (or watches) dies.
fn push_br_dead_binder(buf: &mut Vec<u8>, cookie: u64) {
    buf.extend_from_slice(&BR_DEAD_BINDER.to_ne_bytes());
    buf.extend_from_slice(&cookie.to_ne_bytes());
}

// ============================================================================
// BC_* payload-size extraction.
// ============================================================================

/// Extract the payload size of a BC_* / BR_* command from its ioctl
/// number. The ioctl number encodes the arg size in bits 16..29
/// (the `size` field of `_IOC(dir, type, nr, size)`).
///
/// For example, `BC_TRANSACTION` = `_IOW('c', 0, sizeof(binder_transaction_data))`
/// = `(1<<30) | (64<<16) | ('c'<<8) | 0` = 0x40406300, so
/// `bc_payload_size(BC_TRANSACTION)` returns 64.
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
    fn peer_credentials_reports_kernel_truth() {
        // 6-Z271f: the SO_PEERCRED stamp must report the REAL pid of the
        // peer — the whole point of the upgrade (the guest's own getpid
        // announcement is faked to 1 by the tracer). A socketpair has no
        // listen backlog involved; SO_PEERCRED on one end reports the
        // OTHER end's credentials, which here is this same test process.
        //
        // Environment tolerance: some hardened kernels (this dev
        // container's AlibabaCloud 5.10 among them) zero socket peer
        // credentials entirely — SO_PEERCRED and SCM_CREDENTIALS both
        // return zeros there. The proxy treats pid==0 as "no kernel
        // truth available" and lets the guest's /proc/self/status
        // announcement fill the gap, so the contract here is: pid is
        // EITHER 0 (kernel stripped creds) OR our real pid. It must
        // never be anything else.
        let (a, _b) = UnixStream::pair().expect("socketpair");
        let (pid, uid, gid) = peer_credentials(&a);
        assert!(
            pid == 0 || pid as u32 == std::process::id(),
            "peer pid must be 0 (creds stripped) or our own pid; got {}",
            pid
        );
        if pid != 0 {
            // Creds live: uid/gid must be populated too (all-zero pid
            // with nonzero creds would be a struct-packing bug).
            assert!(
                uid != 0 || gid != 0 || nix_ish_root(),
                "nonzero pid but all-zero uid/gid — ucred layout mismatch?"
            );
        }
    }

    fn nix_ish_root() -> bool {
        // std has no getuid; checking /proc/self/status is fine for a test.
        std::fs::read_to_string("/proc/self/status")
            .map(|s| {
                s.lines()
                    .any(|l| l.starts_with("Uid:") && l.split_whitespace().nth(1) == Some("0"))
            })
            .unwrap_or(false)
    }

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
        // Locked table per 6-Z114 PROTOCOL.md §1.1 — verified against
        // /usr/include/linux/android/binder.h (this build host) AND
        // bionic's android-11.0.0_r1 mirror of the same header
        // (the one the ROM's userspace was actually built against).
        // The split is the kernel's own and ABI-frozen: top-level ioctls
        // type 'b' (0x62), BC_* type 'c' (0x63), BR_* type 'r' (0x72).
        //
        // BC_* (each is `[u32 cmd][payload]` in the write_buffer):
        assert_eq!(
            BC_TRANSACTION, 0x40406300,
            "BC_TRANSACTION = _IOW('c',0,64)"
        );
        assert_eq!(BC_REPLY, 0x40406301, "BC_REPLY = _IOW('c',1,64)");
        assert_eq!(BC_FREE_BUFFER, 0x40086303, "BC_FREE_BUFFER = _IOW('c',3,8)");
        assert_eq!(BC_INCREFS, 0x40046304, "BC_INCREFS = _IOW('c',4,4)");
        assert_eq!(BC_ACQUIRE, 0x40046305, "BC_ACQUIRE = _IOW('c',5,4)");
        assert_eq!(BC_RELEASE, 0x40046306, "BC_RELEASE = _IOW('c',6,4)");
        assert_eq!(BC_DECREFS, 0x40046307, "BC_DECREFS = _IOW('c',7,4)");
        assert_eq!(
            BC_INCREFS_DONE, 0x40106308,
            "BC_INCREFS_DONE = _IOW('c',8,16)"
        );
        assert_eq!(
            BC_ACQUIRE_DONE, 0x40106309,
            "BC_ACQUIRE_DONE = _IOW('c',9,16)"
        );
        assert_eq!(
            BC_REGISTER_LOOPER, 0x0000630B,
            "BC_REGISTER_LOOPER = _IO('c',11)"
        );
        assert_eq!(BC_ENTER_LOOPER, 0x0000630C, "BC_ENTER_LOOPER = _IO('c',12)");
        assert_eq!(BC_EXIT_LOOPER, 0x0000630D, "BC_EXIT_LOOPER = _IO('c',13)");
        assert_eq!(
            BC_REQUEST_DEATH_NOTIFICATION, 0x400C630E,
            "BC_REQUEST_DEATH_NOTIFICATION = _IOW('c',14,12)"
        );
        assert_eq!(
            BC_CLEAR_DEATH_NOTIFICATION, 0x400C630F,
            "BC_CLEAR_DEATH_NOTIFICATION = _IOW('c',15,12)"
        );
        assert_eq!(
            BC_DEAD_BINDER_DONE, 0x40086310,
            "BC_DEAD_BINDER_DONE = _IOW('c',16,8)"
        );
        // BC_TRANSACTION_SG / BC_REPLY_SG use struct binder_transaction_data_sg
        // (64-byte binder_transaction_data + 8-byte buffers_size = 72 bytes),
        // so the _IOW size field is 0x48, not 0x40. The struct size MUST
        // match the kernel's or the guest's libbinder.so (which uses the
        // kernel literal) silently drops every scatter-gather transaction.
        assert_eq!(
            BC_TRANSACTION_SG, 0x40486311,
            "BC_TRANSACTION_SG = _IOW('c',17,72)"
        );
        assert_eq!(BC_REPLY_SG, 0x40486312, "BC_REPLY_SG = _IOW('c',18,72)");

        // BR_* (each is `[u32 br][payload]` in the read_buffer):
        assert_eq!(BR_ERROR, 0x80047200, "BR_ERROR = _IOR('r',0,4)");
        assert_eq!(BR_OK, 0x00007201, "BR_OK = _IO('r',1)");
        assert_eq!(
            BR_TRANSACTION, 0x80407202,
            "BR_TRANSACTION = _IOR('r',2,64)"
        );
        assert_eq!(BR_REPLY, 0x80407203, "BR_REPLY = _IOR('r',3,64)");
        assert_eq!(BR_DEAD_REPLY, 0x00007205, "BR_DEAD_REPLY = _IO('r',5)");
        assert_eq!(
            BR_TRANSACTION_COMPLETE, 0x00007206,
            "BR_TRANSACTION_COMPLETE = _IO('r',6)"
        );
        assert_eq!(BR_NOOP, 0x0000720C, "BR_NOOP = _IO('r',12)");
        assert_eq!(BR_SPAWN_LOOPER, 0x0000720D, "BR_SPAWN_LOOPER = _IO('r',13)");
        assert_eq!(
            BR_DEAD_BINDER, 0x8008720F,
            "BR_DEAD_BINDER = _IOR('r',15,8)"
        );
        assert_eq!(
            BR_CLEAR_DEATH_NOTIFICATION_DONE, 0x80087210,
            "BR_CLEAR_DEATH_NOTIFICATION_DONE = _IOR('r',16,8)"
        );
        assert_eq!(BR_FAILED_REPLY, 0x00007211, "BR_FAILED_REPLY = _IO('r',17)");
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

    // 6-Z151: ALL THREE binder contexts must be exposed as symlinks, or
    // libhidlbase's `access("/dev/hwbinder", F_OK)` pre-check ENOENTs
    // → defaultServiceManager() returns null → wait_for_keymaster abort
    // → init InitFatalReboot loop (run 32863013472, head e7a16e0).
    #[test]
    fn create_binder_device_creates_hwbinder_and_vndbinder_symlinks() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 3).expect("create_binder_device");
        assert!(path.ends_with("vm3/dev/binder"));

        for name in &["binder", "hwbinder", "vndbinder"] {
            let link = format!("{}/dev/{}", rootfs, name);
            let meta = fs::symlink_metadata(&link).unwrap_or_else(|_| panic!("{} metadata", link));
            assert!(
                meta.file_type().is_symlink(),
                "{rootfs}/dev/{name} should be a symlink (got {meta:?})",
            );
            let target = fs::read_link(&link).unwrap_or_else(|_| panic!("read_link {link}"));
            assert_eq!(
                target.to_string_lossy(),
                "../vm3/dev/binder",
                "{link} should target ../vm3/dev/binder",
            );
        }

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
        stream
            .read_exact(&mut payload)
            .expect("read response payload");
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

        // 6-Z152: time the request — the proxy must block for at least
        // IDLE_POLL_TICK (250ms) before returning BR_NOOP, emulating the
        // kernel's blocking read. Without this, surfaceflinger busy-loops
        // at ~100Hz and pins the ptrace tracer.
        let send_start = std::time::Instant::now();
        stream.write_all(&req).expect("write request");

        // Read response: [i32 ret][u32 arg_len][u32 read_size][read_size bytes].
        let mut hdr = [0u8; 8];
        stream.read_exact(&mut hdr).expect("read response header");
        let elapsed = send_start.elapsed();
        let ret = i32::from_ne_bytes(hdr[0..4].try_into().unwrap());
        let arg_len = u32::from_ne_bytes(hdr[4..8].try_into().unwrap()) as usize;
        assert_eq!(ret, 0);
        assert!(
            arg_len >= 4,
            "BINDER_WRITE_READ response should have a read_size header"
        );

        // 6-Z152: the response must take AT LEAST IDLE_POLL_TICK to arrive
        // (allow 30ms slack for CI scheduling jitter). This is the ground
        // truth that the blocking-idle behaviour is engaged.
        let min_expected = IDLE_POLL_TICK
            .checked_sub(Duration::from_millis(30))
            .unwrap();
        assert!(
            elapsed >= min_expected,
            "idle BINDER_WRITE_READ must block for >= {:?} (got {:?}) — the 6-Z152 blocking-idle fix is missing",
            min_expected,
            elapsed
        );

        let mut resp = vec![0u8; arg_len];
        stream.read_exact(&mut resp).expect("read response payload");
        let read_size = u32::from_ne_bytes(resp[0..4].try_into().unwrap()) as usize;
        assert_eq!(
            read_size, 4,
            "idle BINDER_WRITE_READ should return exactly one BR_NOOP"
        );

        let br_cmd = u32::from_ne_bytes(resp[4..8].try_into().unwrap());
        assert_eq!(br_cmd, BR_NOOP, "expected BR_NOOP");

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- ThreadPool ----------------------------------------------

    // -------- 6-Z265: kernel-true reply delivery for v1 (real-libbinder)
    // clients ----------------------------------------------------------
    //
    // EVIDENCE (run 33334415274, OrangeFox R12 lavender): the guest's
    // REAL libbinder.so sends plain-v1 BC_TRANSACTION (no v2 trailer).
    // The old wire dropped the reply bytes for v1 and returned
    // tr.data_ptr=0 — real libbinder dereferences that pointer → SIGSEGV
    // si_addr=0x0 in libbinder.so → recovery died 7 times (init kept
    // restarting it = the "soft reboots to flash back again" report) and
    // keystore2 crash-looped 56 times. The proxy must now append the
    // reply blob to the response even for v1 requests; the hook backs
    // the pointer with real memory.

    #[test]
    fn z265_v1_transaction_response_carries_reply_blob_trailer() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));

        let mut stream = UnixStream::connect(&path).expect("connect");

        // A REAL libbinder BC_TRANSACTION: target handle 0 (servicemanager),
        // code 1 (SVC_MGR_GET_SERVICE), NO v2 trailer after the BC stream.
        let mut tx = [0u8; std::mem::size_of::<BinderTransactionData>()];
        tx[0..4].copy_from_slice(&0u32.to_ne_bytes()); // target handle 0
        tx[16..20].copy_from_slice(&1u32.to_ne_bytes()); // code = GET_SERVICE

        let mut payload = Vec::new();
        payload.extend_from_slice(&((4 + tx.len()) as u32).to_ne_bytes()); // write_size
        payload.extend_from_slice(&256u32.to_ne_bytes()); // read_capacity
        payload.extend_from_slice(&BC_TRANSACTION.to_ne_bytes()); // cmd
        payload.extend_from_slice(&tx);

        let mut req = Vec::new();
        req.extend_from_slice(&BINDER_WRITE_READ.to_ne_bytes());
        req.extend_from_slice(&(payload.len() as u32).to_ne_bytes());
        req.extend_from_slice(&payload);
        stream.write_all(&req).expect("write request");

        let mut hdr = [0u8; 8];
        stream.read_exact(&mut hdr).expect("read response header");
        let ret = i32::from_ne_bytes(hdr[0..4].try_into().unwrap());
        let arg_len = u32::from_ne_bytes(hdr[4..8].try_into().unwrap()) as usize;
        assert_eq!(ret, 0);
        let mut resp = vec![0u8; arg_len];
        stream.read_exact(&mut resp).expect("read response payload");

        // [u32 read_size][BR stream][trailer: magic + count + blob…]
        let read_size = u32::from_ne_bytes(resp[0..4].try_into().unwrap()) as usize;
        assert!(read_size >= 72, "expect BR_TRANSACTION_COMPLETE + BR_REPLY");
        let cmd0 = u32::from_ne_bytes(resp[4..8].try_into().unwrap());
        assert_eq!(cmd0, BR_TRANSACTION_COMPLETE, "batch starts with COMPLETE");
        let cmd1 = u32::from_ne_bytes(resp[8..12].try_into().unwrap());
        assert_eq!(cmd1, BR_REPLY, "then BR_REPLY");
        // The on-wire tr still carries data_ptr=0 (the hook patches it to
        // the backing allocation it makes for the client).
        let wire_data_ptr = u64::from_ne_bytes(resp[8 + 4 + 48..8 + 4 + 56].try_into().unwrap());
        assert_eq!(wire_data_ptr, 0, "wire tr.data_ptr stays 0 (hook patches)");

        // The trailer MUST be present for the v1 request now.
        let tail = &resp[4 + read_size..];
        assert!(tail.len() >= 8, "v1 response must carry the blob trailer");
        let magic = u32::from_ne_bytes(tail[0..4].try_into().unwrap());
        assert_eq!(magic, WIRE_V2_MAGIC, "trailer magic");
        let count = u32::from_ne_bytes(tail[4..8].try_into().unwrap());
        assert_eq!(count, 1, "one reply blob");
        let dlen = u32::from_ne_bytes(tail[8..12].try_into().unwrap()) as usize;
        let olen = u32::from_ne_bytes(tail[12..16].try_into().unwrap()) as usize;
        // 6-Z271x: status-ok (4) + flat_binder_object (24) + stability (4)
        assert_eq!(dlen, 32, "status-ok (4) + flat (24) + stability i32 (4)");
        assert_eq!(olen, 8, "one offsets entry (binder_size_t = u64)");
        assert!(
            tail.len() >= 16 + dlen + olen,
            "trailer must carry the full blob bytes"
        );
        // Reply must parse as AIDL Status::ok (EX_NONE = 0)…
        let status = i32::from_ne_bytes(tail[16..20].try_into().unwrap());
        assert_eq!(status, 0, "EX_NONE");
        // …followed by a BINDER_TYPE_BINDER null-binder flat object.
        let ftype = u32::from_ne_bytes(tail[20..24].try_into().unwrap());
        assert_eq!(ftype, BINDER_TYPE_BINDER, "null binder (service miss)");

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn z265_oneway_spam_detection_ioctl_is_acknowledged() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));

        let mut stream = UnixStream::connect(&path).expect("connect");

        // The exact number real libbinder sends (Android 11+):
        // _IOW('b', 16, __u32) = 0x40046210.
        assert_eq!(BINDER_ENABLE_ONEWAY_SPAM_DETECTION, 0x4004_6210);
        let mut req = Vec::new();
        req.extend_from_slice(&BINDER_ENABLE_ONEWAY_SPAM_DETECTION.to_ne_bytes());
        req.extend_from_slice(&4u32.to_ne_bytes());
        req.extend_from_slice(&0u32.to_ne_bytes());
        stream.write_all(&req).expect("write request");

        let mut hdr = [0u8; 8];
        stream.read_exact(&mut hdr).expect("read response header");
        let ret = i32::from_ne_bytes(hdr[0..4].try_into().unwrap());
        assert_eq!(ret, 0, "ENABLE_ONEWAY_SPAM_DETECTION must ACK, not EINVAL");

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- ThreadPool (original) ------------------------------------

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

    // -------- Parcel codec round-trips (6-Z114 §3.2) -----------------

    /// `Parcel::writeString16` → `readString16` round-trip for ASCII,
    /// BMP non-ASCII, and empty string. Verifies the length prefix,
    /// always-written NUL, and 4-byte pad are all consumed symmetrically.
    #[test]
    fn parcel_string16_round_trip() {
        for s in [
            "",
            "activity",
            "android.os.IServiceManager",
            "café",
            "日本語",
        ] {
            let mut w = ParcelWriter::new();
            w.write_string16(s);
            // Reader position after the string must equal writer length
            // — proves we consumed the NUL + pad the same way.
            let (data, _) = w.into_parts();
            let mut r = ParcelReader::new(&data);
            let out = r
                .read_string16()
                .expect("read_string16 returned Some")
                .expect("string16 was non-null");
            assert_eq!(out, s, "string16 round-trip mismatch for {:?}", s);
            assert_eq!(
                r.remaining(),
                0,
                "reader should have consumed the whole buffer for {:?}",
                s
            );
        }
    }

    /// AIDL interface-token header (`Parcel::writeInterfaceToken`)
    /// round-trips exactly: strict / work / tag / descriptor all match.
    #[test]
    fn parcel_aidl_header_round_trip() {
        let mut w = ParcelWriter::new();
        w.write_i32(0); // strict_mode_policy
        w.write_i32(-1); // work_source_uid (kUnsetWorkSource)
        w.write_u32(AIDL_HEADER_TAG_SYST);
        w.write_string16(SVC_MGR_IFACE_DESCRIPTOR);
        let (data, _) = w.into_parts();
        let mut r = ParcelReader::new(&data);
        let (strict, work, tag, iface) = r
            .read_aidl_header()
            .expect("read_aidl_header returned Some");
        assert_eq!(strict, 0, "strict_mode_policy");
        assert_eq!(work, -1, "work_source_uid");
        assert_eq!(tag, AIDL_HEADER_TAG_SYST, "header tag");
        assert_eq!(
            iface.as_deref(),
            Some(SVC_MGR_IFACE_DESCRIPTOR),
            "interface descriptor"
        );
        assert_eq!(r.remaining(), 0, "header must consume the whole buffer");
    }

    /// `write_flat_binder` MUST also append the object's byte offset to
    /// the offsets array — both the kernel's translation table and the
    /// Parcel object bookkeeping depend on it (6-Z114 §3.2).
    #[test]
    fn parcel_write_flat_binder_appends_offset() {
        let mut w = ParcelWriter::new();
        // First write an i32 status (4 bytes) so the flat object lands at
        // a non-zero offset — proves the offset is data-relative, not 0.
        w.write_status_ok();
        let obj = FlatBinderObject {
            r#type: BINDER_TYPE_HANDLE,
            flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
            binder: 0xF000_0001, // proxy handle, low 32 bits
            cookie: 0,
        };
        let off = w.write_flat_binder(&obj);
        assert_eq!(off, 4, "flat object must land after the i32 status prefix");
        let (data, offsets) = w.into_parts();
        assert_eq!(
            data.len(),
            4 + 24,
            "data = i32 status + 24-byte flat object"
        );
        assert_eq!(offsets.len(), 8, "offsets array = one u64 offset");
        let parsed_off = u64::from_ne_bytes(offsets[..].try_into().unwrap());
        assert_eq!(
            parsed_off, 4,
            "offsets[0] must equal the object's data offset"
        );
        // Read it back and verify field-for-field.
        let mut r = ParcelReader::new(&data);
        let _status = r.read_i32().expect("status prefix");
        let back = r.read_flat_binder().expect("flat_binder object");
        assert_eq!(back.r#type, BINDER_TYPE_HANDLE);
        assert_eq!(back.flags, FLAT_FLAGS_LIBBINDER_DEFAULT);
        assert_eq!(back.binder, 0xF000_0001);
        assert_eq!(back.cookie, 0);
    }

    // -------- ServiceRegistry (6-Z114 §3.3 / §3.4) ------------------

    #[test]
    fn service_registry_add_then_get_returns_allocated_handle() {
        let mut reg = ServiceRegistry::new();
        let h1 = reg.add("activity");
        let h2 = reg.add("package");
        // Handles come from PROXY_HANDLE_BASE + 1, monotonically.
        assert_eq!(h1, PROXY_HANDLE_BASE + 1);
        assert_eq!(h2, PROXY_HANDLE_BASE + 2);
        assert_eq!(reg.get("activity"), Some(h1));
        assert_eq!(reg.get("package"), Some(h2));
        assert_eq!(reg.get("nope"), None, "miss must return None");
        assert_eq!(reg.len(), 2);
        assert!(!reg.is_empty());
    }

    // -------- servicemanager proxy end-to-end over the v2 wire -------
    // (6-Z114 §3 + §4.4 — exercises the full parse → reply → blob trailer
    // path through BinderProxy)

    /// Helper: build a `binder_transaction_data` payload for a
    /// BC_TRANSACTION to handle 0 with the given code + flags. The
    /// data/offsets pointers are 0 (the v2 client patches them from the
    /// blob index).
    fn make_bc_transaction_payload(code: u32, flags: u32) -> [u8; 64] {
        let mut tx = [0u8; 64];
        // target.handle = 0 (servicemanager)
        tx[0..4].copy_from_slice(&0u32.to_ne_bytes());
        // code
        tx[16..20].copy_from_slice(&code.to_ne_bytes());
        // flags
        tx[20..24].copy_from_slice(&flags.to_ne_bytes());
        // data_size / offsets_size stay 0 on the wire (v2: blob carries bytes)
        tx
    }

    /// Helper: build an AIDL request parcel for `android.os.IServiceManager`
    /// — `[i32 0 strict][i32 -1 work][i32 SYST tag][string16 descriptor]`
    /// followed by the caller-provided per-code args writer. Each flat
    /// object's offset in the args writer is shifted by the size of the
    /// AIDL header so the merged offsets array stays correct relative to
    /// the merged data buffer.
    fn make_servicemanager_request_parcel(args: &mut ParcelWriter) -> (Vec<u8>, Vec<u8>) {
        let mut w = ParcelWriter::new();
        w.write_i32(0); // strict
        w.write_i32(-1); // work_source
        w.write_u32(AIDL_HEADER_TAG_SYST);
        w.write_string16(SVC_MGR_IFACE_DESCRIPTOR);
        let args_offset = w.data.len() as u64;
        // Move args.data and args.offsets out so we can iterate the
        // offsets without contending with the data borrow.
        let args_data = std::mem::take(&mut args.data);
        let args_offsets = std::mem::take(&mut args.offsets);
        w.data.extend_from_slice(&args_data);
        // Shift each u64 offset by the AIDL header's byte size so it
        // points into the merged data buffer at the right place.
        for chunk in args_offsets.chunks_exact(8) {
            let off = u64::from_ne_bytes(chunk.try_into().unwrap()) + args_offset;
            w.offsets.extend_from_slice(&off.to_ne_bytes());
        }
        w.into_parts()
    }

    /// Helper: build a v2 BINDER_WRITE_READ wire payload for one
    /// BC_TRANSACTION carrying one parcel blob.
    fn make_v2_write_read_payload(
        bc_stream: &[u8],
        blob_data: &[u8],
        blob_offsets: &[u8],
        read_capacity: u32,
    ) -> Vec<u8> {
        let write_size = bc_stream.len() as u32;
        let mut p = Vec::new();
        p.extend_from_slice(&write_size.to_ne_bytes());
        p.extend_from_slice(&read_capacity.to_ne_bytes());
        p.extend_from_slice(bc_stream);
        // v2 trailer
        p.extend_from_slice(&WIRE_V2_MAGIC.to_ne_bytes());
        p.extend_from_slice(&1u32.to_ne_bytes()); // one blob
        p.extend_from_slice(&(blob_data.len() as u32).to_ne_bytes());
        p.extend_from_slice(&(blob_offsets.len() as u32).to_ne_bytes());
        p.extend_from_slice(blob_data);
        p.extend_from_slice(blob_offsets);
        p
    }

    /// Helper: frame a payload as `[u32 cmd][u32 arg_len][payload]` and
    /// send it; read back the framed response `[i32 ret][u32 arg_len][...]`.
    fn exchange(stream: &mut UnixStream, cmd: u32, payload: &[u8]) -> (i32, Vec<u8>) {
        let mut req = Vec::with_capacity(8 + payload.len());
        req.extend_from_slice(&cmd.to_ne_bytes());
        req.extend_from_slice(&(payload.len() as u32).to_ne_bytes());
        req.extend_from_slice(payload);
        stream.write_all(&req).expect("write request");
        let mut hdr = [0u8; 8];
        stream.read_exact(&mut hdr).expect("read response header");
        let ret = i32::from_ne_bytes(hdr[0..4].try_into().unwrap());
        let arg_len = u32::from_ne_bytes(hdr[4..8].try_into().unwrap()) as usize;
        let mut payload = vec![0u8; arg_len];
        stream
            .read_exact(&mut payload)
            .expect("read response payload");
        (ret, payload)
    }

    /// ADD_SERVICE then GET_SERVICE over the v2 wire: the GET reply must
    /// carry a `BINDER_TYPE_HANDLE` flat object whose `binder` field
    /// equals the proxy-allocated handle, listed in the offsets array.
    #[test]
    fn servicemanager_proxy_v2_add_then_get_returns_handle() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));
        let mut stream = UnixStream::connect(&path).expect("connect");

        // ---- BC_TRANSACTION ADD_SERVICE "my_svc" ----
        let mut args = ParcelWriter::new();
        args.write_string16("my_svc");
        args.write_flat_binder(&FlatBinderObject {
            r#type: BINDER_TYPE_BINDER,
            flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
            binder: 0xdead, // guest weakrefs ptr — not yet tracked
            cookie: 0xbeef, // guest BBinder ptr — not yet tracked
        });
        args.write_i32(0); // allowIsolated
        args.write_i32(0); // dumpPriority
        let (req_data, req_off) = make_servicemanager_request_parcel(&mut args);

        // Build the BC_TRANSACTION stream: [u32 cmd][64-byte tr_data].
        let mut bc = Vec::with_capacity(4 + 64);
        bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_ADD_SERVICE, 0));
        let payload = make_v2_write_read_payload(&bc, &req_data, &req_off, 4096);

        let (ret, resp) = exchange(&mut stream, BINDER_WRITE_READ, &payload);
        assert_eq!(ret, 0, "ADD_SERVICE WRITE_READ should succeed");
        // Response shape: [u32 read_size][BR_TRANSACTION_COMPLETE][BR_REPLY][64-byte tr]
        //                  [u32 WIRE_V2_MAGIC][u32 1][u32 data_len][u32 off_len][data][offsets]
        let read_size = u32::from_ne_bytes(resp[0..4].try_into().unwrap()) as usize;
        // 4 bytes BR_TRANSACTION_COMPLETE + 4 bytes BR_REPLY + 64 bytes tr = 72.
        assert_eq!(
            read_size,
            4 + 4 + 64,
            "ADD reply BR stream = COMPLETE + REPLY + 64-byte tr"
        );
        let br_complete = u32::from_ne_bytes(resp[4..8].try_into().unwrap());
        assert_eq!(br_complete, BR_TRANSACTION_COMPLETE);
        let br_reply = u32::from_ne_bytes(resp[8..12].try_into().unwrap());
        assert_eq!(br_reply, BR_REPLY);
        // ADD reply parcel = [i32 0] (status) only — no flat object.
        // Locate the v2 trailer and verify the blob.
        let mut off = 4 + read_size;
        let magic = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap());
        assert_eq!(magic, WIRE_V2_MAGIC, "response must be v2");
        off += 4;
        let blob_count = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap());
        assert_eq!(blob_count, 1, "ADD response carries one reply blob");
        off += 4;
        let data_len = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap()) as usize;
        let off_len = u32::from_ne_bytes(resp[off + 4..off + 8].try_into().unwrap()) as usize;
        assert_eq!(data_len, 4, "ADD reply = [i32 0] status only");
        assert_eq!(off_len, 0, "ADD reply has no flat objects");
        let status = i32::from_ne_bytes(resp[off + 8..off + 12].try_into().unwrap());
        assert_eq!(status, 0, "ADD reply status = EX_NONE");

        // ---- BC_TRANSACTION GET_SERVICE "my_svc" ----
        let mut args2 = ParcelWriter::new();
        args2.write_string16("my_svc");
        let (req_data2, req_off2) = make_servicemanager_request_parcel(&mut args2);
        let mut bc2 = Vec::with_capacity(4 + 64);
        bc2.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc2.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_GET_SERVICE, 0));
        let payload2 = make_v2_write_read_payload(&bc2, &req_data2, &req_off2, 4096);
        let (ret2, resp2) = exchange(&mut stream, BINDER_WRITE_READ, &payload2);
        assert_eq!(ret2, 0, "GET_SERVICE WRITE_READ should succeed");
        let read_size2 = u32::from_ne_bytes(resp2[0..4].try_into().unwrap()) as usize;
        assert_eq!(read_size2, 4 + 4 + 64, "GET reply BR stream");
        let br_complete2 = u32::from_ne_bytes(resp2[4..8].try_into().unwrap());
        assert_eq!(br_complete2, BR_TRANSACTION_COMPLETE);
        let br_reply2 = u32::from_ne_bytes(resp2[8..12].try_into().unwrap());
        assert_eq!(br_reply2, BR_REPLY);
        // Locate the v2 trailer reply blob.
        let mut off2 = 4 + read_size2;
        assert_eq!(
            u32::from_ne_bytes(resp2[off2..off2 + 4].try_into().unwrap()),
            WIRE_V2_MAGIC
        );
        off2 += 4;
        assert_eq!(
            u32::from_ne_bytes(resp2[off2..off2 + 4].try_into().unwrap()),
            1
        );
        off2 += 4;
        let data_len2 = u32::from_ne_bytes(resp2[off2..off2 + 4].try_into().unwrap()) as usize;
        let off_len2 = u32::from_ne_bytes(resp2[off2 + 4..off2 + 8].try_into().unwrap()) as usize;
        assert_eq!(
            data_len2,
            4 + 24 + 4,
            "GET reply = [i32 0 status] + 24-byte flat + i32 stability (6-Z271x)"
        );
        assert_eq!(off_len2, 8, "GET reply offsets = one u64 offset");
        let blob2 = &resp2[off2 + 8..off2 + 8 + data_len2];
        let status2 = i32::from_ne_bytes(blob2[0..4].try_into().unwrap());
        assert_eq!(status2, 0, "GET reply status = EX_NONE");
        // Flat-object layout (24 bytes): u32 type, u32 flags, u64 binder, u64 cookie.
        let flat_type = u32::from_ne_bytes(blob2[4..8].try_into().unwrap());
        assert_eq!(
            flat_type, BINDER_TYPE_HANDLE,
            "GET hit → BINDER_TYPE_HANDLE"
        );
        // 6-Z271x: the android-12+ stability annotation follows the flat;
        // first get on this conn → A12 Category form (6-Z272e).
        let stability = i32::from_ne_bytes(blob2[28..32].try_into().unwrap());
        assert_eq!(
            stability, STABILITY_ANNOTATION_VINTF_A12,
            "GET hit → A12 Category annotation on a fresh connection"
        );
        // The proxy handle lives in the `binder` u64 field (low 32 bits on
        // remote refs — 6-Z114 §3.2).
        let flat_handle = u64::from_ne_bytes(blob2[12..20].try_into().unwrap()) as u32;
        // 6-Z271/6-Z298: the four in-proxy virtual services are
        // registered at proxy construction (handles 0xF0000001-4 — the
        // 6-Z298 health service is the fourth), so the first GUEST
        // service allocates 0xF0000005.
        assert_eq!(
            flat_handle,
            PROXY_HANDLE_BASE + 5,
            "proxy handle = 0xF0000000 + 5 (after the 4 virtual services)"
        );
        // The reply offsets array must list the flat object's offset (= 4,
        // after the i32 status prefix).
        let reply_offsets = &resp2[off2 + 8 + data_len2..off2 + 8 + data_len2 + off_len2];
        let listed_off = u64::from_ne_bytes(reply_offsets[..].try_into().unwrap());
        assert_eq!(
            listed_off, 4,
            "reply offsets[0] = flat object's data offset"
        );

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    /// GET_SERVICE for an unregistered name must reply with a `null binder`
    /// — `BINDER_TYPE_BINDER` with `cookie = 0` (the client's
    /// `readStrongBinder` decodes that as nullptr — 6-Z114 §3.3).
    #[test]
    fn servicemanager_proxy_v2_get_miss_returns_null_binder() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));
        let mut stream = UnixStream::connect(&path).expect("connect");

        let mut args = ParcelWriter::new();
        args.write_string16("does_not_exist");
        let (req_data, req_off) = make_servicemanager_request_parcel(&mut args);
        let mut bc = Vec::with_capacity(4 + 64);
        bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_GET_SERVICE, 0));
        let payload = make_v2_write_read_payload(&bc, &req_data, &req_off, 4096);
        let (ret, resp) = exchange(&mut stream, BINDER_WRITE_READ, &payload);
        assert_eq!(ret, 0);

        let read_size = u32::from_ne_bytes(resp[0..4].try_into().unwrap()) as usize;
        assert_eq!(
            u32::from_ne_bytes(resp[4..8].try_into().unwrap()),
            BR_TRANSACTION_COMPLETE
        );
        assert_eq!(
            u32::from_ne_bytes(resp[8..12].try_into().unwrap()),
            BR_REPLY
        );

        let mut off = 4 + read_size;
        assert_eq!(
            u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap()),
            WIRE_V2_MAGIC
        );
        off += 8; // magic(4) + blob_count(4)
        let data_len = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap()) as usize;
        let off_len = u32::from_ne_bytes(resp[off + 4..off + 8].try_into().unwrap()) as usize;
        assert_eq!(
            data_len,
            4 + 24 + 4,
            "miss reply = [i32 0] + null flat + stability i32"
        );
        assert_eq!(off_len, 8, "null flat object still listed in offsets");
        let blob = &resp[off + 8..off + 8 + data_len];
        let status = i32::from_ne_bytes(blob[0..4].try_into().unwrap());
        assert_eq!(
            status, 0,
            "miss is still a successful transaction (EX_NONE)"
        );
        // Flat-object layout (24 bytes): u32 type, u32 flags, u64 binder, u64 cookie.
        let flat_type = u32::from_ne_bytes(blob[4..8].try_into().unwrap());
        assert_eq!(
            flat_type, BINDER_TYPE_BINDER,
            "miss → AIDL null binder (BINDER_TYPE_BINDER with cookie 0)"
        );
        let binder = u64::from_ne_bytes(blob[12..20].try_into().unwrap());
        let cookie = u64::from_ne_bytes(blob[20..28].try_into().unwrap());
        assert_eq!(binder, 0, "null binder: binder field = 0");
        assert_eq!(cookie, 0, "null binder: cookie = 0");
        // 6-Z271x: null binders are annotated with the A12 Category null
        // (version 1, level UNDECLARED) — the first get on a fresh conn
        // uses the Category form (6-Z272e).
        let null_stability = i32::from_ne_bytes(blob[28..32].try_into().unwrap());
        assert_eq!(
            null_stability, STABILITY_ANNOTATION_NULL_A12,
            "miss → null-binder stability annotation = Category(version 1, level 0)"
        );

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- 6-Z271: full guest↔guest bus round trip ----------------
    // Connection A registers a service (real addService with a flat
    // binder); connection B looks it up and transacts to the routed
    // handle; the proxy delivers BR_TRANSACTION to A; A's BC_REPLY is
    // routed back to B as BR_REPLY. This is the master-path behavior the
    // 6-Z114 registry never had.

    /// Build a BINDER_WRITE_READ v2 payload with MULTIPLE BC commands and
    /// their matching blobs (blob order = command order in the stream).
    fn make_v2_write_read_multi_payload(
        bc_stream: &[u8],
        blobs: &[(&[u8], &[u8])],
        read_capacity: u32,
    ) -> Vec<u8> {
        let write_size = bc_stream.len() as u32;
        let mut p = Vec::new();
        p.extend_from_slice(&write_size.to_ne_bytes());
        p.extend_from_slice(&read_capacity.to_ne_bytes());
        p.extend_from_slice(bc_stream);
        p.extend_from_slice(&WIRE_V2_MAGIC.to_ne_bytes());
        p.extend_from_slice(&(blobs.len() as u32).to_ne_bytes());
        for (d, o) in blobs {
            p.extend_from_slice(&(d.len() as u32).to_ne_bytes());
            p.extend_from_slice(&(o.len() as u32).to_ne_bytes());
            p.extend_from_slice(d);
            p.extend_from_slice(o);
        }
        p
    }

    #[test]
    fn z271_bus_full_guest_to_guest_transaction_round_trip() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));

        // ---- Connection A (the future server): addService("svc_a") ----
        let mut stream_a = UnixStream::connect(&path).expect("connect A");
        let mut args = ParcelWriter::new();
        args.write_string16("svc_a");
        args.write_flat_binder(&FlatBinderObject {
            r#type: BINDER_TYPE_BINDER,
            flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
            binder: 0x1234, // guest local ptr
            cookie: 0x5678, // guest cookie
        });
        args.write_i32(0);
        args.write_i32(0);
        let (ad, ao) = make_servicemanager_request_parcel(&mut args);
        let mut bc = Vec::with_capacity(4 + 64);
        bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_ADD_SERVICE, 0));
        let payload = make_v2_write_read_payload(&bc, &ad, &ao, 4096);
        let (ret, resp) = exchange(&mut stream_a, BINDER_WRITE_READ, &payload);
        assert_eq!(ret, 0, "ADD_SERVICE WRITE_READ ok");
        let read_size = u32::from_ne_bytes(resp[0..4].try_into().unwrap()) as usize;
        assert_eq!(
            u32::from_ne_bytes(resp[8..12].try_into().unwrap()),
            BR_REPLY,
            "ADD replies with BR_REPLY"
        );
        let _ = read_size;

        // ---- Connection B: getService("svc_a") → routed handle ----
        let mut stream_b = UnixStream::connect(&path).expect("connect B");
        let mut args2 = ParcelWriter::new();
        args2.write_string16("svc_a");
        let (bd, bo) = make_servicemanager_request_parcel(&mut args2);
        let mut bc2 = Vec::with_capacity(4 + 64);
        bc2.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc2.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_GET_SERVICE, 0));
        let payload2 = make_v2_write_read_payload(&bc2, &bd, &bo, 4096);
        let (ret2, resp2) = exchange(&mut stream_b, BINDER_WRITE_READ, &payload2);
        assert_eq!(ret2, 0);
        let read_size2 = u32::from_ne_bytes(resp2[0..4].try_into().unwrap()) as usize;
        let off2 = 4 + read_size2 + 8; // skip [read_size][BR stream][magic][count]
        let dl2 = u32::from_ne_bytes(resp2[off2..off2 + 4].try_into().unwrap()) as usize;
        let blob2 = &resp2[off2 + 8..off2 + 8 + dl2];
        let routed_handle = u64::from_ne_bytes(blob2[12..20].try_into().unwrap()) as u32;
        assert_eq!(
            routed_handle,
            PROXY_HANDLE_BASE + 5,
            "svc_a handle = 0xF0000005 (after the 4 virtual services)"
        );

        // ---- Connection B: transact(code=42) to the routed handle ----
        // 6-Z271i: the transaction ioctl completes with ONLY
        // BR_TRANSACTION_COMPLETE (kernel semantics — no blocking inside
        // the proxy); B's reply arrives on its next read.
        let mut tx_b = [0u8; 64];
        tx_b[0..4].copy_from_slice(&routed_handle.to_ne_bytes());
        tx_b[16..20].copy_from_slice(&42u32.to_ne_bytes());
        let tx_data: &[u8] = b"ping-payload";
        let tx_off = Vec::new();
        let mut bc3 = Vec::with_capacity(4 + 64);
        bc3.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc3.extend_from_slice(&tx_b);
        let payload3 = make_v2_write_read_multi_payload(&bc3, &[(tx_data, &tx_off)], 4096);
        let (ret_t, resp_t) = exchange(&mut stream_b, BINDER_WRITE_READ, &payload3);
        assert_eq!(ret_t, 0, "B's transaction WRITE_READ ok");
        assert_eq!(
            u32::from_ne_bytes(resp_t[0..4].try_into().unwrap()) as usize,
            4,
            "B's sync transaction returns an empty-but-COMPLETE read buffer"
        );
        assert_eq!(
            u32::from_ne_bytes(resp_t[4..8].try_into().unwrap()),
            BR_TRANSACTION_COMPLETE,
            "6-Z271i: COMPLETE only in the transaction ioctl"
        );

        // ---- Connection A: read-only ioctl → receives BR_TRANSACTION ----
        // write_size = 0, read_capacity = 4096.
        let mut wr_a = Vec::new();
        wr_a.extend_from_slice(&0u32.to_ne_bytes());
        wr_a.extend_from_slice(&4096u32.to_ne_bytes());
        let (ret_a, resp_a) = exchange(&mut stream_a, BINDER_WRITE_READ, &wr_a);
        assert_eq!(ret_a, 0, "server delivery WRITE_READ ok");
        let read_a = u32::from_ne_bytes(resp_a[0..4].try_into().unwrap()) as usize;
        // [BR_TRANSACTION][64-byte tr] — and the v2 trailer with the blob.
        let br = u32::from_ne_bytes(resp_a[4..8].try_into().unwrap());
        assert_eq!(br, BR_TRANSACTION, "A receives BR_TRANSACTION");
        assert_eq!(
            read_a,
            4 + 64,
            "BR_TRANSACTION + 64-byte binder_transaction_data"
        );
        let tr = &resp_a[8..8 + 64];
        // Target union = PTR form: ptr low/high + cookie (0x1234 / 0x5678).
        let ptr_lo = u32::from_ne_bytes(tr[0..4].try_into().unwrap());
        let ptr_hi = u32::from_ne_bytes(tr[4..8].try_into().unwrap());
        let cookie = u64::from_ne_bytes(tr[8..16].try_into().unwrap());
        assert_eq!(ptr_lo, 0x1234, "target.ptr low word");
        assert_eq!(ptr_hi, 0, "target.ptr high word");
        assert_eq!(cookie, 0x5678, "target.cookie = owner cookie");
        let code = u32::from_ne_bytes(tr[16..20].try_into().unwrap());
        assert_eq!(code, 42, "delivered code");
        let data_size = u64::from_ne_bytes(tr[32..40].try_into().unwrap());
        assert_eq!(data_size, tx_data.len() as u64, "delivered parcel size");
        // Trailer blob carries the request parcel bytes.
        let magic_a = u32::from_ne_bytes(resp_a[4 + read_a..4 + read_a + 4].try_into().unwrap());
        assert_eq!(magic_a, WIRE_V2_MAGIC, "delivery carries v2 trailer");

        // ---- Connection A: BC_REPLY (with its own blob) ----
        let reply_data: &[u8] = b"pong-reply";
        let mut reply = [0u8; 64];
        reply[16..20].copy_from_slice(&0u32.to_ne_bytes()); // reply code unused
        let mut bc4 = Vec::with_capacity(4 + 64);
        bc4.extend_from_slice(&BC_REPLY.to_ne_bytes());
        bc4.extend_from_slice(&reply);
        let payload4 = make_v2_write_read_multi_payload(&bc4, &[(reply_data, &tx_off)], 0);
        let (ret_r, resp_r) = exchange(&mut stream_a, BINDER_WRITE_READ, &payload4);
        assert_eq!(ret_r, 0);
        assert_eq!(
            u32::from_ne_bytes(resp_r[0..4].try_into().unwrap()),
            0,
            "BC_REPLY ioctl returns an empty read buffer"
        );

        // ---- Connection B: read-only ioctl → the deferred BR_REPLY ----
        let mut wr_b = Vec::new();
        wr_b.extend_from_slice(&0u32.to_ne_bytes());
        wr_b.extend_from_slice(&4096u32.to_ne_bytes());
        let (ret_b, resp_b) = exchange(&mut stream_b, BINDER_WRITE_READ, &wr_b);
        assert_eq!(ret_b, 0);
        let read_b = u32::from_ne_bytes(resp_b[0..4].try_into().unwrap()) as usize;
        let br2 = u32::from_ne_bytes(resp_b[4..8].try_into().unwrap());
        assert_eq!(
            br2, BR_REPLY,
            "B's deferred BR_REPLY lands on its next read"
        );
        assert_eq!(read_b, 4 + 64, "REPLY + tr");
        let off_b = 4 + read_b + 8;
        let dl_b = u32::from_ne_bytes(resp_b[off_b..off_b + 4].try_into().unwrap()) as usize;
        assert_eq!(dl_b, reply_data.len(), "reply blob = A's parcel bytes");
        let got = &resp_b[off_b + 8..off_b + 8 + dl_b];
        assert_eq!(got, reply_data, "B receives A's exact reply payload");

        drop(stream_a);
        drop(stream_b);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    // -------- 6-Z271: virtual service handlers over the wire ---------

    #[test]
    fn z271g_process_pool_steal_sibling_conn_takes_queued_tx() {
        // Real binder queues incoming transactions on the PROCESS's todo
        // list: any ready pool thread may take them. With per-thread
        // proxy conns the registering conn may be busy, so a sibling
        // conn of the same guest PROCESS (same sender_pid) must be able
        // to steal the queued node work. This test never lets conn A
        // read — conn A2 (same pid) takes the delivery instead.
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let _handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));

        let ident_payload = |pid: u32| {
            let mut p = Vec::with_capacity(12);
            p.extend_from_slice(&pid.to_ne_bytes());
            p.extend_from_slice(&0u32.to_ne_bytes());
            p.extend_from_slice(&0u32.to_ne_bytes());
            p
        };

        // ---- Conn A (pid 7777): addService("svc_a") ----
        let mut stream_a = UnixStream::connect(&path).expect("connect A");
        let (ret_i, _r) = exchange(&mut stream_a, WIRE_CMD_IDENT, &ident_payload(7777));
        assert_eq!(ret_i, 0, "IDENT A accepted");
        let mut args = ParcelWriter::new();
        args.write_string16("svc_a");
        args.write_flat_binder(&FlatBinderObject {
            r#type: BINDER_TYPE_BINDER,
            flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
            binder: 0x1234,
            cookie: 0x5678,
        });
        args.write_i32(0);
        args.write_i32(0);
        let (ad, ao) = make_servicemanager_request_parcel(&mut args);
        let mut bc = Vec::with_capacity(4 + 64);
        bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_ADD_SERVICE, 0));
        let payload = make_v2_write_read_payload(&bc, &ad, &ao, 4096);
        let (ret, resp) = exchange(&mut stream_a, BINDER_WRITE_READ, &payload);
        assert_eq!(ret, 0, "ADD_SERVICE ok");
        assert_eq!(
            u32::from_ne_bytes(resp[8..12].try_into().unwrap()),
            BR_REPLY
        );
        // A now parks WITHOUT reading (its inbox is where the tx queues).

        // ---- Conn B (pid 8888): getService → handle ----
        let mut stream_b = UnixStream::connect(&path).expect("connect B");
        let (ret_i2, _r2) = exchange(&mut stream_b, WIRE_CMD_IDENT, &ident_payload(8888));
        assert_eq!(ret_i2, 0);
        let mut args2 = ParcelWriter::new();
        args2.write_string16("svc_a");
        let (bd, bo) = make_servicemanager_request_parcel(&mut args2);
        let mut bc2 = Vec::with_capacity(4 + 64);
        bc2.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc2.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_GET_SERVICE, 0));
        let payload2 = make_v2_write_read_payload(&bc2, &bd, &bo, 4096);
        let (ret2, resp2) = exchange(&mut stream_b, BINDER_WRITE_READ, &payload2);
        assert_eq!(ret2, 0);
        let read_size2 = u32::from_ne_bytes(resp2[0..4].try_into().unwrap()) as usize;
        let off2 = 4 + read_size2 + 8;
        let dl2 = u32::from_ne_bytes(resp2[off2..off2 + 4].try_into().unwrap()) as usize;
        let blob2 = &resp2[off2 + 8..off2 + 8 + dl2];
        let routed_handle = u64::from_ne_bytes(blob2[12..20].try_into().unwrap()) as u32;

        // ---- Conn B: transact(code=42) — completes in-ioctl, parks ----
        let mut tx_b = [0u8; 64];
        tx_b[0..4].copy_from_slice(&routed_handle.to_ne_bytes());
        tx_b[16..20].copy_from_slice(&42u32.to_ne_bytes());
        let tx_data: &[u8] = b"steal-payload";
        let tx_off = Vec::new();
        let mut bc3 = Vec::with_capacity(4 + 64);
        bc3.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc3.extend_from_slice(&tx_b);
        let payload3 = make_v2_write_read_multi_payload(&bc3, &[(tx_data, &tx_off)], 4096);
        let (ret3, resp3) = exchange(&mut stream_b, BINDER_WRITE_READ, &payload3);
        assert_eq!(ret3, 0);
        assert_eq!(
            u32::from_ne_bytes(resp3[4..8].try_into().unwrap()),
            BR_TRANSACTION_COMPLETE,
            "6-Z271i: B's transaction ioctl completes with COMPLETE only"
        );
        // B now parks WITHOUT reading (its reply is pending).

        // ---- Conn A2 (pid 7777, SIBLING): read-only ioctl STEALS ----
        let mut stream_a2 = UnixStream::connect(&path).expect("connect A2");
        let (ret_i3, _r3) = exchange(&mut stream_a2, WIRE_CMD_IDENT, &ident_payload(7777));
        assert_eq!(ret_i3, 0);
        let mut wr_a2 = Vec::new();
        wr_a2.extend_from_slice(&0u32.to_ne_bytes());
        wr_a2.extend_from_slice(&4096u32.to_ne_bytes());
        let (ret_a2, resp_a2) = exchange(&mut stream_a2, BINDER_WRITE_READ, &wr_a2);
        assert_eq!(ret_a2, 0);
        let br = u32::from_ne_bytes(resp_a2[4..8].try_into().unwrap());
        assert_eq!(
            br, BR_TRANSACTION,
            "sibling conn receives the transaction (steal)"
        );
        let tr = &resp_a2[8..8 + 64];
        let code = u32::from_ne_bytes(tr[16..20].try_into().unwrap());
        assert_eq!(code, 42, "stolen delivery carries the code");
        let cookie = u64::from_ne_bytes(tr[8..16].try_into().unwrap());
        assert_eq!(cookie, 0x5678, "owner cookie preserved across steal");

        // ---- Conn A2: BC_REPLY → B resolves ----
        let reply_data: &[u8] = b"stolen-reply";
        let mut reply = [0u8; 64];
        reply[16..20].copy_from_slice(&0u32.to_ne_bytes());
        let mut bc4 = Vec::with_capacity(4 + 64);
        bc4.extend_from_slice(&BC_REPLY.to_ne_bytes());
        bc4.extend_from_slice(&reply);
        let payload4 = make_v2_write_read_multi_payload(&bc4, &[(reply_data, &tx_off)], 0);
        let (ret_r, _resp_r) = exchange(&mut stream_a2, BINDER_WRITE_READ, &payload4);
        assert_eq!(ret_r, 0);

        // ---- Conn B: read-only ioctl → the deferred BR_REPLY ----
        let mut wr_b = Vec::new();
        wr_b.extend_from_slice(&0u32.to_ne_bytes());
        wr_b.extend_from_slice(&4096u32.to_ne_bytes());
        let (ret_b, resp_b) = exchange(&mut stream_b, BINDER_WRITE_READ, &wr_b);
        assert_eq!(ret_b, 0);
        let read_b = u32::from_ne_bytes(resp_b[0..4].try_into().unwrap()) as usize;
        let br2 = u32::from_ne_bytes(resp_b[4..8].try_into().unwrap());
        assert_eq!(
            br2, BR_REPLY,
            "B gets the stolen conn's reply on its own read"
        );
        let off_b = 4 + read_b + 8;
        let dl_b = u32::from_ne_bytes(resp_b[off_b..off_b + 4].try_into().unwrap()) as usize;
        assert_eq!(dl_b, reply_data.len());
        let got = &resp_b[off_b + 8..off_b + 8 + dl_b];
        assert_eq!(got, reply_data, "reply bytes exact");

        drop(stream_a);
        drop(stream_b);
        drop(stream_a2);
        drop(_handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn z271i_self_transaction_same_conn_services_own_request() {
        // keystore2's km_compat chain (run 33428365193 decode): a process
        // registers android.security.compat and then transacts on it from
        // ITS OWN process. Kernel semantics: the transaction ioctl
        // returns BR_TRANSACTION_COMPLETE; the SAME connection pops the
        // BR_TRANSACTION on its next ioctl, services it, and its
        // BC_REPLY resolves the original call INTO ITS OWN reply queue.
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let _handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));
        let mut stream = UnixStream::connect(&path).expect("connect");

        // ---- addService("self_svc") + getService → own handle ----
        let mut args = ParcelWriter::new();
        args.write_string16("self_svc");
        args.write_flat_binder(&FlatBinderObject {
            r#type: BINDER_TYPE_BINDER,
            flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
            binder: 0xaaaa,
            cookie: 0xbeef,
        });
        args.write_i32(0);
        args.write_i32(0);
        let (ad, ao) = make_servicemanager_request_parcel(&mut args);
        let mut bc = Vec::with_capacity(4 + 64);
        bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_ADD_SERVICE, 0));
        let payload = make_v2_write_read_payload(&bc, &ad, &ao, 4096);
        let (ret, _resp) = exchange(&mut stream, BINDER_WRITE_READ, &payload);
        assert_eq!(ret, 0, "ADD_SERVICE ok");

        let mut args2 = ParcelWriter::new();
        args2.write_string16("self_svc");
        let (gd, go) = make_servicemanager_request_parcel(&mut args2);
        let mut bc2 = Vec::with_capacity(4 + 64);
        bc2.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc2.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_GET_SERVICE, 0));
        let payload2 = make_v2_write_read_payload(&bc2, &gd, &go, 4096);
        let (ret2, resp2) = exchange(&mut stream, BINDER_WRITE_READ, &payload2);
        assert_eq!(ret2, 0);
        let rs2 = u32::from_ne_bytes(resp2[0..4].try_into().unwrap()) as usize;
        let o2 = 4 + rs2 + 8;
        let dl2 = u32::from_ne_bytes(resp2[o2..o2 + 4].try_into().unwrap()) as usize;
        let blob2 = &resp2[o2 + 8..o2 + 8 + dl2];
        let self_handle = u64::from_ne_bytes(blob2[12..20].try_into().unwrap()) as u32;
        assert_eq!(
            self_handle,
            PROXY_HANDLE_BASE + 5,
            "own service handle (after the 4 virtual services)"
        );

        // ---- transact(code=7) on the OWN handle ----
        let mut tx = [0u8; 64];
        tx[0..4].copy_from_slice(&self_handle.to_ne_bytes());
        tx[16..20].copy_from_slice(&7u32.to_ne_bytes());
        let req: &[u8] = b"self-req";
        let no_off: Vec<u8> = Vec::new();
        let mut bc3 = Vec::with_capacity(4 + 64);
        bc3.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc3.extend_from_slice(&tx);
        let payload3 = make_v2_write_read_multi_payload(&bc3, &[(req, &no_off)], 4096);
        let (ret3, resp3) = exchange(&mut stream, BINDER_WRITE_READ, &payload3);
        assert_eq!(ret3, 0);
        assert_eq!(
            u32::from_ne_bytes(resp3[4..8].try_into().unwrap()),
            BR_TRANSACTION_COMPLETE,
            "self-transaction is ACCEPTED (was a hard FAIL pre-6-Z271i)"
        );
        assert_eq!(
            u32::from_ne_bytes(resp3[0..4].try_into().unwrap()) as usize,
            4
        );

        // ---- same conn: read-only ioctl → its OWN BR_TRANSACTION ----
        let mut wr = Vec::new();
        wr.extend_from_slice(&0u32.to_ne_bytes());
        wr.extend_from_slice(&4096u32.to_ne_bytes());
        let (ret4, resp4) = exchange(&mut stream, BINDER_WRITE_READ, &wr);
        assert_eq!(ret4, 0);
        assert_eq!(
            u32::from_ne_bytes(resp4[4..8].try_into().unwrap()),
            BR_TRANSACTION,
            "the same connection receives its own request"
        );
        let tr = &resp4[8..8 + 64];
        assert_eq!(
            u32::from_ne_bytes(tr[16..20].try_into().unwrap()),
            7,
            "own code delivered"
        );
        assert_eq!(
            u64::from_ne_bytes(tr[8..16].try_into().unwrap()),
            0xbeef,
            "own cookie delivered"
        );
        assert_eq!(
            u32::from_ne_bytes(tr[0..4].try_into().unwrap()),
            0xaaaa,
            "own ptr delivered"
        );

        // ---- same conn: BC_REPLY → resolved by the SAME ioctl ----
        let rep: &[u8] = b"self-rep";
        let reply = [0u8; 64];
        let mut bc5 = Vec::with_capacity(4 + 64);
        bc5.extend_from_slice(&BC_REPLY.to_ne_bytes());
        bc5.extend_from_slice(&reply);
        let payload5 = make_v2_write_read_multi_payload(&bc5, &[(rep, &no_off)], 4096);
        let (ret5, resp5) = exchange(&mut stream, BINDER_WRITE_READ, &payload5);
        assert_eq!(ret5, 0);
        assert_eq!(
            u32::from_ne_bytes(resp5[4..8].try_into().unwrap()),
            BR_REPLY,
            "self BC_REPLY answered in the same ioctl (reply queue drained)"
        );
        let rs5 = u32::from_ne_bytes(resp5[0..4].try_into().unwrap()) as usize;
        let o5 = 4 + rs5 + 8;
        let dl5 = u32::from_ne_bytes(resp5[o5..o5 + 4].try_into().unwrap()) as usize;
        assert_eq!(dl5, rep.len());
        assert_eq!(&resp5[o5 + 8..o5 + 8 + dl5], rep, "own reply bytes exact");

        drop(stream);
        drop(_handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn z271i_nested_transaction_does_not_clobber_outer_reply() {
        // A services B's call; WHILE still owing B its BC_REPLY, A makes
        // a nested sync call to C's service. The nested reply must not
        // swallow A's outstanding outer transaction (its inflight_txn),
        // and A's later BC_REPLY still resolves B's original call.
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let _handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));

        // Helper: register a service on its own connection.
        let add_service = |name: &str, cookie: u64| -> UnixStream {
            let mut s = UnixStream::connect(&path).expect("connect");
            let mut w = ParcelWriter::new();
            w.write_string16(name);
            w.write_flat_binder(&FlatBinderObject {
                r#type: BINDER_TYPE_BINDER,
                flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                binder: 0x1111,
                cookie,
            });
            w.write_i32(0);
            w.write_i32(0);
            let (d, o) = make_servicemanager_request_parcel(&mut w);
            let mut bc = Vec::with_capacity(4 + 64);
            bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
            bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_ADD_SERVICE, 0));
            let p = make_v2_write_read_payload(&bc, &d, &o, 4096);
            let (r, _) = exchange(&mut s, BINDER_WRITE_READ, &p);
            assert_eq!(r, 0, "addService({name})");
            s
        };
        // Helper: getService from a connection, return the handle.
        let get_service = |s: &mut UnixStream, name: &str| -> u32 {
            let mut w = ParcelWriter::new();
            w.write_string16(name);
            let (d, o) = make_servicemanager_request_parcel(&mut w);
            let mut bc = Vec::with_capacity(4 + 64);
            bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
            bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_GET_SERVICE, 0));
            let p = make_v2_write_read_payload(&bc, &d, &o, 4096);
            let (r, resp) = exchange(s, BINDER_WRITE_READ, &p);
            assert_eq!(r, 0);
            let rs = u32::from_ne_bytes(resp[0..4].try_into().unwrap()) as usize;
            let off = 4 + rs + 8;
            let dl = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap()) as usize;
            let blob = &resp[off + 8..off + 8 + dl];
            u64::from_ne_bytes(blob[12..20].try_into().unwrap()) as u32
        };
        let read_only = |s: &mut UnixStream| -> Vec<u8> {
            let mut wr = Vec::new();
            wr.extend_from_slice(&0u32.to_ne_bytes());
            wr.extend_from_slice(&4096u32.to_ne_bytes());
            let (r, resp) = exchange(s, BINDER_WRITE_READ, &wr);
            assert_eq!(r, 0);
            resp
        };
        let transact = |s: &mut UnixStream, handle: u32, code: u32| -> Vec<u8> {
            let mut tx = [0u8; 64];
            tx[0..4].copy_from_slice(&handle.to_ne_bytes());
            tx[16..20].copy_from_slice(&code.to_ne_bytes());
            let mut bc = Vec::with_capacity(4 + 64);
            bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
            bc.extend_from_slice(&tx);
            let p = make_v2_write_read_multi_payload(&bc, &[(&[], &[])], 4096);
            let (r, resp) = exchange(s, BINDER_WRITE_READ, &p);
            assert_eq!(r, 0);
            resp
        };
        let send_reply = |s: &mut UnixStream, payload: &[u8]| {
            let reply = [0u8; 64];
            let mut bc = Vec::with_capacity(4 + 64);
            bc.extend_from_slice(&BC_REPLY.to_ne_bytes());
            bc.extend_from_slice(&reply);
            let p = make_v2_write_read_multi_payload(&bc, &[(payload, &[])], 0);
            let (r, _) = exchange(s, BINDER_WRITE_READ, &p);
            assert_eq!(r, 0);
        };

        let mut conn_a = add_service("svc_a", 0xaaaa);
        let mut conn_b = UnixStream::connect(&path).expect("connect B");
        let mut conn_c = add_service("svc_c", 0xcccc);

        // ---- B calls svc_a (code 9) — parks with COMPLETE ----
        let h_a = get_service(&mut conn_b, "svc_a");
        let resp_b1 = transact(&mut conn_b, h_a, 9);
        assert_eq!(
            u32::from_ne_bytes(resp_b1[4..8].try_into().unwrap()),
            BR_TRANSACTION_COMPLETE
        );

        // ---- A pops the outer request (inflight = outer txn) ----
        let resp_a1 = read_only(&mut conn_a);
        assert_eq!(
            u32::from_ne_bytes(resp_a1[4..8].try_into().unwrap()),
            BR_TRANSACTION,
            "A receives B's outer request"
        );

        // ---- A makes a NESTED call to svc_c (code 11) ----
        let h_c = get_service(&mut conn_a, "svc_c");
        let resp_a2 = transact(&mut conn_a, h_c, 11);
        assert_eq!(
            u32::from_ne_bytes(resp_a2[4..8].try_into().unwrap()),
            BR_TRANSACTION_COMPLETE,
            "nested call accepted while A still owes the outer reply"
        );

        // ---- C pops the nested request and replies ----
        let resp_c1 = read_only(&mut conn_c);
        assert_eq!(
            u32::from_ne_bytes(resp_c1[4..8].try_into().unwrap()),
            BR_TRANSACTION,
            "C receives A's nested request"
        );
        let tr_c = &resp_c1[8..8 + 64];
        assert_eq!(u32::from_ne_bytes(tr_c[16..20].try_into().unwrap()), 11);
        send_reply(&mut conn_c, b"nested-rep");

        // ---- A's next read: the NESTED reply, outer txn intact ----
        let resp_a3 = read_only(&mut conn_a);
        assert_eq!(
            u32::from_ne_bytes(resp_a3[4..8].try_into().unwrap()),
            BR_REPLY,
            "A gets the nested reply"
        );

        // ---- A now answers the OUTER call — B resolves ----
        send_reply(&mut conn_a, b"outer-rep");
        let resp_b2 = read_only(&mut conn_b);
        assert_eq!(
            u32::from_ne_bytes(resp_b2[4..8].try_into().unwrap()),
            BR_REPLY,
            "B gets the outer reply after A's nested call round-trip"
        );
        let rs_b = u32::from_ne_bytes(resp_b2[0..4].try_into().unwrap()) as usize;
        let o_b = 4 + rs_b + 8;
        let dl_b = u32::from_ne_bytes(resp_b2[o_b..o_b + 4].try_into().unwrap()) as usize;
        assert_eq!(
            &resp_b2[o_b + 8..o_b + 8 + dl_b],
            b"outer-rep",
            "outer bytes exact"
        );

        drop(conn_a);
        drop(conn_b);
        drop(conn_c);
        drop(_handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn z271i_reply_timeout_resolves_as_br_failed_reply() {
        // A routed sync transaction whose owner never reads must resolve
        // on the requester's read after the bounded REPLY_TIMEOUT — the
        // requester is never wedged forever.
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let _handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));

        // Owner parks forever (never reads).
        let mut stream_a = UnixStream::connect(&path).expect("connect A");
        let mut w = ParcelWriter::new();
        w.write_string16("dead_svc");
        w.write_flat_binder(&FlatBinderObject {
            r#type: BINDER_TYPE_BINDER,
            flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
            binder: 0x2222,
            cookie: 0x3333,
        });
        w.write_i32(0);
        w.write_i32(0);
        let (d, o) = make_servicemanager_request_parcel(&mut w);
        let mut bc = Vec::with_capacity(4 + 64);
        bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_ADD_SERVICE, 0));
        let p = make_v2_write_read_payload(&bc, &d, &o, 4096);
        let (r, _) = exchange(&mut stream_a, BINDER_WRITE_READ, &p);
        assert_eq!(r, 0);

        // Requester: getService + transact, then wait past the budget.
        let mut stream_b = UnixStream::connect(&path).expect("connect B");
        let mut w2 = ParcelWriter::new();
        w2.write_string16("dead_svc");
        let (d2, o2) = make_servicemanager_request_parcel(&mut w2);
        let mut bc2 = Vec::with_capacity(4 + 64);
        bc2.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc2.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_GET_SERVICE, 0));
        let p2 = make_v2_write_read_payload(&bc2, &d2, &o2, 4096);
        let (r2, resp2) = exchange(&mut stream_b, BINDER_WRITE_READ, &p2);
        assert_eq!(r2, 0);
        let rs2 = u32::from_ne_bytes(resp2[0..4].try_into().unwrap()) as usize;
        let o3 = 4 + rs2 + 8;
        let dl2 = u32::from_ne_bytes(resp2[o3..o3 + 4].try_into().unwrap()) as usize;
        let blob2 = &resp2[o3 + 8..o3 + 8 + dl2];
        let h = u64::from_ne_bytes(blob2[12..20].try_into().unwrap()) as u32;

        let mut tx = [0u8; 64];
        tx[0..4].copy_from_slice(&h.to_ne_bytes());
        tx[16..20].copy_from_slice(&5u32.to_ne_bytes());
        let mut bc3 = Vec::with_capacity(4 + 64);
        bc3.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc3.extend_from_slice(&tx);
        let p3 = make_v2_write_read_multi_payload(&bc3, &[(&[], &[])], 4096);
        let (r3, _resp3) = exchange(&mut stream_b, BINDER_WRITE_READ, &p3);
        assert_eq!(r3, 0);

        // Sleep past REPLY_TIMEOUT (with slack), then read.
        std::thread::sleep(REPLY_TIMEOUT + Duration::from_millis(300));
        let mut wr = Vec::new();
        wr.extend_from_slice(&0u32.to_ne_bytes());
        wr.extend_from_slice(&4096u32.to_ne_bytes());
        let (r4, resp4) = exchange(&mut stream_b, BINDER_WRITE_READ, &wr);
        assert_eq!(r4, 0);
        assert_eq!(
            u32::from_ne_bytes(resp4[4..8].try_into().unwrap()),
            BR_FAILED_REPLY,
            "the expired sync call resolves as BR_FAILED_REPLY"
        );

        drop(stream_a);
        drop(stream_b);
        drop(_handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn z271i_death_notification_and_teardown_resolve_callers() {
        // §15 death/teardown coverage:
        //  (a) a watcher with BC_REQUEST_DEATH_NOTIFICATION receives
        //      [BR_DEAD_BINDER][cookie] when the owning conn disconnects;
        //  (b) a caller whose sync transaction is queued on a dying
        //      server's inbox gets [BR_FAILED_REPLY] instead of hanging
        //      out its full REPLY_TIMEOUT.
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let _handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));

        let add_service = |name: &str, cookie: u64| -> UnixStream {
            let mut s = UnixStream::connect(&path).expect("connect");
            let mut w = ParcelWriter::new();
            w.write_string16(name);
            w.write_flat_binder(&FlatBinderObject {
                r#type: BINDER_TYPE_BINDER,
                flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                binder: 0x4444,
                cookie,
            });
            w.write_i32(0);
            w.write_i32(0);
            let (d, o) = make_servicemanager_request_parcel(&mut w);
            let mut bc = Vec::with_capacity(4 + 64);
            bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
            bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_ADD_SERVICE, 0));
            let p = make_v2_write_read_payload(&bc, &d, &o, 4096);
            let (r, _) = exchange(&mut s, BINDER_WRITE_READ, &p);
            assert_eq!(r, 0);
            s
        };
        let get_service = |s: &mut UnixStream, name: &str| -> u32 {
            let mut w = ParcelWriter::new();
            w.write_string16(name);
            let (d, o) = make_servicemanager_request_parcel(&mut w);
            let mut bc = Vec::with_capacity(4 + 64);
            bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
            bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_GET_SERVICE, 0));
            let p = make_v2_write_read_payload(&bc, &d, &o, 4096);
            let (r, resp) = exchange(s, BINDER_WRITE_READ, &p);
            assert_eq!(r, 0);
            let rs = u32::from_ne_bytes(resp[0..4].try_into().unwrap()) as usize;
            let off = 4 + rs + 8;
            let dl = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap()) as usize;
            let blob = &resp[off + 8..off + 8 + dl];
            u64::from_ne_bytes(blob[12..20].try_into().unwrap()) as u32
        };
        let read_only = |s: &mut UnixStream| -> Vec<u8> {
            let mut wr = Vec::new();
            wr.extend_from_slice(&0u32.to_ne_bytes());
            wr.extend_from_slice(&4096u32.to_ne_bytes());
            let (r, resp) = exchange(s, BINDER_WRITE_READ, &wr);
            assert_eq!(r, 0);
            resp
        };

        // ---- (b) queued sync work resolved on server teardown ----
        let conn_srv = add_service("doomed_svc", 0xdead);
        let mut conn_cli = UnixStream::connect(&path).expect("connect cli");
        let h_doomed = get_service(&mut conn_cli, "doomed_svc");
        let mut tx = [0u8; 64];
        tx[0..4].copy_from_slice(&h_doomed.to_ne_bytes());
        tx[16..20].copy_from_slice(&3u32.to_ne_bytes());
        let mut bc = Vec::with_capacity(4 + 64);
        bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc.extend_from_slice(&tx);
        let p_tx = make_v2_write_read_multi_payload(&bc, &[(&[], &[])], 4096);
        let (r_tx, resp_tx) = exchange(&mut conn_cli, BINDER_WRITE_READ, &p_tx);
        assert_eq!(r_tx, 0);
        assert_eq!(
            u32::from_ne_bytes(resp_tx[4..8].try_into().unwrap()),
            BR_TRANSACTION_COMPLETE,
            "sync call parked (queued on the doomed server's inbox)"
        );
        // Server dies WITHOUT reading the queued work.
        drop(conn_srv);
        std::thread::sleep(Duration::from_millis(120));
        let resp_fail = read_only(&mut conn_cli);
        assert_eq!(
            u32::from_ne_bytes(resp_fail[4..8].try_into().unwrap()),
            BR_FAILED_REPLY,
            "teardown resolves the queued sync call as BR_FAILED_REPLY"
        );

        // ---- (a) death notification for a watcher ----
        let conn_srv2 = add_service("watched_svc", 0xcafe);
        let mut conn_watch = UnixStream::connect(&path).expect("connect watcher");
        let h_watched = get_service(&mut conn_watch, "watched_svc");
        // BC_REQUEST_DEATH_NOTIFICATION: [cmd][handle u32][cookie u64].
        let mut dn = Vec::with_capacity(4 + 12);
        dn.extend_from_slice(&BC_REQUEST_DEATH_NOTIFICATION.to_ne_bytes());
        dn.extend_from_slice(&h_watched.to_ne_bytes());
        dn.extend_from_slice(&0x1234_5678_9abc_def0u64.to_ne_bytes());
        let p_dn = make_v2_write_read_multi_payload(&dn, &[], 4096);
        let (r_dn, _resp_dn) = exchange(&mut conn_watch, BINDER_WRITE_READ, &p_dn);
        assert_eq!(r_dn, 0, "death notification request accepted");
        // Owner dies → watcher's next read gets [BR_DEAD_BINDER][cookie].
        drop(conn_srv2);
        std::thread::sleep(Duration::from_millis(120));
        let resp_death = read_only(&mut conn_watch);
        assert_eq!(
            u32::from_ne_bytes(resp_death[4..8].try_into().unwrap()),
            BR_DEAD_BINDER,
            "watcher receives BR_DEAD_BINDER"
        );
        let cookie = u64::from_ne_bytes(resp_death[8..16].try_into().unwrap());
        assert_eq!(
            cookie, 0x1234_5678_9abc_def0,
            "the requested death cookie comes back verbatim"
        );

        drop(conn_cli);
        drop(conn_watch);
        drop(_handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn z271_virtual_vibrator_gets_registered_and_answers_get_service() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));
        let mut stream = UnixStream::connect(&path).expect("connect");

        // getService("android.hardware.vibrator.IVibrator/default") must
        // HIT (pre-6-Z271 this burned a ~5 s waitForService per tap).
        let mut args = ParcelWriter::new();
        args.write_string16("android.hardware.vibrator.IVibrator/default");
        let (d, o) = make_servicemanager_request_parcel(&mut args);
        let mut bc = Vec::with_capacity(4 + 64);
        bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_CHECK_SERVICE, 0));
        let payload = make_v2_write_read_payload(&bc, &d, &o, 4096);
        let (ret, resp) = exchange(&mut stream, BINDER_WRITE_READ, &payload);
        assert_eq!(ret, 0);
        let read_size = u32::from_ne_bytes(resp[0..4].try_into().unwrap()) as usize;
        let off = 4 + read_size + 8;
        let dl = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap()) as usize;
        assert_eq!(
            dl,
            4 + 24 + 4,
            "hit reply = status + flat handle + stability (6-Z271x)"
        );
        let blob = &resp[off + 8..off + 8 + dl];
        let flat_type = u32::from_ne_bytes(blob[4..8].try_into().unwrap());
        assert_eq!(flat_type, BINDER_TYPE_HANDLE, "virtual service HIT");
        let vhandle = u64::from_ne_bytes(blob[12..20].try_into().unwrap()) as u32;
        assert_eq!(vhandle, PROXY_HANDLE_BASE + 1, "vibrator = first virtual");

        // Transact code 1 (getCapabilities) → [EX_NONE][caps=0].
        let mut tx = [0u8; 64];
        tx[0..4].copy_from_slice(&vhandle.to_ne_bytes());
        tx[16..20].copy_from_slice(&1u32.to_ne_bytes());
        let tx_data: &[u8] = &[];
        let mut bc2 = Vec::with_capacity(4 + 64);
        bc2.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc2.extend_from_slice(&tx);
        let payload2 = make_v2_write_read_multi_payload(&bc2, &[(tx_data, &[])], 4096);
        let (ret2, resp2) = exchange(&mut stream, BINDER_WRITE_READ, &payload2);
        assert_eq!(ret2, 0);
        let read2 = u32::from_ne_bytes(resp2[0..4].try_into().unwrap()) as usize;
        let off2 = 4 + read2 + 8;
        let dl2 = u32::from_ne_bytes(resp2[off2..off2 + 4].try_into().unwrap()) as usize;
        let blob2 = &resp2[off2 + 8..off2 + 8 + dl2];
        assert_eq!(dl2, 8, "getCapabilities reply = status + i32 caps");
        let status = i32::from_ne_bytes(blob2[0..4].try_into().unwrap());
        let caps = i32::from_ne_bytes(blob2[4..8].try_into().unwrap());
        assert_eq!(status, 0, "EX_NONE");
        assert_eq!(caps, 0, "caps = 0 (plain on/off only)");

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    /// 6-Z271x: THE android-12+ binder stability annotation. The reply the
    /// client's `unflattenBinder` walks must be
    /// `[i32 EX_NONE][flat_binder_object @ off 4][i32 stability]` —
    /// `finishUnflattenBinder` reads the i32 AFTER the flat and
    /// `Stability::setRepr` aborts the parse (BAD_TYPE → null binder →
    /// keystore2 NAME_NOT_FOUND, the 6-Z271w chain) when it is missing or
    /// undeclared-for-non-null. Byte-verified here for the getService hit
    /// (VINTF 63) and miss (UNDECLARED 0) shapes end-to-end over the v2
    /// wire, mirroring the android-13 Parcel.cpp readObject walk (object
    /// table entry at the read position, 24-byte flat, trailing i32).
    #[test]
    fn z271x_sm_reply_carries_binder_stability_annotation() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));
        let mut stream = UnixStream::connect(&path).expect("connect");

        // ---- HIT: getService a name the legacy-path proxy will answer.
        // Use a virtual service name (always registered at proxy start).
        let mut args = ParcelWriter::new();
        args.write_string16("android.hardware.vibrator.IVibrator/default");
        let (d, o) = make_servicemanager_request_parcel(&mut args);
        let mut bc = Vec::with_capacity(4 + 64);
        bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_CHECK_SERVICE, 0));
        let payload = make_v2_write_read_payload(&bc, &d, &o, 4096);
        let (ret, resp) = exchange(&mut stream, BINDER_WRITE_READ, &payload);
        assert_eq!(ret, 0);
        let read_size = u32::from_ne_bytes(resp[0..4].try_into().unwrap()) as usize;
        let off = 4 + read_size + 8;
        let dlen = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap()) as usize;
        assert_eq!(dlen, 32, "hit blob = EX_NONE(4) + flat(24) + stability(4)");
        let blob = &resp[off + 8..off + 8 + dlen];
        // Client walk 1: Status::readFromParcel consumes EX_NONE at 0.
        assert_eq!(
            i32::from_ne_bytes(blob[0..4].try_into().unwrap()),
            0,
            "EX_NONE"
        );
        // Client walk 2: readObject(false) at DPOS=4 — flat must be listed
        // in the offsets table AT the read position, and read 24 bytes.
        let flat_type = u32::from_ne_bytes(blob[4..8].try_into().unwrap());
        assert_eq!(flat_type, BINDER_TYPE_HANDLE);
        let flat_handle = u64::from_ne_bytes(blob[12..20].try_into().unwrap()) as u32;
        assert_eq!(
            flat_handle,
            PROXY_HANDLE_BASE + 1,
            "vibrator = first virtual"
        );
        // Client walk 3: finishUnflattenBinder's readInt32 — the A12
        // Category form on the FIRST get of a connection (6-Z272e:
        // level<<24 | version 1; the A12 client decodes level=VINTF,
        // version=1 ≥ kBinderWireFormatOldest).
        let stability = i32::from_ne_bytes(blob[28..32].try_into().unwrap());
        assert_eq!(
            stability, STABILITY_ANNOTATION_VINTF_A12,
            "first get → A12 Category form (0x3F000001)"
        );
        // isDeclaredLevel semantics: the level byte must be VINTF (0x3F)
        // and the version byte ≥ 1, or the A12 client rejects the reply.
        assert_eq!(stability >> 24, 0b1111_11, "level byte = VINTF");
        assert_eq!(stability & 0xFF, 1, "wire version byte = 1");

        // ---- MISS: the null-binder reply carries the Category null
        // (different service name → no format flip on this fresh conn).
        let mut args2 = ParcelWriter::new();
        args2.write_string16("does_not_exist");
        let (d2, o2) = make_servicemanager_request_parcel(&mut args2);
        let mut bc2 = Vec::with_capacity(4 + 64);
        bc2.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc2.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_GET_SERVICE, 0));
        let payload2 = make_v2_write_read_payload(&bc2, &d2, &o2, 4096);
        let (ret2, resp2) = exchange(&mut stream, BINDER_WRITE_READ, &payload2);
        assert_eq!(ret2, 0);
        let read2 = u32::from_ne_bytes(resp2[0..4].try_into().unwrap()) as usize;
        let off2 = 4 + read2 + 8;
        let dlen2 = u32::from_ne_bytes(resp2[off2..off2 + 4].try_into().unwrap()) as usize;
        assert_eq!(
            dlen2, 32,
            "miss blob = EX_NONE(4) + null flat(24) + stability(4)"
        );
        let blob2 = &resp2[off2 + 8..off2 + 8 + dlen2];
        let flat_type2 = u32::from_ne_bytes(blob2[4..8].try_into().unwrap());
        assert_eq!(flat_type2, BINDER_TYPE_BINDER, "miss → null binder");
        let null_stability = i32::from_ne_bytes(blob2[28..32].try_into().unwrap());
        assert_eq!(
            null_stability, STABILITY_ANNOTATION_NULL_A12,
            "null binders carry the Category null (version 1, level 0)"
        );

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    /// 6-Z271z: the virtual-service arg decode must consume the AIDL
    /// interface-token header FIRST. A real android-13 client's
    /// `IVibrator.on(5000)` request opens with
    /// `[i32 strict][i32 work][i32 tag][string16 descriptor]` before the
    /// `i32 timeoutMs` arg — before this fix the dispatch fed the raw
    /// parcel to `virtual_vibrator`, which read the strict-mode word as
    /// the timeout (real clients write 0/-1 → EX_UNSUPPORTED_OPERATION →
    /// the host vibration silently never fired). Unreachable until
    /// 6-Z271x (no client could parse our SM replies), so never observed
    /// in CI. Observable via the reply: on(5000) → EX_NONE (+ host
    /// forward), on(0) → EX_UNSUPPORTED_OPERATION.
    #[test]
    fn z271z_aidl_request_header_is_consumed_before_virtual_args() {
        let build_on_request = |timeout_ms: i32| -> RequestBlob {
            let mut req = ParcelWriter::new();
            req.write_i32(0); // strict_mode_policy
            req.write_i32(-1); // work_source_uid (kUnsetWorkSource)
            req.write_u32(AIDL_HEADER_TAG_SYST);
            req.write_string16("android.hardware.vibrator.IVibrator");
            req.write_i32(timeout_ms); // on(in int timeoutMs)
            let (data, offsets) = req.into_parts();
            RequestBlob { data, offsets }
        };
        // on(5000) with the real client shape → forwarded to the host.
        match virtual_service_transaction(
            VirtualService::Vibrator,
            3,
            Some(&build_on_request(5000)),
        ) {
            TransactionResult::Reply { data, .. } => {
                let status = i32::from_ne_bytes(data[0..4].try_into().unwrap());
                assert_eq!(
                    status, 0,
                    "on(5000) with a real interface-token header must forward \
                     the timeout (EX_NONE), not misread the header as args"
                );
            }
            _ => panic!("on(5000) must reply"),
        }
        // Degenerate timeout still refuses honestly (arg decode intact).
        match virtual_service_transaction(VirtualService::Vibrator, 3, Some(&build_on_request(0))) {
            TransactionResult::Reply { data, .. } => {
                let status = i32::from_ne_bytes(data[0..4].try_into().unwrap());
                assert_eq!(
                    status, EX_UNSUPPORTED_OPERATION,
                    "on(0) → EX_UNSUPPORTED_OPERATION proves the arg decode \
                     consumed the header (0 is no longer the header word)"
                );
            }
            _ => panic!("on(0) must still reply"),
        }
        // Legacy v1 empty parcel (no blob): header read returns None,
        // dispatch still answers getCapabilities (code 1, no args).
        match virtual_service_transaction(VirtualService::Vibrator, 1, None) {
            TransactionResult::Reply { data, .. } => {
                assert_eq!(data.len(), 8, "getCapabilities reply = status + caps");
            }
            _ => panic!("getCapabilities must reply"),
        }
    }

    /// 6-Z272f: `IBinder::INTERFACE_TRANSACTION` ('_NTF') — the FIRST
    /// transaction every real client sends to a fresh proxy. The reply
    /// must be the BARE descriptor string16 (BBinder::onTransact
    /// default-case semantics — NO exception header), or the client's
    /// fromBinder/asInterface machinery fails one level before its first
    /// real call (keystore2's connect_keymint panic chain of run
    /// 33539861041: clients reached the services and got
    /// EX_UNSUPPORTED_OPERATION).
    #[test]
    fn z272f_interface_transaction_answers_descriptor() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));
        let mut stream = UnixStream::connect(&path).expect("connect");

        let mut iface_tx = |target: u32| -> (usize, Vec<u8>, usize, Vec<u8>) {
            let mut tx = [0u8; 64];
            tx[0..4].copy_from_slice(&target.to_ne_bytes());
            tx[16..20].copy_from_slice(&INTERFACE_TRANSACTION.to_ne_bytes());
            let tx_data: &[u8] = &[];
            let mut bc = Vec::with_capacity(4 + 64);
            bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
            bc.extend_from_slice(&tx);
            let payload = make_v2_write_read_multi_payload(&bc, &[(tx_data, &[])], 4096);
            let (ret, resp) = exchange(&mut stream, BINDER_WRITE_READ, &payload);
            assert_eq!(ret, 0);
            let read_size = u32::from_ne_bytes(resp[0..4].try_into().unwrap()) as usize;
            let off = 4 + read_size + 8;
            let dlen = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap()) as usize;
            let olen = u32::from_ne_bytes(resp[off + 4..off + 8].try_into().unwrap()) as usize;
            let blob = resp[off + 8..off + 8 + dlen + olen].to_vec();
            (dlen, blob[..dlen].to_vec(), olen, blob[dlen..].to_vec())
        };

        // Virtual vibrator handle: bare descriptor string16, no offsets.
        let vh = PROXY_HANDLE_BASE + 1;
        let (dlen, data, olen, _offs) = iface_tx(vh);
        assert!(olen == 0, "descriptor reply has no binder objects");
        let mut r = ParcelReader::new(&data);
        let desc = r
            .read_string16()
            .expect("descriptor string16")
            .expect("descriptor non-null");
        assert_eq!(desc, "android.hardware.vibrator.IVibrator");
        assert_eq!(r.remaining(), 0, "no trailing bytes (no EX_NONE header)");
        let _ = dlen;

        // The context manager answers its own descriptor.
        let (_d2, data2, o2, _o2) = iface_tx(SVC_MGR_HANDLE);
        assert_eq!(o2, 0);
        let mut r2 = ParcelReader::new(&data2);
        let desc2 = r2
            .read_string16()
            .expect("sm descriptor")
            .expect("non-null");
        assert_eq!(desc2, SVC_MGR_IFACE_DESCRIPTOR);

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    /// 6-Z272e: the per-connection annotation format SELF-TUNES. A
    /// same-service re-get inside the window = the waitForService retry
    /// signature of a reply the client failed to parse → the format
    /// flips to the plain android-11/13+ level (sticky); a DIFFERENT
    /// service name does NOT flip (a working A12 client must never see
    /// the wrong format).
    #[test]
    fn z272e_annotation_flips_on_same_name_retry() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));
        let mut stream = UnixStream::connect(&path).expect("connect");

        let mut get = |name: &str| -> i32 {
            let mut args = ParcelWriter::new();
            args.write_string16(name);
            let (d, o) = make_servicemanager_request_parcel(&mut args);
            let mut bc = Vec::with_capacity(4 + 64);
            bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
            bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_CHECK_SERVICE, 0));
            let payload = make_v2_write_read_payload(&bc, &d, &o, 4096);
            let (ret, resp) = exchange(&mut stream, BINDER_WRITE_READ, &payload);
            assert_eq!(ret, 0);
            let read_size = u32::from_ne_bytes(resp[0..4].try_into().unwrap()) as usize;
            let off = 4 + read_size + 8;
            let dlen = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap()) as usize;
            assert_eq!(dlen, 32);
            let blob = &resp[off + 8..off + 8 + dlen];
            i32::from_ne_bytes(blob[28..32].try_into().unwrap())
        };

        // 1st get → A12 Category form.
        assert_eq!(
            get("android.hardware.vibrator.IVibrator/default"),
            STABILITY_ANNOTATION_VINTF_A12,
            "first get on a fresh conn = A12 Category"
        );
        // Real waitForService retry signature: the SAME service re-asked
        // ~100 ms later after the reply failed to parse → flip to the
        // plain android-11/13+ level.
        assert_eq!(
            get("android.hardware.vibrator.IVibrator/default"),
            STABILITY_ANNOTATION_VINTF,
            "same-name retry flips to the plain A11/A13+ level"
        );
        // The flip is sticky across different names (keystore2's compat
        // get after the keymint retries must stay plain).
        assert_eq!(
            get("android.hardware.security.keymint.IKeyMintDevice/default"),
            STABILITY_ANNOTATION_VINTF,
            "the flip is sticky across different names"
        );

        // A SECOND connection (a fresh client) starts over at Category:
        // different names must NOT flip a working A12 client.
        let mut stream2 = UnixStream::connect(&path).expect("connect 2");
        let mut get2 = |name: &str| -> i32 {
            let mut args = ParcelWriter::new();
            args.write_string16(name);
            let (d, o) = make_servicemanager_request_parcel(&mut args);
            let mut bc = Vec::with_capacity(4 + 64);
            bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
            bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_CHECK_SERVICE, 0));
            let payload = make_v2_write_read_payload(&bc, &d, &o, 4096);
            let (ret, resp) = exchange(&mut stream2, BINDER_WRITE_READ, &payload);
            assert_eq!(ret, 0);
            let read_size = u32::from_ne_bytes(resp[0..4].try_into().unwrap()) as usize;
            let off = 4 + read_size + 8;
            let dlen = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap()) as usize;
            let blob = &resp[off + 8..off + 8 + dlen];
            i32::from_ne_bytes(blob[28..32].try_into().unwrap())
        };
        assert_eq!(
            get2("android.hardware.vibrator.IVibrator/default"),
            STABILITY_ANNOTATION_VINTF_A12,
            "fresh conn starts at the A12 Category form"
        );
        assert_eq!(
            get2("android.hardware.security.keymint.IKeyMintDevice/default"),
            STABILITY_ANNOTATION_VINTF_A12,
            "different name does not flip the format"
        );

        drop(stream2);
        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    // ------------------------------------------------------------------
    // 6-Z272h — synthesized structured-parcelable + Status reply wires.
    //
    // Run 33543923394 (R12 lavender) pinned the last keystore2 stall:
    // the client RECEIVED the getHardwareInfo reply (80 B, freed via
    // BC_FREE_BUFFER) yet failed with "Binder exception code
    // TRANSACTION_FAILED, 0." — the reply violated the android-12+
    // STRUCTURED parcelable wire (no leading size i32, so keystore2's
    // `sized_read` read versionNumber=300 as the parcelable size and
    // bounded past the buffer → NOT_ENOUGH_DATA → EX_TRANSACTION_FAILED
    // via parse_exception_code/Status::from(status_t)). The
    // EX_SERVICE_SPECIFIC error replies were equally off-wire (the
    // message string16 + stack-trace word were missing entirely).
    // ------------------------------------------------------------------

    /// One sync transaction against a virtual service handle through a
    /// live proxy; returns the reply blob (data, offsets).
    fn z272h_virtual_tx(stream: &mut UnixStream, target: u32, code: u32) -> (Vec<u8>, Vec<u8>) {
        // Real AIDL request parcel: interface-token header (no args).
        let mut w = ParcelWriter::new();
        w.write_i32(0); // strict
        w.write_i32(-1); // work source
        w.write_u32(AIDL_HEADER_TAG_SYST);
        w.write_string16("android.hardware.security.keymint.IKeyMintDevice");
        let (d, o) = w.into_parts();
        let mut bc = Vec::with_capacity(4 + 64);
        bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        let mut tx = make_bc_transaction_payload(code, 0x10);
        tx[0..4].copy_from_slice(&target.to_ne_bytes());
        bc.extend_from_slice(&tx);
        let payload = make_v2_write_read_payload(&bc, &d, &o, 4096);
        let (ret, resp) = exchange(stream, BINDER_WRITE_READ, &payload);
        assert_eq!(ret, 0);
        let read_size = u32::from_ne_bytes(resp[0..4].try_into().unwrap()) as usize;
        let off = 4 + read_size + 8;
        let dlen = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap()) as usize;
        let olen = u32::from_ne_bytes(resp[off + 4..off + 8].try_into().unwrap()) as usize;
        let blob = resp[off + 8..off + 8 + dlen + olen].to_vec();
        (blob[..dlen].to_vec(), blob[dlen..].to_vec())
    }

    /// 6-Z272h: getHardwareInfo must be a SIZED structured parcelable —
    /// [EX_NONE][size (self-inclusive)][version=300][level=TEE][name][author][false]
    /// — exactly what keystore2's generated `sized_read` consumes.
    #[test]
    fn z272h_get_hardware_info_is_sized_structured_parcelable() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));
        let mut stream = UnixStream::connect(&path).expect("connect");

        let keymint = PROXY_HANDLE_BASE + 2;
        let (data, offsets) = z272h_virtual_tx(&mut stream, keymint, 1);
        assert!(offsets.is_empty(), "no binder objects in the info reply");

        let mut r = ParcelReader::new(&data);
        assert_eq!(r.read_i32(), Some(0), "EX_NONE status word");
        assert_eq!(
            r.read_i32(),
            Some(1),
            "parcelable null-flag word = NON_NULL_PARCELABLE_FLAG (read by \
             DeserializeOption::deserialize_option_from BEFORE read_from_parcel)"
        );
        let size = r.read_i32().expect("sized-parcelable size word");
        assert!(
            size >= 4,
            "size must cover itself (android sized_read contract)"
        );
        assert_eq!(
            size as usize,
            data.len() - 8,
            "size word spans itself + fields (flag word excluded), nothing more"
        );
        assert_eq!(r.read_i32(), Some(300), "versionNumber = KeyMint V3");
        assert_eq!(
            r.read_i32(),
            Some(SECURITY_LEVEL_TRUSTED_ENVIRONMENT),
            "securityLevel = TRUSTED_ENVIRONMENT (the android-13 variant \
             keystore2's mandatory-TEE construction accepts; -2 is not a \
             valid SecurityLevel at all)"
        );
        let name = r.read_string16().expect("name string").expect("non-null");
        assert_eq!(name, "TwoyiSoftwareKeyMint");
        let author = r.read_string16().expect("author").expect("non-null");
        assert_eq!(author, "twoyi");
        assert_eq!(r.read_i32(), Some(0), "timestampTokenRequired = false");
        assert_eq!(r.remaining(), 0, "reply fully consumed");

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    /// 6-Z272h: SharedSecretParameters must carry the structured
    /// parcelable size word too (keystore2's negotiation reads it with
    /// the same `sized_read`).
    #[test]
    fn z272h_sharedsecret_parameters_is_sized_structured_parcelable() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));
        let mut stream = UnixStream::connect(&path).expect("connect");

        let shared = PROXY_HANDLE_BASE + 3;
        let (data, offsets) = z272h_virtual_tx(&mut stream, shared, 1);
        assert!(offsets.is_empty());

        let mut r = ParcelReader::new(&data);
        assert_eq!(r.read_i32(), Some(0), "EX_NONE");
        assert_eq!(r.read_i32(), Some(1), "NON_NULL_PARCELABLE_FLAG word");
        let size = r.read_i32().expect("size word");
        assert!(size >= 4);
        assert_eq!(size as usize, data.len() - 8, "size spans itself+fields");
        let seed_len = r.read_i32().expect("seed length");
        assert_eq!(seed_len, 32, "deterministic 32-byte seed");
        let seed: Vec<u8> = r.buf[r.pos..r.pos + 32].to_vec();
        r.pos += 32;
        assert_eq!(seed[0], 7u8, "seed[0] = (0*31+7)");
        assert_eq!(seed[1], 38u8, "seed[1] = (1*31+7)");
        assert_eq!(r.read_i32(), Some(0), "empty nonce");
        assert_eq!(r.remaining(), 0);

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    /// 6-Z272h: EX_SERVICE_SPECIFIC error replies follow the REAL
    /// Status wire — [code][string16 message][i32 0 stack-trace][i32
    /// service code] — so the client's readFromParcel lands on the
    /// service code instead of consuming it as a message length.
    #[test]
    fn z272h_service_specific_error_full_status_wire() {
        let result = virtual_error_reply(EX_SERVICE_SPECIFIC, KM_ERROR_HARDWARE_TYPE_UNAVAILABLE);
        let TransactionResult::Reply { data, offsets } = result else {
            panic!("error reply must be a Reply");
        };
        assert!(offsets.is_empty());
        // [EX_SERVICE_SPECIFIC 4][message string16 8][stack 4][code 4] = 20 B
        assert_eq!(data.len(), 20, "full status wire shape");
        let mut r = ParcelReader::new(&data);
        assert_eq!(r.read_i32(), Some(EX_SERVICE_SPECIFIC), "exception word");
        let msg = r
            .read_string16()
            .expect("message present")
            .expect("empty message, not null");
        assert_eq!(msg, "", "empty message string16");
        assert_eq!(r.read_i32(), Some(0), "empty remote stack trace header");
        assert_eq!(
            r.read_i32(),
            Some(KM_ERROR_HARDWARE_TYPE_UNAVAILABLE),
            "service-specific code is the LAST word"
        );
        assert_eq!(r.remaining(), 0);

        // EX_NONE stays the bare 4-byte word (write_status_ok shape).
        let TransactionResult::Reply { data: ok, .. } = virtual_error_reply(EX_NONE, 0) else {
            panic!("EX_NONE reply must be a Reply");
        };
        assert_eq!(ok, vec![0, 0, 0, 0], "EX_NONE = bare i32 0");
    }

    /// 6-Z272h: the structured-parcelable writer emits the null-flag
    /// word then patches a self-inclusive size word (the exact contract
    /// of android's DeserializeOption + sized_write pair).
    #[test]
    fn z272h_structured_parcelable_writer_flag_and_size() {
        let mut w = ParcelWriter::new();
        w.write_i32(7); // pre-existing word (e.g. EX_NONE) — offsets shift
        w.write_structured_parcelable(|w| {
            w.write_i32(1);
            w.write_i32(2);
            w.write_i32(3);
        });
        let (data, _) = w.into_parts();
        // [7][flag=1][size=16][1][2][3] = 24 bytes total.
        assert_eq!(data.len(), 24);
        assert_eq!(i32::from_ne_bytes(data[0..4].try_into().unwrap()), 7);
        assert_eq!(
            i32::from_ne_bytes(data[4..8].try_into().unwrap()),
            1,
            "NON_NULL_PARCELABLE_FLAG"
        );
        let size = i32::from_ne_bytes(data[8..12].try_into().unwrap());
        assert_eq!(size, 16, "size = 4 (itself) + 12 (fields); flag excluded");
        assert_eq!(
            i32::from_ne_bytes(data[12..16].try_into().unwrap()),
            1,
            "first field survives unshifted"
        );
    }

    /// 6-Z276: registerForNotifications → later addService fires a one-way
    /// onRegistration BR_TRANSACTION targeted at the watcher's local
    /// callback object, in BOTH dialects; unregister drops the watcher;
    /// conn death drops the watcher.
    #[test]
    fn z276_registration_callbacks_fire_oneway() {
        let mut bus = BusState::new();
        let watcher = bus.register_conn();
        let w_ptr: u64 = 0xABCD_0001;
        let w_cookie: u64 = 0xDEAD_BEEF;

        // Watch a service that does NOT exist yet (AIDL dialect).
        bus.add_watcher(
            "some.hal.IFoo/default",
            ServiceWatcher {
                conn: watcher,
                ptr: w_ptr,
                cookie: w_cookie,
                hidl: false,
            },
        );

        // The service registers — the watcher's mailbox gets exactly one
        // one-way callback transaction.
        let h = bus.add_guest_service("some.hal.IFoo/default", PROXY_CONN_ID, 0, 0);
        bus.fire_registration_callbacks("some.hal.IFoo/default", h, false);

        let bx = bus.conns.get(&watcher).expect("watcher conn");
        assert_eq!(bx.inbox.len(), 1, "exactly one onRegistration queued");
        let fired = match bx.inbox.front() {
            Some(InboxItem::Tx(tx)) => {
                assert_eq!(tx.code, 1, "onRegistration code");
                assert!(tx.one_way, "callback is one-way");
                assert_eq!(tx.txn_id, 0, "no reply bookkeeping for one-way");
                assert_eq!(tx.ptr, w_ptr, "targeted at the watcher's local cb");
                assert_eq!(tx.cookie, w_cookie);
                assert_eq!(tx.flags, TF_ONE_WAY);
                let blob = tx.blob.as_ref().expect("AIDL callback carries a parcel");
                // Decode the parcel's UTF-16 string16 regions to check the
                // AIDL token + name (string16 = [i32 len][len × u16le][NUL]).
                let decode_string16_at = |data: &[u8], off: usize| -> Option<String> {
                    let len = i32::from_ne_bytes(data[off..off + 4].try_into().ok()?) as usize;
                    let mut u: Vec<u16> = Vec::with_capacity(len);
                    for i in 0..len {
                        let b = data[off + 4 + i * 2..off + 6 + i * 2].try_into().ok()?;
                        u.push(u16::from_le_bytes(b));
                    }
                    Some(String::from_utf16_lossy(&u))
                };
                let _ = decode_string16_at;
                // Walk: [strict][work][tag] then string16s.
                assert_eq!(
                    u32::from_ne_bytes(blob.data[8..12].try_into().unwrap()),
                    AIDL_HEADER_TAG_SYST,
                    "SYST header tag"
                );
                let desc = decode_string16_at(&blob.data, 12).expect("descriptor string16");
                assert_eq!(desc, "android.os.IServiceCallback");
                // The name string16 follows (12 + 4 + 2*(len+1) + pad).
                let desc_len = i32::from_ne_bytes(blob.data[12..16].try_into().unwrap()) as usize;
                let name_off = 12 + 4 + 2 * (desc_len as usize + 1);
                let name_off = (name_off + 3) & !3; // 4-byte pad
                let name = decode_string16_at(&blob.data, name_off).expect("name string16");
                assert_eq!(name, "some.hal.IFoo/default");
                true
            }
            other => panic!("expected Tx, got {:?}", other.is_some()),
        };
        assert!(fired);
        // Fired once → the watcher list is consumed (fire-once semantics).
        assert!(!bus.watchers.contains_key("some.hal.IFoo/default"));

        // unregister + conn-death cleanup paths.
        bus.add_watcher(
            "another.hal.IBar/default",
            ServiceWatcher {
                conn: watcher,
                ptr: 0x1111,
                cookie: 0,
                hidl: false,
            },
        );
        bus.add_watcher(
            "hidl.vendor.IBaz/default",
            ServiceWatcher {
                conn: watcher,
                ptr: 0x2222,
                cookie: 0,
                hidl: true,
            },
        );
        bus.remove_watcher("another.hal.IBar/default", watcher, 0x1111);
        assert!(!bus.watchers.contains_key("another.hal.IBar/default"));
        bus.remove_watchers_of_conn(watcher);
        assert!(!bus.watchers.contains_key("hidl.vendor.IBaz/default"));

        // HIDL watcher parcel shape: registration + fire.
        let w2 = bus.register_conn();
        bus.add_watcher(
            "android.hardware.health@2.1::IHealth/default",
            ServiceWatcher {
                conn: w2,
                ptr: 0x4444,
                cookie: 0x5555,
                hidl: true,
            },
        );
        let h2 = bus.add_guest_service(
            "android.hardware.health@2.1::IHealth/default",
            PROXY_CONN_ID,
            0,
            0,
        );
        bus.fire_registration_callbacks("android.hardware.health@2.1::IHealth/default", h2, false);
        let bx2 = bus.conns.get(&w2).expect("hidl watcher conn");
        match bx2.inbox.front() {
            Some(InboxItem::Tx(tx)) => {
                let blob = tx.blob.as_ref().expect("HIDL callback carries a parcel");
                // HIDL data = [i32 len]["android.hardware.health@2.1::IHealth"]
                // + NUL + pad, [i32 len]["default"] + NUL + pad, [i32 0].
                let fq_len = i32::from_ne_bytes(blob.data[0..4].try_into().unwrap());
                assert_eq!(
                    fq_len as usize,
                    "android.hardware.health@2.1::IHealth".len()
                );
                let s = String::from_utf8_lossy(&blob.data);
                assert!(s.contains("default"), "instance hidl_string present");
                assert!(
                    s.contains("android.hardware.health@2.1::IHealth"),
                    "fqName hidl_string present"
                );
            }
            _ => panic!("HIDL onRegistration not queued"),
        }
    }

    // ====================================================================
    // 6-Z298: the in-proxy virtual `android.hardware.health.IHealth/default`
    // ====================================================================

    /// Synthetic sysfs snapshot: the "full tree" a materialised +
    /// host-managed battery directory produces.
    fn z298_full_battery_values() -> crate::battery::GuestBatteryValues {
        crate::battery::GuestBatteryValues {
            capacity_pct: Some(75),
            status_str: Some("Discharging".into()),
            voltage_uv: Some(4_200_000),
            temp_decic: Some(280),
            charge_counter_uah: Some(262_500),
            current_now_ua: Some(-300_000),
            current_avg_ua: Some(-290_000),
            cycle_count: Some(3),
            present: true,
            technology: Some("Li-ion".into()),
            health_str: Some("Good".into()),
        }
    }

    fn z298_reply_words(data: &[u8]) -> Vec<i32> {
        data.chunks_exact(4)
            .map(|c| i32::from_ne_bytes(c.try_into().unwrap()))
            .collect()
    }

    #[test]
    fn z298_health_service_registered_under_its_aidl_name() {
        let b = BusState::new();
        let name = "android.hardware.health.IHealth/default";
        let entry = b
            .services
            .get(name)
            .expect("virtual IHealth/default must be registered at proxy start");
        assert_eq!(
            entry.virtual_kind,
            Some(VirtualService::Health),
            "the registry entry must carry the virtual kind"
        );
        // The descriptor (INTERFACE_TRANSACTION reply body) matches the
        // AIDL name — verified against android-15.0.0_r1 (the descriptor
        // string lives in android.hardware.health-V4-ndk.so, which ships
        // in the lineage-22.2 recovery ramdisk).
        assert_eq!(
            VirtualService::Health.descriptor(),
            "android.hardware.health.IHealth"
        );
        // Interface version: V4 (the A15 additions getBatteryHealthData /
        // BatteryHealthData; the corpus's own client library is -V4-ndk).
        match virtual_service_transaction(VirtualService::Health, 0, None) {
            TransactionResult::Reply { data, .. } => {
                assert_eq!(z298_reply_words(&data), vec![0, 4], "[EX_NONE][4]");
            }
            _ => panic!("interface-version must reply"),
        }
    }

    #[test]
    fn z298_get_capacity_and_charge_status_follow_the_sysfs_snapshot() {
        // The two methods battery_utils.cpp consumes (IsBatteryOk → the
        // sideload battery gate). Full tree → honest values.
        match virtual_health_with_values(
            7,
            &mut ParcelReader::new(&[]),
            &z298_full_battery_values(),
        ) {
            TransactionResult::Reply { data, .. } => {
                assert_eq!(
                    z298_reply_words(&data),
                    vec![0, 75],
                    "getCapacity → [EX_NONE][75]"
                );
            }
            _ => panic!("getCapacity must reply"),
        }
        match virtual_health_with_values(
            9,
            &mut ParcelReader::new(&[]),
            &z298_full_battery_values(),
        ) {
            TransactionResult::Reply { data, .. } => {
                // "Discharging" → BatteryStatus.DISCHARGING = 3 — the
                // value that makes recovery render the header WITHOUT
                // the charging "+".
                assert_eq!(
                    z298_reply_words(&data),
                    vec![0, 3],
                    "getChargeStatus → [EX_NONE][DISCHARGING]"
                );
            }
            _ => panic!("getChargeStatus must reply"),
        }
        // Battery-less / absent tree → the .aidl-documented
        // EX_UNSUPPORTED_OPERATION (never fabricated data). The exception
        // wire shape is the 6-Z272h one: [i32 exc][string16 msg][i32 trace]
        // = 16 bytes for a non-EX_NONE exception.
        let empty = crate::battery::GuestBatteryValues::default();
        for code in [4, 5, 6, 7, 9] {
            match virtual_health_with_values(code, &mut ParcelReader::new(&[]), &empty) {
                TransactionResult::Reply { data, .. } => {
                    assert_eq!(
                        z298_reply_words(&data),
                        vec![EX_UNSUPPORTED_OPERATION, 0, 0, 0],
                        "code {} with no sysfs → UNSUPPORTED (16-byte exception shape)",
                        code
                    );
                }
                _ => panic!("code {} must reply", code),
            }
        }
    }

    #[test]
    fn z298_health_info_parcel_matches_the_aidl_field_order() {
        // HealthInfo has 25 fields after the status word: 16×i32, one
        // string16 ("Li-ion" = 4 + 7×2 = 18 → pad 20), 3×i32 (current
        // avg + the two empty arrays), i64, 3×i32 → 4 + 64 + 20 + 4 +
        // 4 + 4 + 4 + 8 + 4 + 4 + 4 = 124 bytes.
        match virtual_health_with_values(
            12,
            &mut ParcelReader::new(&[]),
            &z298_full_battery_values(),
        ) {
            TransactionResult::Reply { data, .. } => {
                assert_eq!(data.len(), 124, "HealthInfo wire size");
                let w = z298_reply_words(&data);
                assert_eq!(w[0], 0, "EX_NONE");
                assert_eq!(w[1], 0, "chargerAcOnline (host charges over USB)");
                assert_eq!(w[2], 0, "chargerUsbOnline (Discharging → offline)");
                assert_eq!(w[5], 0, "maxChargingCurrentMicroamps (unknown)");
                assert_eq!(w[6], 0, "maxChargingVoltageMicrovolts (unknown)");
                assert_eq!(w[7], 3, "batteryStatus = DISCHARGING");
                assert_eq!(w[8], 2, "batteryHealth = GOOD");
                assert_eq!(w[9], 1, "batteryPresent");
                assert_eq!(w[10], 75, "batteryLevel");
                assert_eq!(w[11], 4200, "batteryVoltageMillivolts (uV→mV)");
                assert_eq!(w[12], 280, "batteryTemperatureTenthsCelsius");
                assert_eq!(w[13], -300_000, "batteryCurrentMicroamps");
                assert_eq!(w[14], 3, "batteryCycleCount");
                assert_eq!(w[16], 262_500, "batteryChargeCounterUah");
                // batteryTechnology string16 at word offset 17 (byte 68):
                // [len=6][3 words of UTF-16 + NUL/pad].
                assert_eq!(w[17], 6, "batteryTechnology utf16 length");
                assert_eq!(
                    &data[72..84],
                    "Li-ion"
                        .encode_utf16()
                        .flat_map(u16::to_ne_bytes)
                        .collect::<Vec<u8>>(),
                    "batteryTechnology utf16 payload"
                );
                assert_eq!(w[22], -290_000, "batteryCurrentAverageMicroamps");
                assert_eq!(w[23], 0, "diskStats: empty array");
                assert_eq!(w[24], 0, "storageInfos: empty array");
                assert_eq!(w[25], -1, "batteryCapacityLevel: UNSUPPORTED");
                // batteryChargeTimeToFullNowSeconds is the only i64 —
                // at byte offset 104 (word offsets 26+27).
                let secs = i64::from_ne_bytes(data[104..112].try_into().unwrap());
                assert_eq!(secs, 0, "batteryChargeTimeToFullNowSeconds");
                assert_eq!(w[28], 0, "batteryFullChargeDesignCapacityUah");
                assert_eq!(w[29], 1, "chargingState = NORMAL");
                assert_eq!(w[30], 1, "chargingPolicy = DEFAULT");
                assert_eq!(data.len(), 31 * 4, "all fields accounted for");
            }
            _ => panic!("getHealthInfo must reply"),
        }
        // Charging → the USB charger flips online + capacityLevel of a
        // zero-capacity battery renders UNSUPPORTED (-1), never a fake.
        let mut charging = z298_full_battery_values();
        charging.status_str = Some("Charging".into());
        charging.capacity_pct = Some(0);
        match virtual_health_with_values(12, &mut ParcelReader::new(&[]), &charging) {
            TransactionResult::Reply { data, .. } => {
                let w = z298_reply_words(&data);
                assert_eq!(w[2], 1, "chargerUsbOnline (Charging)");
                assert_eq!(w[7], 2, "batteryStatus = CHARGING");
                assert_eq!(w[10], 0, "batteryLevel = 0 (host-honest 0%)");
                assert_eq!(w[25], -1, "capacityLevel UNSUPPORTED (raw % only)");
            }
            _ => panic!("getHealthInfo(charging) must reply"),
        }
    }

    #[test]
    fn z298_get_battery_health_data_shape() {
        // [EX_NONE][3×i64 zeros][nullable serial = -1][partStatus=0].
        match virtual_health_with_values(
            15,
            &mut ParcelReader::new(&[]),
            &z298_full_battery_values(),
        ) {
            TransactionResult::Reply { data, .. } => {
                assert_eq!(data.len(), 4 + 8 + 8 + 8 + 4 + 4, "BatteryHealthData size");
                let words = z298_reply_words(&data);
                assert_eq!(words[0], 0);
                // The three i64s occupy word slots 1..=6.
                assert_eq!(words[1], 0, "manufacturing date unknown");
                assert_eq!(words[6], 0, "state of health (high word)");
                assert_eq!(words[7], -1, "batterySerialNumber = NULL (i32 -1)");
                assert_eq!(words[8], 0, "batteryPartStatus = UNSUPPORTED");
            }
            _ => panic!("getBatteryHealthData must reply"),
        }
    }

    #[test]
    fn z298_void_methods_and_arrays_reply_ok() {
        // registerCallback(1) / unregisterCallback(2) / update(3) /
        // setChargingPolicy(13) → bare EX_NONE.
        for code in [1, 2, 3, 13] {
            match virtual_health_with_values(
                code,
                &mut ParcelReader::new(&[]),
                &z298_full_battery_values(),
            ) {
                TransactionResult::Reply { data, .. } => {
                    assert_eq!(z298_reply_words(&data), vec![0], "code {}", code);
                }
                _ => panic!("code {} must reply", code),
            }
        }
        // getStorageInfo(10) / getDiskStats(11) → [EX_NONE][empty].
        for code in [10, 11] {
            match virtual_health_with_values(
                code,
                &mut ParcelReader::new(&[]),
                &z298_full_battery_values(),
            ) {
                TransactionResult::Reply { data, .. } => {
                    assert_eq!(z298_reply_words(&data), vec![0, 0], "code {}", code);
                }
                _ => panic!("code {} must reply", code),
            }
        }
        // getEnergyCounterNwh(8) / getChargingPolicy(14) → UNSUPPORTED
        // (no backing file, exactly the .aidl's documented semantic;
        // 16-byte 6-Z272h exception shape).
        for code in [8, 14] {
            match virtual_health_with_values(
                code,
                &mut ParcelReader::new(&[]),
                &z298_full_battery_values(),
            ) {
                TransactionResult::Reply { data, .. } => {
                    assert_eq!(
                        z298_reply_words(&data),
                        vec![EX_UNSUPPORTED_OPERATION, 0, 0, 0],
                        "code {}",
                        code
                    );
                }
                _ => panic!("code {} must reply", code),
            }
        }
        // Unknown codes → UNSUPPORTED (never BR_FAILED_REPLY: a failed
        // reply on a live service handle wedges the client's waitFor-
        // Response the same way an unimplemented method does on a real
        // device).
        match virtual_health_with_values(
            99,
            &mut ParcelReader::new(&[]),
            &z298_full_battery_values(),
        ) {
            TransactionResult::Reply { data, .. } => {
                assert_eq!(
                    z298_reply_words(&data),
                    vec![EX_UNSUPPORTED_OPERATION, 0, 0, 0]
                );
            }
            _ => panic!("unknown code must still reply"),
        }
    }

    #[test]
    fn z298_guest_addservice_overrides_the_virtual_health_service() {
        // If a guest ever ships its own health HAL it must win — native
        // servicemanager overwrite semantics, and the proxy handler
        // stands down (transactions route to the guest owner).
        let mut b = BusState::new();
        let name = "android.hardware.health.IHealth/default";
        let virtual_handle = b.services.get(name).unwrap().handle;
        let conn = PROXY_CONN_ID + 7;
        let h = b.add_guest_service(name, conn, 0xdead, 0x1234);
        assert_eq!(h, virtual_handle, "same name → same handle");
        let entry = b.services.get(name).unwrap();
        assert_eq!(entry.owner, conn, "owner switched to the guest");
        assert_eq!(entry.ptr, 0xdead);
        assert_eq!(
            entry.virtual_kind, None,
            "the in-proxy handler must stand down"
        );
    }
}
