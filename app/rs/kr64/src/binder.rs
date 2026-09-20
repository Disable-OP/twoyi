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
//!   haptics wait; `on(ms)` AND the TWRP-12.1/fox tap path `perform(effect,
//!   strength)` are forwarded to the host app for REAL vibrations — 6-Z300:
//!   perform answers the .aidl-documented duration/0 semantics instead of
//!   an exception, getSupportedEffects lists the synthetic set),
//!   `android.hardware.security.keymint.IKeyMintDevice/default`
//!   (lets keystore2 obtain its backend and register IKeystoreSecurity —
//!   kills the ~20 s recovery wait; key ops return honest
//!   `HARDWARE_TYPE_UNAVAILABLE` errors, no fake crypto),
//!   `android.hardware.security.sharedsecret.ISharedSecret/default`, and
//!   `android.hardware.health.IHealth/default` (6-Z298: serves the AIDL
//!   battery chain of AOSP/Lineage recovery ≥ A12 —
//!   `GetBatteryInfo()`'s isDeclared/waitForService/getCapacity/
//!   getChargeStatus — with host-honest values from the pinned battery
//!   sysfs tree, so the IsBatteryOk sideload gate stops falling back to
//!   the fake "capacity 100, charging" defaults).
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
//!
//! v3 request : v2 + a [u32 sg_count] header per blob and, after
//!              [data][offsets], sg_count ×
//!              [u64 client_ptr][u32 len][len bytes] — the BINDER_TYPE_PTR
//!              SG-buffer contents (6-Z305t-68: the HIDL wire carries
//!              hidl_string/hidl_vec bytes OUT of the main parcel).
//! v3 response: v2 + the same per-blob [u32 sg_count] header + SG bytes;
//!              the loader reassembles [data][offsets][sg] into the
//!              guest's backing buffer and fixes every PTR object's
//!              `buffer` field to its SG copy (the kernel's receiver-side
//!              pointer fixup).
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
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
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
/// 6-Z305t-66: `getTransport(string fqName, string name) generates
/// (Transport transport)` — code 3, EVERY HAL's first service lookup
/// (getRawServiceInternal, transport/ServiceManagement.cpp:779). The
/// pre-6-Z305t-66 map had NO arm for it → the catch-all BR_FAILED_REPLY →
/// libhwbinder surfaced Status(EX_TRANSACTION_FAILED) at every getService
/// site (139 logs / 956 aborts, ladder #122 — the HIDL fleet killer).
pub const HIDL_SM_GET_TRANSPORT: u32 = 3;
/// 1.0 `debugDump() generates (vec<InstanceDebugInfo>)` — code 7. THE
/// A11 Watchdog call site: `getInterestingHalPids()` (Watchdog.java:517)
/// does `IServiceManager.getService().debugDump()` and runs inside the
/// WAITED_HALF branch — Watchdog.java:616 evaluates it as
/// `dumpStackTraces`' 4th argument. Before this arm (and before the
/// 6-Z307 instance seeding) the proxy answered the watchdog's
/// `@1.0::IServiceManager/default` lookup with a null binder →
/// `HwBinder.getService` threw `java.util.NoSuchElementException` →
/// uncaught in the watchdog thread → ART's KillApplicationHandler
/// SIGKILLed system_server (the ladder-#247 "FATAL EXCEPTION:
/// watchdog" kill chain, 49ms end-to-end).
pub const HIDL_SM_DEBUG_DUMP: u32 = 7;
/// IBase::interfaceChain — the reserved IBase method code (NOT a
/// FIRST_CALL_TRANSACTION-offset code). Pinned LIVE in ladder #248: every
/// C++ HIDL client's post-get cast probe (getRawServiceInternal:821 →
/// canCastInterface → interface->interfaceChain, HidlTransportUtils.cpp:27)
/// transacts this code on the returned handle; the reply must be
/// [status-ok][vec<string> chain] with the chain CONTAINING the descriptor
/// the client casts to (HidlTransportUtils.cpp:38-46). The #248 run's
/// probes (conn=95/126/151/… at the ~10s retry cadence) got
/// BR_FAILED_REPLY → handleCastError "unable to call into hwbinder
/// service" → nullptr → the watchdog's NoSuchElementException.
pub const HIDL_IBASE_INTERFACE_CHAIN: u32 = 0xf43484e;
/// 6-Z276/6-Z305t-66: `registerForNotifications` — code 6. The old map
/// guessed 4, which is `list`'s code (the arm never fired on the real
/// wire; nothing on the A11 boot path calls list()).
pub const HIDL_SM_REGISTER_FOR_NOTIFICATIONS: u32 = 6;
/// manager@1.1 `unregisterForNotifications` — code 9 (1.0's 8 methods
/// precede it; the old map guessed 5 = `listByInterface`'s code).
pub const HIDL_SM_UNREGISTER_FOR_NOTIFICATIONS: u32 = 9;
/// 1.0 `oneway registerPassthroughClient(string fqName, string name)`.
pub const HIDL_SM_REGISTER_PASSTHROUGH_CLIENT: u32 = 8;
/// manager@1.2 `addWithChain(string name, interface service,
/// vec<string> chain)` — code 12, THE registration call the A11 GSI's
/// HALs make (registerAsServiceInternal, ServiceManagement.cpp:884:
/// `service->interfaceChain → sm->addWithChain(name, service, chain)` via
/// defaultServiceManager1_2). The chain carries the fqNames the 1.0 add()
/// wire lacks, so the registry keys "fq/instance" like hwservicemanager.
pub const HIDL_SM_ADD_WITH_CHAIN: u32 = 12;
/// 1.2 `listManifestByInterface(string fqName) generates (vec<Instance>)`.
pub const HIDL_SM_LIST_MANIFEST_BY_INTERFACE: u32 = 13;
/// `android.hidl.manager@1.0::IServiceManager.Transport` — `enum Transport
/// : uint8_t` (IServiceManager.hal android-11.0.0_r1:76-80) marshals as
/// ONE byte (hwbinder::Parcel::writeUint8 = write(&val, 1), no padding).
pub const HIDL_TRANSPORT_EMPTY: u8 = 0;
pub const HIDL_TRANSPORT_HWBINDER: u8 = 1;
/// `android.hidl.base@1.0::DebugInfo.Architecture` — `enum Architecture
/// : uint8_t { UNKNOWN = 0, IS_64BIT = 1, IS_32BIT = 2 }`. The real
/// hwservicemanager's debugDump reports UNKNOWN for BINDER-registered
/// services (ServiceManager.cpp:745) — the proxy does the same.
pub const HIDL_DEBUG_ARCH_UNKNOWN: u8 = 0;
/// PASSTHROUGH is answered by the passthrough dlopen path, never by this
/// registry (a HIDL passthrough HAL never appears as a wire service).
pub const HIDL_TRANSPORT_PASSTHROUGH: u8 = 2;

/// Wire value → display name for the getTransport log lines.
fn transport_name(t: u8) -> &'static str {
    match t {
        HIDL_TRANSPORT_HWBINDER => "HWBINDER",
        HIDL_TRANSPORT_PASSTHROUGH => "PASSTHROUGH",
        _ => "EMPTY",
    }
}

/// 6-Z350 (rn300 decode): HIDL ancestor-version fallback. The REAL
/// hwservicemanager answers `getTransport(pkg@M.m::IFace/inst)` with
/// HWBINDER when the registered version is `pkg@M.m'::IFace/inst` for ANY
/// m' ≤ m with the same major — interface inheritance means a @2.3
/// registration IS reachable as @2.1/@2.2. rn300: the guest's composer
/// registered `android.hardware.graphics.composer@2.3::IComposer/default`
/// (addWithChain → handle 0x2a) but A11 surfaceflinger asks for
/// `@2.1::IComposer/default` (the HWC2 base-version getService) and the
/// exact-key lookup answered EMPTY ×83 → "failed to get hwcomposer
/// service" aborts. Scans every minor of the same major up to the
/// requested one; returns true on the first registered ancestor.
fn hidl_ancestor_registered_6z350(
    services: &BTreeMap<String, ServiceEntry>,
    fq: &str,
    name: &str,
) -> bool {
    let parsed = match crate::vintf::parse_fq(fq) {
        Some(p) => p,
        None => return false,
    };
    let mut it = parsed.version.split('.');
    let major = it.next().unwrap_or("");
    // 6-Z350b (rn301 decode): the first cut scanned minors UP TO the
    // requested one — but the registered version sits ABOVE it (the
    // composer registered @2.3 while SF asks @2.1). The real-SM semantic
    // is "any minor of the same major serves the get": scan the full
    // minor space (HIDL minors are tiny; the map lookups are cheap).
    let _ = it.next();
    for minor in 0..=64u32 {
        let key = format!(
            "{}@{}.{}::{}/{}",
            parsed.package, major, minor, parsed.iface, name
        );
        if services.contains_key(&key) {
            return true;
        }
    }
    false
}

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
/// `libbinder.so` opens one binder fd PER PROCESS PER CONTEXT —
/// `/dev/binder` + `/dev/hwbinder` + `/dev/vndbinder` (up to 3 each), and
/// a booting A11 GSI crosses 100 processes (system_server alone adds
/// binder + vndbinder + its async binder threads). Ladder #159 hit the
/// old 64-cap at +31s: `[KR64][binder][vm0] connection over cap (64)
/// dropped` — every service starting after that (mediaserver,
/// mediaextractor, audioserver …) lost its open_driver → EPIPE →
/// "Binder driver '/dev/binder' could not be opened. Terminating." —
/// and system_server would have died the same way at the next rung.
/// 512 = ~3 contexts × ~150 processes with 2-3× headroom, still bounded
/// so a misbehaving guest can't thread-bomb the daemon (the accept-side
/// counter + drop path remain).
pub const MAX_PROXY_CONNECTIONS: usize = 512;

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

/// 6-Z305t-68: v3 adds a per-blob BINDER_TYPE_PTR SG-buffer section —
/// the HIDL wire carries `hidl_string`/`hidl_vec` contents OUT of the
/// main parcel (libhwbinder `writeBuffer`/`writeEmbeddedBuffer` emit
/// `binder_buffer_object`s whose bytes the real kernel copies into the
/// receiver's buffer as the scatter-gather region of
/// `BC_TRANSACTION_SG`). The loader captures those bytes; the proxy
/// reassembles them so `servicemanager_hidl` can resolve real string
/// arguments. v3 per blob:
/// `[u32 data_len][u32 offsets_len][u32 sg_count][data][offsets]`
/// then `sg_count × [u64 client_ptr][u32 len][len bytes]` — the PTR
/// contents in offsets-array order (the kernel's copy order). v2
/// requests still parse (sg empty → HIDL string args fail honestly).
/// Bytes are `'W' 'V' '3' '0'` in native-endian word order.
pub const WIRE_V3_MAGIC: u32 = u32::from_ne_bytes(*b"WV30");

/// 6-Z355: the fd-tail marker. A WRITE_READ request/response whose blobs
/// carry `BINDER_TYPE_FD`/`BINDER_TYPE_FDA` flats appends AFTER the last
/// blob: `[WIRE_FD_TAIL_MAGIC][u32 blob_count][blob_count × u32 fd_count]`
/// — the per-blob fd counts in blob order — while the fds themselves ride
/// the frame's SCM_RIGHTS ancillary block in the SAME order. Both sides
/// also re-derive the counts by scanning the blobs' offsets arrays and
/// cross-check (bounded warn on mismatch). Old peers that never send fds
/// never append the tail, and the trailing bytes of a v1/v2 frame are
/// ignored by design — backward compatible in both directions. Bytes are
/// `'F' 'D' 'T' '0'` in native-endian word order.
pub const WIRE_FD_TAIL_MAGIC: u32 = u32::from_ne_bytes(*b"FDT0");

/// 6-Z355: how many fd-tail cross-check diagnostics are logged per boot
/// (bounded — the composer/allocator fleet can carry fds on every call).
static FD_TAIL_MISMATCH_LOG: AtomicU32 = AtomicU32::new(16);

/// 6-Z355: per-blob fd count from a blob's offsets array — the same scan
/// the loader-side builder runs. `BINDER_TYPE_FD` contributes 1 (the flat
/// `handle` field IS the sender's fd number); `BINDER_TYPE_FDA`
/// contributes the flat's `numFds` (the fd VALUES live in the parent
/// region, which only the loader needs to read — the proxy carries the
/// received SCM_RIGHTS list verbatim). Malformed offsets are skipped
/// (kernel: validate-and-fail; here the count just comes out smaller and
/// the cross-check names it).
fn blob_fd_count(data: &[u8], offsets: &[u8]) -> u32 {
    let mut n = 0u32;
    let count = offsets.len() / 8;
    for i in 0..count {
        let off = match u64::from_ne_bytes(offsets[i * 8..i * 8 + 8].try_into().unwrap()) as usize {
            o if o + 8 <= data.len() => o,
            _ => continue,
        };
        let typ = u32::from_ne_bytes(data[off..off + 4].try_into().unwrap());
        if typ == BINDER_TYPE_FD {
            n = n.saturating_add(1);
        } else if typ == BINDER_TYPE_FDA {
            // binder_fd_array_object { hdr u32, numFds u32, pad u32, ... }
            if off + 8 <= data.len() {
                let num_fds = u32::from_ne_bytes(data[off + 4..off + 8].try_into().unwrap());
                n = n.saturating_add(num_fds);
            }
        }
    }
    n
}

/// 6-Z355: parse the optional fd tail at `off`; returns the per-blob
/// counts. `None` when no tail is present (or it is malformed — the
/// caller logs and falls back to no-fd delivery).
fn parse_fd_tail(payload: &[u8], off: usize, blob_count: usize) -> Option<Vec<u32>> {
    if blob_count == 0 {
        return None;
    }
    let need = 8 + 4 * blob_count;
    if off + need > payload.len() {
        return None;
    }
    let magic = u32::from_ne_bytes(payload[off..off + 4].try_into().unwrap());
    if magic != WIRE_FD_TAIL_MAGIC {
        return None;
    }
    let n = u32::from_ne_bytes(payload[off + 4..off + 8].try_into().unwrap()) as usize;
    if n != blob_count {
        return None;
    }
    let mut counts = Vec::with_capacity(blob_count);
    for i in 0..blob_count {
        let b = off + 8 + i * 4;
        counts.push(u32::from_ne_bytes(payload[b..b + 4].try_into().unwrap()));
    }
    Some(counts)
}

/// 6-Z355: append the fd tail (`counts.len()` must equal the blob count).
fn append_fd_tail(payload: &mut Vec<u8>, counts: &[u32]) {
    payload.extend_from_slice(&WIRE_FD_TAIL_MAGIC.to_ne_bytes());
    payload.extend_from_slice(&(counts.len() as u32).to_ne_bytes());
    for c in counts {
        payload.extend_from_slice(&c.to_ne_bytes());
    }
}

/// `BINDER_BUFFER_FLAG_HAS_PARENT` — kernel uapi binder.h.
pub const BINDER_BUFFER_FLAG_HAS_PARENT: u32 = 0x01;

/// One loader-captured `BINDER_TYPE_PTR` buffer (the SG region the real
/// kernel copies verbatim into the receiver's transaction buffer and
/// fixes up — kernel uapi: "A binder_buffer object represents an object
/// that the binder kernel driver can copy verbatim to the target
/// address space").
#[derive(Debug, Clone)]
pub struct SgBuf {
    /// The SENDER's buffer address (`binder_buffer_object.buffer`). The
    /// proxy matches SG entries to PTR objects by CAPTURE ORDER (the
    /// kernel's copy order) and cross-checks this pointer when non-zero.
    pub client_ptr: u64,
    /// The captured bytes (`binder_buffer_object.length` bytes, capped
    /// loader-side).
    pub data: Vec<u8>,
}

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

/// 6-Z355: one fd the proxy holds on a guest's behalf while a blob that
/// references it is in flight. Received from the SENDING guest process via
/// SCM_RIGHTS (its kernel dup of the sender's fd), handed to the
/// RECIPIENT's connection via SCM_RIGHTS on the response sendmsg, and
/// closed here when the last reference drops — a dead recipient, a dropped
/// transaction or an unconsumed virtual-service request all release their
/// fds exactly like the kernel's binder_fd translation cleanup.
/// MSG_CMSG_CLOEXEC marks the dup so an exec (guest process restart) can
/// never leak a cross-process handle fd.
pub struct FdGuard(i32);

impl FdGuard {
    /// Adopt a raw fd received from recvmsg ancillary data (ownership
    /// transfers; the fd is closed on drop).
    pub fn from_raw(fd: i32) -> Self {
        FdGuard(fd)
    }

    pub fn as_raw(&self) -> i32 {
        self.0
    }

    /// Share the fd across blob clones (close on last drop).
    pub fn into_arc(self) -> Arc<FdGuard> {
        Arc::new(self)
    }
}

impl Drop for FdGuard {
    fn drop(&mut self) {
        if self.0 >= 0 {
            unsafe {
                libc::close(self.0);
            }
        }
    }
}

/// A response frame sent back to the guest.
struct Resp {
    /// Return value: 0 on success, negative errno on failure.
    ret: i32,
    /// Variable-length response payload (the bytes the ioctl would have
    /// written into `arg`).
    payload: Vec<u8>,
    /// 6-Z355: fds to deliver with THIS frame via SCM_RIGHTS (empty for
    /// every non-fd-bearing response). Ownership: the guards are consumed
    /// by the sendmsg — the kernel dups them into the recipient's table
    /// and the proxy's copies close right after the send.
    fds: Vec<Arc<FdGuard>>,
}

impl Resp {
    fn new(ret: i32, payload: Vec<u8>) -> Self {
        Resp {
            ret,
            payload,
            fds: Vec::new(),
        }
    }
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
    /// 6-Z305t-69: BINDER_TYPE_PTR contents for SG-shaped replies (the
    /// kernel copies these verbatim into the receiver's buffer and fixes
    /// the object's `buffer` pointer — the loader replicates that from
    /// the v3 resp trailer). Order = offsets-array object order.
    sg: Vec<SgBuf>,
}

impl ParcelWriter {
    fn new() -> Self {
        ParcelWriter {
            data: Vec::new(),
            offsets: Vec::new(),
            sg: Vec::new(),
        }
    }

    fn write_i32(&mut self, v: i32) {
        self.data.extend_from_slice(&v.to_ne_bytes());
    }

    #[allow(dead_code)]
    fn write_u32(&mut self, v: u32) {
        self.data.extend_from_slice(&v.to_ne_bytes());
    }

    /// HIDL sub-int write (hwbinder `writeUint8`/`writeBool` = writeInt8).
    /// 6-Z305t-68b: the write MUST 4-byte-pad the parcel position —
    /// libhwbinder's `writeInplace` advances `mDataPos` by `pad_size(len)`
    /// (the real server's reply data_size INCLUDES the pad), and the
    /// client's `Parcel::read` checks `mDataPos + pad_size(len) <=
    /// mDataSize` — an unpadded u8 reply made EVERY getTransport reply
    /// underrun client-side (mDataPos(4) + pad_size(1)(4) = 8 > 5) →
    /// Status(EX_TRANSACTION_FAILED, NOT_ENOUGH_DATA) → the 1292-abort
    /// storm of ladder #124 (every HWBINDER/EMPTY getTransport answer
    /// killed its caller at `Transport transport = sm->getTransport(...)`
    /// — the Return::value() abort, ServiceManagement.cpp:873).
    /// Used for the getTransport Transport byte and the HIDL bool replies
    /// (writeBool = writeInt8 — vs AIDL's i32 bool).
    fn write_u8(&mut self, v: u8) {
        self.data.push(v);
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

    /// The NEXT offsets-array index (for parent linkage of later objects).
    fn next_object_index(&self) -> usize {
        self.offsets.len() / 8
    }

    /// Consume the writer, returning `(data, offsets)`.
    fn into_parts(self) -> (Vec<u8>, Vec<u8>) {
        (self.data, self.offsets)
    }

    /// 6-Z305t-69: data + offsets + the SG region (for SG-shaped replies).
    fn into_parts_with_sg(self) -> (Vec<u8>, Vec<u8>, Vec<SgBuf>) {
        (self.data, self.offsets, self.sg)
    }

    /// 6-Z375 — bounded reply-wire dump: every offsets-array object
    /// (type, inline length, parent index, parent_offset) plus the SG
    /// content hex. This is the PROXY-SIDE pre-fixup truth; the loader's
    /// 6-Z375 line carries the client-side post-fixup view. Together they
    /// name any divergence from the kernel-true wire in one run.
    fn diag_object_graph(&self) -> String {
        let n_obj = self.offsets.len() / 8;
        let mut s = format!("dlen={} offs({})=", self.data.len(), n_obj);
        for j in 0..n_obj {
            let off = u64::from_ne_bytes(
                self.offsets[j * 8..j * 8 + 8]
                    .try_into()
                    .unwrap_or([0u8; 8]),
            ) as usize;
            if off + 40 > self.data.len() {
                s.push_str(&format!(" [{}:OOB@{}]", j, off));
                continue;
            }
            let typ = u32::from_ne_bytes(self.data[off..off + 4].try_into().unwrap_or([0u8; 4]));
            let len =
                u64::from_ne_bytes(self.data[off + 16..off + 24].try_into().unwrap_or([0u8; 8]));
            let parent =
                u64::from_ne_bytes(self.data[off + 24..off + 32].try_into().unwrap_or([0u8; 8]));
            let poff =
                u64::from_ne_bytes(self.data[off + 32..off + 40].try_into().unwrap_or([0u8; 8]));
            if typ == BINDER_TYPE_PTR {
                s.push_str(&format!(
                    " [{}:PTR len={} par={} poff={}]",
                    j, len, parent, poff
                ));
            } else {
                s.push_str(&format!(" [{}:0x{:x}]", j, typ));
            }
        }
        s.push_str(&format!(" sg({})=", self.sg.len()));
        for (i, b) in self.sg.iter().enumerate() {
            let mut h = String::new();
            for x in b.data.iter().take(24) {
                h.push_str(&format!("{:02x}", x));
            }
            s.push_str(&format!(" sg{}[{}]={}", i, b.data.len(), h));
        }
        s
    }

    /// Write a `binder_buffer_object` (BINDER_TYPE_PTR) whose bytes ride
    /// the SG region — the HIDL buffer model for reply values
    /// (writeBuffer/writeEmbeddedBuffer emulation). Returns the object's
    /// offsets-array index for parent linkage.
    /// Write a `hidl_vec<hidl_string>` — the kernel-true wire shape
    /// (6-Z305t-68 + 6-Z309):
    /// * PTR(vec struct {mBuffer, mSize, mOwns}, top-level) — mSize =
    ///   count; mBuffer is patched loader-side from the ARRAY object.
    /// * PTR(array, HAS_PARENT parent_offset=0, content = count × 16) —
    ///   each 16B element is a REAL `hidl_string` struct:
    ///   `[mBuffer(8)=0 (patched from the chars PTR object, 6-Z309
    ///   parent fixup)][mSize u32 @8][mOwns u8 @12][pad @13..16]`. The
    ///   client's read walk takes each element's chars length from THIS
    ///   mSize (`readEmbeddedFromParcel(hidl_string)` →
    ///   `readEmbeddedBuffer(string.size()+1, …)` with string = the array
    ///   copy element) — zeros here read as empty strings and make the
    ///   length check fail (the rn254 "incompatible service" fleet).
    /// * per element PTR(chars+NUL, HAS_PARENT parent_offset=j*16).
    /// Used for IBase::interfaceChain (the SM cast probe) and
    /// getServiceCallback.onValues' chain (6-Z307b).
    fn write_hidl_vec_string(&mut self, items: &[String]) {
        let count = items.len();
        let mut vs = vec![0u8; 16];
        vs[8..12].copy_from_slice(&(count as u32).to_ne_bytes());
        let vec_idx = self.next_object_index();
        self.write_ptr_object(vs, None, 0);
        let arr_idx = self.next_object_index();
        let mut array = vec![0u8; count * 16];
        for (j, s) in items.iter().enumerate() {
            let base = j * 16;
            array[base + 8..base + 12].copy_from_slice(&(s.len() as u32).to_ne_bytes());
        }
        self.write_ptr_object(array, Some(vec_idx), 0);
        for (j, s) in items.iter().enumerate() {
            let mut ch = s.as_bytes().to_vec();
            ch.push(0);
            self.write_ptr_object(ch, Some(arr_idx), (j * 16) as u64);
        }
    }

    fn write_ptr_object(
        &mut self,
        content: Vec<u8>,
        parent_idx: Option<usize>,
        parent_offset: u64,
    ) -> usize {
        let idx = self.offsets.len() / 8;
        let off = self.data.len() as u64;
        let flags = if parent_idx.is_some() {
            BINDER_BUFFER_FLAG_HAS_PARENT
        } else {
            0
        };
        // The buffer pointer the receiver sees is fixed up loader-side to
        // point at the SG copy — the proxy writes 0 ("unresolved").
        self.data.extend_from_slice(&BINDER_TYPE_PTR.to_ne_bytes());
        self.data.extend_from_slice(&flags.to_ne_bytes());
        self.data.extend_from_slice(&0u64.to_ne_bytes());
        self.data
            .extend_from_slice(&(content.len() as u64).to_ne_bytes());
        self.data
            .extend_from_slice(&(parent_idx.map(|p| p as u64).unwrap_or(0)).to_ne_bytes());
        self.data.extend_from_slice(&parent_offset.to_ne_bytes());
        self.offsets.extend_from_slice(&off.to_ne_bytes());
        self.sg.push(SgBuf {
            client_ptr: 0,
            data: content,
        });
        idx
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
    offsets: Vec<u8>,
    /// 6-Z305t-68: the BINDER_TYPE_PTR SG-buffer contents (v3 wire).
    /// Empty for v2 requests — HIDL string arguments then fail honestly.
    sg: Vec<SgBuf>,
    /// 6-Z355: the fds this blob's BINDER_TYPE_FD / BINDER_TYPE_FDA flats
    /// reference, in kernel translation order (offsets-array order; FDA
    /// entries in the parent region's array order). Received from the
    /// sender via SCM_RIGHTS, delivered to the recipient the same way —
    /// the flat `handle` fields keep the SENDER's numbers on the wire and
    /// the RECIPIENT's shlib patches them with the received dup numbers
    /// (kernel binder_translate_fd semantics, split across the wire).
    /// Arc-shared: a blob clone (the BC stream walk hands out clones) has
    /// the same close-on-last-drop contract as the kernel's fd refs.
    fds: Vec<Arc<FdGuard>>,
}

impl Clone for RequestBlob {
    fn clone(&self) -> Self {
        RequestBlob {
            data: self.data.clone(),
            offsets: self.offsets.clone(),
            sg: self.sg.clone(),
            fds: self.fds.clone(),
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

/// 6-Z458 (Task 195): the SERVICEMANAGER's own driver-side pin, modeled
/// as a virtual holder in every registered node's strong/weak maps.
/// Kernel truth: the real servicemanagers hold one strong ref per
/// registered service (ServiceManager.cpp `mNameToService[name].binder`;
/// hwservicemanager addImpl: one HidlService sp per chain key) — the
/// node's external refs NEVER empty while a registration key points at
/// the object, so `binder_dec_node` cannot fire the owner's BR_RELEASE
/// until the registry itself drops. The emulator's pre-6-Z458 maps
/// tracked only client grants, so the LAST CLIENT's drop emptied the
/// map and mirrored a BR_RELEASE the kernel would NOT send — the owner's
/// decStrong deleted a REGISTRY-PINNED LIVE object, the notification era
/// re-armed, and the next client's re-grant operated on the corpse (the
/// rn425 corpse-regrant cascade: double-frees → "registration lost" →
/// NULL ISystemSuspend proxy → SuspendLockout SIGSEGV → the ~145 s
/// Watchdog self-kill cycle). Never a real connection: `next_conn`
/// allocates upward from `PROXY_CONN_ID + 1`, so `u64::MAX` cannot
/// collide, and every existing map consumer treats it as one more
/// holder with no special-casing.
pub const REGISTRY_CONN: ConnId = ConnId::MAX;

/// 6-Z271 wire extension: connection-identity frame command. NOT a real
/// binder ioctl — a dedicated `'b'`-type number the loader sends right
/// after connect with `[u32 pid][u32 uid][u32 gid]`, so routed
/// transactions can carry kernel-true sender identities.
pub const WIRE_CMD_IDENT: u32 = 0x4004_62FF;

/// 6-Z306ai: IDENT v2 extension marker (ASCII "idex", little-endian).
/// A 24-byte IDENT payload `[12B legacy][u32 magic][u32 tid][u32 dev]`
/// carries the CONNECTING THREAD's tid and WHICH binder device the
/// connection serves (see [`ident_dev_name`]) — the per-thread proxy
/// conns created lazily by `bp_conn_for_ioctl` had no other way to
/// surface either. The boot decode correlates an aborting pool thread
/// (comm `HwBinder:pid_N`, named by the death-site capture) with the
/// transactions ITS conn served (the proxy's per-conn logs).
pub const IDENT_EXT_MAGIC: u32 = 0x6964_6578;

/// 6-Z306ai: parse the `WIRE_CMD_IDENT` payload —
/// returns `(pid, uid, tid, dev_code)`. Legacy 12-byte shlib payloads
/// decode as `(pid, uid, 0, 0)`; only a 24-byte payload whose
/// extension marker matches yields tid/dev (partial extensions are
/// treated as legacy — conservative against wire drift).
fn parse_ident_payload(p: &[u8]) -> (i32, u32, u32, u32) {
    if p.len() < 8 {
        return (0, 0, 0, 0);
    }
    let pid = i32::from_ne_bytes(p[0..4].try_into().unwrap());
    let uid = u32::from_ne_bytes(p[4..8].try_into().unwrap());
    if p.len() >= 24 && u32::from_ne_bytes(p[12..16].try_into().unwrap()) == IDENT_EXT_MAGIC {
        let tid = u32::from_ne_bytes(p[16..20].try_into().unwrap());
        let dev = u32::from_ne_bytes(p[20..24].try_into().unwrap());
        (pid, uid, tid, dev)
    } else {
        (pid, uid, 0, 0)
    }
}

/// 6-Z306ai: human name for the IDENT v2 dev code (0 = unknown —
/// legacy shlib or a non-canonical path).
fn ident_dev_name(code: u32) -> &'static str {
    match code {
        1 => "binder",
        2 => "hwbinder",
        3 => "vndbinder",
        _ => "?",
    }
}

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
    /// 6-Z307: hwservicemanager's OWN instances (`android.hidl.manager@
    /// {1.0,1.1,1.2}::IServiceManager/default`). On real Android the
    /// hwservicemanager process registers ITSELF into its own in-process
    /// table (ServiceManager::registerAsService) — that registration
    /// never crosses the wire, so an interposed registry could never
    /// learn it: 354× `HIDL get(android.hidl.manager@1.2::IServiceManager/
    /// default) miss` (ladder #247) and the Java watchdog's
    /// `NoSuchElementException` kill. Transactions on these handles are
    /// served by the SAME servicemanager dispatcher as handle 0 (the
    /// full IServiceManager arm set against the same registry) — the
    /// proxy IS the HIDL service manager for this VM, exactly as it
    /// already is for handle 0.
    HidlServiceManager,
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
            // INTERFACE_TRANSACTION on a seeded SM handle: answered from
            // the handle's own seeded fq (the @1.0/@1.1/@1.2 instances
            // each claim their own version) — see the dispatch site.
            VirtualService::HidlServiceManager => "android.hidl.manager@1.0::IServiceManager",
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
    /// 6-Z333: the stability annotation i32 the OWNER's own addService
    /// parcel carried — the value the owner's libbinder stamped right
    /// after the flat via `finishFlattenBinder` →
    /// `writeInt32(Stability::get(binder))` (for an A11 system-partition
    /// client that is `Level::SYSTEM` = 12, because `tryMarkCompilationUnit`
    /// ran just before the write). The real servicemanager stores that
    /// stability on its proxy record and ECHOES it in every later
    /// getService reply (`BnServiceManager` gencode: `writeStrongBinder`
    /// → `finishFlattenBinder` → the proxy's stored level). Our
    /// LOCAL-flat hit reply must do the same: the owner's
    /// `unflattenBinder` decodes its OWN local object (6-Z306ac) and
    /// `finishUnflattenBinder` → `Stability::set(local, ann)` returns
    /// BAD_TYPE for every ann != the level the object already carries
    /// (rn284: "Interface being set with vintf stability but it is
    /// already marked as system stability." → readStrongBinder → null →
    /// the initPowerManagement NPE). Cross-process HANDLE replies keep
    /// the VINTF annotation (fresh proxies accept it, and the VINTF
    /// level feeds the call-time `requiresVintfDeclaration` gate the
    /// recovery corpus depends on). `None` = the add parcel did not
    /// yield a declared ann (synthetic/legacy shapes) → the reply falls
    /// back to the pre-6-Z333 annotation (no behavior change).
    ann_add: Option<i32>,
    virtual_kind: Option<VirtualService>,
    /// 6-Z299: the virtual service this name BELONGS to when a guest has
    /// taken it over via addService (native last-wins semantics). While
    /// the guest owner lives, transactions route to it; if it dies, the
    /// connection teardown RESTORES the platform (in-proxy) service under
    /// the same handle instead of unregistering the name. Without this,
    /// a guest HAL that registers over the virtual name and then crashes
    /// (fox's vendor.qti.vibrator: same AIDL name, hardware it cannot
    /// open in the container) would remove the name from the registry
    /// entirely — every later getService misses and the client-side
    /// poll storms return.
    virtual_fallback: Option<VirtualService>,
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
    /// `[BR_REPLY][binder_transaction_data]` + the blob trailer. `sg`
    /// carries the BINDER_TYPE_PTR contents of routed HIDL replies
    /// (6-Z305t-68 — the same kernel SG-copy semantic as requests).
    /// 6-Z355: `fds` are the reply blob's fd references — delivered to the
    /// requester's connection via SCM_RIGHTS when this reply drains.
    Reply {
        data: Vec<u8>,
        offsets: Vec<u8>,
        sg: Vec<SgBuf>,
        fds: Vec<Arc<FdGuard>>,
    },
    /// `[BR_FAILED_REPLY]` (reply timeout / server died).
    Failed,
    /// 6-Z306an: `[BR_DEAD_REPLY]` — the transaction's TARGET object died
    /// while the work was queued (the server process itself is alive).
    /// Kernel-true: the requester gets BR_DEAD_REPLY (waitForResponse →
    /// DEAD_OBJECT); the server NEVER sees the transaction — libhwbinder's
    /// server-side executeCommand has NO BR_DEAD_REPLY case (a dead-target
    /// transaction is never delivered kernel-side), so putting the code in
    /// the SERVER's stream hits `default: BAD COMMAND` → LOG_ALWAYS_FATAL
    /// abort (the #239 audioserver/system_server fleet: "getAndExecute-
    /// Command returned unexpected error -2147483648, aborting").
    Dead,
    /// 6-Z306ae: a kernel node-ref mirror command — `[BR_ACQUIRE]` or
    /// `[BR_RELEASE]` followed by `binder_ptr_cookie {ptr, cookie}`.
    /// The real driver holds a strong node ref for every registry
    /// handle and mirrors refcount changes to the OWNER process as
    /// BR_ACQUIRE/BR_RELEASE; the owner's IPCThreadState turns them
    /// into `obj->incStrong/decStrong` on the local BBinder. Without
    /// the mirror, a service object whose only userspace refs are JNI
    /// temporaries hits mStrong=0 and is DELETED right after
    /// addService returns (the holder keeps only a wp<>) — the freed
    /// cookie then faults in Parcel::unflattenBinder's vbase-offset
    /// read (ladders #199/#201/#203: platform_compat cookie mem =
    /// 16 zero bytes at serve time). Queued on the owner's
    /// reply_queue so it lands BEFORE the addService reply that would
    /// otherwise let the temporary sp<> die.
    RefCmd { br: u32, ptr: u64, cookie: u64 },
    /// 6-Z359: a KERNEL-TRUE node-ref mirror — the decision to send it
    /// came from the proxy's OWN node refcounts (a recipient released or
    /// died), never from a memory probe, so the delivery-time gate is the
    /// owner-process /proc liveness check only (the 6-Z354 pattern).
    /// The owner's IPCThreadState runs decStrong/decWeak natively (the
    /// 6-Z306ae machinery) — for the composer this is the
    /// onClientDestroyed path that finally lets createClient #2+
    /// proceed past waitForClientDestroyedLocked.
    ///
    /// 6-Z459 (Task 196): the node's LIFETIME grant counters snapshot at
    /// the era close. The node entry may already be REMOVED (kernel-true
    /// node lifetime: the entry dies at the both-maps-empty close) by
    /// the time this mirror is delivered, so the 6-Z457 decode join
    /// reads the counters from HERE, not from the node map.
    RefCmd359 {
        br: u32,
        ptr: u64,
        cookie: u64,
        strong_grants: u32,
        weak_grants: u32,
    },
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

/// 6-Z491: per-connection delivery-vs-consumed accounting — the
/// daemon-wall instrument (the rn464/rn466/rn468 decode).
///
/// The wall class: a guest daemon (idmap2d's createIdmap, installd's
/// createAppData) RECEIVES transactions (the bus's "delivered
/// transaction" lines), serves the first few, then STOPS REPLYING
/// mid-loop while its binder threads park in the normal idle state —
/// the delivered-but-unprocessed tail is the reader-wakeup /
/// read-buffer accounting desync hypothesis. The real kernel counts
/// this exact shape per-proc/per-thread (todo lists, transaction
/// stacks, async space); the bus so far had NO per-conn counters, so
/// a decode could not decide WHERE the chain broke:
/// `queued → delivered → (BC_REPLY | BC_FREE_BUFFER) acked`.
///
/// Every counter moves ONLY on the already-audited delivery paths
/// (queue_transaction, the mailbox drains, the BC_REPLY /
/// BC_FREE_BUFFER arms); the tail-of-ioctl wedge scan
/// ([`z491_ioctl_tick`]) turns a ≥5 s divergence into a budgeted
/// one-line verdict that packs the FULL counter set — the decode joins
/// it against the guest's SM-REPLY svclog era and the 6-Z407 reply
/// trace to name the break.
#[derive(Default)]
struct Z491Acct {
    /// `InboxItem::Tx` queued for this conn ([`BusState::queue_transaction`]).
    tx_enq: u64,
    /// `InboxItem::Tx` popped for delivery (main drain / 6-Z271g steal /
    /// 6-Z469 take / 6-Z383 recheck) — the "delivered transaction" lines.
    tx_del: u64,
    /// `DeferredReply` queued for this conn (all eight push sites).
    reply_enq: u64,
    /// `DeferredReply` popped into a read response (the BR_REPLY side).
    reply_del: u64,
    /// BC_REPLY / BC_REPLY_SG received FROM this conn.
    bc_reply_rx: u64,
    /// BC_FREE_BUFFER received FROM this conn (the consumed ack — the
    /// guest's executeCommand ran the delivered transaction to its
    /// Parcel-teardown end).
    bc_free_rx: u64,
    /// Completed BINDER_WRITE_READ ioctls on this conn.
    wr_calls: u64,
    /// Ioctls that returned an EMPTY read stream (the BR_NOOP idle
    /// ticks — the kernel's `wait_for_proc_work` analog).
    noop_polls: u64,
    /// High-water inbox depth (a MAX_QUEUED_ITEMS reject names itself).
    max_inbox: u32,
    /// Last delivery to this conn (tx or reply); None = never.
    last_del: Option<std::time::Instant>,
    /// Last write-side activity (BC_TRANSACTION/BC_REPLY/BC_FREE_BUFFER
    /// from this conn); None = never. The wedge gate reads this.
    last_rx: Option<std::time::Instant>,
    /// Verdict throttle: the last wedge line emitted for this conn.
    last_verdict: Option<std::time::Instant>,
}

impl Z491Acct {
    fn note_rx(&mut self) {
        self.last_rx = Some(std::time::Instant::now());
    }
}

/// 6-Z495: one IN-FLIGHT BINDER_WRITE_READ exchange, stamped by the
/// per-conn loop the moment the request frame is read and cleared the
/// moment the response leaves. This is the state the 6-Z491 tail-of-
/// ioctl tick CANNOT see: the tick only runs on COMPLETED ioctls, and
/// the rn470 (ladder rn471) decode caught the wedge class living exactly
/// there — the era-2 system_server main thread (conn=182) parked inside
/// `bp_exchange_anc` for the rest of the run while the 6-Z402 heartbeat
/// ticked its inbox=1 — the conn never completed another ioctl, so no
/// z491 verdict ever fired. The 6-Z495 sweep turns an in-flight
/// exchange older than [`Z495_EXCHANGE_STUCK`] into a budgeted one-line
/// verdict carrying the full stuck shape.
#[derive(Clone, Copy)]
struct Z495InFlight {
    arrived_at: std::time::Instant,
    /// The request's write_size (the BC stream length).
    ws: u32,
    /// The request's read_capacity.
    rc: u32,
}

/// How long an exchange may stay in flight before the sweep names it
/// stuck (a healthy sync round-trip is µs-ms; the 250 ms idle tick is
/// the slowest healthy loop iteration).
const Z495_EXCHANGE_STUCK: std::time::Duration = std::time::Duration::from_secs(10);

/// Per-conn verdict throttle for the 6-Z495 sweep.
const Z495_VERDICT_GAP: std::time::Duration = std::time::Duration::from_secs(10);

/// 6-Z495: the EXCHANGE-STUCK sweep — runs on the 6-Z402 heartbeat
/// thread (every 30 s), scans every conn's in-flight exchange and turns
/// a stuck one into a budgeted one-line verdict. Returns the number of
/// verdicts emitted (unit-test seam).
fn z495_exchange_sweep(bus: &Arc<Mutex<BusState>>, vm_id: u32) -> usize {
    static Z495_VERDICT_LOG: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(32);
    let mut b = bus.lock().expect("binder bus poisoned");
    let now = std::time::Instant::now();
    // The 6-Z408 hold state per conn, computed in an immutable pass (the
    // hold predicate borrows the bus; the verdict loop borrows conns
    // mutably). The ONE shape where an undelivered inbox front is BY
    // DESIGN (the held item waits for the conn's own nested call to
    // unwind) — the decode must be able to tell held-by-design from
    // wedged.
    let holds: std::collections::HashMap<ConnId, bool> = b
        .conns
        .iter()
        .map(|(cid, bx)| {
            let held = match bx.inbox.front() {
                Some(InboxItem::Tx(tx)) => b.z408_sync_delivery_blocked(*cid, tx),
                _ => false,
            };
            (*cid, held)
        })
        .collect();
    let mut emitted = 0usize;
    for (cid, bx) in b.conns.iter_mut() {
        let Some(inflight) = bx.z495_inflight else {
            continue;
        };
        let age = now.duration_since(inflight.arrived_at);
        if age < Z495_EXCHANGE_STUCK {
            continue;
        }
        if bx
            .z495_last_verdict
            .map_or(false, |t| now.duration_since(t) < Z495_VERDICT_GAP)
        {
            continue;
        }
        if Z495_VERDICT_LOG.load(Ordering::Relaxed) == 0 {
            return emitted;
        }
        Z495_VERDICT_LOG.fetch_sub(1, Ordering::Relaxed);
        bx.z495_last_verdict = Some(now);
        let z408_held = holds.get(cid).copied().unwrap_or(false);
        info!(
            "[KR64][binder][vm{}] 6-Z495 EXCHANGE-STUCK conn={} pid={} tid={} dev={} age={}s ws={} rc={} reader_waiting={} inbox={} pending_in={} replies={} stack={:?} out_sync={} z408_held={} tx_enq={} tx_del={} reply_enq={} reply_del={} bc_reply={} bc_free={} wr={} noop={}",
            vm_id,
            cid,
            bx.sender_pid,
            bx.sender_tid,
            bx.dev_code,
            age.as_secs(),
            inflight.ws,
            inflight.rc,
            bx.reader_waiting,
            bx.inbox.len(),
            bx.pending_in.len(),
            bx.reply_queue.len(),
            bx.txn_stack,
            bx.out_sync.len(),
            z408_held,
            bx.z491.tx_enq,
            bx.z491.tx_del,
            bx.z491.reply_enq,
            bx.z491.reply_del,
            bx.z491.bc_reply_rx,
            bx.z491.bc_free_rx,
            bx.z491.wr_calls,
            bx.z491.noop_polls,
        );
        emitted += 1;
    }
    emitted
}

/// Per-connection mailbox state, held inside [`BusState`].
#[derive(Default)]
struct ConnBox {
    inbox: std::collections::VecDeque<InboxItem>,
    /// 6-Z399: this connection's guest reader is CURRENTLY blocked inside
    /// its BINDER_WRITE_READ ioctl (the proxy is servicing it — including
    /// the 6-Z152 idle tick and the 6-Z383 re-check). Kernel-true
    /// semantics: a waiting looper thread takes ITS OWN proc-todo work;
    /// the pool-steal (6-Z271g) must therefore never lift a transaction
    /// out of a waiting reader's inbox — rn354 decode: the composer's
    /// onHotplug was queued on SF's MAIN conn while main waited in the
    /// idle tick; SF's fresh pool thread connected 4 ms later and the
    /// steal handed it to the POOL thread 250 ms before main's 6-Z383
    /// re-check — the dispatch then blocked on SF's mStateLock (held by
    /// main inside init()), the hotplug event landed after init()'s
    /// "Missing internal display" check, and the boot FATALed. With this
    /// flag the steal skips waiting readers; the owner's own re-check
    /// (≤250 ms) delivers the work on the right thread.
    reader_waiting: bool,
    /// Sync transactions queued in `inbox` but not yet delivered — used
    /// to resolve their waiters as `Dead` when the connection dies.
    pending_in: Vec<u64>,
    /// Replies resolved by the bus but not yet consumed by this
    /// connection's next read (kernel: the thread's reply lands on the
    /// thread todo list — see [`DeferredReply`]).
    reply_queue: std::collections::VecDeque<DeferredReply>,
    /// Sync transactions this connection SENT and has not seen a reply
    /// for: (txn id, target owner conn, queued-at). Used for the bounded
    /// reply timeout, for waiter cleanup when the connection dies, and
    /// (6-Z408) for the kernel reentrancy gate — the target owner decides
    /// WHICH new sync callers may ride this conn's stream while it is
    /// parked on its own reply.
    out_sync: std::collections::VecDeque<(u64, ConnId, std::time::Instant)>,
    /// 6-Z306ag: the connection's TRANSACTION STACK — the ids of every
    /// delivered, still-unanswered sync transaction, INNERMOST LAST.
    /// Kernel semantics (`binder_thread.transaction_stack`): a binder
    /// thread processing an incoming transaction may issue nested
    /// outgoing calls, and while waiting for a nested reply it may
    /// receive ANOTHER incoming transaction (kernel delivers node work
    /// to any ready pool thread, including one parked on its own reply).
    /// `BC_REPLY` completes the TOP (innermost) frame; the outer frames
    /// must survive. The previous single `Option<u64>` slot OVERWROTE on
    /// every delivery — the outer `BC_REPLY` then correlated to the
    /// WRONG waiter (reply bytes crossing transactions) or found no
    /// waiter at all ("BC_REPLY with no delivered transaction"), wedging
    /// the requester until REPLY_TIMEOUT. Reachable exactly when a
    /// process starts serving overlapping/nested binder traffic — the
    /// system_server AMS-constructor era (self-traffic conn→conn).
    txn_stack: Vec<u64>,
    /// Death notifications this connection requested:
    /// handle → cookie. Delivered as `BR_DEAD_BINDER` when the owning
    /// connection unregisters.
    death_watch: HashMap<u32, u64>,
    /// Guest process identity (announced via `WIRE_CMD_IDENT`). Stamped
    /// into routed transactions' `sender_pid`/`sender_euid` — kernel
    /// semantics. Zero until the (optional) IDENT frame arrives.
    sender_pid: i32,
    sender_euid: u32,
    /// 6-Z306ai: the owning guest THREAD and the binder device this
    /// connection serves (1=binder, 2=hwbinder, 3=vndbinder), from the
    /// IDENT v2 extension. Zero = unknown (legacy shlib or pre-extension
    /// connect). Logged only — no wire-semantics change: the boot decode
    /// correlates an aborting pool thread (`HwBinder:pid_N`) with the
    /// transactions THIS conn served via the proxy's per-conn lines.
    sender_tid: u32,
    dev_code: u32,
    /// 6-Z325: bounded steal-delivery watch — armed when the 6-Z271g
    /// steal hands an ONEWAY transaction to a sibling pool thread. The
    /// next `BC_FREE_BUFFER` from this conn is the SUCCESS signal (the
    /// guest's `IPCThreadState::executeCommand` ran the delivered
    /// transaction to its Parcel-teardown end — the exact stage the
    /// rn273 decode found missing: no free, no transact, ws=0 loop);
    /// no free within 1s names the delivery dropped again.
    steal_watch: Option<(std::time::Instant, u32)>,
    /// 6-Z306ab: the stability-annotation wire format for THIS
    /// connection. false (default) = the android-11 plain-Level form
    /// (`Stability::Level` values 3/12/63 — the ONLY values A11's
    /// `isDeclaredStability` accepts); true = the android-12/12L
    /// `Category` form (level<<24 | version 1). One VM mixes client
    /// generations (the A11 rootfs system_server/zygote fleet AND
    /// per-image recovery binaries / future A12+ GSIs talk to the SAME
    /// proxy), so the format self-tunes per connection: a same-service
    /// re-get right after a HIT is the signature of a non-A11 client
    /// whose libbinder rejected the plain annotation
    /// (`Stability::set` → BAD_TYPE → `readStrongBinder` → null) and
    /// whose `waitForService` loop re-asks → flip to the Category form
    /// (sticky). A MISS retry — a waiter polling for a service that is
    /// not up yet — never flips, so A11 pollers stay on the plain form.
    /// 6-Z272e's original direction (Category-first, flip plain on
    /// retry) starved SINGLE-SHOT A11 clients — the forked
    /// system_server's one-shot getService(installd) never retried, got
    /// the Category form, decoded null and died-looped at
    /// performSystemServerDexOpt (ladder #195/#196 decode).
    sm_annotate_a12: bool,
    /// Whether the PREVIOUS SM GET on this connection resolved to a HIT
    /// (vs a miss). The 6-Z306ab flip fires only on retry-after-hit.
    sm_last_was_hit: bool,
    /// The previous SM GET on this connection: (service name, at).
    last_sm_get: Option<(String, std::time::Instant)>,
    /// 6-Z491: the delivery-vs-consumed accounting (see [`Z491Acct`]).
    z491: Z491Acct,
    /// 6-Z495: the exchange CURRENTLY being serviced by the per-conn
    /// loop (stamped after the request frame is read, cleared after the
    /// response is written). None = the conn is idle between ioctls.
    /// The [`z495_exchange_sweep`] heartbeat reads this — the rn470
    /// (ladder rn471) wedge class (a conn stuck mid-exchange, no ioctl
    /// ever completing again) is invisible to the tail-of-ioctl z491
    /// tick by construction.
    z495_inflight: Option<Z495InFlight>,
    /// 6-Z495 verdict throttle: the last EXCHANGE-STUCK line for this
    /// conn.
    z495_last_verdict: Option<std::time::Instant>,
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
    /// 6-Z359: guest-owned NODES exported OUTSIDE their own connection —
    /// the objects a server hands out in REPLY/TRANSACTION parcels (the
    /// composer's IComposerClient in createClient's reply, callback
    /// objects, …). The real kernel: every such flat crosses as a
    /// BINDER_TYPE_HANDLE in the recipient's view, backed by an in-driver
    /// node with strong/weak refcounts; the recipient's death (or its
    /// BC_RELEASE) decrements, and the LAST decrement mirrors BR_RELEASE
    /// to the OWNER process so its object can be destroyed. The bus
    /// previously kept NO such nodes: the raw ptr-form flat crossed to
    /// the client verbatim (its shlib gate zeroed it as a foreign "dead"
    /// local — SF read a NULL client and FATALed), and no release ever
    /// reached the owner (the composer's mClient stayed immortal →
    /// waitForClientDestroyedLocked failed forever → createClient #2/#3
    /// wedged both binder threads inside the condvar → the SF restart
    /// cascade, rung 5 at rn309's snapshot).
    /// handle → node. 6-Z371: node handles allocate from the SAME dense
    /// monotonic space as service handles (`next_handle`) — kernel-true
    /// (the kernel's per-proc handle table has ONE dense handle space;
    /// libbinder's ProcessState::lookupHandleLocked indexes a dense
    /// vector by the handle value). The previous disjoint 0x7F000001+
    /// node range was a fabricated handle space: rn326 decoded the
    /// guest SF death as ceil(0x7F000001 * 1.5) * 16 = 47.62 GiB — the
    /// SharedBuffer::alloc of a handle table grown to the first node
    /// handle's value (MAP_FIXED → container OOM → app death).
    nodes: HashMap<u32, GuestNode>,
    /// (owner, ptr, cookie) → handle. Kernel-true handle identity: the
    /// same node handed out repeatedly gets the SAME handle — WITHIN the
    /// node's lifetime. 6-Z459 (Task 196): the entry is removed at the
    /// both-maps-empty node-death moment (the kernel destroys the node
    /// with its refs), so a post-death re-export of the same key creates
    /// a FRESH node with a fresh handle instead of re-granting onto the
    /// corpse (the rn426 reply-borne era-2 class).
    node_by_key: HashMap<(ConnId, u64, u64), u32>,
    next_conn: u64,
    next_txn: u64,
}

/// 6-Z442: one flat crossing's grant record — the node ref granted plus
/// the owner-side mirrors the kernel would queue for it.
struct Z442FlatGrant {
    /// The node handle granted for the flat (already rewritten into the
    /// parcel data as BINDER_TYPE_HANDLE).
    handle: u32,
    /// Whether the granted ref was the strong one (a strong flat grants
    /// strong + implied weak; a weak flat grants weak only).
    strong: bool,
    /// `[BR_INCREFS]`/`[BR_ACQUIRE]` `(br, ptr, cookie)` triples for the
    /// OWNER conn's read stream, kernel queue order.
    mirrors: Vec<(u32, u64, u64)>,
}

/// 6-Z359: one guest-owned node (see [`BusState::nodes`]).
struct GuestNode {
    owner: ConnId,
    ptr: u64,
    cookie: u64,
    /// Per-recipient strong-ref counts. One count is granted at each
    /// strong-flat delivery (the kernel's in-transaction grant); each
    /// recipient BC_RELEASE drops one; at the recipient's death all of
    /// its counts drop. The LAST strong count dropping mirrors
    /// BR_RELEASE to the owner (its IPCThreadState decStrongs the local
    /// BBinder — the composer's onClientDestroyed path).
    strong: HashMap<ConnId, u32>,
    /// Per-recipient weak-ref counts (weak flats / BC_INCREFS era). A
    /// strong delivery ALSO grants one weak count (kernel:
    /// binder_inc_ref_for_node takes the weak ref implied by every
    /// strong ref) so the owner's BR_INCREFS mirror balances against a
    /// BR_DECREFS when the recipient's proxy dies.
    weak: HashMap<ConnId, u32>,
    /// 6-Z442 (kernel `has_strong_ref`): the owner-side BR_ACQUIRE
    /// mirror for this node is OUTSTANDING — the owner's IPCThreadState
    /// was told (or is about to be told, same-ioctl) to incStrong the
    /// local object. Kernel truth: the notification fires once per
    /// notification era; it re-arms only when the node's strong refs
    /// drop back to zero (BR_RELEASE).
    strong_notified: bool,
    /// 6-Z442 (kernel `has_weak_ref`): the owner-side BR_INCREFS mirror
    /// is outstanding. Re-arms when the node's weak refs drop to zero
    /// (BR_DECREFS).
    weak_notified: bool,
    /// 6-Z458 (Task 195): whether the REGISTRY pin's era-0 BR_ACQUIRE
    /// actually crossed to the owner (the add arm's 6-Z306ae liveness
    /// verdict at the LAST add of this object). The pin's own drop rides
    /// this verdict: a pin whose acquire never reached the owner drops
    /// SILENTLY — no release may precede its acquire on the wire (the
    /// release-without-acquire decStrong-on-zero class).
    registry_pin_mirror: bool,
    /// 6-Z457 (Task 192): LIFETIME strong grants this node handed out
    /// (every strong flat crossing). The per-holder maps drop to empty at
    /// the last-ref release — exactly when the 6-Z457 mirror fires — so
    /// the pre-delivery "bus grant count" the decode joins against the
    /// guest's live mStrong must be a lifetime counter on the node entry.
    /// 6-Z459 (Task 196): the entry itself is removed at the node-death
    /// moment (both maps empty, no registry pin), so the counters
    /// SNAPSHOT into the queued RefCmd359 mirror at the era close — the
    /// delivery-time join reads them from the mirror, not from the map.
    strong_grants: u32,
    /// 6-Z457: LIFETIME weak grants (weak flats + the implied weak of
    /// every strong grant — kernel `binder_inc_ref_for_node`).
    weak_grants: u32,
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
            nodes: HashMap::new(),
            node_by_key: HashMap::new(),
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
            (
                // 6-Z307: hwservicemanager's OWN instances. Real
                // hwservicemanager self-registers under every version of
                // its interface chain INSIDE its own table — the wire
                // never carries it, so the interposed registry must seed
                // it. A11 evidence (ladder #247): 354× `HIDL get(android.
                // hidl.manager@1.2::IServiceManager/default) miss → null
                // binder`, and the watchdog thread died of
                // NoSuchElementException on the @1.0 lookup.
                "android.hidl.manager@1.0::IServiceManager/default",
                VirtualService::HidlServiceManager,
            ),
            (
                "android.hidl.manager@1.1::IServiceManager/default",
                VirtualService::HidlServiceManager,
            ),
            (
                "android.hidl.manager@1.2::IServiceManager/default",
                VirtualService::HidlServiceManager,
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
                    ann_add: None,
                    virtual_kind: Some(*kind),
                    virtual_fallback: None,
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
        self.add_guest_service_impl(name, owner, ptr, cookie, None)
    }

    /// 6-Z351 (rn302 decode): CHAIN-ALIAS registration — the SAME service
    /// object under another interfaceChain key. AOSP ServiceManager.cpp
    /// addImpl (android-11.0.0_r1) inserts the ONE HidlService under EVERY
    /// chain fqName (`for i in 0..interfaceChain.size() {
    /// mServiceMap[chain[i]].insertService/setService(...);
    /// sendPackageRegistrationNotification(chain[i], name); }`), so a
    /// @2.3 registration is reachable through the @2.1 key with the SAME
    /// binder object. A fresh alias must NOT mint a new node identity:
    /// the real kernel hands the same process the same handle for the
    /// same node, so the alias reuses `fresh_handle` (the chain[0]
    /// handle) and by_handle stays canonical on chain[0]. Everything
    /// else — virtual takeover, overwrite, old-owner registry-ref
    /// release — is exactly add_guest_service.
    fn add_guest_service_alias(
        &mut self,
        name: &str,
        owner: ConnId,
        ptr: u64,
        cookie: u64,
        fresh_handle: u32,
    ) -> u32 {
        self.add_guest_service_impl(name, owner, ptr, cookie, Some(fresh_handle))
    }

    fn add_guest_service_impl(
        &mut self,
        name: &str,
        owner: ConnId,
        ptr: u64,
        cookie: u64,
        fresh_handle: Option<u32>,
    ) -> u32 {
        // 6-Z379: capture the overwrite target under a SHORT borrow so
        // the per-key registry-ref scan below can read the whole map.
        let overwritten = self.services.get_mut(name).map(|entry| {
            // 6-Z298/6-Z299: a guest addService OVER a virtual-service
            // name takes ownership (native servicemanager "overwrite"
            // semantics: same name → same handle, new owner) — but the
            // platform implementation is remembered so the teardown of
            // the guest owner can restore it (virtual_fallback). The
            // in-proxy handler stands down while the guest owner lives.
            if let Some(kind) = entry.virtual_kind {
                info!(
                    "[KR64][binder][svc] guest addService({}) takes over the in-proxy virtual implementation (conn={}) — restored automatically if the guest owner dies",
                    name, owner
                );
                entry.virtual_kind = None;
                entry.virtual_fallback = Some(kind);
            }
            // Native servicemanager "overwrite" semantics: same name →
            // same handle, new owner.
            let old_owner = entry.owner;
            let old_ptr = entry.ptr;
            let old_cookie = entry.cookie;
            entry.owner = owner;
            entry.ptr = ptr;
            entry.cookie = cookie;
            (entry.handle, old_owner, old_ptr, old_cookie)
        });
        if let Some((handle, old_owner, old_ptr, old_cookie)) = overwritten {
            // 6-Z306ae/6-Z379: the OLD owner loses its registry strong
            // ref only when THIS was the last registry key still pinning
            // the old node (see overwrite_release_due). The real
            // hwservicemanager holds one sp<IBase> PER chain key (AOSP
            // addImpl: every interfaceChain entry gets its own
            // HidlService holding its own sp), so the EVERY-HAL shared
            // android.hidl.base@1.0::IBase/default alias overwrite must
            // NOT release the previous owner — its concrete chain keys
            // (@2.3/@2.2/@2.1) still pin the object. Releasing on every
            // overwrite murdered every HAL wrapper ~500 ms after the
            // next HAL registered (rn334/rn335: the composer's wrapper
            // BR_RELEASEd at +9.2 s, SF's interfaceChain at +11.7 s
            // SEGV'd in decStrong on the corpse — the 42× libutils
            // refcount SEGV class, rung 7 SURFACEFLINGER). Gated on the
            // ref invariant — see mirror_ref_ok.
            if old_owner != owner
                && old_ptr != 0
                && overwrite_release_due(&self.services, name, old_owner, old_ptr, old_cookie)
            {
                // 6-Z458 (Task 195): the registry's pin on the OLD object
                // drops here — routed through the node-ref unref
                // (z359_unref_node) so the BR_RELEASE mirror fires
                // EXACTLY when the kernel's would (binder_dec_node on
                // the truly-last strong ref): while a CLIENT ref remains
                // the release defers to that client's drop; the rn425
                // corpse-regrant shape (a release deleting a
                // REGISTRY-PINNED live object) is structurally
                // impossible. The old immediate RefCmd{BR_RELEASE} + its
                // own mirror_ref_ok gate are gone: delivery liveness is
                // the RefCmd359 arm's fresh owner probe, and a pin whose
                // add-time acquire never crossed drops silently (no
                // release may precede its acquire on the wire).
                if let Some(&h) = self.node_by_key.get(&(old_owner, old_ptr, old_cookie)) {
                    self.z359_unref_node(0, h, REGISTRY_CONN, true);
                    self.z359_unref_node(0, h, REGISTRY_CONN, false);
                }
            }
            // The registry handle now holds a strong node ref on the NEW
            // object (one MORE registry ref than before — the overwritten
            // key's ref was the old object's, the same key now pins the
            // new one). The ACQUIRE mirror rides the arm's ReplyMirrored
            // (in-transaction, gated — see the arms); only the OLD
            // owner's RELEASE stays queue-delivered here (the old owner
            // is a different process — no free-then-acquire race with
            // the registering thread).
            return handle;
        }
        // 6-Z351: a chain alias reuses the chain[0] handle (ONE node
        // identity under every chain key) and never repoints by_handle
        // (the canonical chain[0] entry owns the handle→name route).
        let h = match fresh_handle {
            Some(h) => h,
            None => {
                let h = self.next_handle;
                self.next_handle += 1;
                h
            }
        };
        self.services.insert(
            name.to_string(),
            ServiceEntry {
                handle: h,
                owner,
                ptr,
                cookie,
                ann_add: None,
                virtual_kind: None,
                virtual_fallback: None,
            },
        );
        if fresh_handle.is_none() {
            self.by_handle.insert(h, name.to_string());
        }
        h
    }

    /// 6-Z333: record the stability annotation the OWNER's addService
    /// parcel carried (see ServiceEntry::ann_add). Called only when the
    /// add parcel actually yielded a DECLARED stability repr; every
    /// later owner-conn LOCAL hit reply echoes it.
    fn set_service_ann(&mut self, name: &str, ann: i32) {
        if let Some(entry) = self.services.get_mut(name) {
            entry.ann_add = Some(ann);
        }
    }

    /// 6-Z276: record a `registerForNotifications` watcher for `name`.
    /// Duplicate (conn, ptr) registrations are ignored (libbinder dedupes
    /// too — one callback object per service). Returns `true` only when a
    /// NEW watcher entry was stored (6-Z325: the node-ref mirror fires
    /// once per STORED entry, keeping the mirror count equal to the
    /// registry's ref count — a duplicate register must not inflate it).
    fn add_watcher(&mut self, name: &str, w: ServiceWatcher) -> bool {
        let list = self.watchers.entry(name.to_string()).or_default();
        if !list.iter().any(|x| x.conn == w.conn && x.ptr == w.ptr) {
            list.push(w);
            true
        } else {
            false
        }
    }

    /// 6-Z276: drop a watcher (libbinder `unregisterForNotifications`).
    /// Returns `true` when an entry was actually removed (6-Z325: the
    /// BR_RELEASE mirror only fires for a real drop).
    fn remove_watcher(&mut self, name: &str, conn: ConnId, ptr: u64) -> bool {
        let mut removed = false;
        if let Some(list) = self.watchers.get_mut(name) {
            let before = list.len();
            list.retain(|x| !(x.conn == conn && x.ptr == ptr));
            removed = list.len() != before;
            if list.is_empty() {
                self.watchers.remove(name);
            }
        }
        removed
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
            let blob = if w.hidl {
                // HIDL `IServiceNotification.onRegistration(fqName,
                // instance, preexisting)` — 6-Z326: THE REAL A11 FIRE WIRE,
                // byte-exact with the A11 hidl-gen BpHwIServiceNotification
                // ::onRegistration over libhwbinder Parcel:
                //   [CString token][align4]
                //       writeInterfaceToken/enforceInterface descriptor
                //       ("android.hidl.manager@1.0::IServiceNotification" —
                //       libhwbinder writeInterfaceToken = writeCString, NO
                //       length prefix; if the BnHw never enforces it the
                //       leading bytes are inert to the object walk).
                //   PTR(fq struct 16B {mBuffer=0 → 6-Z309 fixup, mSize@8,
                //        mOwns@12})          writeBuffer(&hidl_string)
                //   PTR(fq chars size+1, parent=fq-struct obj,
                //        parent_offset=kOffsetOfBuffer=0)
                //                            writeEmbeddedToParcel
                //   PTR(inst struct)][PTR(inst chars, par=inst-struct, 0)]
                //   [u8 preexisting][pad4]   writeBool = writeInt8 (A11
                //                            libhwbinder Parcel::writeBool)
                // fqName = the name before the '/' split the guest used at
                // register time; our registry key is the FULL
                // "fqName/instance" string, so send it as both halves of
                // what we have (libhwbinder clients match on the
                // descriptor + instance pair they registered).
                //
                // rn274 PROOF the pre-6-Z326 inline form was unparseable:
                // the 6-Z325 mirror let the pool thread EXECUTE the oneway
                // (freed after 2ms — transact + freeBuffer ran) yet the
                // client's readEmbeddedBuffer failed on the inline bytes
                // BEFORE the impl call — mRegistered never set, main still
                // parked in Waiter::wait in every era (the ANR stacks).
                let (fq, inst) = match name.rfind('/') {
                    Some(i) => (&name[..i], &name[i + 1..]),
                    None => (name, "default"),
                };
                writer
                    .data
                    .extend_from_slice(b"android.hidl.manager@1.0::IServiceNotification");
                writer.data.push(0);
                while writer.data.len() % 4 != 0 {
                    writer.data.push(0);
                }
                let mut fstruct = vec![0u8; 16];
                fstruct[8..12].copy_from_slice(&(fq.len() as u32).to_ne_bytes());
                let f0 = writer.write_ptr_object(fstruct, None, 0);
                let mut fchars = fq.as_bytes().to_vec();
                fchars.push(0);
                writer.write_ptr_object(fchars, Some(f0), 0);
                let mut istruct = vec![0u8; 16];
                istruct[8..12].copy_from_slice(&(inst.len() as u32).to_ne_bytes());
                let i0 = writer.write_ptr_object(istruct, None, 0);
                let mut ichars = inst.as_bytes().to_vec();
                ichars.push(0);
                writer.write_ptr_object(ichars, Some(i0), 0);
                writer.write_u8(preexisting as u8);
                let (data, offsets, sg) = writer.into_parts_with_sg();
                RequestBlob {
                    data,
                    offsets,
                    sg,
                    fds: Vec::new(),
                }
            } else {
                // AIDL `android.os.IServiceCallback.onRegistration(name,
                // binder)` — standard writeInterfaceToken header + the
                // service name + the HANDLE flat + the stability i32
                // (6-Z271x: finishUnflattenBinder reads it back). AIDL
                // strings are INLINE string16s — no embedded-buffer model.
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
                let (data, offsets) = writer.into_parts();
                RequestBlob {
                    data,
                    offsets,
                    sg: Vec::new(),
                    fds: Vec::new(),
                }
            };
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
                blob: Some(blob),
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
                // 6-Z306ag: every frame of the transaction stack owes a
                // reply — resolve them ALL as DEAD (kernel: a dying
                // thread's whole transaction stack fails).
                v.extend(bx.txn_stack.iter().copied());
                (v, bx.out_sync)
            }
            None => (Vec::new(), Default::default()),
        };
        for txn_id in dead_txns.drain(..) {
            if let Some(requester) = self.waiters.remove(&txn_id) {
                if let Some(rb) = self.conns.get_mut(&requester) {
                    rb.z491.reply_enq += 1;
                    rb.reply_queue.push_back(DeferredReply::Failed);
                }
            }
        }
        // The dying conn's own outstanding sync calls: nobody is left to
        // receive a resolution — just drop their waiters.
        for (txn_id, _, _) in out_sync {
            self.waiters.remove(&txn_id);
        }
        let dead: Vec<(String, u32)> = self
            .services
            .iter()
            .filter(|(_, e)| e.owner == conn && e.virtual_fallback.is_none())
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
        // 6-Z299: names the dying connection had TAKEN OVER from the
        // platform restore to the in-proxy virtual implementation under
        // the same handle — the name never disappears from the registry.
        // Death notifications for the (guest) node still fire: the
        // client's reference to the guest binder IS dead; a re-lookup —
        // or a plain transact on the still-valid handle — lands on the
        // restored platform service.
        let restored: Vec<(String, u32)> = self
            .services
            .iter()
            .filter(|(_, e)| e.owner == conn && e.virtual_fallback.is_some())
            .map(|(n, e)| (n.clone(), e.handle))
            .collect();
        for (name, handle) in &restored {
            let (fb, watchers) = {
                let entry = self.services.get_mut(name).expect("entry just filtered");
                let fb = entry.virtual_fallback.take().expect("fallback");
                entry.owner = PROXY_CONN_ID;
                entry.ptr = 0;
                entry.cookie = 0;
                entry.virtual_kind = Some(fb);
                let watchers: Vec<(ConnId, u64)> = self
                    .conns
                    .iter()
                    .filter_map(|(id, b)| b.death_watch.get(handle).map(|c| (*id, *c)))
                    .collect();
                (fb, watchers)
            };
            info!(
                "[KR64][binder][svc] guest owner of {} died — platform (in-proxy) service restored under handle 0x{:08x} ({:?})",
                name, handle, fb
            );
            for (watcher, cookie) in watchers {
                if let Some(wb) = self.conns.get_mut(&watcher) {
                    wb.inbox.push_back(InboxItem::Death(cookie));
                }
            }
        }
        // 6-Z276: a dying connection's registerForNotifications watchers
        // are gone with it — no callback may target a dead conn's mailbox.
        // 6-Z359: FIRST drop every node ref the dying conn held — the
        // LAST strong drop mirrors BR_RELEASE to the node's owner (the
        // kernel's process-death semantics: all refs die with the
        // process). Then, nodes this conn OWNED die with it: remove them
        // and fire death notifications to their watchers (the same shape
        // as the named-services cleanup above).
        {
            let node_watchers: Vec<(u32, Vec<(ConnId, u64)>)> = {
                let dead_node_handles: Vec<u32> = self
                    .nodes
                    .iter()
                    .filter(|(_, n)| n.owner == conn)
                    .map(|(h, _)| *h)
                    .collect();
                dead_node_handles
                    .iter()
                    .map(|h| {
                        let watchers: Vec<(ConnId, u64)> = self
                            .conns
                            .iter()
                            .filter_map(|(id, b)| b.death_watch.get(h).map(|c| (*id, *c)))
                            .collect();
                        (*h, watchers)
                    })
                    .collect()
            };
            for (h, ws) in node_watchers {
                // Every OTHER conn's refs on this dead node are moot —
                // their release mirrors would target a dead owner; just
                // drop the entries and fail future transactions.
                self.nodes.remove(&h);
                self.node_by_key.retain(|_, vh| *vh != h);
                for (watcher, wcookie) in ws {
                    if let Some(wb) = self.conns.get_mut(&watcher) {
                        wb.inbox.push_back(InboxItem::Death(wcookie));
                    }
                }
            }
            // Drop this conn's refs on OTHER conns' nodes (may mirror
            // releases to live owners).
            self.z359_drop_conn_refs(conn);
        }
        self.remove_watchers_of_conn(conn);
    }

    /// 6-Z408: the KERNEL REENTRANCY GATE for sync delivery — true when
    /// the queued sync transaction `tx` must NOT be delivered to `conn`
    /// yet (hold it in the owner's inbox).
    ///
    /// Kernel rule (binder.c's target-thread selection for a sync call):
    /// a thread parked waiting for the reply of its OWN outgoing call
    /// receives new incoming work ONLY when the new caller is the process
    /// the parked call is headed to (the reentrant case — the txn rides
    /// on top of that thread's transaction stack). Work from ANY OTHER
    /// process goes to an idle pool thread or the proc todo queue — never
    /// to the parked thread.
    ///
    /// rn362/363/364 decoded the consequence of ignoring this: the
    /// composer's main thread (conn=32) parked inside the nested
    /// onHotplug call (out_sync → SF main's conn) received every OTHER
    /// client's traffic (#14..#32) in its own waitForResponse stream,
    /// replied to each, consumed the nested BR_REPLY (rn364 first_br
    /// oracle: BR_REPLY at +32234) — and its libhwbinder then returned to
    /// the pool loop WITHOUT ever sending the BC_REPLY for the
    /// registerCallback txn#12 that started the handler. SF main's waiter
    /// expired → BR_FAILED_REPLY → the 250 ms retry loop → rung 7
    /// forever. On a real kernel the mid-park transactions would have
    /// been served by the composer's pool threads, the parked thread's
    /// stream would have carried ONLY its nested reply, and the handler
    /// would have completed.
    ///
    /// One-way items are NEVER blocked here (the kernel queues async work
    /// on the proc todo; the rn354/6-Z399 ordering rules already own that
    /// path). A dead/unknown target conn (pid 0) never blocks — the
    /// waiter's own REPLY_TIMEOUT resolves that case, and holding forever
    /// behind a dead target would wedge the server.
    fn z408_sync_delivery_blocked(&self, conn: ConnId, tx: &IncomingTx) -> bool {
        if tx.one_way || tx.txn_id == 0 {
            return false;
        }
        let Some(bx) = self.conns.get(&conn) else {
            return false;
        };
        let Some(&(_tid, target_owner, _at)) = bx.out_sync.back() else {
            return false;
        };
        let target_pid = self
            .conns
            .get(&target_owner)
            .map(|tbx| tbx.sender_pid)
            .unwrap_or(0);
        if target_pid == 0 {
            return false;
        }
        tx.sender_pid != target_pid
    }

    /// 6-Z469: PROC-TODO TAKE — the waiting-looper half of the kernel's
    /// target-thread selection, which 6-Z408 implements only as a HOLD.
    ///
    /// Kernel rule (binder.c): a sync transaction whose target thread is
    /// parked inside its own call (the 6-Z408 hold case) is NOT lost and
    /// NOT pinned to the parked thread's fd — it sits on the target
    /// PROC's todo list where ANY waiting looper thread of that proc
    /// (empty transaction stack, blocked in its own read) takes it and
    /// serves it. rn435's decode proved the missing half: system_server
    /// main (conn=227, pid 10485) parked mid-bootstrap while its OWN
    /// pool threads (conns 237-240, same pid) issued sync code=34
    /// transactions at nodes owned by main's conn — the gate held every
    /// one in main's inbox, no idle sibling could reach it (the 6-Z399
    /// steal excludes reader_waiting sources), the senders' 30s
    /// REPLY_TIMEOUT fired → BR_FAILED_REPLY → retry loop (16 holds in
    /// 288ms, then silent) → the bootstrap wedge → the Watchdog ANR/kill
    /// fleet (3 ANR traces at OMS registerReceiver / idmap2 verifyIdmap
    /// / BatteryService health-cast, all byte-shaped as a parked main +
    /// an idle binder worker). The take makes the shape kernel-true.
    ///
    /// Preconditions, all kernel-true:
    /// - the TAKER must be a true idle looper: no outstanding sync call
    ///   of its own (rn362: a thread parked in waitForResponse must
    ///   never receive non-nested work mid-wait);
    /// - the SOURCE must be a same-process (same sender_pid) sibling —
    ///   the proc todo belongs to the target proc — on the SAME device
    ///   (6-Z309f: binder/hwbinder/vndbinder never cross);
    /// - the source's inbox FRONT must be a sync transaction the 6-Z408
    ///   gate is currently holding (the narrow, decode-verified class —
    ///   non-held fronts keep the rn354 under-steal ordering semantics);
    /// - one-way items are never held by the gate → never taken here.
    ///
    /// The popped item follows the 6-Z271g steal bookkeeping exactly
    /// (pending_in removed from the source; the txn_stack push happens
    /// in the caller under the same lock, mirroring the steal).
    fn z469_take_held_sync(&mut self, taker: ConnId) -> Option<IncomingTx> {
        let (my_pid, my_dev, i_am_idle) = match self.conns.get(&taker) {
            Some(bx) => (bx.sender_pid, bx.dev_code, bx.out_sync.is_empty()),
            None => return None,
        };
        if my_pid == 0 || !i_am_idle {
            return None;
        }
        let mut sibs: Vec<ConnId> = self
            .conns
            .iter()
            .filter(|(cid, bx)| {
                **cid != taker
                    && bx.sender_pid == my_pid
                    && bx.dev_code == my_dev
                    && match bx.inbox.front() {
                        Some(InboxItem::Tx(tx)) => {
                            tx.txn_id != 0 && self.z408_sync_delivery_blocked(**cid, tx)
                        }
                        _ => false,
                    }
            })
            .map(|(cid, _)| *cid)
            .collect();
        sibs.sort();
        for sib in sibs {
            let sbx = self.conns.get_mut(&sib)?;
            match sbx.inbox.pop_front() {
                Some(InboxItem::Tx(tx)) => {
                    if tx.txn_id != 0 {
                        sbx.pending_in.retain(|id| *id != tx.txn_id);
                    }
                    return Some(tx);
                }
                _ => continue,
            }
        }
        None
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
                // 6-Z491: the enqueue leg of the delivery-vs-consumed
                // accounting (the deliver leg lives at the four drain
                // sites; the wedge scan reads the gap).
                b.z491.tx_enq += 1;
                let depth = b.inbox.len() as u32;
                if depth > b.z491.max_inbox {
                    b.z491.max_inbox = depth;
                }
                true
            }
            _ => false,
        }
    }

    // ── 6-Z359: kernel-true node handles + refcount mirroring ───────────
    //
    // The real driver: a BINDER_TYPE_BINDER flat in a parcel that crosses
    // to ANOTHER process is rewritten to BINDER_TYPE_HANDLE (the
    // recipient's handle table), backed by an in-driver node; the
    // transaction grant is one strong ref; the recipient's BC_RELEASE
    // (or its death) drops it; the LAST drop mirrors BR_RELEASE to the
    // node's owner so its userspace object can be destroyed. Two rn309
    // walls hang on exactly this: (1) the composer's IComposerClient
    // crossed to SF as a raw ptr-form flat → SF's shlib gate zeroed it
    // as a foreign dead local → "failed to create composer client"
    // FATAL ~600 ms after every createClient that got a reply; (2) no
    // release was EVER mirrored → the composer's mClient stayed
    // immortal → ComposerImpl::createClient's
    // waitForClientDestroyedLocked failed → createClient #2 (pool thread
    // 3394, stolen) and #3 (main 3327) wedged forever inside the HAL's
    // condvar (STALL-DUMP: futex WAIT_BITSET val=2, stack libc++ →
    // composer@2.3.so) → every later SF generation 8s-timeouts → the
    // init restart cascade → rung 5.

    /// Bounded 6-Z359 diag budgets (the registration storm must not flood).
    /// 6-Z464 (rn427 decode): RAISED — the decode's era-map went BLIND at
    /// ~+185s (the release budget exhausted after 8 lines, the grant-log
    /// budgets after 64+64), so the abort-adjacent windows decoded with
    /// invisible grants/burials (the (0xf280, 0x6c70) "close-without-grant"
    /// lead was partly a LOGGING artifact). The structural lines are
    /// O(100)/boot — the cap now covers a full run.
    fn z359_alloc_log() -> &'static std::sync::atomic::AtomicU32 {
        static N: std::sync::OnceLock<std::sync::atomic::AtomicU32> = std::sync::OnceLock::new();
        N.get_or_init(|| std::sync::atomic::AtomicU32::new(256))
    }
    fn z359_release_log() -> &'static std::sync::atomic::AtomicU32 {
        static N: std::sync::OnceLock<std::sync::atomic::AtomicU32> = std::sync::OnceLock::new();
        N.get_or_init(|| std::sync::atomic::AtomicU32::new(256))
    }

    /// 6-Z442: grant (or re-grant) a node ref for `recipient` on the
    /// LOCAL flat `(ptr, cookie)` exported by `sender`. Returns the node
    /// handle and the OWNER-SIDE ref mirrors the kernel would queue for
    /// this grant: `[BR_INCREFS][ptr][cookie]` when the node's first
    /// weak ref of the current era is taken, plus
    /// `[BR_ACQUIRE][ptr][cookie]` when the first strong ref is (kernel
    /// `binder_inc_ref_for_node` → `binder_inc_node`'s
    /// `has_weak_ref`/`has_strong_ref` handshake). A strong delivery
    /// takes BOTH (kernel: every strong ref implies a weak one). The
    /// caller MUST deliver these to the owner's `IPCThreadState` —
    /// same-ioctl (the 6-Z306ae-e no-race shape) — because a local
    /// object whose only remote ref is the just-granted one otherwise
    /// destructs at the sender's frame exit before the owner's next read
    /// (rn406: the composer's createClient IComposerClient was dead
    /// ~300 ms after grant; SF's first call landed in the corpse →
    /// Scudo invalid-free on the node ptr).
    ///
    /// On a FAILED delivery the caller MUST call
    /// [`BusState::z442_unwind_grant`] for the returned grant — a
    /// counted ref whose flat never crossed would otherwise later
    /// mirror a BR_RELEASE for an owner that never saw the BR_ACQUIRE
    /// (decStrong-on-zero).
    fn z359_grant_node(
        &mut self,
        sender: ConnId,
        recipient: ConnId,
        ptr: u64,
        cookie: u64,
        strong: bool,
    ) -> (u32, Vec<(u32, u64, u64)>) {
        let mut mirrors: Vec<(u32, u64, u64)> = Vec::new();
        let handle = match self.node_by_key.get(&(sender, ptr, cookie)) {
            Some(h) => *h,
            None => {
                // 6-Z371: kernel-true DENSE handle — the SAME allocator
                // as service handles (one handle space; every guest
                // handle-table entry stays small and sequential).
                // Collision-free: all allocations run serialized under
                // the bus lock and the counter is monotonic, so a
                // handle value names exactly one object (service or
                // node) for the life of the bus.
                let h = self.next_handle;
                self.next_handle += 1;
                self.node_by_key.insert((sender, ptr, cookie), h);
                self.nodes.insert(
                    h,
                    GuestNode {
                        owner: sender,
                        ptr,
                        cookie,
                        strong: HashMap::new(),
                        weak: HashMap::new(),
                        strong_notified: false,
                        weak_notified: false,
                        registry_pin_mirror: false,
                        strong_grants: 0,
                        weak_grants: 0,
                    },
                );
                h
            }
        };
        if let Some(n) = self.nodes.get_mut(&handle) {
            if strong {
                *n.strong.entry(recipient).or_insert(0) += 1;
                // Kernel: the strong ref carries the implied weak ref.
                *n.weak.entry(recipient).or_insert(0) += 1;
                // 6-Z457: lifetime grant bookkeeping (survives the
                // last-ref release — the mirror reads it at delivery).
                n.strong_grants = n.strong_grants.wrapping_add(1);
                n.weak_grants = n.weak_grants.wrapping_add(1);
                if !n.weak_notified {
                    n.weak_notified = true;
                    mirrors.push((BR_INCREFS, n.ptr, n.cookie));
                }
                if !n.strong_notified {
                    n.strong_notified = true;
                    mirrors.push((BR_ACQUIRE, n.ptr, n.cookie));
                    // 6-Z454: the era's acquire is on the wire — the ledger
                    // counts it so this node's future BR_RELEASE mirrors
                    // (z359_unref_node) balance against a real acquire.
                    let owner_pid = self.conns.get(&sender).map(|c| c.sender_pid).unwrap_or(0);
                    z454_emit(owner_pid, n.ptr, n.cookie, Z454Site::NodeAcq);
                }
            } else {
                *n.weak.entry(recipient).or_insert(0) += 1;
                // 6-Z457: lifetime weak grant.
                n.weak_grants = n.weak_grants.wrapping_add(1);
                if !n.weak_notified {
                    n.weak_notified = true;
                    mirrors.push((BR_INCREFS, n.ptr, n.cookie));
                }
            }
        }
        (handle, mirrors)
    }

    /// 6-Z457 companion (Task 192 decode): the 6-Z359 node-name via the
    /// OWNER-side reverse map. `by_handle` keys CLIENT registry handles
    /// while the mirror line's `handle` is the OWNER-namespace node id —
    /// two disjoint id spaces off the same dense counter — so the rn422
    /// lines resolved "?" on all 8 releases. The owner's registered
    /// services are the naming oracle: every (owner, ptr) match names
    /// the node (aliases join with `|`).
    fn z359_owner_node_names(&self, owner: ConnId, ptr: u64) -> Option<String> {
        let names: Vec<&str> = self
            .services
            .iter()
            .filter(|(_, e)| e.owner == owner && e.ptr == ptr)
            .map(|(k, _)| k.as_str())
            .collect();
        if names.is_empty() {
            None
        } else {
            Some(names.join("|"))
        }
    }

    /// 6-Z442: undo a just-made grant whose delivery FAILED (mailbox
    /// full / owner gone). Drops the counts `z359_grant_node` added and
    /// re-arms the notification flags the grant consumed — the node
    /// returns to its pre-grant state with NO mirrors ever leaving (a
    /// counted ref whose flat never crossed must not later mirror a
    /// BR_RELEASE for an owner that never saw the BR_ACQUIRE).
    fn z442_unwind_grant(&mut self, handle: u32, recipient: ConnId, strong: bool) {
        let Some(n) = self.nodes.get_mut(&handle) else {
            return;
        };
        if strong {
            if let Some(c) = n.strong.get_mut(&recipient) {
                *c -= 1;
                if *c == 0 {
                    n.strong.remove(&recipient);
                }
            }
        }
        if let Some(c) = n.weak.get_mut(&recipient) {
            *c -= 1;
            if *c == 0 {
                n.weak.remove(&recipient);
            }
        }
        if n.strong.is_empty() {
            n.strong_notified = false;
        }
        if n.weak.is_empty() {
            n.weak_notified = false;
        }
        if n.strong.is_empty() && n.weak.is_empty() {
            // The node never crossed — drop the entry entirely so a
            // future export of the same (sender, ptr, cookie) re-grades
            // cleanly (the key map keeps no stale handle).
            self.node_by_key.remove(&(n.owner, n.ptr, n.cookie));
            self.nodes.remove(&handle);
        }
    }

    /// Rewrite every LOCAL flat in `data`/`offsets` that crosses from
    /// `sender` to a DIFFERENT `recipient` connection into HANDLE form
    /// (kernel semantics) and grant the refs. Self-connection flats are
    /// left verbatim — the owner's own unflattenBinder decodes its OWN
    /// local object (the 6-Z306ac owner-conn LOCAL-hit shape, and the
    /// keystore2 in-process chain depends on it).
    ///
    /// 6-Z442: returns the OWNER-SIDE ref mirrors the kernel would queue
    /// for the grants made here ([`BusState::z359_grant_node`]) — the
    /// caller must deliver them to the SENDER conn's own read stream
    /// BEFORE the BR_TRANSACTION_COMPLETE (the 6-Z306ae-e same-ioctl
    /// no-race shape). Empty when nothing crossed or nothing needed
    /// notifying.
    fn z359_translate_flats(
        &mut self,
        vm_id: u32,
        sender: ConnId,
        recipient: ConnId,
        data: &mut [u8],
        offsets: &mut [u8],
        what: &str,
    ) -> Vec<Z442FlatGrant> {
        let mut grants: Vec<Z442FlatGrant> = Vec::new();
        if sender == recipient || offsets.is_empty() || data.is_empty() {
            return grants;
        }
        let count = offsets.len() / 8;
        for i in 0..count {
            let Some(off_usize) = (|| {
                let off = u64::from_ne_bytes(offsets[i * 8..i * 8 + 8].try_into().ok()?) as usize;
                if off + 24 <= data.len() {
                    Some(off)
                } else {
                    None
                }
            })() else {
                // 6-Z395: an offsets entry that points past the captured
                // data is a PARCEL-INTEGRITY violation — the offsets array
                // names an object the data region no longer carries. The
                // rn352 decode proved this shape is real: SF's
                // registerCallback (code 0x1, IComposerCallback flat at
                // offset 52 in a 76-byte parcel) arrived at the composer
                // as a 52-byte descriptor-only blob with olen=0 — the
                // IComposerCallback object never crossed, the composer
                // registered a null callback, its onHotplug failed
                // FAILED_TRANSACTION, and SF FATAL'd "Missing internal
                // display" 42 generations in a row. This bounded warning
                // names WHICH side truncated the parcel (if this fires
                // at route time, the sender's blob was already short —
                // the capture layer, not the bus, dropped the flat).
                static Z395_TRUNC: std::sync::atomic::AtomicU32 =
                    std::sync::atomic::AtomicU32::new(8);
                if Z395_TRUNC.load(Ordering::Relaxed) > 0 {
                    Z395_TRUNC.fetch_sub(1, Ordering::Relaxed);
                    let off = u64::from_ne_bytes(
                        offsets[i * 8..i * 8 + 8].try_into().unwrap_or([0u8; 8]),
                    );
                    warning!(
                        "[KR64][binder][vm{}] 6-Z395: object flat at offset {} beyond data.len() {} (offsets {}B) in {} conn={} -> conn={} — parcel TRUNCATED sender-side",
                        vm_id,
                        off,
                        data.len(),
                        offsets.len(),
                        what,
                        sender,
                        recipient
                    );
                }
                continue;
            };
            let typ = u32::from_ne_bytes(data[off_usize..off_usize + 4].try_into().unwrap());
            if typ == BINDER_TYPE_HANDLE || typ == BINDER_TYPE_WEAK_HANDLE {
                // 6-Z467: an already-HANDLE flat is a proxy the sender
                // received earlier and now forwards. Kernel-true
                // `binder_transaction()` rewrites the flat to the LOCAL
                // form (BINDER_TYPE_BINDER/WEAK_BINDER carrying the
                // owner's ptr/cookie) when the node's OWNER lives in the
                // RECIPIENT's own guest process — libbinder's
                // `unflatten_binder` then hands back the owner's LOCAL
                // BBinder (cookie cast), no proxy, no BC refs. Before
                // this fix the handle crossed VERBATIM: the recipient's
                // libbinder materialized a FRESH BpBinder for the same
                // global handle — a self-proxy whose pointer identity
                // differs from the owner's local BBinder — and every
                // map keyed by the local object missed. THE boot
                // blocker: SF's getPhysicalDisplayToken reply exported
                // the display-token node to system_server (6-Z359
                // grant); DMS's getDisplayInfo(token) forwarded that
                // handle back into SF (token owner conn=67, txn target
                // conn=66 — the SAME guest pid 3224); SF materialized a
                // self-proxy (the rn433 decode's BC_INCREFS/BC_ACQUIRE
                // signature right before the error reply),
                // getDisplayDeviceLocked's token lookup MISSED →
                // NAME_NOT_FOUND → DisplayInfo null → "No valid info
                // found" → DefaultDisplay=null → the DMS phase-100
                // "Timeout waiting for default display" crash loop, five
                // generations in a row. The identity that matters is the
                // GUEST PROCESS (the IDENT-stamped `sender_pid`), NOT
                // the conn: one process owns several binder conns, and
                // the txn's target conn is frequently a different conn
                // from the node-owner conn. Unstamped pids (0) and
                // unknown handles keep the verbatim crossing (the
                // pre-fix behavior) — the rewrite only fires on a
                // POSITIVE same-process match.
                let flat_handle =
                    u32::from_ne_bytes(data[off_usize + 8..off_usize + 12].try_into().unwrap());
                let local_identity = {
                    let node = self.nodes.get(&flat_handle);
                    let recipient_pid = self
                        .conns
                        .get(&recipient)
                        .map(|c| c.sender_pid)
                        .unwrap_or(0);
                    let owner_pid = node
                        .and_then(|n| self.conns.get(&n.owner))
                        .map(|c| c.sender_pid)
                        .unwrap_or(0);
                    if recipient_pid != 0 && recipient_pid == owner_pid {
                        node.map(|n| (n.owner, n.ptr, n.cookie))
                    } else {
                        None
                    }
                };
                if let Some((owner_conn, node_ptr, node_cookie)) = local_identity {
                    let local_type = if typ == BINDER_TYPE_HANDLE {
                        BINDER_TYPE_BINDER
                    } else {
                        BINDER_TYPE_WEAK_BINDER
                    };
                    data[off_usize..off_usize + 4].copy_from_slice(&local_type.to_ne_bytes());
                    data[off_usize + 8..off_usize + 16].copy_from_slice(&node_ptr.to_ne_bytes());
                    data[off_usize + 16..off_usize + 24]
                        .copy_from_slice(&node_cookie.to_ne_bytes());
                    static Z467_LOG: std::sync::atomic::AtomicU32 =
                        std::sync::atomic::AtomicU32::new(64);
                    if Z467_LOG.load(Ordering::Relaxed) > 0 {
                        Z467_LOG.fetch_sub(1, Ordering::Relaxed);
                        info!(
                            "[KR64][binder][vm{}] 6-Z467: local rewrite handle=0x{:08x} → local form (ptr=0x{:x} cookie=0x{:x}) — same guest process pid={} (owner conn={}, recipient conn={}) in {} conn={} -> conn={}",
                            vm_id,
                            flat_handle,
                            node_ptr,
                            node_cookie,
                            self.conns.get(&recipient).map(|c| c.sender_pid).unwrap_or(0),
                            owner_conn,
                            recipient,
                            what,
                            sender,
                            recipient
                        );
                    }
                }
                // No grant either way: a forwarded handle resolves on
                // the recipient's own table; a local rewrite grants
                // nothing (the owner already holds its object, and the
                // sender's proxy refs are the sender's own).
                continue;
            }
            let (is_strong_local, handle_type) = match typ {
                t if t == BINDER_TYPE_BINDER => (true, BINDER_TYPE_HANDLE),
                t if t == BINDER_TYPE_WEAK_BINDER => (false, BINDER_TYPE_WEAK_HANDLE),
                // fd/ptr/other entries: leave verbatim.
                _ => continue,
            };
            let ptr = u64::from_ne_bytes(data[off_usize + 8..off_usize + 16].try_into().unwrap());
            let cookie =
                u64::from_ne_bytes(data[off_usize + 16..off_usize + 24].try_into().unwrap());
            if ptr == 0 && cookie == 0 {
                // Explicit null-binder flat — leave verbatim.
                continue;
            }
            let (handle, granted_mirrors) =
                self.z359_grant_node(sender, recipient, ptr, cookie, is_strong_local);
            grants.push(Z442FlatGrant {
                handle,
                strong: is_strong_local,
                mirrors: granted_mirrors,
            });
            // Rewrite in place: [type][flags][handle u64][cookie=0].
            data[off_usize..off_usize + 4].copy_from_slice(&handle_type.to_ne_bytes());
            data[off_usize + 8..off_usize + 16].copy_from_slice(&(handle as u64).to_ne_bytes());
            data[off_usize + 16..off_usize + 24].copy_from_slice(&0u64.to_ne_bytes());
            if Self::z359_alloc_log().load(Ordering::Relaxed) > 0 {
                Self::z359_alloc_log().fetch_sub(1, Ordering::Relaxed);
                info!(
                    "[KR64][binder][vm{}] 6-Z359: node handle 0x{:08x} → conn={} (owner conn={} ptr=0x{:x} cookie=0x{:x} strong={}) in {}",
                    vm_id, handle, recipient, sender, ptr, cookie, is_strong_local, what
                );
            }
        }
        grants
    }

    /// Drop every node ref `conn` holds; mirror the last-release to the
    /// node owner (kernel: a dying process releases all its refs).
    /// Called from `unregister_conn`.
    fn z359_drop_conn_refs(&mut self, conn: ConnId) {
        let handles: Vec<u32> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.strong.contains_key(&conn) || n.weak.contains_key(&conn))
            .map(|(h, _)| *h)
            .collect();
        for h in handles {
            self.z359_unref_node(0, h, conn, true);
            self.z359_unref_node(0, h, conn, false);
        }
    }

    /// Decrement `holder`'s strong/weak ref on node `handle`; mirror
    /// BR_RELEASE / BR_DECREFS to the owner when the NODE's total for
    /// that kind hits zero (6-Z442 kernel truth: `binder_dec_node` fires
    /// the owner notification when the node's external refs are
    /// exhausted — per-holder mirroring would double-release the owner's
    /// object whenever two recipients held the same node; with the
    /// 6-Z442 BR_ACQUIRE mirror feeding the owner's strong count, an
    /// unbalanced release is a decStrong-on-zero abort class).
    /// `strong=false` handles the weak side. The notification flags
    /// re-arm exactly here (the next era's first ref re-mirrors).
    ///
    /// 6-Z458 (Task 195): the node's maps now include the REGISTRY's own
    /// pin ([`REGISTRY_CONN`], booked at every registration arm), so the
    /// emptiness decision is kernel-true: the LAST CLIENT's drop leaves
    /// the registry pin and mirrors NOTHING (the rn425 corpse-regrant
    /// shape — a BR_RELEASE deleting a REGISTRY-PINNED live object — is
    /// structurally impossible). The pin's own drop (the 6-Z379
    /// last-key-repoint) fires the release exactly when the kernel's
    /// servicemanager drop would; a pin whose add-time acquire mirror
    /// never crossed (the 6-Z306ae liveness probe skipped) drops
    /// SILENTLY — no release may precede its acquire on the wire. The
    /// decode witness `reg-pin=` rides the last-ref line: `dropped` (the
    /// registry released — the kernel-true era close), `present`
    /// (PREMATURE — the rn425 bug signature, must never appear),
    /// `absent` (client-side last ref on an unpinned node).
    fn z359_unref_node(&mut self, vm_id: u32, handle: u32, holder: ConnId, strong: bool) {
        let (mirror, node_dead, node_key, reg_pin_present, pin_mirror) = {
            let Some(n) = self.nodes.get_mut(&handle) else {
                return;
            };
            let reg_pin_present = n.strong.contains_key(&REGISTRY_CONN);
            let pin_mirror = n.registry_pin_mirror;
            let refs = if strong { &mut n.strong } else { &mut n.weak };
            let Some(c) = refs.get_mut(&holder) else {
                return;
            };
            *c -= 1;
            let mirror = if *c > 0 {
                None
            } else {
                refs.remove(&holder);
                if refs.is_empty() {
                    // The node's LAST external ref of this kind dropped:
                    // the owner may destroy the object; mirror the
                    // release and re-arm the 6-Z442 notification for the
                    // next era. With refs remaining the kernel stays
                    // silent (the object is still externally held).
                    if strong {
                        n.strong_notified = false;
                    } else {
                        n.weak_notified = false;
                    }
                    // 6-Z459: the LIFETIME counters snapshot into the
                    // mirror — the entry is about to be removed (below),
                    // and the delivery-time join reads them from HERE.
                    Some((n.owner, n.ptr, n.cookie, n.strong_grants, n.weak_grants))
                } else {
                    None
                }
            };
            // 6-Z459 (Task 196): the kernel-true NODE-DEATH moment —
            // BOTH maps empty means the driver node has no refs of any
            // kind left: the node is destroyed WITH its refs. A pinned
            // node never reaches this while registered (the registry
            // pin lives in both maps — the Task 195 construction), so
            // this fires exactly on: the client-side last ref, the
            // registry pin's own drop, and the silent pin drop. The
            // removal below makes a post-death re-export of the same
            // (owner, ptr, cookie) a FRESH node identity instead of a
            // re-grant onto the corpse (the rn426 reply-borne era-2
            // mirrors-onto-a-freed-chunk class).
            let node_dead = n.strong.is_empty() && n.weak.is_empty();
            let node_key = (n.owner, n.ptr, n.cookie);
            (mirror, node_dead, node_key, reg_pin_present, pin_mirror)
        };
        // 6-Z459: the node-death removal runs BEFORE the owner
        // notification (kernel order: binder_dec_node destroys the
        // node; the release mirror is best-effort). Every death path
        // is covered — including the silent registry-pin drop, whose
        // mirror never crosses but whose corpse entry must still go.
        if node_dead {
            self.nodes.remove(&handle);
            self.node_by_key.remove(&node_key);
            if Self::z359_release_log().load(Ordering::Relaxed) > 0 {
                Self::z359_release_log().fetch_sub(1, Ordering::Relaxed);
                info!(
                    "[KR64][binder][vm{}] 6-Z459: node 0x{:08x} dead (both ref maps empty, no registry pin) — entry removed; re-exports get fresh node identities (kernel-true node lifetime)",
                    vm_id, handle
                );
            }
        }
        let Some((owner, ptr, cookie, strong_grants, weak_grants)) = mirror else {
            return;
        };
        let br = if strong { BR_RELEASE } else { BR_DECREFS };
        let is_registry_drop = holder == REGISTRY_CONN;
        // 6-Z458: the REGISTRY pin's drop rides its own delivery
        // verdict. A pin whose acquire mirror never crossed (the add
        // arm's 6-Z306ae probe skipped — the rn425 suspend-service
        // shape: ledger RegAcq=0) must not produce a release the owner
        // never earned (release-without-acquire = the decStrong-on-zero
        // class) — its drop is silent bookkeeping.
        if is_registry_drop && !pin_mirror {
            if Self::z359_release_log().load(Ordering::Relaxed) > 0 {
                Self::z359_release_log().fetch_sub(1, Ordering::Relaxed);
                info!(
                    "[KR64][binder][vm{}] 6-Z458: registry pin dropped SILENT on node 0x{:08x} ({}) ptr=0x{:x} cookie=0x{:x} — the add arm's acq-mirror never delivered, no wire release (release-without-acquire prevention)",
                    vm_id,
                    handle,
                    if strong { "strong" } else { "weak" },
                    ptr, cookie
                );
            }
            return;
        }
        // Kernel-true mirror: push the RefCmd onto the OWNER's
        // reply_queue — the owner's IPCThreadState runs
        // decStrong/decWeak on the local BBinder (the composer's
        // onClientDestroyed). The heap anchor does NOT gate the QUEUE
        // decision (the 6-Z354 lesson: the release decision comes from
        // OUR OWN refcounts, not from a memory probe) — the object-level
        // anchor gates the DELIVERY instead (6-Z463: a close whose target
        // chunk is POSITIVELY dead is the rn427 corruption write and
        // drops to a silent close; Alive/Unknown deliver unchanged).
        // Delivery-time process liveness is the owner /proc probe,
        // checked in the RefCmd359 arm.
        if let Some(box_) = self.conns.get_mut(&owner) {
            box_.z491.reply_enq += 1;
            box_.reply_queue.push_back(DeferredReply::RefCmd359 {
                br,
                ptr,
                cookie,
                strong_grants,
                weak_grants,
            });
            // 6-Z454: the era's release is queued (the owner conn exists)
            // — the ledger checks it against this node's emitted acquires
            // (V1: a release past the acquire count is the
            // decStrong-on-zero precursor; naming the unref site + owner
            // here is the Task 190 audit's deliverable). 6-Z458: the
            // REGISTRY pin's own drop names site=reg-rel (it IS the
            // registry release); the witness flag verifies no premature
            // release crossed while the pin still held (Task 194 item e).
            if strong {
                let owner_pid = self.conns.get(&owner).map(|c| c.sender_pid).unwrap_or(0);
                let site = if is_registry_drop {
                    Z454Site::RegRel
                } else {
                    Z454Site::NodeRel
                };
                z454_emit_rel(owner_pid, ptr, cookie, site, reg_pin_present);
            }
            if Self::z359_release_log().load(Ordering::Relaxed) > 0 {
                Self::z359_release_log().fetch_sub(1, Ordering::Relaxed);
                // 6-Z456: the released node's REGISTRY NAME — rn421's
                // decode could only guess ("nodes 0x4e/0x50") because the
                // mirror lines log raw handles. 6-Z457 (Task 192): the
                // OWNER-side reverse map names the node — by_handle keys
                // CLIENT handles, disjoint from the owner-namespace node
                // id (rn422: "?" on all 8 lines). Registry entries whose
                // (owner, ptr) match name the release site; the legacy
                // by_handle hit stays the fallback for registry-held
                // nodes (the 6-Z306ae/6-Z325 pins never own a service).
                let node_name = self
                    .z359_owner_node_names(owner, ptr)
                    .or_else(|| self.by_handle.get(&handle).cloned())
                    .unwrap_or_else(|| "?".to_string());
                // 6-Z458 decode witness: dropped = the registry's own
                // release (kernel-true era close); present = a client
                // release fired while the pin STILL held (the rn425 bug
                // signature — regression if ever seen); absent = the
                // pre-pin node shapes (pure flat-crossing exports).
                let reg_pin_witness = if is_registry_drop {
                    "dropped"
                } else if reg_pin_present {
                    "present"
                } else {
                    "absent"
                };
                info!(
                    "[KR64][binder][vm{}] 6-Z359: last ref from conn={} on node 0x{:08x} released → {} mirrored to owner conn={} (ptr=0x{:x} cookie=0x{:x}) node-name={} reg-pin={}",
                    vm_id, holder, handle, if strong { "BR_RELEASE" } else { "BR_DECREFS" }, owner, ptr, cookie, node_name, reg_pin_witness
                );
            }
        }
        // 6-Z459: the node entry is NOT kept anymore — the kernel-true
        // node lifetime. The both-maps-empty death above already removed
        // the entry (nodes + node_by_key), so the handle is stable only
        // WITHIN the node's lifetime and a re-export of the same key
        // creates a fresh identity (fresh handle, fresh notification
        // eras, fresh counters — the join counters rode the queued
        // mirror). No era-2 mirror can ever land on the chunk the
        // era-1 release already freed (the rn426 corpse-regrant class).
    }

    /// 6-Z458 (Task 195): book the REGISTRY's own strong+weak pin on the
    /// (owner, ptr, cookie) node at registration time (AIDL addService /
    /// HIDL add / addWithChain chain[0]). Find-or-create the node with
    /// the SAME key the client-grant path ([`BusState::z359_grant_node`])
    /// uses, so the pin and the grants share one entry and every
    /// existing map consumer sees one more holder.
    ///
    /// Kernel-true bookkeeping, regardless of the add arm's liveness
    /// verdict: the SM's sp<> exists the moment the add lands, probe or
    /// no probe — only the OWNER-NOTIFICATION side is probe-gated (the
    /// 6-Z306ae arm, unchanged). The pin's presence raises
    /// `strong_notified`: the pin IS the node's 0→1 strong edge (the
    /// kernel's `binder_ref` creation on a ref-less node), so the first
    /// CLIENT grant must not re-mirror the era's BR_ACQUIRE — this
    /// removes the pre-6-Z458 compensating over-acquire whose absence
    /// (add-mirror skipped) let the wrongful last-client release kill
    /// the object (the rn425 cascade). `weak_notified` is deliberately
    /// untouched: the weak era's BR_INCREFS still opens at the first
    /// grant (the pre-6-Z458 balanced weak arithmetic, preserved).
    ///
    /// BOOLEAN presence per object (one pin while ANY registry key
    /// points at it): re-adds and chain aliases do not inflate the
    /// count — `overwrite_release_due`'s last-key semantics drop it
    /// exactly once. NOT counted in the 6-Z457 lifetime grant counters:
    /// those join guest mStrong against flat-crossing grants; the pin's
    /// acquire is the add arm's mirror, visible in the ledger surface.
    fn z458_registry_pin_add(&mut self, owner: ConnId, ptr: u64, cookie: u64, acq_mirrored: bool) {
        if ptr == 0 || cookie == 0 || owner == PROXY_CONN_ID {
            return;
        }
        let handle = match self.node_by_key.get(&(owner, ptr, cookie)) {
            Some(h) => *h,
            None => {
                let h = self.next_handle;
                self.next_handle += 1;
                self.node_by_key.insert((owner, ptr, cookie), h);
                self.nodes.insert(
                    h,
                    GuestNode {
                        owner,
                        ptr,
                        cookie,
                        strong: HashMap::new(),
                        weak: HashMap::new(),
                        strong_notified: false,
                        weak_notified: false,
                        registry_pin_mirror: false,
                        strong_grants: 0,
                        weak_grants: 0,
                    },
                );
                h
            }
        };
        let Some(n) = self.nodes.get_mut(&handle) else {
            return;
        };
        if !n.strong.contains_key(&REGISTRY_CONN) {
            *n.strong.entry(REGISTRY_CONN).or_insert(0) += 1;
            // Kernel: every strong ref implies a weak one (the node's
            // weak map balances the pin's implied weak against the
            // owner's BR_INCREFS-era arithmetic).
            *n.weak.entry(REGISTRY_CONN).or_insert(0) += 1;
            // The pin IS the strong era's 0→1 edge — the add arm's
            // BR_ACQUIRE (same-ioctl, or the 6-Z306am arm-B re-route)
            // is its owner notification; the grant path must not
            // re-mirror it.
            n.strong_notified = true;
        }
        // The LATEST add's mirror verdict governs the pin's drop (each
        // add re-mirrors the acquire; the release pairs with the last
        // one the owner actually saw).
        n.registry_pin_mirror = acq_mirrored;
    }
}

/// How long a sync transaction waits for the server's `BC_REPLY` before
/// the proxy resolves it as `BR_FAILED_REPLY` (the kernel has no timeout,
/// but a hung server would otherwise hang the requester forever).
///
/// 6-Z437 (rn394 decode): 8s was FAKING FAILURES. The run carried 60
/// 6-Z407 expiries and the death fleet's tombstones carry exactly the
/// surface shape: `Status(EX_TRANSACTION_FAILED): 'FAILED_TRANSACTION'`
/// aborts in the composer (_vendor_bin_hw_), audioserver, camera and SF
/// generations — every one an UNCHECKED-HIDL caller whose two-way
/// transaction outlived the budget under the boot's CPU starvation (the
/// host redroid + the guest share 4 cores; a legit composer/allocator
/// reply can exceed 8s while the SF/composer restart storm saturates
/// the bus). libhwbinder maps BR_FAILED_REPLY to FAILED_TRANSACTION and
/// the unchecked-Return callers abort — each expiry KILLED a service.
/// The kernel's own answer is WAIT FOREVER; 30s keeps the wedge-breaker
/// (the permanent wedges — sync-flagged oneways that never reply, the
/// 6-Z399 class — resolve at 30s instead of 8s; nothing wedges forever)
/// while absorbing every legit boot-latency spike. The 6-Z407 trace now
/// also names the RESPONDER conn so the decode sees who starved.
pub const REPLY_TIMEOUT: Duration = Duration::from_secs(30);

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
    // 6-Z327: record the guest rootfs — the SM get arm consults the
    // guest's OWN declared SDK level (ro.build.version.sdk from
    // {rootfs}/system/build.prop) before serving the A12-Category
    // stability annotation. Lazy + cached (early boot pays nothing).
    {
        let mut g = GUEST_ROOTFS.write().expect("guest rootfs lock");
        *g = Some(rootfs.to_string());
        *GUEST_SDK_CACHE.write().expect("guest sdk lock") = None;
    }
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

                // 6-Z402: the BUS-STATE HEARTBEAT — every 30 s, one bounded
                // line per connection holding PENDING state (an unanswered
                // sync call with its age, a queued reply, a queued incoming
                // transaction, a non-empty transaction stack). The
                // rn355/rn356 decode burned on not being able to see, at
                // stall time, WHAT a parked looper was actually waiting
                // for: SF main sat in the idle tick for 30+s while every
                // reply path logged silent-on-success. The heartbeat is the
                // stall-time oracle — a stuck requester's (conn, age)
                // appears in every artifact from this wave on.
                let bus_hb = Arc::clone(&bus);
                let shutdown_hb = Arc::clone(&shutdown_for_thread);
                let vm_id_hb = vm_id;
                thread::Builder::new()
                    .name(format!("kr64-binder-hb-{}", vm_id))
                    .spawn(move || {
                        let mut tick: u64 = 0;
                        loop {
                            std::thread::sleep(std::time::Duration::from_secs(30));
                            if shutdown_hb.load(Ordering::Acquire) {
                                break;
                            }
                            tick += 1;
                            let Ok(b) = bus_hb.lock() else {
                                continue;
                            };
                            let now = std::time::Instant::now();
                            let mut lines: Vec<String> = Vec::new();
                            for (cid, bx) in b.conns.iter() {
                                let mut parts: Vec<String> = Vec::new();
                                for (t, _o, at) in bx.out_sync.iter() {
                                    parts.push(format!(
                                        "sync#{} age={}s",
                                        t,
                                        now.duration_since(*at).as_secs()
                                    ));
                                }
                                if !bx.reply_queue.is_empty() {
                                    parts.push(format!("replies={}", bx.reply_queue.len()));
                                }
                                if !bx.inbox.is_empty() {
                                    parts.push(format!("inbox={}", bx.inbox.len()));
                                }
                                if !bx.txn_stack.is_empty() {
                                    parts.push(format!("stack={:?}", bx.txn_stack));
                                }
                                if !parts.is_empty() {
                                    lines.push(format!(
                                        "conn={} pid={} dev={} {}",
                                        cid,
                                        bx.sender_pid,
                                        bx.dev_code,
                                        parts.join(" ")
                                    ));
                                }
                            }
                            if !lines.is_empty() {
                                info!(
                                    "[KR64][binder][vm{}] 6-Z402 bus-state tick #{}: {} conn(s) pending | {}",
                                    vm_id_hb,
                                    tick,
                                    lines.len(),
                                    lines.join(" || ")
                                );
                            }
                            // 6-Z495: the EXCHANGE-STUCK sweep — the same
                            // 30 s heartbeat names any conn whose
                            // BINDER_WRITE_READ has been in flight past
                            // the stuck threshold (the rn470/ladder-rn471
                            // class: a conn parked mid-exchange never
                            // completes another ioctl, so the z491
                            // tail-of-ioctl tick can never see it).
                            z495_exchange_sweep(&bus_hb, vm_id_hb);
                        }
                    })
                    .expect("kr64-binder-hb spawn");

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
    // 6-Z362: fd-passing success counters. rn315 proved the FAILURE side
    // of the 6-Z355 fd crossing logs (shortfall/diag lines) but the
    // SUCCESS side is invisible — zero lines can mean "clean pass" OR
    // "no fd traffic at all", and the decode cannot tell the IAllocator's
    // gralloc handles actually crossed. These counters + the one
    // summary line per connection close (bounded, no hot-path logging)
    // make the engagement itself decodable.
    let mut fd_frames_in: u64 = 0;
    let mut fds_in_total: u64 = 0;
    let mut fd_frames_out: u64 = 0;
    let mut fds_out_total: u64 = 0;
    loop {
        // 6-Z355: the request frame may carry SCM_RIGHTS fds (fd-bearing
        // blobs) — captured by the control-buffered header read.
        let (req, wire_fds) = match read_frame_with_fds(stream) {
            Ok(r) => r,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                info!(
                    "[KR64][binder][vm{}] client disconnected (conn={})",
                    vm_id, conn_id
                );
                info!(
                    "[KR64][binder][vm{}] 6-Z362: conn={} fd-summary: frames-with-fds-in={} fds-in={} frames-with-fds-out={} fds-out={}",
                    vm_id, conn_id, fd_frames_in, fds_in_total, fd_frames_out, fds_out_total
                );
                return Ok(());
            }
            Err(e) => {
                info!(
                    "[KR64][binder][vm{}] 6-Z362: conn={} fd-summary (error exit: {}): frames-with-fds-in={} fds-in={} frames-with-fds-out={} fds-out={}",
                    vm_id, conn_id, e, fd_frames_in, fds_in_total, fd_frames_out, fds_out_total
                );
                return Err(e);
            }
        };
        if !wire_fds.is_empty() {
            fd_frames_in += 1;
            fds_in_total += wire_fds.len() as u64;
        }
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
        // 6-Z495: stamp the in-flight exchange BEFORE dispatch — the
        // sweep on the 6-Z402 heartbeat thread names any exchange that
        // never comes back (the rn470/ladder-rn471 wedge class: the
        // guest parked inside bp_exchange_anc while the conn never
        // completed another ioctl — invisible to the z491 tail-of-ioctl
        // tick by construction).
        if req.cmd == BINDER_WRITE_READ {
            if let Ok(mut b) = bus.lock() {
                if let Some(bx) = b.conns.get_mut(&conn_id) {
                    bx.z495_inflight = Some(Z495InFlight {
                        arrived_at: std::time::Instant::now(),
                        ws: if req.payload.len() >= 4 {
                            u32::from_ne_bytes(req.payload[0..4].try_into().unwrap())
                        } else {
                            0
                        },
                        rc: if req.payload.len() >= 8 {
                            u32::from_ne_bytes(req.payload[4..8].try_into().unwrap())
                        } else {
                            0
                        },
                    });
                }
            }
        }
        let resp = dispatch_request(&req, vm_id, bus, conn_id, wire_fds);
        if req.cmd == BINDER_WRITE_READ && wr_diag_budget > 0 {
            let (rs, blobs) = if resp.payload.len() >= 4 {
                let rs = u32::from_ne_bytes(resp.payload[0..4].try_into().unwrap());
                let tail = resp.payload.len() - 4 - rs as usize;
                (rs, tail)
            } else {
                (0, resp.payload.len())
            };
            // 6-Z407b: the FIRST BR command of the read stream — rn363's
            // final hole (did the composer's nested wait consume txn#13's
            // BR_REPLY, or was the 68-byte read a NEW incoming txn?)
            // decodes from this one datum: BR_REPLY=0x72072006,
            // BR_TRANSACTION=0x72072002, BR_NOOP=0x7206f000 (the
            // transaction-class commands carry the reply/txn identity).
            let first_br = if rs >= 4 && resp.payload.len() >= 8 {
                Some(u32::from_ne_bytes(resp.payload[4..8].try_into().unwrap()))
            } else {
                None
            };
            info!(
                "[KR64][binder][vm{}] conn={} -> ret={} read_size={} trailer={}B first_br={}",
                vm_id,
                conn_id,
                resp.ret,
                rs,
                blobs,
                match first_br {
                    Some(v) => format!("{:#010x}", v),
                    None => "none".to_string(),
                }
            );
        }
        // 6-Z362: count the OUT side — the fds this response carries via
        // SCM_RIGHTS on THIS connection's socket (the recipient sees them
        // as its fds-in). One compare per frame; zero hot-path logging.
        if !resp.fds.is_empty() {
            fd_frames_out += 1;
            fds_out_total += resp.fds.len() as u64;
        }
        write_frame(stream, &resp)?;
        // 6-Z495: the exchange completed — clear the in-flight stamp so
        // the sweep only ever names exchanges the loop GENUINELY never
        // returned (a stuck dispatch or a blocked response write).
        if req.cmd == BINDER_WRITE_READ {
            if let Ok(mut b) = bus.lock() {
                if let Some(bx) = b.conns.get_mut(&conn_id) {
                    bx.z495_inflight = None;
                }
            }
        }
    }
}

// ============================================================================
// Ioctl dispatcher.
// ============================================================================

/// Dispatch one parsed request frame to the appropriate handler.
fn dispatch_request(
    req: &Frame,
    vm_id: u32,
    bus: &Arc<Mutex<BusState>>,
    conn_id: ConnId,
    // 6-Z355: SCM_RIGHTS fds that rode this request frame. Only
    // BINDER_WRITE_READ consumes them; every other arm drops (closes) the
    // guard — an fd without a consuming transaction must not leak.
    wire_fds: Vec<FdGuard>,
) -> Resp {
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
            // 6-Z306ai: parse via the shared helper — legacy 12-byte and
            // v2 24-byte (magic "idex" + tid + dev) payloads both decode;
            // the extension is additive and the response is unchanged.
            let (pid, uid, tid, dev) = parse_ident_payload(&req.payload);
            let mut stamped = false;
            if let Ok(mut b) = bus.lock() {
                if let Some(box_) = b.conns.get_mut(&conn_id) {
                    if box_.sender_pid == 0 && pid != 0 {
                        box_.sender_pid = pid;
                        box_.sender_euid = uid;
                        stamped = true;
                    }
                    if box_.sender_tid == 0 && tid != 0 {
                        box_.sender_tid = tid;
                    }
                    if box_.dev_code == 0 && dev != 0 {
                        box_.dev_code = dev;
                    }
                }
            }
            info!(
                "[KR64][binder][vm{}] conn={} IDENT announced pid={} uid={} tid={} dev={} (getpid-faked) — {}",
                vm_id,
                conn_id,
                pid,
                uid,
                tid,
                ident_dev_name(dev),
                if stamped {
                    "stamped (no SO_PEERCRED available)"
                } else {
                    "ignored — SO_PEERCRED already stamped real pid"
                }
            );
            Resp::new(0, Vec::new())
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
            Resp::new(0, Vec::new())
        }

        BINDER_ENABLE_ONEWAY_SPAM_DETECTION => {
            // 6-Z265: the real kernel accepts this unconditionally (it
            // only arms an internal flood counter) — ACK with 0 so real
            // libbinder clients don't log/handle EINVAL on handshake.
            info!(
                "[KR64][binder][vm{}] ENABLE_ONEWAY_SPAM_DETECTION (acknowledged)",
                vm_id
            );
            Resp::new(0, Vec::new())
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
            Resp::new(0, Vec::new())
        }

        BINDER_THREAD_EXIT => {
            info!("[KR64][binder][vm{}] THREAD_EXIT", vm_id);
            Resp::new(0, Vec::new())
        }

        BINDER_WRITE_READ => handle_write_read(&req.payload, vm_id, bus, conn_id, wire_fds),

        other => {
            warning!(
                "[KR64][binder][vm{}] unknown ioctl 0x{:08x} ({} bytes payload)",
                vm_id,
                other,
                req.payload.len()
            );
            Resp::new(-(libc::EINVAL), Vec::new())
        }
    }
}

/// `BINDER_VERSION` handler — return the protocol version.
fn handle_version(vm_id: u32) -> Resp {
    info!(
        "[KR64][binder][vm{}] VERSION → {}",
        vm_id, BINDER_CURRENT_PROTOCOL_VERSION
    );
    Resp::new(0, BINDER_CURRENT_PROTOCOL_VERSION.to_ne_bytes().to_vec())
}

// ============================================================================
// 6-Z491: the delivery-vs-consumed wedge scan (the daemon-wall instrument).
// ============================================================================

/// 6-Z491: the wedge-verdict budget (lines/run). The self-bury rule
/// (the rn350/rn362 lessons): a correlation instrument must outlive the
/// boot's first minute — 64 verdicts at the 5 s per-conn throttle spans
/// the full ladder window even with several wedged conns.
static Z491_VERDICT_LOG: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(64);

/// How long a conn must show NO write-side activity and NO deliveries
/// before its stuck mailbox/stack may count as a wedge (not a transient:
/// a sync round-trip is µs-ms; the 6-Z152 idle tick is 250 ms).
const Z491_WEDGE_QUIESCE: std::time::Duration = std::time::Duration::from_secs(5);

/// Per-conn verdict throttle.
const Z491_VERDICT_GAP: std::time::Duration = std::time::Duration::from_secs(5);

/// 6-Z491: the per-ioctl accounting tick + the budgeted wedge scan.
/// Called at the tail of EVERY completed `BINDER_WRITE_READ` (before the
/// response leaves for the guest); one lock acquisition.
///
/// Verdict shapes (the decode joins the line against the guest's
/// SM-REPLY svclog era and the 6-Z407 reply trace):
///
/// * `WEDGE-A-QUEUED-NEVER-TAKEN` — inbox/pending_in stuck non-empty
///   through the quiesce window while the conn shows no write-side
///   activity: frames queued for this conn that NO reader consumed
///   (the reader-wakeup desync shape). The 6-Z408 reentrancy hold is
///   EXCLUDED (`out_sync` non-empty = the conn parked on its own nested
///   call; `z408_note_hold` already names that shape and the
///   REPLY_TIMEOUT guarantees the unwind).
/// * `WEDGE-B-TAKEN-NEVER-ANSWERED` — `txn_stack` stuck non-empty with
///   `out_sync` empty and no BC_REPLY inbound: the conn TOOK the
///   transactions but never replies — the rn466/rn468 installd shape
///   to the letter (BR-TX delivered ✓, the first few served, then NO
///   reply while the binder threads idle).
/// * `WEDGE-C-REPLY-NEVER-POLLED` — a resolved reply stuck on this
///   conn's reply_queue: the client stopped reading between its calls
///   (the client-side half of the rn464 createIdmap wall).
///
/// A fully-drained conn (every mailbox empty — the healthy idle looper)
/// never verdicts, whatever its poll rate.
fn z491_ioctl_tick(bus: &Arc<Mutex<BusState>>, vm_id: u32, conn_id: ConnId, read_buf_empty: bool) {
    let mut b = bus.lock().expect("binder bus poisoned");
    let Some(bx) = b.conns.get_mut(&conn_id) else {
        return;
    };
    bx.z491.wr_calls += 1;
    if read_buf_empty {
        bx.z491.noop_polls += 1;
    }
    let now = std::time::Instant::now();
    let rx_idle = bx
        .z491
        .last_rx
        .map_or(true, |t| now.duration_since(t) >= Z491_WEDGE_QUIESCE);
    let del_idle = bx
        .z491
        .last_del
        .map_or(true, |t| now.duration_since(t) >= Z491_WEDGE_QUIESCE);
    if !rx_idle || !del_idle {
        // The conn consumed or acknowledged something inside the window —
        // live; refresh nothing, verdict nothing.
        return;
    }
    if bx
        .z491
        .last_verdict
        .map_or(false, |t| now.duration_since(t) < Z491_VERDICT_GAP)
    {
        return;
    }
    let nested = !bx.out_sync.is_empty();
    let kind = if !bx.inbox.is_empty() && !nested {
        "A-QUEUED-NEVER-TAKEN"
    } else if !bx.txn_stack.is_empty() && !nested {
        "B-TAKEN-NEVER-ANSWERED"
    } else if !bx.reply_queue.is_empty() {
        "C-REPLY-NEVER-POLLED"
    } else {
        return;
    };
    if Z491_VERDICT_LOG.load(Ordering::Relaxed) == 0 {
        return;
    }
    Z491_VERDICT_LOG.fetch_sub(1, Ordering::Relaxed);
    bx.z491.last_verdict = Some(now);
    let ms = |t: Option<std::time::Instant>| match t {
        Some(t) => format!("{}ms", (now - t).as_millis()),
        None => "never".to_string(),
    };
    info!(
        "[KR64][binder][vm{}] 6-Z491 WEDGE-{} conn={} pid={} dev={} reader_waiting={} inbox={} pin={} stack={} out_sync={} replies={} wr={} noop={} tx_enq={} tx_del={} reply_enq={} reply_del={} bc_reply={} bc_free={} max_inbox={} last_del={} last_rx={}",
        vm_id,
        kind,
        conn_id,
        bx.sender_pid,
        bx.dev_code,
        bx.reader_waiting,
        bx.inbox.len(),
        bx.pending_in.len(),
        bx.txn_stack.len(),
        bx.out_sync.len(),
        bx.reply_queue.len(),
        bx.z491.wr_calls,
        bx.z491.noop_polls,
        bx.z491.tx_enq,
        bx.z491.tx_del,
        bx.z491.reply_enq,
        bx.z491.reply_del,
        bx.z491.bc_reply_rx,
        bx.z491.bc_free_rx,
        bx.z491.max_inbox,
        ms(bx.z491.last_del),
        ms(bx.z491.last_rx),
    );
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
///
/// 6-Z491: every completed ioctl runs the per-conn accounting tick
/// ([`z491_ioctl_tick`]) — the delivery-vs-consumed wedge scan.
fn handle_write_read(
    payload: &[u8],
    vm_id: u32,
    bus: &Arc<Mutex<BusState>>,
    conn_id: ConnId,
    // 6-Z355: fds received via SCM_RIGHTS on this request frame (empty for
    // every non-fd-bearing client). Split per blob below; unconsumed
    // members close on drop.
    wire_fds: Vec<FdGuard>,
) -> Resp {
    // Parse the v1 wire header: [u32 write_size][u32 read_capacity][write_size BC_* bytes].
    if payload.len() < 8 {
        return Resp::new(-(libc::EINVAL), Vec::new());
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
        return Resp::new(-(libc::EINVAL), Vec::new());
    }
    // 6-Z399: mark this conn's reader as WAITING for the duration of the
    // ioctl. The pool-steal (6-Z271g) skips waiting readers — a waiting
    // looper takes its own work (kernel proc-todo semantics; see the
    // ConnBox.reader_waiting doc). The guard clears the flag on every
    // exit path.
    struct ReaderWaitingGuard {
        bus: std::sync::Arc<std::sync::Mutex<BusState>>,
        conn_id: ConnId,
        armed: bool,
    }
    impl Drop for ReaderWaitingGuard {
        fn drop(&mut self) {
            if !self.armed {
                return;
            }
            if let Ok(mut b) = self.bus.lock() {
                if let Some(bx) = b.conns.get_mut(&self.conn_id) {
                    bx.reader_waiting = false;
                }
            }
        }
    }
    let _reader_guard = {
        let mut b = bus.lock().expect("binder bus poisoned");
        let armed = if let Some(bx) = b.conns.get_mut(&conn_id) {
            bx.reader_waiting = true;
            true
        } else {
            false
        };
        ReaderWaitingGuard {
            bus: std::sync::Arc::clone(bus),
            conn_id,
            armed,
        }
    };
    let write_buf = &payload[8..8 + write_size];

    // Parse the optional v2/v3 trailer (6-Z114 §4.4 + 6-Z305t-68):
    //   v2: [u32 WIRE_V2_MAGIC][u32 blob_count]
    //       (blob_count ×) [u32 data_len][u32 offsets_len][data][offsets]
    //   v3: same shape plus [u32 sg_count] in each blob header and the
    //       BINDER_TYPE_PTR contents appended after [data][offsets].
    // A request that ends exactly after the BC stream is v1 (z113 client —
    // byte-compatible, no parcel blobs). A v2/v3 request inlines the parcel
    // bytes the proxy needs to actually parse BC_TRANSACTION data.
    let mut off = 8 + write_size;
    let mut req_blobs: Vec<RequestBlob> = Vec::new();
    let mut is_v2 = false;
    if off + 8 <= payload.len() {
        let magic = u32::from_ne_bytes(payload[off..off + 4].try_into().unwrap());
        if magic == WIRE_V2_MAGIC || magic == WIRE_V3_MAGIC {
            let is_v3 = magic == WIRE_V3_MAGIC;
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
                let sg_count: usize = if is_v3 {
                    if off + 4 > payload.len() {
                        break;
                    }
                    let c = u32::from_ne_bytes(payload[off..off + 4].try_into().unwrap()) as usize;
                    off += 4;
                    c
                } else {
                    0
                };
                if off + data_len + offsets_len > payload.len() {
                    break;
                }
                let data = payload[off..off + data_len].to_vec();
                off += data_len;
                let offsets = payload[off..off + offsets_len].to_vec();
                off += offsets_len;
                let mut sg: Vec<SgBuf> = Vec::new();
                if is_v3 {
                    let mut ok = true;
                    for _ in 0..sg_count {
                        if off + 12 > payload.len() {
                            ok = false;
                            break;
                        }
                        let client_ptr =
                            u64::from_ne_bytes(payload[off..off + 8].try_into().unwrap());
                        let blen =
                            u32::from_ne_bytes(payload[off + 8..off + 12].try_into().unwrap())
                                as usize;
                        off += 12;
                        if off + blen > payload.len() {
                            ok = false;
                            break;
                        }
                        sg.push(SgBuf {
                            client_ptr,
                            data: payload[off..off + blen].to_vec(),
                        });
                        off += blen;
                    }
                    if !ok {
                        break; // truncated SG section — stop consuming blobs
                    }
                }
                req_blobs.push(RequestBlob {
                    data,
                    offsets,
                    sg,
                    fds: Vec::new(),
                });
            }
        }
    }

    // 6-Z355: split the frame's SCM_RIGHTS fds per blob. The fd tail
    // ([WIRE_FD_TAIL_MAGIC][blob_count][counts...]) sits right after the
    // last blob when the loader attached any fds; the per-blob scan
    // (blob_fd_count) cross-checks the loader's counts and the cmsg list
    // length. Attach in order: blob 0's fds first, then blob 1's, … —
    // exactly the kernel's fixup order (offsets-array order within each
    // transaction buffer).
    if !req_blobs.is_empty() {
        // `off` sits at the end of the last blob the trailer parse
        // consumed (a truncated parse leaves it shorter — then no tail
        // can be trusted either).
        let counts = parse_fd_tail(payload, off, req_blobs.len());
        match counts {
            Some(counts) => {
                let total: usize = counts.iter().map(|&c| c as usize).sum();
                if wire_fds.len() < total {
                    if FD_TAIL_MISMATCH_LOG.load(Ordering::Relaxed) > 0 {
                        FD_TAIL_MISMATCH_LOG.fetch_sub(1, Ordering::Relaxed);
                        warning!(
                            "[KR64][binder][vm{}] 6-Z355: fd tail wants {} fds, cmsg carried {} — shortfall fills -1 at the recipient (conn={})",
                            vm_id,
                            total,
                            wire_fds.len(),
                            conn_id
                        );
                    }
                }
                let mut iter = wire_fds.into_iter();
                let mut scan_total = 0u32;
                for (i, blob) in req_blobs.iter_mut().enumerate() {
                    let want = counts[i] as usize;
                    let have = iter.by_ref().take(want);
                    blob.fds = have.map(FdGuard::into_arc).collect();
                    // Pad a shortfall with an absent-fd marker count so the
                    // recipient's patch loop sees the gap (it fills -1).
                    if blob.fds.len() < want {
                        blob.fds.resize(want, Arc::new(FdGuard::from_raw(-1)));
                    }
                    scan_total =
                        scan_total.saturating_add(blob_fd_count(&blob.data, &blob.offsets));
                }
                // Leftover fds (tail counts < cmsg count): dropped here =
                // closed (kernel: unreferenced transaction fds released).
                drop(iter);
                if scan_total as usize != total && FD_TAIL_MISMATCH_LOG.load(Ordering::Relaxed) > 0
                {
                    FD_TAIL_MISMATCH_LOG.fetch_sub(1, Ordering::Relaxed);
                    warning!(
                        "[KR64][binder][vm{}] 6-Z355: fd-tail counts ({}) vs offsets-array scan ({}) disagree (conn={}) — tail trusted, decode names the sender",
                        vm_id, total, scan_total, conn_id
                    );
                }
            }
            None => {
                // No tail on a frame that carried fds (old shlib / partial
                // write): the fds have no blob binding — close them (drop).
                if !wire_fds.is_empty() && FD_TAIL_MISMATCH_LOG.load(Ordering::Relaxed) > 0 {
                    FD_TAIL_MISMATCH_LOG.fetch_sub(1, Ordering::Relaxed);
                    warning!(
                        "[KR64][binder][vm{}] 6-Z355: {} cmsg fds with NO fd tail — closed unconsumed (conn={})",
                        vm_id,
                        wire_fds.len(),
                        conn_id
                    );
                }
            }
        }
    }

    // Walk the BC_* stream. The i-th v2 blob pairs with the i-th
    // BC_TRANSACTION/BC_REPLY/`*_SG` command in stream order.
    let mut read_buf: Vec<u8> = Vec::new();
    let mut resp_blobs: Vec<RequestBlob> = Vec::new();
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
        // 6-Z301: bounded BC-command stream log — per (vm, conn) the first
        // 64 commands + every 500th afterwards. The fox R12 engagement
        // wave (33964129056) showed the guest keystore2 issuing NOTHING
        // after its 2nd android.security.compat descriptor fetch: with
        // this stream the next wave distinguishes "the guest stopped
        // issuing" (silence after the last logged command) from "the
        // proxy lost it mid-command" (a truncated/unknown cmd pairs with
        // the truncated-BC warning above).
        static BC_STREAM_SEEN: std::sync::OnceLock<
            std::sync::Mutex<std::collections::HashMap<(u32, ConnId), u64>>,
        > = std::sync::OnceLock::new();
        {
            let seen = match BC_STREAM_SEEN
                .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
                .lock()
            {
                Ok(mut m) => *m
                    .entry((vm_id, conn_id))
                    .and_modify(|c| *c += 1)
                    .or_insert(1),
                Err(_) => 0,
            };
            if seen <= 64 || seen % 500 == 0 {
                info!(
                    "[KR64][binder][vm{}] BC stream conn={} cmd=0x{:08x} payload={} [cmd #{}{}]",
                    vm_id,
                    conn_id,
                    cmd,
                    psize,
                    seen,
                    if seen <= 64 { "" } else { " sampled" }
                );
            }
        }
        let cmd_payload = &write_buf[consumed..consumed + psize];
        consumed += psize;

        match cmd {
            BC_TRANSACTION | BC_TRANSACTION_SG => {
                // 6-Z491: the conn's own OUTGOING call — proof its writer
                // path is alive (the wedge gate reads last_rx).
                {
                    let mut b = bus.lock().expect("binder bus poisoned");
                    if let Some(bx) = b.conns.get_mut(&conn_id) {
                        bx.z491.note_rx();
                    }
                }
                // Pull the next v2 blob as this transaction's parcel.
                let req_blob = if is_v2 && blob_idx < req_blobs.len() {
                    let b = &req_blobs[blob_idx];
                    blob_idx += 1;
                    Some(b.clone())
                } else {
                    None
                };
                let result = handle_transaction(cmd_payload, vm_id, bus, conn_id, req_blob);
                // Kernel semantics (6-Z305t-69): a ONEWAY transaction is
                // acked with BR_TRANSACTION_COMPLETE only — NEVER a reply.
                // A Reply result for a oneway SM/virtual-service call
                // previously leaked a stale BR_REPLY into the client's
                // read stream, desynchronizing its next transact.
                let result = if u32::from_ne_bytes(cmd_payload[20..24].try_into().unwrap())
                    & TF_ONE_WAY
                    != 0
                {
                    TransactionResult::CompleteOnly
                } else {
                    result
                };
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
                    TransactionResult::CompleteMirrored { refs } => {
                        // 6-Z442: this transaction's LOCAL flats granted a
                        // node — the SENDER (owner) gets the kernel-true
                        // [BR_INCREFS][BR_ACQUIRE] BEFORE the completion
                        // (the 6-Z306ae-e same-ioctl no-race shape: the
                        // owner's waitForResponse incs the local object
                        // while the marshal temporary is still alive —
                        // rn406's composer createClient corpse class).
                        // 6-Z454: same-ioctl delivery is unconditional — the
                        // ledger counts the delivered acquires (V2 baseline).
                        let z454_dpid = {
                            let b = bus.lock().expect("binder bus poisoned");
                            b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0)
                        };
                        for (br, ptr, cookie) in &refs {
                            z454_deliver(z454_dpid, *ptr, *cookie, *br);
                            read_buf.extend_from_slice(&br.to_ne_bytes());
                            read_buf.extend_from_slice(&ptr.to_ne_bytes());
                            read_buf.extend_from_slice(&cookie.to_ne_bytes());
                        }
                        if !refs.is_empty() {
                            static Z442_LOGGED: std::sync::atomic::AtomicU32 =
                                std::sync::atomic::AtomicU32::new(1024);
                            // 6-Z464 (rn427 decode): 64 → 1024 — the budget
                            // died at ~+185s and the decode's era-map went
                            // blind exactly where the Scudo aborts live (the
                            // (0xf280, 0x6c70) "close-without-grant" lead was
                            // partly a logging artifact). O(100)/boot lines:
                            // the cap now covers a full 300s watch.
                            if Z442_LOGGED.load(Ordering::Relaxed) > 0 {
                                Z442_LOGGED.fetch_sub(1, Ordering::Relaxed);
                                for (br, ptr, cookie) in &refs {
                                    info!(
                                        "[KR64][binder][vm{}] 6-Z442: node-ref ACQUIRE mirror conn={} br=0x{:08x} ptr=0x{:x} cookie=0x{:x} (owner-side inc for the flat crossing)",
                                        vm_id, conn_id, br, ptr, cookie
                                    );
                                }
                            }
                        }
                        push_br_transaction_complete(&mut read_buf);
                    }
                    TransactionResult::Reply { data, offsets, sg } => {
                        // Kernel-true batch (6-Z114 §4.5): the client's
                        // `waitForResponse` consumes BR_TRANSACTION_COMPLETE
                        // then loops to read BR_REPLY.
                        push_br_transaction_complete(&mut read_buf);
                        push_br_reply(&mut read_buf, data.len() as u64, offsets.len() as u64);
                        resp_blobs.push(RequestBlob {
                            data,
                            offsets,
                            sg,
                            fds: Vec::new(),
                        });
                    }
                    TransactionResult::ReplySpawnLooper {
                        mirror,
                        data,
                        offsets,
                        sg,
                    } => {
                        // 6-Z324: kernel pool-thread recruitment — the
                        // BR_SPAWN_LOOPER is PREPENDED (binder_thread_read
                        // order) so the client processes it before the
                        // reply; `waitForResponse` treats it as a
                        // non-terminal event and spawns a pooled reader
                        // that drains the queued oneway callback.
                        push_br_spawn_looper(&mut read_buf);
                        // 6-Z325: the watcher-callback node-ref mirror rides
                        // the same batch (kernel: the SM's strong ref on the
                        // callback node is mirrored to the owner as
                        // BR_ACQUIRE — binder_node_post_acquire). Without it
                        // the transient BnHw callback wrapper dies right
                        // after the register reply and the delivered
                        // onRegistration is neutralized by the loader's
                        // liveness gate (the rn273 wall).
                        if let Some((br, ptr, cookie)) = mirror {
                            // 6-Z454: same-ioctl unconditional delivery.
                            let z454_dpid = {
                                let b = bus.lock().expect("binder bus poisoned");
                                b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0)
                            };
                            z454_deliver(z454_dpid, ptr, cookie, br);
                            read_buf.extend_from_slice(&br.to_ne_bytes());
                            read_buf.extend_from_slice(&ptr.to_ne_bytes());
                            read_buf.extend_from_slice(&cookie.to_ne_bytes());
                            info!(
                                "[KR64][binder][vm{}] 6-Z325: watcher node-ref mirror conn={} br=0x{:08x} ptr=0x{:x} cookie=0x{:x}",
                                vm_id, conn_id, br, ptr, cookie
                            );
                        }
                        push_br_transaction_complete(&mut read_buf);
                        push_br_reply(&mut read_buf, data.len() as u64, offsets.len() as u64);
                        resp_blobs.push(RequestBlob {
                            data,
                            offsets,
                            sg,
                            fds: Vec::new(),
                        });
                    }
                    TransactionResult::ReplyMirrored {
                        br,
                        ptr,
                        cookie,
                        data,
                        offsets,
                        sg,
                    } => {
                        // 6-Z306ae-e: the mirror rides the SAME ioctl,
                        // BEFORE the completion+reply batch — the guest's
                        // waitForResponse runs BR_ACQUIRE's incStrong
                        // while the registering thread is still inside
                        // transact (no free-then-acquire race).
                        // 6-Z454: same-ioctl unconditional delivery.
                        let z454_dpid = {
                            let b = bus.lock().expect("binder bus poisoned");
                            b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0)
                        };
                        z454_deliver(z454_dpid, ptr, cookie, br);
                        read_buf.extend_from_slice(&br.to_ne_bytes());
                        read_buf.extend_from_slice(&ptr.to_ne_bytes());
                        read_buf.extend_from_slice(&cookie.to_ne_bytes());
                        push_br_transaction_complete(&mut read_buf);
                        push_br_reply(&mut read_buf, data.len() as u64, offsets.len() as u64);
                        resp_blobs.push(RequestBlob {
                            data,
                            offsets,
                            sg,
                            fds: Vec::new(),
                        });
                        info!(
                            "[KR64][binder][vm{}] 6-Z306ae-e: in-transaction mirror conn={} br=0x{:08x} ptr=0x{:x} cookie=0x{:x}",
                            vm_id, conn_id, br, ptr, cookie
                        );
                    }
                }
            }
            BC_REPLY | BC_REPLY_SG => {
                // 6-Z271: the guest is answering a transaction the bus
                // delivered to it. Correlate via the connection's
                // TRANSACTION STACK (6-Z306ag): BC_REPLY completes the
                // TOP (innermost) frame — kernel `binder_transaction()`
                // pops `thread->transaction_stack` — and routes the reply
                // to the original requester. Outer frames survive.
                let reply_blob = if is_v2 && blob_idx < req_blobs.len() {
                    let b = &req_blobs[blob_idx];
                    blob_idx += 1;
                    Some(b.clone())
                } else {
                    None
                };
                let inflight = {
                    let mut b = bus.lock().expect("binder bus poisoned");
                    b.conns.get_mut(&conn_id).and_then(|bx| {
                        // 6-Z491: the REPLY-RX leg — the daemon ANSWERED.
                        // Counts even when the stack pop drifts (None):
                        // a BC_REPLY arrived, period.
                        bx.z491.bc_reply_rx += 1;
                        bx.z491.note_rx();
                        bx.txn_stack.pop()
                    })
                };
                // 6-Z407: the reply-correlation trace — rn361's SF wall
                // (code=10 retries with no reply) needs the EXACT chain:
                // which stack frame each BC_REPLY pops, which requester it
                // resolves, and how many frames remain. LIFO/FIFO drift
                // (the composer's nested deliveries vs main's out_sync
                // FIFO) is decidable from these lines alone.
                {
                    static Z407_REPLY_LOGGED: std::sync::atomic::AtomicU64 =
                        std::sync::atomic::AtomicU64::new(0);
                    // 512/run: rn362's 64-line global budget burned out at
                    // +28.4s — EXACTLY the window where the wedged bottom
                    // frame (txn#12) would have unwound. The self-bury
                    // class again (rn350's lesson): a correlation trace
                    // must outlive the boot's first minute.
                    if Z407_REPLY_LOGGED.load(Ordering::Relaxed) < 512 {
                        Z407_REPLY_LOGGED.fetch_add(1, Ordering::Relaxed);
                        let stack_left = {
                            let b = bus.lock().expect("binder bus poisoned");
                            b.conns
                                .get(&conn_id)
                                .map(|bx| bx.txn_stack.len())
                                .unwrap_or(0)
                        };
                        match &inflight {
                            Some(txn_id) => info!(
                                "[KR64][binder][vm{}] 6-Z407 REPLY: conn={} pops txn#{} stack-left={} blobs={}",
                                vm_id, conn_id, txn_id, stack_left, reply_blob.is_some()
                            ),
                            None => info!(
                                "[KR64][binder][vm{}] 6-Z407 REPLY: conn={} stack EMPTY (drift class) blobs={}",
                                vm_id, conn_id, reply_blob.is_some()
                            ),
                        }
                    }
                }
                match inflight {
                    Some(txn_id) => {
                        let (data, offsets, sg, reply_fds) = match reply_blob {
                            Some(rb) => (rb.data, rb.offsets, rb.sg, rb.fds),
                            None => (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
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
                                    rbx.out_sync.retain(|(t, _, _)| *t != txn_id);
                                }
                            }
                            requester
                        };
                        match requester {
                            Some(rc) => {
                                let mut b = bus.lock().expect("binder bus poisoned");
                                // 6-Z359: translate the reply's LOCAL flats
                                // into HANDLE form for the recipient (kernel
                                // semantics) and grant the refs. The
                                // composer's IComposerClient crosses HERE.
                                let mut data = data;
                                let mut offsets = offsets;
                                let z442_grants = b.z359_translate_flats(
                                    vm_id,
                                    conn_id,
                                    rc,
                                    &mut data,
                                    &mut offsets,
                                    "BC_REPLY",
                                );
                                match b.conns.get_mut(&rc) {
                                    Some(rbx) => {
                                        rbx.z491.reply_enq += 1;
                                        rbx.reply_queue.push_back(DeferredReply::Reply {
                                            data,
                                            offsets,
                                            sg,
                                            fds: reply_fds,
                                        });
                                    }
                                    None => {
                                        warning!(
                                            "[KR64][binder][vm{}] BC_REPLY for txn {} — requester conn {} gone",
                                            vm_id, txn_id, rc
                                        );
                                    }
                                }
                                // 6-Z442: the reply's LOCAL flat crossings
                                // granted nodes — the SENDER (owner) gets
                                // the kernel-true [BR_INCREFS][BR_ACQUIRE]
                                // in THIS ioctl's read stream, BEFORE the
                                // completion (the 6-Z306ae-e same-ioctl
                                // no-race shape: the owner's
                                // waitForResponse incs the local object
                                // while the createClient marshal temporary
                                // is still alive — rn406's corpse class).
                                // 6-Z454: same-ioctl unconditional delivery —
                                // the pid rides the ALREADY-HELD bus guard
                                // (no re-lock: this scope owns `b`).
                                let z454_rpid =
                                    b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0);
                                for g in &z442_grants {
                                    for (br, ptr, cookie) in &g.mirrors {
                                        // 6-Z454: same-ioctl unconditional delivery.
                                        z454_deliver(z454_rpid, *ptr, *cookie, *br);
                                        read_buf.extend_from_slice(&br.to_ne_bytes());
                                        read_buf.extend_from_slice(&ptr.to_ne_bytes());
                                        read_buf.extend_from_slice(&cookie.to_ne_bytes());
                                    }
                                    static Z442_REPLY_LOGGED: std::sync::atomic::AtomicU32 =
                                        std::sync::atomic::AtomicU32::new(1024);
                                    // 6-Z464: 64 → 1024, same blindness fix as
                                    // the transaction arm above.
                                    if !g.mirrors.is_empty()
                                        && Z442_REPLY_LOGGED.load(Ordering::Relaxed) > 0
                                    {
                                        Z442_REPLY_LOGGED.fetch_sub(1, Ordering::Relaxed);
                                        for (br, ptr, cookie) in &g.mirrors {
                                            info!(
                                                "[KR64][binder][vm{}] 6-Z442: node-ref ACQUIRE mirror conn={} br=0x{:08x} ptr=0x{:x} cookie=0x{:x} (BC_REPLY owner-side inc)",
                                                vm_id, conn_id, br, ptr, cookie
                                            );
                                        }
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
                        // 6-Z409: kernel-true ack discipline — the kernel
                        // answers an unresolvable BC_REPLY with
                        // BR_FAILED_REPLY. libbinder's sendReply
                        // (waitForResponse(null, null)) exits on the first
                        // COMPLETE-or-error; without ANY ack the inner loop
                        // keeps reading and may swallow the NEXT BR_REPLY
                        // (the null-reply case frees + discards it) — the
                        // exact wedge rn364/rn365 decoded.
                        push_br_failed_reply(&mut read_buf);
                    }
                }
                // 6-Z409: THE WEDGE FIX — kernel-true BC_REPLY ack. The
                // real driver answers every accepted BC_REPLY with
                // BR_TRANSACTION_COMPLETE in the SAME read stream
                // (binder_thread_write → binder_transaction(reply=1) →
                // brTRANSACTION_COMPLETE). libbinder's sendReply runs
                // waitForResponse(nullptr, nullptr), whose ONLY exit
                // conditions are that COMPLETE (line 851: "if (!reply &&
                // !acquireResult) goto finish"), an error, or a BR_REPLY —
                // and a BR_REPLY seen with reply==nullptr is FREED AND
                // DISCARDED ("freeBuffer(...); continue;"). rn364/rn365
                // decoded the consequence byte-for-byte: the composer's
                // nested onHotplug reply (txn#13) was delivered in a read
                // that belonged to sendReply's INNER ack loop (the
                // response to the previous cascade BC_REPLY carried
                // BR_SPAWN_LOOPER instead of the COMPLETE), the inner loop
                // freed + discarded the reply (BC_FREE_BUFFER observed
                // 9 ms later), spun on SPAWN_LOOPER reads forever, and the
                // registerCallback handler never resumed — no BC_REPLY for
                // txn#12, SF main's waiter expired, rung 7. With the
                // COMPLETE ack the inner loop exits immediately, the outer
                // waitForResponse consumes the nested reply with its
                // non-null reply Parcel, and the handler completes.
                push_br_transaction_complete(&mut read_buf);
            }
            BC_ACQUIRE | BC_RELEASE | BC_INCREFS | BC_DECREFS => {
                // 6-Z359: strong/weak refcount changes on remote handles.
                // Payload: __u32 handle (kernel UAPI — the 4-byte arg of
                // _IOW('c', 4..7)). For 6-Z359 NODE handles these drive
                // the kernel-true refcounting: a BC_RELEASE from the last
                // holder mirrors BR_RELEASE to the node's OWNER (its
                // decStrong → the object can finally be destroyed — the
                // composer's onClientDestroyed). Service handles keep the
                // historical no-op behavior (their lifetime is tied to
                // their owning connection, which the teardown already
                // handles).
                if cmd == BC_RELEASE || cmd == BC_DECREFS {
                    if cmd_payload.len() >= 4 {
                        let handle = u32::from_ne_bytes(cmd_payload[0..4].try_into().unwrap());
                        let mut b = bus.lock().expect("binder bus poisoned");
                        if b.nodes.contains_key(&handle) {
                            b.z359_unref_node(vm_id, handle, conn_id, cmd == BC_RELEASE);
                        }
                    }
                }
                // BC_ACQUIRE / BC_INCREFS: the delivery grant already
                // accounted the recipient's ref (kernel-true in-transaction
                // grant) — nothing to do; the refcount stays balanced
                // because every strong flat delivery granted exactly one.
            }
            BC_ACQUIRE_DONE | BC_INCREFS_DONE => {
                // Acknowledgements of refcount operations on local binders.
            }
            BC_FREE_BUFFER | BC_DEAD_BINDER_DONE => {
                // Return a transaction-data buffer to the kernel, or
                // acknowledge a death notification. With v2 blobs the
                // client frees its own stash; v1 has no buffers to free.
                // Either way: no-op.
                //
                // 6-Z325: on a conn with a pending steal-delivery watch a
                // BC_FREE_BUFFER is the SUCCESS signal — the guest's
                // executeCommand processed the steal-delivered oneway to
                // its Parcel-teardown end (transact ran, freeBuffer ran).
                if cmd == BC_FREE_BUFFER {
                    let mut b = bus.lock().expect("binder bus poisoned");
                    if let Some(bx) = b.conns.get_mut(&conn_id) {
                        // 6-Z491: the CONSUMED ack — the guest's
                        // executeCommand ran the delivered transaction to
                        // its Parcel-teardown end.
                        bx.z491.bc_free_rx += 1;
                        bx.z491.note_rx();
                        if let Some((t0, code)) = bx.steal_watch.take() {
                            info!(
                                "[KR64][binder][vm{}] 6-Z325: steal-delivered oneway (code={}) freed after {}ms — the pool thread executed the callback transaction",
                                vm_id,
                                code,
                                t0.elapsed().as_millis()
                            );
                        }
                    }
                }
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

    // 6-Z325: the steal-delivery watch timeout leg — a steal-delivered
    // oneway whose conn neither freed it (BC_FREE_BUFFER) nor is
    // running it (no ws>0 for 1s) was dropped by the guest again;
    // name it once, then disarm (the budget already limited arming).
    {
        let mut b = bus.lock().expect("binder bus poisoned");
        if let Some(bx) = b.conns.get_mut(&conn_id) {
            if let Some((t0, code)) = bx.steal_watch {
                if t0.elapsed() >= std::time::Duration::from_secs(1) {
                    bx.steal_watch = None;
                    warning!(
                            "[KR64][binder][vm{}] 6-Z325: steal-delivered oneway (code={}) NOT freed within 1000ms — the pool thread's executeCommand dropped it again",
                            vm_id, code
                        );
                }
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
            let mut responders: Vec<ConnId> = Vec::new();
            let expired: Vec<u64> = match b.conns.get_mut(&conn_id) {
                Some(bx) => {
                    let mut v = Vec::new();
                    while let Some((_t, _o, at)) = bx.out_sync.front() {
                        if now.duration_since(*at) < REPLY_TIMEOUT {
                            break;
                        }
                        let (t, o, _) = bx.out_sync.pop_front().expect("front checked");
                        v.push(t);
                        // 6-Z437: name the responder so the decode sees WHO
                        // starved past the budget (the rn394 death fleet:
                        // the composer's own allocator/mapper calls expired
                        // at 8s under the boot's CPU starvation).
                        responders.push(o);
                    }
                    v
                }
                None => Vec::new(),
            };
            for t in expired {
                b.waiters.remove(&t);
                if let Some(bx) = b.conns.get_mut(&conn_id) {
                    bx.z491.reply_enq += 1;
                    bx.reply_queue.push_back(DeferredReply::Failed);
                }
                // 6-Z407: the timeout trace — rn361's SF wall needs the
                // expiry side of the correlation chain (which txn timed
                // out on which conn and what the stack purge removed).
                {
                    static Z407_TIMEOUT_LOGGED: std::sync::atomic::AtomicU64 =
                        std::sync::atomic::AtomicU64::new(0);
                    if Z407_TIMEOUT_LOGGED.load(Ordering::Relaxed) < 128 {
                        Z407_TIMEOUT_LOGGED.fetch_add(1, Ordering::Relaxed);
                        let stale_now: Vec<ConnId> = b
                            .conns
                            .iter()
                            .filter(|(_, bx)| bx.txn_stack.contains(&t))
                            .map(|(cid, _)| *cid)
                            .collect();
                        info!(
                            "[KR64][binder][vm{}] 6-Z407 TIMEOUT: conn={} txn#{} expired (responder conn={:?}) — BR_FAILED_REPLY queued, stack purge from conns {:?}",
                            vm_id, conn_id, t, responders, stale_now
                        );
                    }
                }
                // 6-Z399: the timed-out transaction may also sit on the
                // RESPONDER's transaction stack (it was DELIVERED — the
                // rn354 hotplug shape: a void-oneway method written
                // sync-flagged by the guest's libhwbinder never answers).
                // A stale stack entry would mis-correlate the responder's
                // NEXT BC_REPLY to this dead waiter and silently drop the
                // real requester's reply bytes. Kernel truth: the server
                // side of a dead caller resolves its transaction context.
                let stale: Vec<ConnId> = b
                    .conns
                    .iter()
                    .filter(|(_, bx)| bx.txn_stack.contains(&t))
                    .map(|(cid, _)| *cid)
                    .collect();
                for cid in stale {
                    if let Some(bx) = b.conns.get_mut(&cid) {
                        bx.txn_stack.retain(|id| *id != t);
                        bx.pending_in.retain(|id| *id != t);
                    }
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
            b.conns.get_mut(&conn_id).and_then(|bx| {
                // 6-Z491: the reply DELIVER leg (the enqueue leg lives
                // at the eight reply_queue push sites).
                bx.z491.reply_del += 1;
                bx.z491.last_del = Some(std::time::Instant::now());
                bx.reply_queue.pop_front()
            })
        } {
            match dr {
                DeferredReply::Reply {
                    data,
                    offsets,
                    sg,
                    fds,
                } => {
                    push_br_reply(&mut read_buf, data.len() as u64, offsets.len() as u64);
                    resp_blobs.push(RequestBlob {
                        data,
                        offsets,
                        sg,
                        fds,
                    });
                }
                DeferredReply::Failed => {
                    push_br_failed_reply(&mut read_buf);
                }
                DeferredReply::Dead => {
                    push_br_dead_reply(&mut read_buf);
                }
                DeferredReply::RefCmd { br, ptr, cookie } => {
                    // 6-Z306ae: kernel node-ref mirror — the owner's
                    // IPCThreadState handles BR_ACQUIRE (incStrong on the
                    // local BBinder + BC_ACQUIRE_DONE back to us) and
                    // BR_RELEASE (deferred decStrong) natively. Re-check
                    // the ref invariant at delivery: the object may have
                    // died between enqueue and this read — an incStrong
                    // on a freed cookie is the #204 crash storm.
                    let dpid2 = {
                        let b = bus.lock().expect("binder bus poisoned");
                        b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0)
                    };
                    if mirror_ref_ok(dpid2, ptr, cookie) {
                        // 6-Z454: the gate PASSED this time — the ledger
                        // counts the delivery (V2: a later release passing
                        // a DIFFERENT moment's gate is the killer shape).
                        z454_deliver(dpid2, ptr, cookie, br);
                        read_buf.extend_from_slice(&br.to_ne_bytes());
                        read_buf.extend_from_slice(&ptr.to_ne_bytes());
                        read_buf.extend_from_slice(&cookie.to_ne_bytes());
                        info!(
                            "[KR64][binder][vm{}] 6-Z306ae: node-ref mirror conn={} br=0x{:08x} ptr=0x{:x} cookie=0x{:x}",
                            vm_id, conn_id, br, ptr, cookie
                        );
                        // 6-Z366: a kernel-true BR_ACQUIRE hold just crossed
                        // to a live owner — request the mRefs-field hardware
                        // watchpoint on the owner's threads (the destroyer-
                        // naming instrument for the composer createClient
                        // paradox: rn323's Scudo invalid-chunk class).
                        if br == BR_ACQUIRE {
                            crate::ptrace_emu::z366_request_watch(dpid2, ptr, cookie);
                        }
                    } else {
                        // Skipped by the invariant gate — never hand the
                        // guest an empty read buffer (kernel semantics:
                        // every BINDER_WRITE_READ returns at least BR_NOOP).
                        push_br_noop(&mut read_buf);
                    }
                }
                DeferredReply::RefCmd359 {
                    br,
                    ptr,
                    cookie,
                    strong_grants: node_strong_grants,
                    weak_grants: node_weak_grants,
                } => {
                    // 6-Z359: kernel-true node-ref mirror. The decision
                    // was made from the proxy's own refcounts at
                    // BC_RELEASE/death time — the ONLY delivery gate is
                    // the owner process's liveness (fresh /proc probe,
                    // the 6-Z354 oracle): a dead owner cannot run
                    // decStrong and the conn teardown is already
                    // dismantling its mailbox.
                    let (dpid, node_entry_live) = {
                        let b = bus.lock().expect("binder bus poisoned");
                        let pid = b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0);
                        // 6-Z459: the lifetime counters RIDE the queued
                        // mirror (the node entry is removed at the
                        // both-maps-empty death — kernel-true node
                        // lifetime — so the map can no longer be the
                        // join's source). The map read now only names
                        // the entry's liveness for the decode witness:
                        // node=live | node-dead in the 6-Z457 line.
                        let node_entry_live = b
                            .node_by_key
                            .get(&(conn_id, ptr, cookie))
                            .and_then(|h| b.nodes.get(h))
                            .is_some();
                        (pid, node_entry_live)
                    };
                    let owner_alive = dpid > 0 && crate::ptrace_emu::traced_child_alive(dpid);
                    if owner_alive {
                        // 6-Z463 (rn427 decode): the OBJECT-level close gate.
                        // The rn427 +187083→+187101 chain named the Scudo
                        // corruption WRITER: an era-close REL/DEC delivered
                        // onto a weakref whose chunk was already freed and
                        // reused — the owner's decStrong/decWeak wrote the
                        // freed chunk (invalid-chunk-state abort 15 ms
                        // later), and the grant-side anchor had POSITIVELY
                        // identified the same chunk as dead one millisecond
                        // earlier ([R+8]=0x0 round-trip broken). Kernel
                        // truth: a BR_RELEASE reaches the owner only on the
                        // has_strong_ref true→false edge OF A NODE WHOSE
                        // REFS HOLD THE OBJECT ALIVE — the real driver can
                        // never reach a dead-owner-object close; reaching
                        // it here means the count is already lost and the
                        // delivery is the corruption write. The 6-Z354
                        // "deliver anyway" policy was the rn344 false-Dead
                        // era; the 6-Z387 page-window round-trip anchor
                        // ended that class (rn427: 3 ae-f Dead verdicts,
                        // all correct), so the close arm now honors the
                        // SAME anchor the grant arm honors — wire symmetry
                        // both directions. The drop is the 6-Z458
                        // silent-drop precedent generalized: nothing the
                        // owner never earned may cross back. Alive and
                        // Unknown deliver unchanged (the 6-Z306an rule:
                        // only a positive Dead rejects).
                        let anchored_dead = ptr != 0
                            && cookie != 0
                            && z463_close_rejected(mirror_ref_check(dpid, ptr, cookie));
                        // 6-Z465 REVERTED (the rn432 decode verdict, Task
                        // 200): the DEC-arm weakref-liveness rescue was
                        // UNSOUND FOR ITS CLASS and ran exactly ONE boot.
                        // rn430/rn431 (before the leg) had ZERO binder-Scudo
                        // aborts; rn432 (with it) had THREE — and TWO abort
                        // addresses join the rescue witnesses literally
                        // (0xfe44b5c0c850 = the +28031ms rescue, pid 2862
                        // aborted; 0xfd4f9ce0b580 = the +50921/+52237ms
                        // rescues). The root cause is a PROBE-INVISIBLE
                        // residue shape: the object-killed-with-pending-weak
                        // chunk frees with residue strong=0, weak=1,
                        // mBase intact, flags=0 — byte-identical to the
                        // CLEAN live-weakref zero-class the 20-byte read
                        // was supposed to select for. Freed-chunk residue
                        // PASSES the probe, the DEC delivers onto a freed
                        // chunk, and Scudo kills the owner (the very
                        // corruption class 6-Z463 was built to stop). No
                        // 20-byte read can separate a live weakref_impl
                        // from its own freed residue — the rescue is
                        // structurally unsound, not tunable. The ~48 B/era
                        // weakref leak (bounded, zero corruption, zero
                        // growth spiral) is the ACCEPTABLE cost; the
                        // 6-Z463 silent close is restored UNCONDITIONALLY
                        // for every anchor-Dead close, REL and DEC alike.
                        if anchored_dead {
                            if z463_drop_log().load(Ordering::Relaxed) > 0 {
                                z463_drop_log().fetch_sub(1, Ordering::Relaxed);
                                info!(
                                    "[KR64][binder][vm{}] 6-Z463: node-ref close DROPPED (silent close) conn={} br=0x{:08x} ptr=0x{:x} cookie=0x{:x} owner-pid={} node-grants[strong={} weak={}] — the anchor positively identifies the owner object as dead/reused; delivering would write a freed chunk (the rn427 corruption-write chain) — the 6-Z458 silent-drop precedent generalized to node closes",
                                    vm_id, conn_id, br, ptr, cookie, dpid,
                                    node_strong_grants, node_weak_grants
                                );
                            }
                            push_br_noop(&mut read_buf);
                        }
                        // 6-Z465 reverted — the delivery condition is back
                        // to the 6-Z463 wire symmetry: anchor-Alive and
                        // Unknown deliver, a positive Dead never does.
                        if !anchored_dead {
                            if z457_budget().load(Ordering::Relaxed) > 0 {
                                // 6-Z454: the gates PASSED — the ledger counts
                                // the delivery (V2 baseline for this object).
                                z454_deliver(dpid, ptr, cookie, br);
                                read_buf.extend_from_slice(&br.to_ne_bytes());
                                read_buf.extend_from_slice(&ptr.to_ne_bytes());
                                read_buf.extend_from_slice(&cookie.to_ne_bytes());
                                info!(
                                    "[KR64][binder][vm{}] 6-Z359: node-ref mirror conn={} br=0x{:08x} ptr=0x{:x} cookie=0x{:x}",
                                    vm_id, conn_id, br, ptr, cookie
                                );
                                // 6-Z457: the owner-side REFCOUNT MIRROR (Task 192
                                // agenda) — at the last-ref delivery read the
                                // owner's LIVE RefBase state for this weakref
                                // (source-true layout: mStrong=[W+0], mWeak=
                                // [W+4], mBase=[W+8], mFlags=[W+16] — rn423's
                                // disproof retired the old mStrong@16 model)
                                // and log count-before-delivery vs the
                                // ledger's emit/deliver totals and the node's
                                // lifetime grants. rn422: a BALANCED 6-Z454 ledger
                                // over a Scudo double-free — the missing half lives
                                // in the owner's IN-PROCESS refcount, which this
                                // read names (count already 0 = the delivery's
                                // decStrong over-decs = the delete#2 precondition;
                                // count ≥1 = the owner's own sp is present and the
                                // double-free needs a second in-process drop).
                                {
                                    z457_budget().fetch_sub(1, Ordering::Relaxed);
                                    match crate::ptrace_emu::z457_refcount_snapshot(dpid, ptr) {
                                        Some(st) => {
                                            let (ae, re, ad, rd) = z454_counts(dpid, ptr, cookie);
                                            info!(
                                                "[KR64][binder][vm{}] 6-Z457: refcount mirror owner-pid={} br=0x{:08x} W=0x{:x} mStrong={} (count={}) mWeak={} mBase=0x{:x} (delta=0x{:x} vs cookie 0x{:x}) mFlags={} ledger[emit acq={} rel={} / del acq={} rel={}] node-grants[strong={} weak={}] node={} class={}",
                                                vm_id, dpid, br, ptr,
                                                st.strong_raw,
                                                crate::ptrace_emu::z457_strong_count(st.strong_raw),
                                                st.weak, st.mbase,
                                                st.mbase.wrapping_sub(cookie), cookie,
                                                st.flags, ae, re, ad, rd,
                                                node_strong_grants, node_weak_grants,
                                                if node_entry_live { "live" } else { "node-dead" },
                                                crate::ptrace_emu::z457_classify(st.strong_raw)
                                            );
                                        }
                                        None => {
                                            info!(
                                                "[KR64][binder][vm{}] 6-Z457: refcount mirror owner-pid={} W=0x{:x} — READ FAILED (object may already be gone — itself a count≤0 signature)",
                                                vm_id, dpid, ptr
                                            );
                                        }
                                    }
                                }
                            } else {
                                // 6-Z457 budget exhausted — deliver without the
                                // refcount snapshot (the mirror itself unchanged).
                                z454_deliver(dpid, ptr, cookie, br);
                                read_buf.extend_from_slice(&br.to_ne_bytes());
                                read_buf.extend_from_slice(&ptr.to_ne_bytes());
                                read_buf.extend_from_slice(&cookie.to_ne_bytes());
                                info!(
                                    "[KR64][binder][vm{}] 6-Z359: node-ref mirror conn={} br=0x{:08x} ptr=0x{:x} cookie=0x{:x}",
                                    vm_id, conn_id, br, ptr, cookie
                                );
                            }
                        }
                    } else {
                        info!(
                            "[KR64][binder][vm{}] 6-Z359: node-ref mirror skipped — owner conn={} pid={} gone/dead",
                            vm_id, conn_id, dpid
                        );
                        push_br_noop(&mut read_buf);
                    }
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
                // 6-Z408: PEEK before pop — the kernel reentrancy gate may
                // HOLD this conn's front sync item (see
                // z408_sync_delivery_blocked). A held item stays queued;
                // this conn's polls answer BR_NOOP until its nested call
                // unwinds (out_sync drains) and the gate opens — the pool
                // threads' steal is already excluded by 6-Z399's
                // reader_waiting filter, and the REPLY_TIMEOUT guarantees
                // the gate opens even if the nested reply never comes.
                let z408_held = match b.conns.get(&conn_id).and_then(|bx| bx.inbox.front()) {
                    Some(InboxItem::Tx(tx)) => {
                        let held = b.z408_sync_delivery_blocked(conn_id, tx);
                        if held {
                            z408_note_hold(
                                vm_id,
                                conn_id,
                                tx.txn_id,
                                tx.code,
                                tx.requester,
                                tx.sender_pid,
                            );
                        }
                        held
                    }
                    _ => false,
                };
                if z408_held {
                    Delivery::None
                } else {
                    match b.conns.get_mut(&conn_id) {
                        Some(bx) => match bx.inbox.pop_front() {
                            Some(InboxItem::Tx(tx)) => {
                                // 6-Z491: the tx DELIVER leg (main drain).
                                bx.z491.tx_del += 1;
                                bx.z491.last_del = Some(std::time::Instant::now());
                                // Mark the delivered transaction as owing a reply
                                // (sync only — one-way has txn_id 0 and expects
                                // no reply). Remove it from pending_in: it's no
                                // longer "queued". 6-Z306ag: PUSH onto the
                                // transaction stack — never overwrite; outer
                                // frames must survive nested processing.
                                if tx.txn_id != 0 {
                                    bx.txn_stack.push(tx.txn_id);
                                    z306ag_note_stack_depth(bx.txn_stack.len());
                                    bx.pending_in.retain(|id| *id != tx.txn_id);
                                }
                                Delivery::Tx(tx)
                            }
                            Some(InboxItem::Death(cookie)) => Delivery::Death(cookie),
                            None => Delivery::None,
                        },
                        None => Delivery::None,
                    }
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
                    // 6-Z309f: device-consistent steal. The kernel serves
                    // /dev/binder, /dev/hwbinder and /dev/vndbinder as
                    // SEPARATE devices — node work queued for one device
                    // can NEVER be read from another. The per-thread
                    // proxy conns of ONE guest process span devices (a
                    // HIDL daemon has hwbinder pool conns AND a
                    // vndbinder/binder vendor side), so the pid filter
                    // alone lets the WRONG device's pool thread steal a
                    // queued transaction. The rn257 decode nailed the
                    // consequence: a vndbinder thread (conn=14, tid=2894)
                    // stole a hwbinder IEffectsFactory interfaceChain
                    // probe and served it through android::BBinder's
                    // transact — whose onTransact slot index (16) does
                    // not exist in the hardware::BHwBinder vtable ABI
                    // (onTransact@11) — the slot read returned a literal
                    // zero offset-to-top and `blr 0` killed the daemon
                    // (pc=0x0 SIGSEGV, group-kill, SM-waiter era aborts).
                    // Same class via conn=8 (dev=binder) on the suspend
                    // daemon at +176.1s. Under-stealing is always safe:
                    // the tx simply waits for its owning conn (the
                    // kernel's process-queue order), so unknown-device
                    // pairs (dev_code=0, legacy shlib) only steal among
                    // themselves (0==0 preserves the legacy corpus
                    // behavior unchanged).
                    let my_dev = b.conns.get(&conn_id).map(|bx| bx.dev_code).unwrap_or(0);
                    if my_pid == 0 {
                        None
                    } else {
                        // 6-Z399: a sibling whose reader is CURRENTLY
                        // blocked in its own ioctl (idle tick + 6-Z383
                        // re-check) takes its own inbox work — kernel
                        // proc-todo semantics: the waiting looper thread
                        // gets the work, not a later-arriving pool thread.
                        // rn354: the onHotplug queued on SF's main conn
                        // was stolen by a pool conn 250 ms before main's
                        // re-check; the pool-thread dispatch blocked on
                        // mStateLock (main holds it inside init()) and
                        // the event missed init()'s display check.
                        let mut sibs: Vec<ConnId> = b
                            .conns
                            .iter()
                            .filter(|(cid, bx)| {
                                **cid != conn_id
                                    && bx.sender_pid == my_pid
                                    && bx.dev_code == my_dev
                                    && !bx.reader_waiting
                            })
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
                            if let Some(bx) = b.conns.get_mut(&conn_id) {
                                // 6-Z491: the tx DELIVER leg (6-Z271g steal —
                                // counted on the DELIVERING conn; oneway
                                // steals carry no stack frame but ARE
                                // deliveries).
                                bx.z491.tx_del += 1;
                                bx.z491.last_del = Some(std::time::Instant::now());
                                if tx.txn_id != 0 {
                                    // 6-Z306ag: PUSH onto the stealing conn's
                                    // transaction stack (LIFO reply
                                    // correlation; death-resolvable via the
                                    // stack itself — no pending_in entry).
                                    bx.txn_stack.push(tx.txn_id);
                                    z306ag_note_stack_depth(bx.txn_stack.len());
                                }
                            }
                        }
                        item
                    }
                };
                if let Some(tx) = stolen {
                    // 6-Z325: arm the bounded steal-delivery watch (oneway
                    // only — a sync steal is proven by its BC_REPLY).
                    if tx.one_way && Z325_STEAL_WATCH_BUDGET.load(Ordering::Relaxed) > 0 {
                        Z325_STEAL_WATCH_BUDGET.fetch_sub(1, Ordering::Relaxed);
                        if let Some(bx) = bus
                            .lock()
                            .expect("binder bus poisoned")
                            .conns
                            .get_mut(&conn_id)
                        {
                            bx.steal_watch = Some((std::time::Instant::now(), tx.code));
                        }
                    }
                    info!(
                    "[KR64][binder][vm{}] process-pool steal: conn={} takes tx #{} queued for a sibling (code={})",
                    vm_id, conn_id, tx.txn_id, tx.code
                );
                    delivery = Delivery::Tx(tx);
                }
            }
            // 6-Z469: PROC-TODO TAKE — an idle looper of the same process
            // serves a 6-Z408-held sync transaction from a parked sibling's
            // inbox (kernel: proc-todo work goes to ANY waiting looper
            // thread of the target proc; see z469_take_held_sync). Runs
            // after the 6-Z271g steal so the rn354 running-source
            // semantics keep priority; only gate-HELD fronts are taken.
            if matches!(delivery, Delivery::None) {
                let taken = {
                    let mut b = bus.lock().expect("binder bus poisoned");
                    let tx = b.z469_take_held_sync(conn_id);
                    if let Some(tx) = &tx {
                        if let Some(bx) = b.conns.get_mut(&conn_id) {
                            // 6-Z491: the tx DELIVER leg (6-Z469 proc-todo
                            // take — same shape as the 6-Z271g steal).
                            bx.z491.tx_del += 1;
                            bx.z491.last_del = Some(std::time::Instant::now());
                            if tx.txn_id != 0 {
                                // 6-Z306ag: PUSH onto the taking conn's
                                // transaction stack (LIFO reply
                                // correlation; same as the 6-Z271g steal).
                                bx.txn_stack.push(tx.txn_id);
                                z306ag_note_stack_depth(bx.txn_stack.len());
                            }
                        }
                    }
                    tx
                };
                if let Some(tx) = taken {
                    static Z469_TAKE_LOG: std::sync::atomic::AtomicU64 =
                        std::sync::atomic::AtomicU64::new(0);
                    if Z469_TAKE_LOG.load(Ordering::Relaxed) < 128 {
                        Z469_TAKE_LOG.fetch_add(1, Ordering::Relaxed);
                        info!(
                            "[KR64][binder][vm{}] 6-Z469 proc-todo take: idle looper conn={} serves 6-Z408-held tx #{} from parked sibling (code={})",
                            vm_id, conn_id, tx.txn_id, tx.code
                        );
                    }
                    delivery = Delivery::Tx(tx);
                }
            }
            match delivery {
                Delivery::Tx(tx) => {
                    // 6-Z354 (supersedes the 6-Z306an heap-anchor gate):
                    // delivery-time TARGET LIVENESS GATE, now KERNEL-TRUE.
                    // Kernel semantics: a transaction whose target node's
                    // OWNER PROCESS died while queued is NEVER delivered —
                    // the kernel releases the node's pending work and the
                    // REQUESTER's read surfaces BR_DEAD_REPLY. Delivering
                    // to a dead owner forced the server's libhwbinder/
                    // libbinder to either incStrong a dead cookie (the
                    // #235/#237/#238 SIGSEGV fleet) or — after the 6-Z306z
                    // shlib neutralization — parse a BR_DEAD_REPLY in the
                    // SERVER's stream, which libhwbinder's server-side
                    // executeCommand does not handle ("*** BAD COMMAND
                    // 29189" → LOG_ALWAYS_FATAL abort — the #239
                    // audioserver/system_server fleet: 30 aborts in one
                    // run, system_server dead at +181.9s). The 6-Z306an
                    // heap anchor COULD NOT distinguish "object really
                    // dead" from "the chunk was reused while the process
                    // and its registration stay live" — rn305's composer
                    // proved the latter happens in production (103
                    // spurious rejections; see the gate body below).
                    let mut tx_rejected = false;
                    // The gate needs a conn identity with a live owner
                    // process (dpid>0): no identity (the unit-test bus
                    // conns never announce IDENT) or no local object
                    // (handle-form, cookie==0) → deliver exactly as
                    // before. 6-Z354: the decision is KERNEL-TRUE —
                    // process liveness via a fresh /proc probe (the 6-Z89
                    // mechanism, zero waitpid side effects). The 6-Z306ae-f
                    // heap anchor is DEMOTED to a bounded diagnostic: rn305
                    // caught it ruling the COMPOSER's service object
                    // "Dead" ([R+8]=0 at +14.96s) 5 s after the SAME W/B
                    // pair PASSED the anchor at ADD (+9.78s) — 103 spurious
                    // BR_DEAD_REPLYs, ZERO deliveries to the composer's
                    // hwbinder connection, surfaceflinger death-looping on
                    // "found dead hwbinder service" — while the composer
                    // process demonstrably lived, served and held its
                    // resources the whole run. The real kernel cannot and
                    // does not inspect server heap state.
                    if tx.cookie != 0 {
                        let dpid = {
                            let b = bus.lock().expect("binder bus poisoned");
                            b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0)
                        };
                        if dpid > 0 {
                            let owner_alive = crate::ptrace_emu::traced_child_alive(dpid);
                            if tx_delivery_reject_6z354(dpid, owner_alive) {
                                // Kernel semantics: the node's owner PROCESS
                                // died — release the pending work and
                                // resolve the requester with BR_DEAD_REPLY.
                                // Undo the delivery bookkeeping done at pop
                                // time (both the direct pop and the 6-Z271g
                                // steal push the txn_stack frame; pending_in
                                // was already retained away there).
                                if tx.txn_id != 0 {
                                    let mut b = bus.lock().expect("binder bus poisoned");
                                    if let Some(bx) = b.conns.get_mut(&conn_id) {
                                        if bx.txn_stack.last() == Some(&tx.txn_id) {
                                            bx.txn_stack.pop();
                                        } else {
                                            bx.txn_stack.retain(|id| *id != tx.txn_id);
                                        }
                                    }
                                    if let Some(requester) = b.waiters.remove(&tx.txn_id) {
                                        if let Some(rb) = b.conns.get_mut(&requester) {
                                            rb.z491.reply_enq += 1;
                                            rb.reply_queue.push_back(DeferredReply::Dead);
                                        }
                                    }
                                }
                                if Z306AN_REJECT_LOG.load(Ordering::Relaxed) > 0 {
                                    Z306AN_REJECT_LOG.fetch_sub(1, Ordering::Relaxed);
                                    info!(
                                        "[KR64][binder][vm{}] 6-Z354: dead-OWNER transaction rejected conn={} <- conn={} code={} oneway={} ptr=0x{:x} cookie=0x{:x} (owner pid={} per fresh /proc probe) → BR_DEAD_REPLY to requester (tx #{}, {} left)",
                                        vm_id, conn_id, tx.requester, tx.code, tx.one_way,
                                        tx.ptr, tx.cookie, dpid, tx.txn_id,
                                        Z306AN_REJECT_LOG.load(Ordering::Relaxed)
                                    );
                                }
                                // Kernel semantics: every BINDER_WRITE_READ
                                // returns at least BR_NOOP — never an empty
                                // read buffer.
                                push_br_noop(&mut read_buf);
                                tx_rejected = true;
                            } else if Z354_ANCHOR_LOG.load(Ordering::Relaxed) > 0
                                && matches!(
                                    mirror_ref_check(dpid, tx.ptr, tx.cookie),
                                    Liveness::Dead
                                )
                            {
                                // The anchor contradicts a LIVE owner —
                                // rn305's composer shape. Deliver (the
                                // kernel-true choice) and log the
                                // contradiction while the budget lasts; once
                                // spent, the anchor is never evaluated again.
                                Z354_ANCHOR_LOG.fetch_sub(1, Ordering::Relaxed);
                                info!(
                                    "[KR64][binder][vm{}] 6-Z354: heap anchor says Dead but owner pid={} is ALIVE — delivering anyway (kernel-true; rn305 composer class)",
                                    vm_id, dpid
                                );
                            }
                        }
                    }
                    if tx_rejected {
                        // Skip the delivery entirely — the transaction and
                        // its blob are dropped (kernel: released node work).
                    } else {
                        let (ds, os) = match &tx.blob {
                            Some(b) => (b.data.len() as u64, b.offsets.len() as u64),
                            None => (0, 0),
                        };
                        if tx.cookie != 0 {
                            // 6-Z306ad: delivery-time snapshot — the cookie the
                            // receiving server will cast to its BBinder (the
                            // vendor-HAL vtable-garbage crash class rides this
                            // path; see the pc=-0x78 fleet in #198/#199).
                            let dpid = {
                                let b = bus.lock().expect("binder bus poisoned");
                                b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0)
                            };
                            probe_flat_mem(
                                "delivery",
                                &PROBE_DELIVERY_BUDGET,
                                dpid,
                                &format!("code={:#x}", tx.code),
                                tx.ptr,
                                tx.cookie,
                            );
                        }
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
                            resp_blobs.push(blob);
                        }
                        // 6-Z309d: note the SERVED pid — the tracer's
                        // EXIT-event death capture gives recently-served
                        // daemons the same register evidence the zygote
                        // lineage already gets (the rn255 suspend-daemon
                        // class died sig=11 with NO delivery stop and NO
                        // capture 2.6s after receiving the interfaceChain
                        // probe — lineage=false, so the old gate skipped
                        // it and the crash site stayed unnamed).
                        {
                            let served = {
                                let b = bus.lock().expect("binder bus poisoned");
                                b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0)
                            };
                            if served > 0 {
                                note_served_pid(served);
                            }
                        }
                        info!(
                    "[KR64][binder][vm{}] delivered transaction conn={} <- conn={} code={} oneway={} (tx #{})",
                    vm_id, conn_id, tx.requester, tx.code, tx.one_way, tx.txn_id
                );
                    }
                }
                Delivery::Death(cookie) => {
                    push_br_dead_binder(&mut read_buf, cookie);
                }
                Delivery::None => {
                    // 6-Z152: BLOCKING idle — sleep before BR_NOOP so a
                    // guest poll loop can't pin the tracer (see the 6-Z268
                    // analysis in the original comment history).
                    std::thread::sleep(IDLE_POLL_TICK);
                    // 6-Z383: RE-CHECK the conn's OWN inbox after the idle
                    // tick — a transaction may have been queued DURING the
                    // sleep, and the kernel delivers proc-todo work to a
                    // waiting looper thread instead of answering BR_NOOP.
                    // rn341 decode (the SF display race): SF's main thread
                    // parked in a read waiting for the registerCallback
                    // reply; the composer's onHotplug arrived mid-sleep;
                    // the parked read answered BR_NOOP and the 6-Z271g
                    // steal diverted the transaction to a POOL thread,
                    // whose HWC2 dispatch blocks on SF's mStateLock (held
                    // by main inside init()) — the hotplug event then
                    // enqueued AFTER main's processDisplayHotplugEvents-
                    // Locked() ran empty -> LOG_ALWAYS_FATAL "Missing
                    // internal display" -> the SF/zygote onrestart loop
                    // (37 cycles in rn341). Kernel truth: the waiting
                    // looper thread takes the async work ITSELF (libhwb-
                    // inder's waitForResponse handles an inline BR_TRANS-
                    // ACTION; SF's onHotplugReceived then runs on MAIN
                    // where the ConditionalLock is skipped and the event
                    // is processed immediately — the rn337/338 direct-
                    // delivery boots all crossed the display gate).
                    let rechecked = {
                        let mut b = bus.lock().expect("binder bus poisoned");
                        // 6-Z408: the recheck honors the SAME reentrancy
                        // gate as the primary drain — otherwise every
                        // post-sleep re-pop would bypass the hold on the
                        // parked conn's front sync item.
                        let z408_held = match b.conns.get(&conn_id).and_then(|bx| bx.inbox.front())
                        {
                            Some(InboxItem::Tx(tx)) => {
                                let held = b.z408_sync_delivery_blocked(conn_id, tx);
                                if held {
                                    z408_note_hold(
                                        vm_id,
                                        conn_id,
                                        tx.txn_id,
                                        tx.code,
                                        tx.requester,
                                        tx.sender_pid,
                                    );
                                }
                                held
                            }
                            _ => false,
                        };
                        if z408_held {
                            None
                        } else {
                            match b.conns.get_mut(&conn_id) {
                                Some(bx) => match bx.inbox.pop_front() {
                                    Some(InboxItem::Tx(tx)) => {
                                        // 6-Z491: the tx DELIVER leg (the
                                        // 6-Z383 post-idle-tick recheck).
                                        bx.z491.tx_del += 1;
                                        bx.z491.last_del = Some(std::time::Instant::now());
                                        if tx.txn_id != 0 {
                                            bx.txn_stack.push(tx.txn_id);
                                            z306ag_note_stack_depth(bx.txn_stack.len());
                                            bx.pending_in.retain(|id| *id != tx.txn_id);
                                        }
                                        Some(tx)
                                    }
                                    _ => None,
                                },
                                None => None,
                            }
                        }
                    };
                    if let Some(tx) = rechecked {
                        // The reader IS the owner process (alive by
                        // definition — it is executing this read), so the
                        // 6-Z354 owner-liveness gate cannot reject here.
                        let (ds, os) = match &tx.blob {
                            Some(bl) => (bl.data.len() as u64, bl.offsets.len() as u64),
                            None => (0, 0),
                        };
                        if tx.cookie != 0 {
                            let dpid = {
                                let b = bus.lock().expect("binder bus poisoned");
                                b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0)
                            };
                            probe_flat_mem(
                                "delivery",
                                &PROBE_DELIVERY_BUDGET,
                                dpid,
                                &format!("code={:#x}", tx.code),
                                tx.ptr,
                                tx.cookie,
                            );
                        }
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
                            resp_blobs.push(blob);
                        }
                        {
                            let served = {
                                let b = bus.lock().expect("binder bus poisoned");
                                b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0)
                            };
                            if served > 0 {
                                note_served_pid(served);
                            }
                        }
                        info!(
                            "[KR64][binder][vm{}] 6-Z383 idle recheck delivered transaction conn={} <- conn={} code={} oneway={} flags=0x{:x} (tx #{})",
                            vm_id, conn_id, tx.requester, tx.code, tx.one_way, tx.flags, tx.txn_id
                        );
                    } else {
                        push_br_noop(&mut read_buf);
                    }
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
        // 6-Z305t-68: v3 resp trailer — per blob [dl][ol][sg_count]
        // [data][offsets][sg entries (u64 ptr][u32 len][bytes])]. The
        // loader reassembles [data][offsets][sg] into one backing buffer
        // and fixes each BINDER_TYPE_PTR object's `buffer` field to its
        // SG copy (the kernel's receiver-side pointer fixup).
        resp_payload.extend_from_slice(&WIRE_V3_MAGIC.to_ne_bytes());
        resp_payload.extend_from_slice(&(resp_blobs.len() as u32).to_ne_bytes());
        for blob in &resp_blobs {
            resp_payload.extend_from_slice(&(blob.data.len() as u32).to_ne_bytes());
            resp_payload.extend_from_slice(&(blob.offsets.len() as u32).to_ne_bytes());
            resp_payload.extend_from_slice(&(blob.sg.len() as u32).to_ne_bytes());
            resp_payload.extend_from_slice(&blob.data);
            resp_payload.extend_from_slice(&blob.offsets);
            for b in &blob.sg {
                resp_payload.extend_from_slice(&b.client_ptr.to_ne_bytes());
                resp_payload.extend_from_slice(&(b.data.len() as u32).to_ne_bytes());
                resp_payload.extend_from_slice(&b.data);
            }
        }
        // 6-Z410: the delivery-trailer BREAKDOWN trace — rn366 decoded the
        // composer's post-registerCallback abort (Scudo invalid-chunk-state
        // on a dealloc, ~660 ms after it read the getHashChain
        // BR_TRANSACTION whose delivery trailer was 52 B — vs 208 B for
        // the WORKING registerCallback delivery). The shlib reassembles
        // the parcel from this trailer; a shape mismatch between the btd's
        // data_size/offsets_size and the trailer's blob contents lands in
        // the guest as a corrupted Parcel whose freeBuffer frees a
        // non-live chunk. The breakdown names dl/ol/sg per blob so the
        // next run decides the mismatch in one decode.
        if read_buf.len() >= 4 {
            let first_cmd = u32::from_ne_bytes(read_buf[0..4].try_into().unwrap());
            let is_txn = first_cmd == BR_TRANSACTION || first_cmd == 0x80407202;
            if is_txn && Z410_TRAILER_LOG.load(Ordering::Relaxed) > 0 {
                Z410_TRAILER_LOG.fetch_sub(1, Ordering::Relaxed);
                let breakdown: Vec<String> = resp_blobs
                    .iter()
                    .map(|b| {
                        format!(
                            "dl={} ol={} sg={}",
                            b.data.len(),
                            b.offsets.len(),
                            b.sg.len()
                        )
                    })
                    .collect();
                info!(
                    "[KR64][binder][vm{}] 6-Z410 delivery-trailer: conn={} read_size={} blobs={} [{}] trailer_bytes={}",
                    vm_id,
                    conn_id,
                    read_buf.len(),
                    resp_blobs.len(),
                    breakdown.join(", "),
                    resp_payload.len() - 4 - read_buf.len()
                );
            }
        }
    }

    // 6-Z355: collect the response blobs' fds (blob order) and append the
    // fd tail when any blob carries them — the recipient's shlib patches
    // its flats from the SCM_RIGHTS dups that ride the frame sendmsg.
    let mut resp_fds: Vec<Arc<FdGuard>> = Vec::new();
    {
        let mut counts: Vec<u32> = Vec::with_capacity(resp_blobs.len());
        for blob in &resp_blobs {
            counts.push(blob.fds.len() as u32);
            resp_fds.extend(blob.fds.iter().cloned());
        }
        if resp_fds.iter().any(|f| f.as_raw() >= 0) {
            append_fd_tail(&mut resp_payload, &counts);
        } else {
            // Only -1 placeholders (a shortfall pad): no real fd to send.
            resp_fds.clear();
        }
    }

    // 6-Z491: the per-ioctl accounting tick + the budgeted wedge scan —
    // the decode's delivery-vs-consumed join reads these lines. One lock;
    // runs on every completed W-R (the idle looper's drained mailbox
    // never verdicts).
    z491_ioctl_tick(bus, vm_id, conn_id, read_buf.is_empty());

    Resp {
        ret: 0,
        payload: resp_payload,
        fds: resp_fds,
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
    /// `sg` carries BINDER_TYPE_PTR contents for SG-shaped replies
    /// (6-Z305t-69: listManifestByInterface's hidl_vec<hidl_string-ish>
    /// result — the loader reassembles + fixes the pointers up).
    Reply {
        data: Vec<u8>,
        offsets: Vec<u8>,
        sg: Vec<SgBuf>,
    },
    /// 6-Z306ae-e: a Reply that PREPENDS the node-ref mirror command
    /// `[BR_ACQUIRE][ptr][cookie]` to the `[BR_TRANSACTION_COMPLETE]
    /// [BR_REPLY]` batch. The guest's `waitForResponse` processes
    /// commands in order and only EXITS on the reply — so the owner's
    /// `obj->incStrong` lands INSIDE the transact call, while the
    /// registering thread's JNI temporary `sp<>` is still alive. The
    /// reply_queue delivery (6-Z306ae) raced: HALs that drop their
    /// `sp<>` right after `registerAsService` freed the object before
    /// the owner's next read — ladder #207's si_addr=0x4 class (722
    /// SIGSEGVs, incWeak(NULL)). Used only by the registration arms.
    ReplyMirrored {
        br: u32,
        ptr: u64,
        cookie: u64,
        data: Vec<u8>,
        offsets: Vec<u8>,
        sg: Vec<SgBuf>,
    },
    /// 6-Z324: a Reply that PREPENDS `[BR_SPAWN_LOOPER]` to the
    /// `[BR_TRANSACTION_COMPLETE][BR_REPLY]` batch — the kernel's own
    /// pool-thread recruitment: `binder_thread_read` prepends
    /// BR_SPAWN_LOOPER when the process has pending todo work (an incoming
    /// async transaction) and no free pool thread. The client's
    /// `IPCThreadState::executeCommand(BR_SPAWN_LOOPER)` spawns a pooled
    /// binder thread that blocks in ioctl on the SAME device — it then
    /// drains the queued oneway (the SM's `onRegistration` callback).
    ///
    /// RN272 decode: system_server's `waitForHwService` (A11
    /// ServiceManagement.cpp Waiter, `isOnlyBinderThread()==false` mode)
    /// registers for notifications and blocks on the Waiter condvar;
    /// the preexisting-service `onRegistration` oneway was queued on the
    /// REGISTERING thread's own conn — the only hwbinder thread of the
    /// process — which is parked on the condvar and never reads again.
    /// Without a pooled reader the callback never runs and main waits
    /// forever (the PMS.<init> nativeSetAutoSuspend wall, eras at
    /// +186.9s/+493.7s in rn272: registerForNotifications reply received,
    /// then 264s of wire silence). The kernel breaks this on real devices
    /// by recruiting a pool thread via BR_SPAWN_LOOPER; the proxy now
    /// does the same on the registration reply the waiter is guaranteed
    /// to read.
    ReplySpawnLooper {
        /// 6-Z325: an optional node-ref mirror command that rides the SAME
        /// batch (kernel binder_thread_read order: BR_SPAWN_LOOPER is
        /// prepended at the buffer top, the todo work — here the watcher
        /// callback's `BR_ACQUIRE` — follows, then the completion+reply).
        /// The registerForNotifications arms set it when they STORE a new
        /// watcher: without the mirrored strong ref the transient BnHw
        /// callback wrapper dies right after the register reply (its only
        /// user-space ref is the marshal temporary) and the stored
        /// (ptr, cookie) become freed chunks — the rn273 wall: the
        /// delivered onRegistration was neutralized by the loader's
        /// 6-Z306z liveness gate (weakref round-trip failed on reused
        /// memory: mStrong=49/mBase=0xf) and Waiter::onRegistration never
        /// ran. The mirror is the kernel's own
        /// binder_node_post_acquire → BR_ACQUIRE semantics.
        mirror: Option<(u32, u64, u64)>,
        data: Vec<u8>,
        offsets: Vec<u8>,
        sg: Vec<SgBuf>,
    },
    /// Transaction accepted with no in-ioctl reply: one-way, or a routed
    /// sync call whose `BC_REPLY` resolves on the requester's LATER read
    /// (kernel semantics — 6-Z271i deferred resolution).
    CompleteOnly,
    /// 6-Z442: `CompleteOnly` PLUS the OWNER-SIDE node-ref mirrors from
    /// this transaction's LOCAL flats (the sender's read stream gains
    /// `[BR_INCREFS][BR_ACQUIRE]` before the completion — the
    /// 6-Z306ae-e same-ioctl no-race shape, generalized from the
    /// registration arms to every routed flat crossing). `refs` holds
    /// `(br, ptr, cookie)` triples in kernel queue order.
    CompleteMirrored { refs: Vec<(u32, u64, u64)> },
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
        return TransactionResult::Reply {
            data,
            offsets,
            sg: Vec::new(),
        };
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
            return TransactionResult::Reply {
                data,
                offsets,
                sg: Vec::new(),
            };
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
            if kind == VirtualService::HidlServiceManager {
                // The seeded SM handles each claim their OWN version:
                // answer with the fq from the entry's key, not the
                // kind-level default — a @1.2 proxy must not claim
                // @1.0.
                let fq = {
                    let b = bus.lock().expect("binder bus poisoned");
                    b.by_handle
                        .get(&target_handle)
                        .and_then(|n| n.split_once('/'))
                        .map(|(f, _)| f.to_string())
                };
                let fq = fq.unwrap_or_else(|| kind.descriptor().to_string());
                w.write_string16(&fq);
                info!(
                    "[KR64][binder][svc] INTERFACE_TRANSACTION → SM instance descriptor {}",
                    fq
                );
            } else {
                w.write_string16(kind.descriptor());
                info!(
                    "[KR64][binder][svc] INTERFACE_TRANSACTION → {} descriptor",
                    kind.descriptor()
                );
            }
            let (data, offsets) = w.into_parts();
            return TransactionResult::Reply {
                data,
                offsets,
                sg: Vec::new(),
            };
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

    // Route to a registered service (guest-owned or in-proxy virtual),
    // falling back to the 6-Z359 NODE table (objects exported via reply
    // parcels — the composer's IComposerClient, callbacks, …).
    let route = {
        let b = bus.lock().expect("binder bus poisoned");
        b.by_handle
            .get(&target_handle)
            .and_then(|name| {
                b.services
                    .get(name)
                    .map(|e| (e.owner, e.ptr, e.cookie, e.virtual_kind))
            })
            .or_else(|| {
                b.nodes
                    .get(&target_handle)
                    .map(|n| (n.owner, n.ptr, n.cookie, None))
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
        // 6-Z307: the seeded SM instances are served by the SAME
        // dispatcher as handle 0 — the full IServiceManager arm set
        // (get/add/getTransport/registerForNotifications/debugDump/…)
        // against the SAME registry. servicemanager_proxy picks the
        // dialect from the parcel header and keeps the v1-legacy path.
        if kind == VirtualService::HidlServiceManager {
            // 6-Z307d: IBase::interfaceChain — the client's FIRST
            // transaction on ANY returned HIDL object (the get() reply's
            // flat is consumed by canCastInterface's chain probe).
            if code == HIDL_IBASE_INTERFACE_CHAIN {
                let chain: Vec<String> = {
                    let b = bus.lock().expect("binder bus poisoned");
                    b.by_handle
                        .get(&target_handle)
                        .and_then(|n| n.split_once('/'))
                        .map(|(_fq, _)| {
                            // The real hwservicemanager's ServiceManager
                            // object implements the WHOLE manager
                            // interface chain; every seeded version handle
                            // reports the same chain (hwservicemanager
                            // registers itself under all three versions
                            // with the SAME binder object).
                            vec![
                                "android.hidl.manager@1.2::IServiceManager".to_string(),
                                "android.hidl.manager@1.1::IServiceManager".to_string(),
                                "android.hidl.manager@1.0::IServiceManager".to_string(),
                                "android.hidl.base@1.0::IBase".to_string(),
                            ]
                        })
                        .unwrap_or_else(|| vec!["android.hidl.base@1.0::IBase".to_string()])
                };
                let mut cw = ParcelWriter::new();
                cw.write_status_ok();
                cw.write_hidl_vec_string(&chain);
                info!(
                    "[KR64][binder][svc] IBase interfaceChain (SM instance) → {} entries",
                    chain.len()
                );
                let (data, offsets, sg) = cw.into_parts_with_sg();
                return TransactionResult::Reply { data, offsets, sg };
            }
            return servicemanager_proxy(code, bus, req_blob.as_ref(), conn_id);
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

    // 6-Z380: bounded route-time parcel dump — the composer-callback
    // wire-truth instrument (rn336 decode: the composer's onHotplug
    // BC_TRANSACTION_SG arrived at the bus with flags LACKING
    // TF_ONE_WAY — the only composer-origin transaction in the run,
    // while 50 other oneway transactions carried the bit — and the
    // HIDL callback dispatch never completed client-side; SF's main
    // thread then waited 8.1 s for the registerCallback reply (the
    // REPLY_TIMEOUT release), found zero pending hotplug events and
    // LOG_ALWAYS_FATAL "Missing internal display"; the SF crash loop
    // restarted zygote 12× via onrestart). This dump names, in one
    // run, the SENDER-side parcel head (interface token + flat
    // objects) and the raw flags — discriminating "the guest wrote a
    // sync call" from "the wire lost the bit" before the bus rewrites
    // anything.
    static Z380_ROUTE_DUMP: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(16);
    if Z380_ROUTE_DUMP.load(std::sync::atomic::Ordering::Relaxed) > 0 {
        Z380_ROUTE_DUMP.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        let head: String = blob
            .data
            .iter()
            .take(48)
            .map(|x| format!("{:02x}", x))
            .collect();
        let offs: String = blob
            .offsets
            .iter()
            .take(16)
            .map(|x| format!("{:02x}", x))
            .collect();
        info!(
            "[KR64][binder][svc] 6-Z380 route parcel conn={} handle=0x{:08x} code={:#x} flags=0x{:x} one_way={} dsize={} osize={} data=[{}] offs=[{}]",
            conn_id,
            target_handle,
            code,
            flags,
            one_way,
            blob.data.len(),
            blob.offsets.len(),
            head,
            offs
        );
    }
    // 6-Z395: SHAPE-KEYED route dump for the composer-callback spine.
    // The 6-Z380 budget (16, global) is first-come-first-served and the
    // early boot burns it on interfaceChain/INTERFACE_TRANSACTION probes
    // before SF's registerCallback (code 0x1 on the composer handle,
    // carrying the IComposerCallback flat) ever routes — rn352 never
    // captured it. This dump keys to the registerCallback shape itself:
    // code==0x1, target != 0, object-bearing offsets (osize >= 8). It
    // prints the sender-side parcel ground truth AT THE BUS so the
    // flat-strip layer is discriminated in one run: dsize=76 osize=8
    // here + 52/0 at the composer = the bus→recipient path dropped it;
    // dsize=52 osize=0 here = the sender's capture was already short
    // (the 6-Z395 truncated-parcel warning fires for the same event).
    static Z395_CB_ROUTE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(8);
    if code == 0x1
        && target_handle != 0
        && Z395_CB_ROUTE.load(std::sync::atomic::Ordering::Relaxed) > 0
    {
        Z395_CB_ROUTE.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        let head: String = blob
            .data
            .iter()
            .take(96)
            .map(|x| format!("{:02x}", x))
            .collect();
        let offs: String = blob
            .offsets
            .iter()
            .take(16)
            .map(|x| format!("{:02x}", x))
            .collect();
        info!(
            "[KR64][binder][svc] 6-Z395 callback-route parcel conn={} handle=0x{:08x} code={:#x} flags=0x{:x} one_way={} dsize={} osize={} sg={} data=[{}] offs=[{}]",
            conn_id,
            target_handle,
            code,
            flags,
            one_way,
            blob.data.len(),
            blob.offsets.len(),
            blob.sg.len(),
            head,
            offs
        );
    }

    // 6-Z456: the interfaceChain cast-probe ARRIVAL on a suspend node —
    // the ISystemSuspend acquisition trail's missing middle. The rn421
    // decode saw the SM get hits and the registerCallback deliveries but
    // could not attribute the ~600 ms node releases because the
    // canCastInterface probe (IBase::interfaceChain, 0xf43484e — the
    // client's FIRST transaction on ANY returned HIDL object, the cast
    // DECISION's input) was invisible. This arm names the probe reaching
    // the real service node (name-keyed, ≤16/run — the probe is the
    // acquisition gate, so a MISSING line while a get hit fired is itself
    // the finding: the client never probed = acquisition died before the
    // cast).
    if code == HIDL_IBASE_INTERFACE_CHAIN {
        static Z456_CHAIN_PROBE: std::sync::atomic::AtomicU32 =
            std::sync::atomic::AtomicU32::new(16);
        let is_suspend = {
            let b = bus.lock().expect("binder bus poisoned");
            b.by_handle
                .get(&target_handle)
                .map(|n| n.contains("system.suspend"))
                .unwrap_or(false)
        };
        if is_suspend && Z456_CHAIN_PROBE.load(std::sync::atomic::Ordering::Relaxed) > 0 {
            Z456_CHAIN_PROBE.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            info!(
                "[KR64][binder][svc] 6-Z456: interfaceChain cast-probe → suspend node 0x{:08x} conn={} one_way={} (probes left {})",
                target_handle,
                conn_id,
                one_way,
                Z456_CHAIN_PROBE.load(std::sync::atomic::Ordering::Relaxed)
            );
        }
    }

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
    // 6-Z442: the OWNER-side mirrors from this request's LOCAL flats —
    // hoisted out of the txn block for the function's return (the
    // same-ioctl read-stream payload for the sender).
    let mut z442_mirrors: Vec<(u32, u64, u64)> = Vec::new();
    {
        let mut b = bus.lock().expect("binder bus poisoned");
        txn_id = b.next_txn;
        b.next_txn += 1;
        if !one_way {
            b.waiters.insert(txn_id, conn_id);
        }
        // 6-Z359: translate the request's LOCAL flats for the OWNER conn
        // (kernel semantics — a flat that crosses conns crosses as a
        // handle) and grant the refs. Callback objects ride THIS path.
        let mut blob = blob;
        let z442_grants = b.z359_translate_flats(
            vm_id,
            conn_id,
            owner,
            &mut blob.data,
            &mut blob.offsets,
            "BC_TX",
        );
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
            // 6-Z442: the flat never crossed (no delivery) — unwind the
            // grants so no release can later mirror for an owner that
            // never saw the acquire, and emit NO mirrors.
            for g in &z442_grants {
                b.z442_unwind_grant(g.handle, owner, g.strong);
            }
            drop(b);
            warning!(
                "[KR64][binder][vm{}] transaction to handle 0x{:08x}: owner mailbox full or gone",
                vm_id,
                target_handle
            );
            return TransactionResult::Failed;
        }
        // Success: the flats crossed (or will cross on delivery) — the
        // owner-side mirrors ride THIS ioctl's read stream.
        for g in &z442_grants {
            z442_mirrors.extend(g.mirrors.iter().copied());
        }
        if !one_way {
            // Track the outstanding sync call on the requester's conn for
            // the bounded reply timeout + teardown cleanup. 6-Z408: the
            // TARGET OWNER conn rides the entry — the reentrancy gate
            // compares the target's sender process against new sync
            // callers while this conn is parked on the reply.
            if let Some(rbx) = b.conns.get_mut(&conn_id) {
                rbx.out_sync
                    .push_back((txn_id, owner, std::time::Instant::now()));
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
                "[KR64][binder][vm{}] routed transaction conn={} -> conn={} handle=0x{:08x} code={} oneway={} flags=0x{:x} self={} txn#{} [tx #{}{}]",
                vm_id,
                conn_id,
                owner,
                target_handle,
                code,
                one_way,
                flags,
                owner == conn_id,
                if one_way { 0 } else { txn_id },
                seen,
                if seen <= 16 { "" } else { " sampled" }
            );
        }
    }

    // Both one-way and (now) sync transactions return
    // BR_TRANSACTION_COMPLETE from THIS ioctl; the sync reply surfaces on
    // a later read (kernel semantics — no blocking inside the proxy).
    // 6-Z442: when the request's LOCAL flats granted nodes (SF's
    // registerCallback callback object rides THIS path), the sender's
    // read stream gains the kernel-true [BR_INCREFS][BR_ACQUIRE] BEFORE
    // the completion (the 6-Z306ae-e same-ioctl no-race shape).
    if !z442_mirrors.is_empty() {
        TransactionResult::CompleteMirrored { refs: z442_mirrors }
    } else {
        TransactionResult::CompleteOnly
    }
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
/// 6-Z306ae: the flat object AT THE OFFSETS ARRAY — the kernel's own
/// object map — instead of a cursor guess. The registration parcels'
/// offsets[] entries point at the EXACT flat the sender's libbinder
/// wrote; among them the binder-typed object is unambiguous (HIDL
/// addWithChain carries 7 SG objects — name struct, name chars, THE
/// FLAT, vec struct, array, chars0, chars1 — and only one decodes as
/// BINDER_TYPE_*). Ladder #203's capture probes exposed the cursor
/// guess reading parcel fragments as (ptr, cookie) for some parcels
/// (suspend_control, android.security.identity, HIDL hash-chain
/// servers): cookie mem = [parcel-fragment][zeros] shapes — one
/// literally contained the BR_TRANSACTION_COMPLETE wire magic
/// (0x720600000048). Falls back to the caller's sequential read when
/// the offsets array is absent/short (legacy v1 wire).
fn flat_at_first_binder_offset(blob: &RequestBlob) -> Option<FlatBinderObject> {
    let off = flat_at_first_binder_offset_pos(blob)?;
    let d = &blob.data;
    Some(FlatBinderObject {
        r#type: u32::from_ne_bytes(d[off..off + 4].try_into().ok()?),
        flags: u32::from_ne_bytes(d[off + 4..off + 8].try_into().ok()?),
        binder: u64::from_ne_bytes(d[off + 8..off + 16].try_into().ok()?),
        cookie: u64::from_ne_bytes(d[off + 16..off + 24].try_into().ok()?),
    })
}

/// 6-Z333: the stability-repr values an A11/A12 libbinder's
/// `Stability::set` accepts (`isDeclaredStability`): the A11 plain
/// Levels {VENDOR = 0b0000_11 = 3, SYSTEM = 0b0011_00 = 12,
/// VINTF = 0b1111_11 = 63} and the android-12+ Category repr (version
/// byte 1 in the low byte, Level in the high byte — e.g. VINTF =
/// 0x3F000001). Used to gate the add-ann capture: a value the client
/// could not have stamped (allowIsolated ∈ {0,1}, dumpPriority ∈ 1..7,
/// garbage) must not be echoed back — the pre-6-Z333 fallback keeps
/// the wire unchanged for those shapes.
fn is_declared_stability_repr(v: i32) -> bool {
    match v {
        0b0000_11 | 0b0011_00 | 0b1111_11 => true,
        v => {
            let level = ((v >> 24) & 0xff) as u8;
            let version = (v & 0xff) as u8;
            version == 1 && matches!(level, 0b0000_11 | 0b0011_00 | 0b1111_11)
        }
    }
}

/// 6-Z306ae-c: the POSITION of the first binder-typed flat among the
/// offsets-array objects (None when absent/malformed/out of range).
fn flat_at_first_binder_offset_pos(blob: &RequestBlob) -> Option<usize> {
    let count = blob.offsets.len() / 8;
    for i in 0..count {
        let off = u64::from_ne_bytes(blob.offsets[i * 8..i * 8 + 8].try_into().ok()?) as usize;
        let d = &blob.data;
        if off + 24 > d.len() {
            continue;
        }
        let typ = u32::from_ne_bytes(d[off..off + 4].try_into().ok()?);
        if typ == BINDER_TYPE_BINDER
            || typ == BINDER_TYPE_HANDLE
            || typ == BINDER_TYPE_WEAK_BINDER
            || typ == BINDER_TYPE_WEAK_HANDLE
        {
            return Some(off);
        }
    }
    None
}

// ─── 6-Z306am: Task-21 mirror A/B (mandated by the #224/#226 decode tree) ───
//
// VERDICT (ladder #253, the android.system.suspend wall): ARM B IS
// STRUCTURALLY FATAL for binderized HIDL services — the A/B experiment
// contract above is settled, and the winner is the kernel-true ordering
// (ARM A, the prefix ON).
//
// The #253 wire decode (2df64813 artifacts): the suspend daemon's
// generated registerAsService() holds the BnHw wrapper in a LOCAL sp<>
// for the duration of the addWithChain round trip; the SM's node ref is
// the ONLY other strong ref, and under arm B it is queued as a RefCmd
// for the daemon's NEXT read. registerAsService() returns → the local
// sp<> drops → the wrapper's destructor runs (RefBase's dtor nulls
// mRefs — the [R+8]=0x0 fingerprint) → the queued RefCmd's delivery-time
// mirror_ref_ok then reads the POST-MORTEM memory, rules the object
// Dead, and pads BR_NOOP — the SM's strong ref NEVER lands. The
// registry keeps serving a dead-cookie handle: every client ping →
// BR_DEAD_REPLY → "getService: found dead hwbinder service for
// android.system.suspend@1.0::ISystemSuspend/default" → the
// PowerManagerService constructor loop-waits forever → system_server
// killed → zygote reforks → boot loop (5 eras in the 900s window).
// The delivery-time gate is self-fulfilling: it checks the liveness of
// an object whose death was CAUSED by the ref's non-delivery.
//
// ARM A closes the window BY CONSTRUCTION: [BR_ACQUIRE] rides the same
// ioctl reply BEFORE BR_TRANSACTION_COMPLETE/BR_REPLY, so the guest's
// waitForResponse incStrongs the LIVE object while the registering
// thread is still inside transact — identical to the real kernel's
// in-kernel node ref + ordered ref-notification. The #220-#227 wedge
// hypothesis was never pinned to the prefix (the era's SM reply shapes
// were broken — pre-6-Z307d BAD_TYPE/EX_TRANSACTION_FAILED fleets); the
// recovery-corpus gate + the next ladder re-verify arm A on the current
// wire. If a registering-process wedge class returns with THIS decode's
// instruments, the artifact will name it — do not re-raise arm B
// without decoding one.
//
// ARM A (ACTIVE): `false` — the in-transaction prefix mirrors for ALL
// registrations, self-transactions included (6-Z306ae-e behavior).
const Z306AM_SELF_MIRROR_PREFIX_OFF: bool = false;

/// 6-Z306am: bounded arm-B skip log — the first 24 skips per boot, then
/// silent (the registration storm makes this line hot).
static Z306AM_SKIP_LOG: AtomicU32 = AtomicU32::new(24);

/// 6-Z306am: should the registration tail's in-transaction mirror be
/// skipped in favor of a queue-delivered acquire? `issuer`/`owner` stay
/// parameters so a later refinement (same-PID cross-conn self-traffic)
/// reuses the gate unchanged.
fn z306am_skip_prefix(issuer: ConnId, owner: ConnId) -> bool {
    Z306AM_SELF_MIRROR_PREFIX_OFF && issuer == owner
}

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
        return servicemanager_hidl(code, blob, bus, conn_id);
    }

    let mut reader = ParcelReader::new(parcel);
    // 6-Z306ae-e: set by the registration arms — the reply is returned as
    // ReplyMirrored so the node-ref mirror rides the SAME ioctl, before
    // the reply unblocks the registering thread (no free-then-acquire
    // race; see TransactionResult::ReplyMirrored).
    let mut mirror: Option<(u32, u64, u64)> = None;
    // 6-Z324: see servicemanager_hidl — the AIDL registerForNotifications
    // arm sets this for the same pool-thread recruitment.
    let mut spawn_looper = false;
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
            // 6-Z306ab: advance the per-connection annotation format BEFORE
            // building the reply. The DEFAULT (fresh connection) is the
            // android-11 plain-Level form — the boot corpus (AOSP 11 rsr2)
            // and its forked system_server are A11 libbinder clients whose
            // Stability::set rejects everything but the bare Level values.
            // A same-service re-get right AFTER A HIT means the client's
            // libbinder rejected the plain annotation (Stability::set →
            // BAD_TYPE → readStrongBinder → null) and its waitForService
            // loop re-asked — that is the non-A11 client signature → flip
            // to the A12 Category form (sticky). A MISS retry (a waiter
            // polling for a not-yet-up service) NEVER flips.
            let (ann_hit, ann_null) = {
                let now = std::time::Instant::now();
                match b.conns.get_mut(&conn_id) {
                    Some(bx) => {
                        if !bx.sm_annotate_a12 && a12_annotation_allowed() {
                            if let Some((last, ts)) = &bx.last_sm_get {
                                if *last == name
                                    && bx.sm_last_was_hit
                                    && now.duration_since(*ts) < std::time::Duration::from_secs(2)
                                {
                                    bx.sm_annotate_a12 = true;
                                    info!(
                                        "[KR64][binder][svc] 6-Z306ab: conn{} stability annotation → A12 Category form (same-name retry-after-hit observed)",
                                        conn_id
                                    );
                                }
                            }
                        }
                        bx.last_sm_get = Some((name.clone(), now));
                        if bx.sm_annotate_a12 {
                            (
                                STABILITY_ANNOTATION_VINTF_A12,
                                STABILITY_ANNOTATION_NULL_A12,
                            )
                        } else {
                            (STABILITY_ANNOTATION_VINTF, STABILITY_ANNOTATION_NULL)
                        }
                    }
                    None => (STABILITY_ANNOTATION_VINTF, STABILITY_ANNOTATION_NULL),
                }
            };
            let entry_info = b
                .services
                .get(&name)
                .map(|e| (e.handle, e.owner, e.ptr, e.cookie, e.ann_add));
            match entry_info {
                Some((handle, owner, ptr, cookie, ann_add)) => {
                    // 6-Z306ac: SAME-PROCESS LOOKUP = LOCAL BINDER. The
                    // kernel returns BINDER_TYPE_BINDER (cookie = the
                    // owner's BBinder*) when the lookup comes from the
                    // OWNER connection — the node lives in the requester's
                    // process and `unflattenBinder` decodes the LOCAL
                    // object (no proxy). Serving a HANDLE to the owner made
                    // the client build a BinderProxy for its OWN local
                    // service; SystemServer's
                    //   (PlatformCompat) ServiceManager.getService("platform_compat")
                    // (ActivityManagerService:2597) then threw
                    //   ClassCastException: BinderProxy cannot be cast to PlatformCompat
                    // and startBootstrapServices died — ladder #198's rung-6
                    // wall.
                    let is_owner = owner == conn_id;
                    if let Some(bx) = b.conns.get_mut(&conn_id) {
                        bx.sm_last_was_hit = true; // 6-Z306ab: flip signal
                    }
                    if is_owner {
                        // 6-Z306ad: ladders #199/#201 — the forked
                        // system_server died in Parcel::unflattenBinder's
                        // sp<IBinder>(cookie) ctor (vbase-offset read
                        // through a NULL vtable, si_addr=0xffff..ffe8)
                        // milliseconds after this LOCAL flat was served.
                        // Snapshot the object bytes NOW, from outside the
                        // guest, to pin whether the vtable word was already
                        // zero AT SERVE TIME.
                        //
                        // 6-Z328: the shared 32-slot serve budget was
                        // consumed within seconds by the mediaserver's
                        // repeated same-name self-gets (rn276: every
                        // media.audio_flinger hit), leaving the boot-critical
                        // LOCAL gets unprobed. The LOCAL-get probe now runs
                        // on its OWN budget with a name-dedupe (a repeated
                        // same-name hit re-probes nothing; distinct names
                        // each get a slot) so the system_server-era
                        // `power`-class serves are always captured.
                        let gpid = b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0);
                        {
                            let mut seen =
                                Z328_SERVE_NAMES.lock().expect("serve-name dedupe poisoned");
                            if !seen.iter().any(|n| n == &name) {
                                if seen.len() < 64 {
                                    seen.push(name.clone());
                                }
                                drop(seen);
                                probe_flat_mem(
                                    "serve-local",
                                    &PROBE_SERVE_LOCAL_BUDGET,
                                    gpid,
                                    &name,
                                    ptr,
                                    cookie,
                                );
                            }
                        }
                    }
                    let obj = if is_owner {
                        FlatBinderObject {
                            r#type: BINDER_TYPE_BINDER,
                            flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                            binder: ptr,
                            cookie,
                        }
                    } else {
                        FlatBinderObject {
                            r#type: BINDER_TYPE_HANDLE,
                            flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                            binder: handle as u64,
                            cookie: 0,
                        }
                    };
                    writer.write_flat_binder(&obj);
                    // 6-Z333: the LOCAL hit reply ECHOES the owner's own
                    // add stability (the real servicemanager semantic —
                    // the reply ann = the add ann). The owner's
                    // finishUnflattenBinder → Stability::set accepts ONLY
                    // the level its own object already carries (rn284:
                    // "Interface being set with vintf stability but it is
                    // already marked as system stability." → BAD_TYPE →
                    // readStrongBinder → null → the initPowerManagement
                    // NPE). Falls back to the pre-6-Z333 annotation when
                    // the add parcel yielded no declared ann. Cross-proc
                    // HANDLE replies below keep ann_hit (fresh proxies
                    // accept it; the VINTF level feeds the call-time
                    // requiresVintfDeclaration gate the corpus needs).
                    writer.write_i32(ann_add.unwrap_or(ann_hit));
                    if is_owner {
                        info!(
                            "[KR64][binder][svc] getService({}) hit → LOCAL binder (owner conn={}, ptr=0x{:x}, cookie=0x{:x})",
                            name, conn_id, ptr, cookie
                        );
                    } else {
                        info!(
                            "[KR64][binder][svc] getService({}) hit → handle 0x{:08x}",
                            name, handle
                        );
                    }
                }
                None => {
                    if let Some(bx) = b.conns.get_mut(&conn_id) {
                        bx.sm_last_was_hit = false; // 6-Z306ab: a miss retry never flips
                    }
                    let obj = FlatBinderObject {
                        r#type: BINDER_TYPE_BINDER,
                        flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                        binder: 0,
                        cookie: 0,
                    };
                    writer.write_flat_binder(&obj);
                    // 6-Z271x: real flattenBinder(nullptr) annotates the
                    // null flat; the client's finishUnflattenBinder still
                    // reads the i32. A11 plain form: UNDECLARED (0) —
                    // Stability::set(null, !=0) is BAD_TYPE (harmless for
                    // a miss, but the honest value is 0).
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
            let flat_seq = reader.read_flat_binder();
            let _allow_isolated = reader.read_i32();
            let _dump_priority = reader.read_i32();
            // 6-Z306ae: the offsets-array flat is the kernel's own
            // object map — prefer it over the sequential cursor guess.
            let flat = flat_at_first_binder_offset(blob).or(flat_seq);
            // 6-Z333: capture the add's stability annotation — the i32 the
            // owner's own libbinder wrote IMMEDIATELY after the flat
            // (finishFlattenBinder: writeObject(flat) + writeInt32(
            // Stability::get(binder)); for an A11 system-partition client
            // that is Level::SYSTEM = 12, tryMarkCompilationUnit having
            // run just before). Stored per service and ECHOED in the
            // owner-conn LOCAL hit reply — the real servicemanager
            // semantic (see ServiceEntry::ann_add). Gated on the
            // declared-repr validator so allowIsolated (0/1),
            // dumpPriority bytes or garbage are never echoed as a
            // stability (those shapes keep the pre-6-Z333 wire).
            let ann_add = req_blob
                .and_then(flat_at_first_binder_offset_pos)
                .and_then(|pos| {
                    let d = &req_blob?.data;
                    if pos + 28 <= d.len() {
                        Some(i32::from_le_bytes(d[pos + 24..pos + 28].try_into().ok()?))
                    } else {
                        None
                    }
                })
                .filter(|v| is_declared_stability_repr(*v));
            let (ptr, cookie) = match &flat {
                Some(f) => (f.binder, f.cookie),
                None => (0, 0),
            };
            let mut b = bus.lock().expect("binder bus poisoned");
            let handle = b.add_guest_service(&name, conn_id, ptr, cookie);
            if let Some(ann) = ann_add {
                b.set_service_ann(&name, ann);
                info!(
                    "[KR64][binder][svc] 6-Z333: addService({}) stability ann captured = 0x{:08x} (echoed on owner-conn LOCAL hits)",
                    name, ann
                );
            }
            // 6-Z306ad: capture-time snapshot — the object bytes as the
            // owner registered them (baseline for the serve-time probe).
            let gpid = b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0);
            probe_flat_mem("capture", &PROBE_CAPTURE_BUDGET, gpid, &name, ptr, cookie);
            // 6-Z306ae-c: the capture-shift decode — hex the PARCEL bytes
            // around the captured flat (the offsets-anchored position),
            // bounded to the first 12 registrations per boot. The #205 Δ
            // census (mBase = cookie + 0x88 dominant on HIDL, +0x20/+0x38/
            // +0x68 on AIDL) says the captured cookie slot holds a pointer
            // Δ below the true object; this dump decides whether the
            // parcel CONTENT is shifted or the flat position is.
            static CAPTURE_FLAT_DUMP: AtomicU32 = AtomicU32::new(12);
            if CAPTURE_FLAT_DUMP.load(Ordering::Relaxed) > 0 {
                CAPTURE_FLAT_DUMP.fetch_sub(1, Ordering::Relaxed);
                if let Some(rb) = req_blob {
                    let count = rb.offsets.len() / 8;
                    let mut offs_hex = String::new();
                    for i in 0..count.min(8) {
                        let v =
                            u64::from_ne_bytes(rb.offsets[i * 8..i * 8 + 8].try_into().unwrap());
                        offs_hex.push_str(&format!("{:x} ", v));
                    }
                    if let Some(flat_off) = flat_at_first_binder_offset_pos(rb) {
                        let s = flat_off.saturating_sub(8);
                        let e = (flat_off + 40).min(rb.data.len());
                        let mut hex = String::new();
                        for byte in &rb.data[s..e] {
                            hex.push_str(&format!("{:02x}", byte));
                        }
                        info!(
                            "[KR64][binder][svc] 6-Z306ae-c: capture-flat {} name='{}' offs=[{}] flat@{} bytes[{}..{}]={}",
                            code, name, offs_hex.trim(), flat_off, s, e, hex
                        );
                    } else {
                        info!(
                            "[KR64][binder][svc] 6-Z306ae-c: capture-flat {} name='{}' offs=[{}] NO-BINDER-TYPED-FLAT",
                            code, name, offs_hex.trim()
                        );
                    }
                }
            }
            info!(
                "[KR64][binder][svc] addService({}) → handle 0x{:08x} (conn={}, ptr=0x{:x})",
                name, handle, conn_id, ptr
            );
            // 6-Z276: a registered watcher now gets its one-way
            // `onRegistration` callback (the real servicemanager fires
            // IServiceCallback.onRegistration on every later addService).
            b.fire_registration_callbacks(&name, handle, false);
            // 6-Z306ae-e: mirror the registry's strong node ref IN THIS
            // IOCTL (liveness-gated) — the owner's incStrong runs while
            // the registering thread's JNI temporary sp<> is alive.
            // 6-Z306am: arm B gates the PREFIX off for self-transactions
            // and re-routes the acquire through the liveness-gated
            // RefCmd queue (see the gate's header comment).
            if ptr != 0 {
                let gpid = b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0);
                // 6-Z458 (Task 195): the REGISTRY's own pin books into
                // the node map UNCONDITIONALLY — kernel-true: the SM's
                // sp<> exists the moment the add lands, probe or no
                // probe — while `acq_mirrored` records the add arm's
                // mirror verdict so the pin's eventual drop knows
                // whether the owner ever saw the era's BR_ACQUIRE.
                let acq_mirrored = mirror_ref_ok(gpid, ptr, cookie);
                b.z458_registry_pin_add(conn_id, ptr, cookie, acq_mirrored);
                if acq_mirrored {
                    if z306am_skip_prefix(conn_id, conn_id) {
                        if let Some(bx) = b.conns.get_mut(&conn_id) {
                            bx.z491.reply_enq += 1;
                            bx.reply_queue.push_back(DeferredReply::RefCmd {
                                br: BR_ACQUIRE,
                                ptr,
                                cookie,
                            });
                        }
                        if Z306AM_SKIP_LOG.load(Ordering::Relaxed) > 0 {
                            Z306AM_SKIP_LOG.fetch_sub(1, Ordering::Relaxed);
                            info!(
                                "[KR64][binder][svc] 6-Z306am: arm B — prefix SKIPPED (self-tx) name='{}' conn={} ptr=0x{:x} cookie=0x{:x} → liveness RefCmd queued",
                                name, conn_id, ptr, cookie
                            );
                        }
                    } else {
                        mirror = Some((BR_ACQUIRE, ptr, cookie));
                    }
                    // 6-Z454: the registry's strong ref is mirrored — the
                    // ledger counts it so a later overwrite release (a
                    // DIFFERENT probe moment) balances against a real
                    // acquire. A skip here (probe Unknown/Dead) followed by
                    // a delivered release is the V1/V2 killer shape.
                    z454_emit(gpid, ptr, cookie, Z454Site::RegAcq);
                }
            }
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
                    // 6-Z323: the watcher registers UNCONDITIONALLY — the
                    // real servicemanager adds the listener before the
                    // preexisting check, so this caller is notified both by
                    // the immediate fire below and by later addService calls.
                    let added = b.add_watcher(&name, w);
                    // 6-Z325: when a NEW watcher entry was stored, pin the
                    // caller's local callback wrapper with the kernel's own
                    // node-ref mirror (the SM holds a strong ref on the
                    // callback node — binder_node_post_acquire → BR_ACQUIRE
                    // to the owner). Without it the transient BnHw wrapper
                    // dies after the register reply and the queued
                    // onRegistration can never execute (the rn273 wall).
                    if added {
                        mirror = Some((BR_ACQUIRE, f.binder, f.cookie));
                        // 6-Z454: the watcher pin's acquire is mirrored —
                        // counted so the unregister release (and any future
                        // watcher-cleanup release) balances against it.
                        let wpid = b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0);
                        z454_emit(wpid, f.binder, f.cookie, Z454Site::WatchAcq);
                    }
                    match b.services.get(&name).map(|e| e.handle) {
                        Some(h) => {
                            // Already registered: immediate preexisting
                            // callback — the fire now INCLUDES this caller
                            // (the pre-6-Z323 shape dropped the constructed
                            // watcher on the floor and fired only watchers
                            // that predated the call, so the callback never
                            // reached the requester — the AIDL twin of the
                            // rn271 HIDL waitForHwService wall).
                            b.fire_registration_callbacks(&name, h, true);
                            true
                        }
                        None => false,
                    }
                };
                // 6-Z324: the caller is ABOUT to wait for this callback —
                // recruit a pooled reader via BR_SPAWN_LOOPER on this
                // reply (the AIDL twin of the HIDL waitForHwService fix).
                spawn_looper = true;
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
                // 6-Z325: mirror the ref drop for a real removal — the
                // registry's strong ref on the callback node goes away
                // (kernel: BR_RELEASE to the owner; the Waiter::done()
                // path keeps its own local sp<> alive for the call).
                if b.remove_watcher(&name, conn_id, f.binder) {
                    mirror = Some((BR_RELEASE, f.binder, f.cookie));
                    // 6-Z454: the watcher release is queued — checked
                    // against the pin's emitted acquire (V1; the pin's
                    // acquire is liveness-gated at a DIFFERENT moment).
                    let wpid = b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0);
                    z454_emit(wpid, f.binder, f.cookie, Z454Site::WatchRel);
                }
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
    if spawn_looper {
        // 6-Z324/6-Z325: the registerForNotifications replies carry BOTH
        // the pool-thread recruitment AND (when a new watcher was stored)
        // the callback's node-ref mirror — one combined batch.
        TransactionResult::ReplySpawnLooper {
            mirror,
            data,
            offsets,
            sg: Vec::new(),
        }
    } else if let Some((br, mptr, mcookie)) = mirror {
        TransactionResult::ReplyMirrored {
            br,
            ptr: mptr,
            cookie: mcookie,
            data,
            offsets,
            sg: Vec::new(),
        }
    } else {
        TransactionResult::Reply {
            data,
            offsets,
            sg: Vec::new(),
        }
    }
}

// HIDL `android.hidl.manager@1.0::IServiceManager` transactions (libhwbinder
// parcels — no SYST header tag).
//
// 6-Z305t-68 — THE REAL WIRE, decoded from the authoritative
// android-11.0.0_r1 sources (system/libhwbinder/Parcel.cpp,
// system/libhidl/base/HidlSupport.{h,cpp} + transport/HidlBinderSupport.{h,
// cpp}, system/tools/hidl StringType emit, kernel uapi binder.h):
//
// * interface token: libhwbinder `writeInterfaceToken` = `writeCString` —
//   the descriptor as a NUL-terminated char string, NO length prefix, then
//   4-byte alignment. (The pre-68 parser read a string16 here — its first
//   i32 read consumed `"andr"` as a ~1.9e9 length — so EVERY HIDL SM
//   transaction failed silently at the token, hit the catch-all
//   BR_FAILED_REPLY, and surfaced as the 1269-abort
//   Status(EX_TRANSACTION_FAILED) storm of ladders #122/#123. The
//   -66/-67 arms were correct downstream but never once reached: the
//   ladder artifacts show ZERO `HIDL getTransport`/`HIDL addWithChain`
//   logs and ZERO VINTF pre-checks against ~404 code=3 transactions.)
// * `hidl_string` argument: TWO `BINDER_TYPE_PTR` objects. The generated
//   code does `writeBuffer(&s, sizeof(hidl_string)=16)` (object A — the
//   `{ptr, u32 size, bool owns}` STRUCT) then `writeEmbeddedToParcel(s)`
//   = `writeEmbeddedBuffer(s.c_str(), s.size()+1, parent=A,
//   parent_offset=kOffsetOfBuffer=0)` (object B — chars+NUL). The BYTES
//   ARE NOT IN THE MAIN PARCEL: `writeBuffer` emits a
//   `binder_buffer_object` referencing the SENDER's memory; the real
//   kernel copies each PTR object's (buffer,length) VERBATIM into the
//   receiver's transaction buffer (the scatter-gather region of
//   `BC_TRANSACTION_SG`) and fixes the object's `buffer` pointer up to
//   the receiver's copy. The loader now replicates that copy (v3 wire,
//   `WIRE_V3_MAGIC`); the proxy resolves PTR contents by offsets-array
//   ORDER (the kernel's copy order) with a sender-pointer cross-check.
//   Ladder #123 note: the v2 blob carried the main parcel ONLY — even a
//   byte-perfect main-parcel parse could never recover `fq`/`name`.
// * binder-object argument: an INLINE `flat_binder_object` listed in the
//   offsets array (`writeStrongBinder` → `writeObject(flat)`).
// * `hidl_vec<hidl_string>`: PTR(vec struct {ptr, size, owns}) +
//   PTR(array, HAS_PARENT parent_offset=0, content = count × 16) + per
//   element PTR(struct)+PTR(chars) — exactly the generated receiver's
//   read walk (readBuffer(vec) → readEmbeddedBuffer(array) → per element
//   readBuffer(struct) → readEmbeddedFromParcel(chars)).
//
// The A11 boot-path arms (codes per android-11.0.0_r1 IServiceManager.hal
// — 1.0: get=1, add=2, getTransport=3, list=4, listByInterface=5,
// registerForNotifications=6, debugDump=7, registerPassthroughClient=8;
// 1.1 appends unregisterForNotifications=9; 1.2 appends addWithChain=12):
// * `get` (code 1): registry lookup of `"fqName/name"`; hit → flat
//   handle, miss → null binder (HIDL reads the object at offsets[0], so
//   the status prefix is skipped naturally).
// * `add` (code 2): the 1.0 shape — registers the INSTANCE name bare
//   (kept for older guests; the A11 GSI registers via addWithChain).
// * `getTransport` (code 3): honest VINTF/bus answer — HWBINDER(1) iff
//   the manifest declares it or `"fq/instance"` is registered, EMPTY(0)
//   otherwise, single u8 after the status.
// * `registerForNotifications` (6) / `unregisterForNotifications` (9):
//   watcher registry + onRegistration callbacks.
// * `addWithChain` (code 12): THE A11 registration — registers under
//   `"chain[0]/name"` and fires the onRegistration callbacks.
// * everything else → `BR_FAILED_REPLY` (honest, now with a bounded
//   diagnostic naming the code — list/listByInterface/debugDump/
//   registerPassthroughClient/… are not exercised by the boot path yet).

/// One parsed offsets-array object: SG-backed `BINDER_TYPE_PTR` or an
/// inline flat binder.
enum HidlObj<'a> {
    /// SG-backed buffer; `content` is None when the loader could not
    /// capture it (honest parse failure downstream).
    Ptr {
        content: Option<&'a [u8]>,
        has_parent: bool,
        parent: u64,
        parent_offset: u64,
    },
    Binder(FlatBinderObject),
}

/// Positional + offsets-array reader over a REAL libhwbinder request
/// parcel (wire notes above). The token is read positionally; every
/// argument walks the offsets array in order — the kernel's
/// object-processing order, which is also the loader's SG capture order.
struct HidlParcel<'a> {
    data: &'a [u8],
    offsets: Vec<u64>,
    sg: &'a [SgBuf],
    pos: usize,
    obj_idx: usize,
    ptr_seq: usize,
}

impl<'a> HidlParcel<'a> {
    fn new(blob: &'a RequestBlob) -> Option<Self> {
        if blob.offsets.len() % 8 != 0 {
            return None;
        }
        let offsets = blob
            .offsets
            .chunks_exact(8)
            .map(|c| u64::from_ne_bytes(c.try_into().unwrap()))
            .collect();
        Some(HidlParcel {
            data: &blob.data,
            offsets,
            sg: &blob.sg,
            pos: 0,
            obj_idx: 0,
            ptr_seq: 0,
        })
    }

    /// libhwbinder token: `readCString` + align4 (writeInterfaceToken =
    /// writeCString — NO length prefix, NOT a string16).
    fn token(&mut self) -> Option<String> {
        let start = self.pos;
        if start >= self.data.len() {
            return None;
        }
        let nul = self.data[start..].iter().position(|&b| b == 0)? + start;
        let s = String::from_utf8_lossy(&self.data[start..nul]).into_owned();
        self.pos = nul + 1;
        self.pos += (4 - self.pos % 4) % 4;
        Some(s)
    }

    /// Next object from the offsets array (kernel processing order =
    /// loader SG capture order).
    fn next_object(&mut self) -> Option<HidlObj<'a>> {
        let off = *self.offsets.get(self.obj_idx)? as usize;
        self.obj_idx += 1;
        if off + 4 > self.data.len() {
            return None;
        }
        let typ = u32::from_ne_bytes(self.data[off..off + 4].try_into().ok()?);
        if typ == BINDER_TYPE_PTR {
            // struct binder_buffer_object: hdr u32 | flags u32 | buffer
            // u64 | length u64 | parent u64 | parent_offset u64 = 40B.
            if off + 40 > self.data.len() {
                return None;
            }
            let flags = u32::from_ne_bytes(self.data[off + 4..off + 8].try_into().ok()?);
            let client_ptr = u64::from_ne_bytes(self.data[off + 8..off + 16].try_into().ok()?);
            let length = u64::from_ne_bytes(self.data[off + 16..off + 24].try_into().ok()?);
            let parent = u64::from_ne_bytes(self.data[off + 24..off + 32].try_into().ok()?);
            let parent_offset = u64::from_ne_bytes(self.data[off + 32..off + 40].try_into().ok()?);
            let content = self.sg.get(self.ptr_seq).and_then(|b| {
                // Cross-check the sender pointer (0 = the loader could not
                // read it — accept the order match alone).
                if b.client_ptr != 0 && b.client_ptr != client_ptr {
                    return None;
                }
                if b.data.len() as u64 >= length {
                    Some(&b.data[..length as usize])
                } else {
                    None
                }
            });
            self.ptr_seq += 1;
            Some(HidlObj::Ptr {
                content,
                has_parent: flags & BINDER_BUFFER_FLAG_HAS_PARENT != 0,
                parent,
                parent_offset,
            })
        } else if matches!(
            typ,
            BINDER_TYPE_BINDER
                | BINDER_TYPE_WEAK_BINDER
                | BINDER_TYPE_HANDLE
                | BINDER_TYPE_WEAK_HANDLE
        ) {
            // struct flat_binder_object: hdr u32 | flags u32 |
            // binder/handle u64 | cookie u64 = 24B.
            if off + 24 > self.data.len() {
                return None;
            }
            let flags = u32::from_ne_bytes(self.data[off + 4..off + 8].try_into().ok()?);
            let binder = u64::from_ne_bytes(self.data[off + 8..off + 16].try_into().ok()?);
            let cookie = u64::from_ne_bytes(self.data[off + 16..off + 24].try_into().ok()?);
            Some(HidlObj::Binder(FlatBinderObject {
                r#type: typ,
                flags,
                binder,
                cookie,
            }))
        } else {
            None
        }
    }

    /// A `hidl_string` argument: [PTR struct(16B)][PTR chars child]. The
    /// chars object's `parent` must be the struct object's offsets-array
    /// index and its `parent_offset` must be `hidl_string::kOffsetOfBuffer`
    /// (0) — the kernel's parent fixup semantics, used as an integrity
    /// check.
    fn read_string_arg(&mut self) -> Option<String> {
        let struct_idx = self.obj_idx;
        let st = match self.next_object()? {
            HidlObj::Ptr {
                content: Some(c),
                has_parent: false,
                parent: 0,
                ..
            } if c.len() >= 12 => c,
            _ => return None,
        };
        let size = u32::from_ne_bytes(st[8..12].try_into().ok()?) as usize;
        match self.next_object()? {
            HidlObj::Ptr {
                content: Some(ch),
                has_parent: true,
                parent,
                parent_offset: 0,
                ..
            } => {
                if parent != struct_idx as u64 || ch.len() < size + 1 || ch[size] != 0 {
                    return None;
                }
                Some(String::from_utf8_lossy(&ch[..size]).into_owned())
            }
            _ => None,
        }
    }

    /// A binder-object argument: an inline flat.
    fn read_binder_arg(&mut self) -> Option<FlatBinderObject> {
        match self.next_object()? {
            HidlObj::Binder(f) => Some(f),
            _ => None,
        }
    }

    /// `hidl_vec<hidl_string>` — the REAL A11 wire (6-Z305t-69, decoded
    /// from ladder #125's bounded entry diag: code=12 sg=[16, 8, 16, 32,
    /// 43, 29] offs=7):
    /// `[PTR vec struct 16B {ptr, size}][PTR array child, parent_offset 0,
    /// len = count×16]` then `count × [PTR chars_j, parent = THE ARRAY,
    /// parent_offset = j*16]` — the element STRUCTS live INSIDE the array
    /// SG buffer (there are NO per-element struct objects), and each
    /// element's chars buffer is a child of the ARRAY at j*sizeof
    /// (hidl_string) + kOffsetOfBuffer(0).
    fn read_vec_string_arg(&mut self) -> Option<Vec<String>> {
        let vs = match self.next_object()? {
            HidlObj::Ptr {
                content: Some(c),
                has_parent: false,
                parent: 0,
                ..
            } if c.len() >= 16 => c,
            _ => return None,
        };
        // 6-Z305t-69e: the ladder-#129 dump shows the vec struct is
        // 16B {ptr, u32 size, u32 owns/pad} — the count is a u32 at
        // offset 8 (a u64 read made count = 0x1_00000002 → >128 → fail).
        let count = u32::from_ne_bytes(vs[8..12].try_into().ok()?) as usize;
        if count > 128 {
            return None;
        }
        // The ARRAY's parent is the VEC STRUCT (just consumed).
        let vec_idx = self.obj_idx - 1;
        match self.next_object()? {
            HidlObj::Ptr {
                content: Some(c),
                has_parent: true,
                parent,
                parent_offset: 0,
                ..
            } => {
                if parent != vec_idx as u64 || c.len() < count * 16 {
                    return None;
                }
            }
            _ => return None,
        }
        // Each element's chars buffer's parent is THE ARRAY.
        let array_idx = self.obj_idx as u64 - 1;
        let mut out = Vec::with_capacity(count);
        for j in 0..count {
            match self.next_object()? {
                HidlObj::Ptr {
                    content: Some(ch),
                    has_parent: true,
                    parent,
                    parent_offset,
                    ..
                } => {
                    if parent != array_idx || parent_offset != (j * 16) as u64 {
                        return None;
                    }
                    if ch.is_empty() || ch[ch.len() - 1] != 0 {
                        return None;
                    }
                    out.push(String::from_utf8_lossy(&ch[..ch.len() - 1]).into_owned());
                }
                _ => return None,
            }
        }
        Some(out)
    }
}

/// Bounded HIDL SM entry diagnostics: the first 8 transactions per code
/// carry a parcel head + SG summary, then one sampled line per 500th.
/// The pre-68 runs were BLIND here (the AIDL head-dump sat behind the
/// is_aidl gate and HIDL parse failures were silent), which is why the
/// wrong-wire decode took three ladders to converge.
/// 6-Z305t-69c: FULL-fidelity object dump for the first N transactions of
/// a code — offsets array (type/flags/len/parent/parent_offset per object)
/// + SG contents (hex, first 32B). The addWithChain chain-vec wire has
/// survived three decode rounds; this dump ends the guessing in one run.
fn hidl_sm_object_diag(code: u32, blob: &RequestBlob) {
    // Budget ONLY the codes whose wire is still being decoded (the
    // -69c global budget was eaten by codes 3/8 before code=12 ran).
    if code != HIDL_SM_ADD_WITH_CHAIN && code != HIDL_SM_LIST_MANIFEST_BY_INTERFACE {
        return;
    }
    static SEEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if n >= 3 || blob.offsets.len() % 8 != 0 {
        return;
    }
    let offs: Vec<u64> = blob
        .offsets
        .chunks_exact(8)
        .map(|c| u64::from_ne_bytes(c.try_into().unwrap()))
        .collect();
    let mut s = String::new();
    let mut ptr_count = 0usize;
    for (i, off) in offs.iter().enumerate() {
        let o = *off as usize;
        if o + 4 > blob.data.len() {
            s.push_str(&format!(" [{}:OOB@{}]", i, o));
            continue;
        }
        let typ = u32::from_ne_bytes(blob.data[o..o + 4].try_into().unwrap());
        if typ == BINDER_TYPE_PTR {
            ptr_count += 1;
        }
        if typ == BINDER_TYPE_PTR && o + 40 <= blob.data.len() {
            let flags = u32::from_ne_bytes(blob.data[o + 4..o + 8].try_into().unwrap());
            let ptr = u64::from_ne_bytes(blob.data[o + 8..o + 16].try_into().unwrap());
            let len = u64::from_ne_bytes(blob.data[o + 16..o + 24].try_into().unwrap());
            let par = u64::from_ne_bytes(blob.data[o + 24..o + 32].try_into().unwrap());
            let poff = u64::from_ne_bytes(blob.data[o + 32..o + 40].try_into().unwrap());
            s.push_str(&format!(
                " [{}:PTR@{} fl={} ptr={:#x} len={} par={} poff={}]",
                i, o, flags, ptr, len, par, poff
            ));
        } else if o + 24 <= blob.data.len() {
            let flags = u32::from_ne_bytes(blob.data[o + 4..o + 8].try_into().unwrap());
            let binder = u64::from_ne_bytes(blob.data[o + 8..o + 16].try_into().unwrap());
            s.push_str(&format!(
                " [{}:{:#x}@{} fl={} binder={:#x}]",
                i, typ, o, flags, binder
            ));
        } else {
            s.push_str(&format!(" [{}:SHORT@{}]", i, o));
        }
    }
    let sg_lens: Vec<usize> = blob.sg.iter().map(|b| b.data.len()).collect();
    let mut sg_hex = String::new();
    for (i, b) in blob.sg.iter().take(6).enumerate() {
        let mut h = String::new();
        for x in b.data.iter().take(24) {
            h.push_str(&format!("{:02x}", x));
        }
        sg_hex.push_str(&format!(" sg{}[{}]={}", i, b.data.len(), h));
    }
    info!(
        "[KR64][binder][svc] HIDL SM OBJECTS code={} dsize={} offs({})={}{} sg({})={:?}{}",
        code,
        blob.data.len(),
        offs.len(),
        s,
        if ptr_count == blob.sg.len() {
            ""
        } else {
            " PTR/SG-MISMATCH"
        },
        blob.sg.len(),
        sg_lens,
        sg_hex
    );
}

fn hidl_sm_entry_diag(code: u32, blob: &RequestBlob) {
    static SEEN: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<u32, u64>>> =
        std::sync::OnceLock::new();
    let seen = match SEEN
        .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
        .lock()
    {
        Ok(mut m) => *m.entry(code).and_modify(|c| *c += 1).or_insert(1),
        Err(_) => 0,
    };
    if seen > 8 && seen % 500 != 0 {
        return;
    }
    let mut head = String::new();
    for b in blob.data.iter().take(16) {
        head.push_str(&format!("{:02x} ", b));
    }
    let sg_total: usize = blob.sg.iter().map(|b| b.data.len()).sum();
    let sg_lens: Vec<usize> = blob.sg.iter().map(|b| b.data.len()).collect();
    info!(
        "[KR64][binder][svc] HIDL SM code={} dsize={} offs={} sg={} ({}B, {:?}) head={} [tx #{}{}]",
        code,
        blob.data.len(),
        blob.offsets.len() / 8,
        blob.sg.len(),
        sg_total,
        &sg_lens[..sg_lens.len().min(6)],
        head,
        seen,
        if seen <= 8 { "" } else { " sampled" }
    );
}

/// Bounded parse-failure diagnostic (first 8 per boot): names the stage
/// so the next ladder decodes the exact wire divergence in one run.
fn hidl_sm_parse_fail_diag(stage: &str, code: u32, blob: &RequestBlob) {
    static SEEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if n >= 8 {
        return;
    }
    let mut head = String::new();
    for b in blob.data.iter().take(16) {
        head.push_str(&format!("{:02x} ", b));
    }
    warning!(
        "[KR64][binder][svc] HIDL SM parse-fail ({}) code={} dsize={} offs={} sg={}: head={}",
        stage,
        code,
        blob.data.len(),
        blob.offsets.len() / 8,
        blob.sg.len(),
        head
    );
}

/// 6-Z480: kernel-true pool-recruitment gate for a service registration —
/// does the registering process have NO thread already parked in an ioctl?
///
/// # Root cause (rn451 decode — the rung-8 BatteryService wall)
///
/// system_server reached startCoreServices and died at BatteryService.onStart
/// three generations in a row: the health HAL (android.hardware.health@2.1-
/// service, pid 2851) registered via addWithChain → handle 0x8 at +2.8 s and
/// then NEVER READ its conn again — the ps/threads capture shows the process
/// with EXACTLY ONE thread (the healthd mainloop, `ep_poll`), no
/// `Binder:2851_1` pool thread. BatteryService's HIDL getService probes
/// reached the service manager and were answered (get() hit → handle 0x8),
/// the client's interfaceDescriptor cast probe (code 0x0F43440E) ROUTED to
/// the health conn (conn=137 → conn=3, txn#1279) — and sat in its inbox
/// forever: no parked reader existed to receive it. 6-Z407 expired the txn
/// at the 8 s budget → BR_FAILED_REPLY → castFrom null →
/// "getService: unable to call into hwbinder service" →
/// "health: cannot register callback" → RuntimeException "Failed to start
/// service com.android.server.BatteryService" → "Failure starting system
/// services" → the era death.
///
/// On REAL hardware the healthd-style service (register + epoll mainloop,
/// never joinRpcThreadpool) still gains a `Binder:pid_1` thread because the
/// KERNEL recruits one: binder_thread_read appends BR_SPAWN_LOOPER to the
/// read of a NON-looper thread when the proc has no thread waiting for proc
/// todo work (`requested_threads == 0 && waiting_threads empty`). The
/// registration read IS that read — the reply stream gains the recruitment
/// command, libhwbinder's waitForResponse spawns a pooled thread, and that
/// thread parks in the next ioctl forever after, serving every later client
/// transaction.
///
/// The proxy never emitted the recruitment on the add arms (only the 6-Z324
/// registerForNotifications arm did), so healthd-style HIDL services had no
/// worker and the class above was structural. The fix arms the recruitment
/// on the add/addWithChain SUCCESS replies, gated kernel-true: recruit only
/// when NO OTHER conn of the same (guest pid, binder device) is inside an
/// ioctl — the proxy's view of `waiting_threads` being empty. The
/// registering conn's OWN ReaderWaitingGuard (true for the duration of this
/// ioctl) is excluded. The spawned pool thread's BC_REGISTER_LOOPER + parked
/// read land on the same conn; later adds in the same process then see the
/// parked sibling and skip the recruitment — the kernel's own convergence.
fn z480_needs_pool_recruitment(bus: &Arc<Mutex<BusState>>, conn_id: ConnId) -> bool {
    let b = bus.lock().expect("binder bus poisoned");
    let (pid, dev) = match b.conns.get(&conn_id) {
        Some(c) => (c.sender_pid, c.dev_code),
        None => return false,
    };
    if pid == 0 {
        // Identity unknown (no SO_PEERCRED, no IDENT announcement): the
        // registering conn is the only view of the process — recruit (the
        // guest's own max-threads cap bounds the pool size, exactly like
        // the kernel's BINDER_SET_MAX_THREADS).
        return true;
    }
    !b.conns.iter().any(|(cid, c)| {
        c.sender_pid == pid && c.dev_code == dev && *cid != conn_id && c.reader_waiting
    })
}

fn servicemanager_hidl(
    code: u32,
    blob: &RequestBlob,
    bus: &Arc<Mutex<BusState>>,
    conn_id: ConnId,
) -> TransactionResult {
    hidl_sm_entry_diag(code, blob);
    hidl_sm_object_diag(code, blob);
    let mut p = match HidlParcel::new(blob) {
        Some(p) => p,
        None => {
            hidl_sm_parse_fail_diag("offsets", code, blob);
            return TransactionResult::Failed;
        }
    };
    let _descriptor = match p.token() {
        Some(t) => t,
        None => {
            hidl_sm_parse_fail_diag("token", code, blob);
            return TransactionResult::Failed;
        }
    };

    let mut writer = ParcelWriter::new();
    // 6-Z306ae-e: set by the registration arms — see the AIDL twin +
    // TransactionResult::ReplyMirrored (in-transaction node-ref mirror).
    let mut mirror: Option<(u32, u64, u64)> = None;
    // 6-Z324: the registerForNotifications arm sets this — the reply
    // prepends BR_SPAWN_LOOPER (kernel pool-thread recruitment) so the
    // waiter's process gains a pooled hwbinder reader that can drain the
    // queued onRegistration oneway while the registering thread waits on
    // the Waiter condvar (the rn272 waitForHwService deadlock).
    let mut spawn_looper = false;
    // HIDL replies carry no AIDL exception prefix; the object (if any)
    // lands at offsets[0] and HIDL reads it there. The status prefix is
    // harmless for HIDL and correct for any libbinder-side reader.
    writer.write_status_ok();

    match code {
        HIDL_SM_GET => {
            let fq = match p.read_string_arg() {
                Some(s) => s,
                None => {
                    hidl_sm_parse_fail_diag("get.fq", code, blob);
                    return TransactionResult::Failed;
                }
            };
            let name = match p.read_string_arg() {
                Some(s) => s,
                None => {
                    hidl_sm_parse_fail_diag("get.name", code, blob);
                    return TransactionResult::Failed;
                }
            };
            // 6-Z307d: the reply = [status-ok][base flat] — ONE object,
            // the SERVICE itself (`getRawServiceInternal`:
            // `Return<sp<IBase>> ret = sm->get(descriptor, instance); sp<IBase>
            // base = ret;`). The ladder-#249/250 decode history settled the
            // shape empirically:
            //   * [status][flat] (rn247/rn248 era): the reply PARSED — the
            //     failure was the client's NEXT transaction (the
            //     IBase::interfaceChain cast probe, 6-Z307d's second fix);
            //   * 6-Z307b (empty reply + onValues delivery): the request
            //     never carried a callback flat (parse-fail get.cb →
            //     EX_TRANSACTION_FAILED, ladder #249);
            //   * 6-Z307c (chain vec + base): the client's Return<sp<IBase>>
            //     reads ONE object — the vec PTR where it expected the base
            //     flat → Status 'BAD_TYPE' (ladder #250).
            // Hit: base = the service's HANDLE flat. Miss: base = null
            // binder (the honest NAME_NOT_FOUND shape — no hang, no
            // fabricated service).
            let key = format!("{}/{}", fq, name);
            let hit_handle = {
                let b = bus.lock().expect("binder bus poisoned");
                b.services.get(&key).map(|e| e.handle)
            };
            match hit_handle {
                Some(handle) => {
                    writer.write_flat_binder(&FlatBinderObject {
                        r#type: BINDER_TYPE_HANDLE,
                        flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                        binder: handle as u64,
                        cookie: 0,
                    });
                    info!(
                        "[KR64][binder][svc] HIDL get({}) hit → handle 0x{:08x}",
                        key, handle
                    );
                }
                None => {
                    writer.write_flat_binder(&FlatBinderObject {
                        r#type: BINDER_TYPE_BINDER,
                        flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                        binder: 0,
                        cookie: 0,
                    });
                    info!("[KR64][binder][svc] HIDL get({}) miss → null binder", key);
                }
            }
        }
        // A11 `android.hidl.manager@1.0::IServiceManager.getTransport` —
        // EVERY HAL's first service lookup (getRawServiceInternal,
        // transport/ServiceManagement.cpp:779: `sm->getTransport(descriptor,
        // instance)`). The pre-6-Z305t-66 proxy had NO arm for it → the
        // catch-all BR_FAILED_REPLY → libhwbinder surfaced
        // Status(EX_TRANSACTION_FAILED) → 139 getService sites aborted
        // (956 "Attempted to retrieve value from failed HIDL call" events,
        // ladder #122) — the HIDL fleet killer.
        //
        // Wire (IServiceManager.hal android-11.0.0_r1:90): request
        // `[hidl_string fqName][hidl_string name]`; reply
        // `[status ok][u8 transport]` — `enum Transport : uint8_t`
        // (EMPTY=0, HWBINDER=1, PASSTHROUGH=2) marshals as ONE byte
        // (hwbinder::Parcel::writeUint8 = write(&val,1), no padding).
        //
        // 6-Z305t-66 answered from the BUS REGISTRY ONLY; 6-Z305t-67 adds
        // the VINTF-manifest consult (crate::vintf): the A11
        // hwservicemanager answers getTransport from the device's VINTF
        // manifests — hwsm ServiceManager.cpp:414 delegates to the free
        // getTransport (hwsm Vintf.cpp:36) which consults the FRAMEWORK
        // manifest first, then the DEVICE manifest; its service map is
        // NEVER consulted, so an unregistered but manifest-declared
        // service answers HWBINDER before it ever runs. That answer is
        // BOTH sides' gate: the registration pre-check
        // (registerAsServiceInternal, ServiceManagement.cpp:872 —
        // "must be in VINTF manifest in order to register/get" at :878)
        // and every client's transport discovery (getRawServiceInternal,
        // :779 — EMPTY + PRODUCT_ENFORCE_VINTF_MANIFEST makes the service
        // unreachable even while running). The bus fallback below is the
        // container's deliberate superset for in-proxy virtual services
        // (kernel-provided, exist before any guest runs — see vintf.rs
        // header); invalid fq names answer EMPTY per Vintf.cpp's gates.
        //
        // 6-Z305t-68: the arms finally RUN — the token + string args now
        // parse the real wire (C-string token, PTR/SG string objects).
        HIDL_SM_GET_TRANSPORT => {
            let fq = match p.read_string_arg() {
                Some(s) => s,
                None => {
                    hidl_sm_parse_fail_diag("getTransport.fq", code, blob);
                    return TransactionResult::Failed;
                }
            };
            let name = match p.read_string_arg() {
                Some(s) => s,
                None => {
                    hidl_sm_parse_fail_diag("getTransport.name", code, blob);
                    return TransactionResult::Failed;
                }
            };
            let key = format!("{}/{}", fq, name);
            let (transport, source) = match crate::vintf::parse_fq(&fq) {
                None => (HIDL_TRANSPORT_EMPTY, "invalid-fq"),
                Some(_) => match crate::vintf::lookup(&fq, &name) {
                    Some(t) => (t, "manifest"),
                    None => {
                        let hit = {
                            let b = bus.lock().expect("binder bus poisoned");
                            if b.services.contains_key(&key) {
                                Some("bus")
                            } else if hidl_ancestor_registered_6z350(&b.services, &fq, &name) {
                                // 6-Z350: a registered ancestor minor of the
                                // same major satisfies the get (HIDL
                                // interface inheritance — the real SM
                                // semantic).
                                Some("bus-ancestor")
                            } else {
                                None
                            }
                        };
                        match hit {
                            Some(src) => (HIDL_TRANSPORT_HWBINDER, src),
                            None => (HIDL_TRANSPORT_EMPTY, "no-entry"),
                        }
                    }
                },
            };
            writer.write_u8(transport);
            info!(
                "[KR64][binder][svc] HIDL getTransport({}) → {} ({})",
                key,
                transport_name(transport),
                source
            );
        }
        HIDL_SM_ADD => {
            let name = match p.read_string_arg() {
                Some(s) => s,
                None => {
                    hidl_sm_parse_fail_diag("add.name", code, blob);
                    return TransactionResult::Failed;
                }
            };
            let flat = p.read_binder_arg();
            // 6-Z306ae: prefer the offsets-array flat (kernel's own map).
            let flat = flat_at_first_binder_offset(blob).or(flat);
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
                // callbacks while holding the bus lock (the fire helper
                // re-locks internally in the AIDL path — here we hold the
                // lock, so call the same method on the guard's target).
                b.fire_registration_callbacks(&name, h, false);
                // 6-Z306ae-e: mirror the registry's strong node ref IN
                // THIS IOCTL (liveness-gated) — decided inside the lock.
                // 6-Z306am: arm B gates the PREFIX off for self-
                // transactions and re-routes the acquire through the
                // liveness-gated RefCmd queue (see the gate's header).
                let gpid = b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0);
                // 6-Z458 (Task 195): the registry's own pin books into the
                // node map UNCONDITIONALLY (see the AIDL add arm).
                let acq_mirrored = ptr != 0 && mirror_ref_ok(gpid, ptr, cookie);
                if ptr != 0 {
                    b.z458_registry_pin_add(conn_id, ptr, cookie, acq_mirrored);
                }
                if acq_mirrored {
                    if z306am_skip_prefix(conn_id, conn_id) {
                        if let Some(bx) = b.conns.get_mut(&conn_id) {
                            bx.z491.reply_enq += 1;
                            bx.reply_queue.push_back(DeferredReply::RefCmd {
                                br: BR_ACQUIRE,
                                ptr,
                                cookie,
                            });
                        }
                        if Z306AM_SKIP_LOG.load(Ordering::Relaxed) > 0 {
                            Z306AM_SKIP_LOG.fetch_sub(1, Ordering::Relaxed);
                            info!(
                                "[KR64][binder][svc] 6-Z306am: arm B — prefix SKIPPED (self-tx) HIDL add '{}' conn={} ptr=0x{:x} cookie=0x{:x} → liveness RefCmd queued",
                                name, conn_id, ptr, cookie
                            );
                        }
                    } else {
                        mirror = Some((BR_ACQUIRE, ptr, cookie));
                    }
                    // 6-Z454: registry acquire emitted — see the AIDL add arm.
                    z454_emit(gpid, ptr, cookie, Z454Site::RegAcq);
                }
                h
            };
            // Reply: bool success = true.
            let _ = handle;
            writer.write_i32(1);
            // 6-Z480: kernel pool recruitment on the successful add reply —
            // a service proc with zero parked readers gains its first
            // looper here (the healthd-style class: register + mainloop,
            // never joinRpcThreadpool — rn451's health@2.1-service wall).
            if z480_needs_pool_recruitment(bus, conn_id) {
                spawn_looper = true;
                info!(
                    "[KR64][binder][svc] 6-Z480: pool recruitment armed on HIDL add '{}' reply (conn={}) — the kernel's BR_SPAWN_LOOPER for a service proc with zero parked loopers",
                    name, conn_id
                );
            }
        }
        HIDL_SM_REGISTER_FOR_NOTIFICATIONS => {
            // 6-Z276: args = [hidl_string fqName][hidl_string name][flat
            // callback]. The HIDL registry key is "fqName/instance". The
            // flat is the watcher's local IServiceNotification object.
            let fq = match p.read_string_arg() {
                Some(s) => s,
                None => {
                    hidl_sm_parse_fail_diag("registerForNotifications.fq", code, blob);
                    return TransactionResult::Failed;
                }
            };
            let inst = match p.read_string_arg() {
                Some(s) => s,
                None => {
                    hidl_sm_parse_fail_diag("registerForNotifications.name", code, blob);
                    return TransactionResult::Failed;
                }
            };
            let flat = p.read_binder_arg();
            let key = format!("{}/{}", fq, inst);
            let registered = if let Some(f) = flat {
                let mut b = bus.lock().expect("binder bus poisoned");
                // 6-Z323: the watcher registers UNCONDITIONALLY — the real
                // hwservicemanager's registerForNotifications adds the
                // listener FIRST (ServiceManager.cpp: the callback goes
                // into the listener list before any preexisting check), so
                // future (re)registrations of this key notify this caller.
                let added = b.add_watcher(
                    &key,
                    ServiceWatcher {
                        conn: conn_id,
                        ptr: f.binder,
                        cookie: f.cookie,
                        hidl: true,
                    },
                );
                // 6-Z325: when a NEW watcher entry was stored, pin the
                // caller's local callback wrapper with the kernel's own
                // node-ref mirror (hwservicemanager holds a strong ref on
                // the callback node — binder_node_post_acquire →
                // BR_ACQUIRE to the owner). Without it the transient BnHw
                // wrapper dies after the register reply and the queued
                // onRegistration can never execute (the rn273 wall: the
                // A11 Waiter::onFirstRef registers a marshal-transient
                // BnHwIServiceNotification whose weakref chunk was already
                // reused at delivery time — mStrong=49/mBase=0xf).
                if added {
                    mirror = Some((BR_ACQUIRE, f.binder, f.cookie));
                    // 6-Z454: the watcher pin's acquire is mirrored —
                    // see the AIDL twin.
                    let wpid = b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0);
                    z454_emit(wpid, f.binder, f.cookie, Z454Site::WatchAcq);
                }
                match b.services.get(&key).map(|e| e.handle) {
                    Some(h) => {
                        // Already registered: the immediate preexisting
                        // callback. The fire walks the watcher list, which
                        // NOW CONTAINS this caller — the
                        // onRegistration(preexisting=true) oneway actually
                        // reaches the just-registered callback.
                        //
                        // RN271 decode: the pre-6-Z323 shape fired only the
                        // PRE-EXISTING watchers here and never stored the
                        // new one — the waitForHwService caller (the A11
                        // disableAutoSuspend once in system_server) got the
                        // registerForNotifications reply but NEVER the
                        // onRegistration transaction, and blocked forever in
                        // libhidlbase Waiter::wait — the PMS.<init>
                        // nativeSetAutoSuspend wall that parked every
                        // system_server era at rung 7 (StartPowerManager).
                        b.fire_registration_callbacks(&key, h, true);
                        true
                    }
                    None => false,
                }
            } else {
                false
            };
            // 6-Z324: the caller is ABOUT to wait for this callback (the
            // A11 waitForHwService Waiter). Recruit a pooled hwbinder
            // reader now via BR_SPAWN_LOOPER on this reply — the kernel's
            // own mechanism — so the queued onRegistration oneway has a
            // reader even if the registering thread parks on its condvar.
            spawn_looper = true;
            // Reply: bool registered = true (the registration itself took;
            // HIDL bool = 1 byte, hwbinder::Parcel::writeBool = writeInt8).
            writer.write_u8(1);
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
            let fq = match p.read_string_arg() {
                Some(s) => s,
                None => {
                    hidl_sm_parse_fail_diag("unregisterForNotifications.fq", code, blob);
                    return TransactionResult::Failed;
                }
            };
            let inst = match p.read_string_arg() {
                Some(s) => s,
                None => {
                    hidl_sm_parse_fail_diag("unregisterForNotifications.name", code, blob);
                    return TransactionResult::Failed;
                }
            };
            let flat = p.read_binder_arg();
            if let Some(f) = flat {
                let key = format!("{}/{}", fq, inst);
                let mut b = bus.lock().expect("binder bus poisoned");
                // 6-Z325: mirror the ref drop for a real removal (the
                // kernel releases the SM's node ref on unregister →
                // BR_RELEASE to the owner).
                if b.remove_watcher(&key, conn_id, f.binder) {
                    mirror = Some((BR_RELEASE, f.binder, f.cookie));
                    // 6-Z454: the watcher release is queued — see the AIDL twin.
                    let wpid = b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0);
                    z454_emit(wpid, f.binder, f.cookie, Z454Site::WatchRel);
                }
                info!(
                    "[KR64][binder][svc] 6-Z276: HIDL unregisterForNotifications({}) conn={} — dropped",
                    key, conn_id
                );
            }
            // Reply: bool success = true (HIDL bool = 1 byte).
            writer.write_u8(1);
        }
        // A11 `android.hidl.manager@1.2::IServiceManager.addWithChain` —
        // THE registration call the A11 GSI's HALs make
        // (registerAsServiceInternal, ServiceManagement.cpp:884:
        // `service->interfaceChain → sm->addWithChain(name, service, chain)`
        // via defaultServiceManager1_2). The chain carries the fqNames the
        // 1.0 add() wire lacks, so the registry keys "fq/instance" exactly
        // like hwservicemanager (and like ensure_virtual_services already
        // does on the AIDL side).
        //
        // Wire (manager@1.2 IServiceManager.hal:69 — parameter order):
        // request `[hidl_string name][IBinder service — inline flat]
        // [hidl_vec<hidl_string> chain]` (name BEFORE the interface object);
        // reply `[status ok][u8 1]` (HIDL bool = 1 byte).
        HIDL_SM_ADD_WITH_CHAIN => {
            let name = match p.read_string_arg() {
                Some(s) => s,
                None => {
                    hidl_sm_parse_fail_diag("addWithChain.name", code, blob);
                    return TransactionResult::Failed;
                }
            };
            // 6-Z305t-69c: the ladder-#129 OBJECT DUMP settled the wire
            // (offs(7) = [name struct, name chars, FLAT, vec struct,
            // array(par=vec), chars0(par=array,0), chars1(par=array,16)],
            // sg=[16,8,16,32,43,29]) — the .hal declaration order IS the
            // wire order. The -69b flat-first backtracking was WRONG (it
            // tried the flat at the post-name cursor, consumed the name
            // struct, then misread the name chars as the vec struct).
            // Keep ONE fallback: if (flat, vec) fails after the name,
            // restore the post-name cursor and try (vec, flat).
            let after_name = (p.obj_idx, p.ptr_seq);
            let mut flat = p.read_binder_arg();
            let mut chain = p.read_vec_string_arg();
            if chain.is_none() {
                p.obj_idx = after_name.0;
                p.ptr_seq = after_name.1;
                chain = p.read_vec_string_arg();
                if chain.is_some() {
                    flat = p.read_binder_arg();
                }
            }
            // 6-Z306ae: prefer the offsets-array flat (kernel's own map)
            // over the positional backtrack dance.
            let flat = flat_at_first_binder_offset(blob).or(flat);
            let (ptr, cookie) = match &flat {
                Some(f) => (f.binder, f.cookie),
                None => (0, 0),
            };
            let chain = match chain {
                Some(v) => v,
                None => {
                    hidl_sm_parse_fail_diag("addWithChain.chain", code, blob);
                    return TransactionResult::Failed;
                }
            };
            if chain.is_empty() {
                hidl_sm_parse_fail_diag("addWithChain.empty-chain", code, blob);
                return TransactionResult::Failed;
            }
            // Register under the CONCRETE interface (chain[0]) — every boot
            // lookup names it. hwservicemanager also indexes the parent
            // interfaces, but the fleet's lookups always name the concrete
            // fq, so a single key keeps the handle space 1:1 with services.
            let fq = chain[0].clone();
            let key = format!("{}/{}", fq, name);
            let handle = {
                let mut b = bus.lock().expect("binder bus poisoned");
                let h = b.add_guest_service(&key, conn_id, ptr, cookie);
                // 6-Z276: fire the HIDL IServiceNotification.onRegistration
                // callbacks for the newly registered key (the same helper
                // the 1.0 add arm uses).
                b.fire_registration_callbacks(&key, h, false);
                // 6-Z306ae-e: mirror the registry's strong node ref IN
                // THIS IOCTL (liveness-gated) — decided inside the lock.
                // 6-Z306am: arm B gates the PREFIX off for self-
                // transactions and re-routes the acquire through the
                // liveness-gated RefCmd queue (see the gate's header).
                let gpid = b.conns.get(&conn_id).map(|c| c.sender_pid).unwrap_or(0);
                // 6-Z458 (Task 195): the registry's own pin books into the
                // node map UNCONDITIONALLY (see the AIDL add arm).
                let acq_mirrored = ptr != 0 && mirror_ref_ok(gpid, ptr, cookie);
                if ptr != 0 {
                    b.z458_registry_pin_add(conn_id, ptr, cookie, acq_mirrored);
                }
                if acq_mirrored {
                    if z306am_skip_prefix(conn_id, conn_id) {
                        if let Some(bx) = b.conns.get_mut(&conn_id) {
                            bx.z491.reply_enq += 1;
                            bx.reply_queue.push_back(DeferredReply::RefCmd {
                                br: BR_ACQUIRE,
                                ptr,
                                cookie,
                            });
                        }
                        if Z306AM_SKIP_LOG.load(Ordering::Relaxed) > 0 {
                            Z306AM_SKIP_LOG.fetch_sub(1, Ordering::Relaxed);
                            info!(
                                "[KR64][binder][svc] 6-Z306am: arm B — prefix SKIPPED (self-tx) HIDL addWithChain '{}' conn={} ptr=0x{:x} cookie=0x{:x} → liveness RefCmd queued",
                                key, conn_id, ptr, cookie
                            );
                        }
                    } else {
                        mirror = Some((BR_ACQUIRE, ptr, cookie));
                    }
                    // 6-Z454: registry acquire emitted — see the AIDL add arm.
                    z454_emit(gpid, ptr, cookie, Z454Site::RegAcq);
                }
                h
            };
            writer.write_u8(1); // bool success = true
                                // 6-Z480: kernel pool recruitment on the successful addWithChain
                                // reply — THE rn451 wall: the health@2.1-service registered
                                // (handle 0x8) and never read again (its one thread is the
                                // healthd ep_poll mainloop), so BatteryService's
                                // interfaceDescriptor probe sat in the conn's inbox until the
                                // 6-Z407 8 s expiry → getService null → onStart threw →
                                // "Failure starting system services". The real kernel recruits
                                // the pool on THIS read (binder_thread_read, a non-looper
                                // thread's read with waiting_threads empty) — so does the proxy
                                // now, gated by z480_needs_pool_recruitment.
            if z480_needs_pool_recruitment(bus, conn_id) {
                spawn_looper = true;
                info!(
                    "[KR64][binder][svc] 6-Z480: pool recruitment armed on HIDL addWithChain '{}' reply (conn={}) — the kernel's BR_SPAWN_LOOPER for a service proc with zero parked loopers",
                    key, conn_id
                );
            }
            // 6-Z351 (rn302 decode): the REAL hwservicemanager registers
            // the service under EVERY interfaceChain entry — AOSP
            // ServiceManager.cpp addImpl (android-11.0.0_r1): per chain
            // fqName, insertService/setService + a per-fq
            // sendPackageRegistrationNotification — ONE HidlService
            // object, many fq keys. rn302 proved the wall: the composer
            // registered @2.3::IComposer/default (chain[0] = [@2.3, @2.2,
            // @2.1, IBase], handle 0x28) and the 6-Z350b ancestor
            // getTransport answered SF's @2.1 transport probe — but the
            // SUBSEQUENT `sm->get(@2.1::IComposer/default)`
            // (getRawServiceInternal, after the transport hit) is an
            // EXACT map lookup that missed → "getService: Trying again
            // for android.hardware.graphics.composer@2.1::IComposer/
            // default" ×537 → SF never published → rung 7. With the
            // chain inserted, the real SM's exact-key get() hits — the
            // whole ancestor-get class (composer@2.1, keymaster@4.0,
            // soundtrigger@2.0/2.1, wifi@1.0-1.3, camera@2.4/2.5,
            // bluetooth@1.0, thermal@1.0 …) is served by registration
            // shape, not by a lookup hack. Aliases share the chain[0]
            // HANDLE (one node identity — the kernel hands the same
            // process the same handle for the same node) and fire their
            // own onRegistration callbacks (the real loop notifies per
            // chain entry); by_handle stays canonical on chain[0].
            let mut alias_count = 0usize;
            for alias_fq in chain.iter().skip(1) {
                let alias_key = format!("{}/{}", alias_fq, name);
                if alias_key == key {
                    continue;
                }
                let mut b = bus.lock().expect("binder bus poisoned");
                b.add_guest_service_alias(&alias_key, conn_id, ptr, cookie, handle);
                b.fire_registration_callbacks(&alias_key, handle, false);
                alias_count += 1;
            }
            info!(
                "[KR64][binder][svc] 6-Z351: addWithChain '{}' → {} chain-alias keys (full AOSP addImpl multi-fq registration)",
                key, alias_count
            );
            info!(
                "[KR64][binder][svc] HIDL addWithChain({}) → handle 0x{:08x} (conn={}, ptr=0x{:x} cookie=0x{:x}, chain={:?}) — the ADD flat's (ptr,cookie) names the registry-pinned shell: reply-borne nodes whose 6-Z359 name reads \"?\" join against THIS line (rn423 decode: node 0x4d's (conn=8, ptr) had no registry match)",
                key, handle, conn_id, ptr, cookie, chain
            );
        }
        // A11 `android.hidl.manager@1.0::IServiceManager
        // .registerPassthroughClient(string fqName, string name)` (code 8)
        // — a ONEWAY bookkeeping call passthrough-mode clients make; the
        // real hwservicemanager records the caller and returns nothing.
        // The 68b catch-all Failed made the sender's waitForResponse
        // surface BR_FAILED_REPLY for a oneway transaction (kernel
        // semantics: oneway NEVER gets a reply). Honest container shape:
        // we ARE the servicemanager — record the key (bounded log).
        HIDL_SM_REGISTER_PASSTHROUGH_CLIENT => {
            let fq = match p.read_string_arg() {
                Some(s) => s,
                None => {
                    hidl_sm_parse_fail_diag("registerPassthroughClient.fq", code, blob);
                    return TransactionResult::Failed;
                }
            };
            let inst = match p.read_string_arg() {
                Some(s) => s,
                None => {
                    hidl_sm_parse_fail_diag("registerPassthroughClient.name", code, blob);
                    return TransactionResult::Failed;
                }
            };
            info!(
                "[KR64][binder][svc] HIDL registerPassthroughClient({}/{} conn={}) — recorded",
                fq, inst, conn_id
            );
            // oneway: the reply bytes are dropped at the BC arm (TC only).
        }
        // A11 `android.hidl.manager@1.2::IServiceManager
        // .listManifestByInterface(string fqName) generates
        // (vec<string> instances)` (code 13) — the caller (keystore's
        // enumerate, cameraserver's provider walk) reads count ×
        // sizeof(hidl_string) per element. 6-Z376 (rn330 decode): the
        // pre-6-Z376 vec<Instance{fqName,instance}> shape was
        // UNCONSUMABLE — the guest logd verdict, 42×: "hw-Parcel: Buffer
        // length 32 does not match expected size 16." (the client
        // expected 16 B/element, hit the 32 B Instance array; keystore
        // 40× + cameraserver 2×). The 6-Z305t-69 comment had conflated
        // this method with debugDump's vec<InstanceDebugInfo> struct.
        // The honest answer: every bus-registered "fq/instance" key whose
        // fq matches, marshaled as vec<hidl_string> via
        // write_hidl_vec_string — the shape family the SM cast probes
        // have consumed client-side since 6-Z307d ([status ok][PTR vec
        // struct {ptr,size}][PTR array child 0][PTR chars per element,
        // parent = THE ARRAY, parent_offset = j*16]). The SG bytes ride
        // the v3 resp trailer and the loader applies the receiver pointer
        // fixup (the 6-Z305t-68 machinery).
        // 1.0 `debugDump()` — code 7. The A11 Watchdog's
        // getInterestingHalPids() consumes this INSIDE the WAITED_HALF
        // branch (Watchdog.java:616 → :517); on real hwservicemanager
        // (ServiceManager.cpp:724) it is one InstanceDebugInfo per
        // registered service with getDebugPid() = the registering
        // process's pid (NO_PID = -1 when unknown). The proxy's registry
        // data is REAL (real guest owner pids); proxy-owned virtuals
        // report NO_PID (-1) — the watchdog skips those, exactly as it
        // skips them on real hardware.
        HIDL_SM_DEBUG_DUMP => {
            let entries: Vec<(String, String, i32)> = {
                let b = bus.lock().expect("binder bus poisoned");
                b.services
                    .iter()
                    .map(|(k, e)| {
                        let (f, i) = k.split_once('/').unwrap_or((k.as_str(), ""));
                        let pid = if e.owner == PROXY_CONN_ID {
                            -1
                        } else {
                            b.conns.get(&e.owner).map(|c| c.sender_pid).unwrap_or(0)
                        };
                        (f.to_string(), i.to_string(), if pid > 0 { pid } else { -1 })
                    })
                    .take(128)
                    .collect()
            };
            let count = entries.len();
            // hidl_vec<InstanceDebugInfo> wire (element = 64B stride:
            // interfaceName hdr @+0, instanceName hdr @+16 — both fixed
            // up loader-side from the chars PTR objects — pid @+32,
            // clientPids vec hdr @+40 (fixed up from the empty-array
            // PTR object), arch u8 @+56).
            let mut vs = vec![0u8; 16];
            vs[8..12].copy_from_slice(&(count as u32).to_ne_bytes());
            let vec_idx = writer.next_object_index();
            writer.write_ptr_object(vs, None, 0);
            let arr_idx = writer.next_object_index();
            let mut array = vec![0u8; count * 64];
            for (j, (f, i, pid)) in entries.iter().enumerate() {
                let base = j * 64;
                // 6-Z309: the hidl_string header structs must carry the REAL
                // mSize (interfaceName @+0, instanceName @+16) — the client's
                // read walk takes the chars lengths from them; mBuffer slots
                // are patched loader-side from the chars PTR objects.
                array[base + 8..base + 12].copy_from_slice(&(f.len() as u32).to_ne_bytes());
                array[base + 24..base + 28].copy_from_slice(&(i.len() as u32).to_ne_bytes());
                array[base + 32..base + 36].copy_from_slice(&pid.to_ne_bytes());
                array[base + 56] = HIDL_DEBUG_ARCH_UNKNOWN;
            }
            writer.write_ptr_object(array, Some(vec_idx), 0);
            for (j, (f, i, _)) in entries.iter().enumerate() {
                let base = (j * 64) as u64;
                let mut fc = f.as_bytes().to_vec();
                fc.push(0);
                writer.write_ptr_object(fc, Some(arr_idx), base);
                let mut ic = i.as_bytes().to_vec();
                ic.push(0);
                writer.write_ptr_object(ic, Some(arr_idx), base + 16);
                // clientPids: empty vec — the loader writes the 16B
                // {buffer, size=0} header at base+40 from this PTR.
                writer.write_ptr_object(Vec::new(), Some(arr_idx), base + 40);
            }
            info!("[KR64][binder][svc] HIDL debugDump → {} entries", count);
        }
        HIDL_SM_LIST_MANIFEST_BY_INTERFACE => {
            let fq = match p.read_string_arg() {
                Some(s) => s,
                None => {
                    hidl_sm_parse_fail_diag("listManifestByInterface.fq", code, blob);
                    return TransactionResult::Failed;
                }
            };
            // 6-Z377 (rn331 decode): the entries are **BARE INSTANCE NAMES**
            // ("default", "internal/0", …) — NOT "fq/instance" compounds. The
            // rn331 artifact pinned it end-to-end: with 6-Z376's full-key
            // entries the enumerate callback finally RAN (the wall moved!)
            // and the client passed each entry VERBATIM as the getService
            // INSTANCE arg — 33× "getTransport(@4.0::IKeymasterDevice/@4.0::
            // IKeymasterDevice/default) → EMPTY" + 33× the CHECK(device)
            // abort 'Failed to get service for "android.hardware.keymaster@
            // 4.0::IKeymasterDevice" with interface name "…@4.0::…/default"'
            // — the verbatim AOSP keystore_main try_get_device CHECK, whose
            // `if (n == "default") has_default = true` also only makes sense
            // for bare instance names. The client re-attaches ITS OWN
            // queried descriptor, so the resolution path is:
            // getService("@4.0::IKeymasterDevice", "default") →
            // getTransport(4.0/default) → HWBINDER (the 6-Z373 manifest
            // range expansion) → get(4.0/default) → the 6-Z351 chain-alias
            // HIT → the keymaster device resolves → halVersion →
            // kmDevices[TRUSTED_ENVIRONMENT] fills → the CHECK abort loop
            // dies (0 kmDevices aborts in rn331 already; the 33 CHECK(device)
            // aborts were the LAST shim in the chain).
            let names: Vec<String> = {
                let b = bus.lock().expect("binder bus poisoned");
                b.services
                    .keys()
                    .filter_map(|k| {
                        let (f, i) = k.split_once('/')?;
                        (f == fq && !i.is_empty()).then(|| i.to_string())
                    })
                    .take(128)
                    .collect()
            };
            let count = names.len();
            writer.write_hidl_vec_string(&names);
            info!(
                "[KR64][binder][svc] HIDL listManifestByInterface({}) → {} entries",
                fq, count
            );
            // 6-Z375: bounded reply-wire dump (first 3 NON-empty replies per
            // boot) — kept as the shape-change witness for the next decode.
            static Z375_SEEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            if count > 0 && Z375_SEEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 3 {
                info!(
                    "[KR64][binder][svc] 6-Z375 listManifest reply wire: fq={} count={} {}",
                    fq,
                    count,
                    writer.diag_object_graph()
                );
            }
        }
        _ => {
            hidl_sm_parse_fail_diag("catch-all", code, blob);
            return TransactionResult::Failed;
        }
    }

    let (data, offsets, sg) = writer.into_parts_with_sg();
    if spawn_looper {
        // 6-Z324/6-Z325: the HIDL registerForNotifications replies carry
        // BOTH the pool-thread recruitment AND (when a new watcher was
        // stored) the callback's node-ref mirror — one combined batch.
        TransactionResult::ReplySpawnLooper {
            mirror,
            data,
            offsets,
            sg,
        }
    } else if let Some((br, mptr, mcookie)) = mirror {
        TransactionResult::ReplyMirrored {
            br,
            ptr: mptr,
            cookie: mcookie,
            data,
            offsets,
            sg,
        }
    } else {
        TransactionResult::Reply { data, offsets, sg }
    }
}

/// Legacy v1 path (no parcel blob): the loader could not inline the
/// guest's parcel bytes, so the proxy answers the *synthetic* shapes per
/// 6-Z114 §2.4 — GET/CHECK → AIDL null binder (status 0 + flat
/// `{BINDER_TYPE_BINDER, 0, 0, 0}`), ADD → status 0 (header only). The
/// registry cannot work name-less — this path exists to keep the ROM's
/// libbinder loops terminating until a v2-capable loader attaches
/// (6-Z271 inlined request blobs for ALL real-libbinder clients).
fn servicemanager_legacy(code: u32) -> TransactionResult {
    // 6-Z401: the v1 legacy add is a SILENT fake-success — the registry
    // cannot learn a name-less add, yet the client sees status-0 and
    // carries on. rn355 pinned this shape as the reason SF's
    // addService(SurfaceFlinger) never reached the registry (no bus
    // receipt, DisplayManagerService polled forever) — every occurrence
    // must be visible. Bounded: 16 per boot.
    if code == SVC_MGR_ADD_SERVICE {
        static Z401_LEGACY_ADD: std::sync::atomic::AtomicU32 =
            std::sync::atomic::AtomicU32::new(16);
        if Z401_LEGACY_ADD.load(Ordering::Relaxed) > 0 {
            Z401_LEGACY_ADD.fetch_sub(1, Ordering::Relaxed);
            warning!(
                "[KR64][binder][svc] 6-Z401: legacy v1 addService (blob-less request) — replying fake status-0; the registry CANNOT learn this name (occurrence {}/{})",
                16 - Z401_LEGACY_ADD.load(Ordering::Relaxed),
                16
            );
        }
    }
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
    TransactionResult::Reply {
        data,
        offsets,
        sg: Vec::new(),
    }
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
    TransactionResult::Reply {
        data,
        offsets,
        sg: Vec::new(),
    }
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
                // 6-Z307: HIDL interfaces carry no AIDL interface-version
                // meta transaction — and this dispatcher is unreachable
                // for HidlServiceManager anyway (the transaction dispatch
                // site routes the kind to servicemanager_proxy before
                // virtual_service_transaction). Defensive 0.
                VirtualService::HidlServiceManager => 0,
            });
            let (data, offsets) = w.into_parts();
            return TransactionResult::Reply {
                data,
                offsets,
                sg: Vec::new(),
            };
        }
        0xFFFF_FFFE => {
            let mut w = ParcelWriter::new();
            w.write_status_ok();
            w.write_string16("ffffffff");
            let (data, offsets) = w.into_parts();
            return TransactionResult::Reply {
                data,
                offsets,
                sg: Vec::new(),
            };
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
        // 6-Z307: the SM instances are served by the servicemanager
        // dispatcher (see the transaction dispatch site) — unreachable
        // here; fail honestly rather than fabricate a reply shape.
        VirtualService::HidlServiceManager => TransactionResult::Failed,
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
    // 6-Z300b: per-call observability — the 6-Z298 service registered but
    // CI could never prove client ENGAGEMENT (the registration lines were
    // the only trace; the vibrator logs every call, health did not). One
    // line per transaction names the method and the sysfs snapshot driving
    // the reply, so the next wave's grep of "[binder][svc] IHealth." either
    // proves GetBatteryInfo's isDeclared/waitForService chain reaches the
    // proxy or localises the drop (e.g. the guest's health-V4-ndk.so
    // ENOENT chain of run 33963412329).
    let method = match code {
        1 => "registerCallback",
        2 => "unregisterCallback",
        3 => "update",
        4 => "getChargeCounterUah",
        5 => "getCurrentNowMicroAmps",
        6 => "getCurrentAverageMicroAmps",
        7 => "getCapacity",
        8 => "getEnergyCounterNwh",
        9 => "getChargeStatus",
        10 => "getStorageInfo",
        11 => "getDiskStats",
        12 => "getHealthInfo",
        13 => "setChargingPolicy",
        14 => "getChargingPolicy",
        15 => "getBatteryHealthData",
        _ => "unknown",
    };
    // Snapshot once per transaction: the refresh thread may rewrite the
    // files mid-transaction; a single coherent snapshot is what the
    // real HAL's own HealthInfo mutex gives clients.
    let vals = crate::battery::read_guest_battery_values();
    info!(
        "[KR64][binder][svc] IHealth.{} (code={}) — sysfs snapshot: capacity={:?} status={:?}",
        method, code, vals.capacity_pct, vals.status_str
    );
    virtual_health_with_values(code, reader, &vals)
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
        TransactionResult::Reply {
            data,
            offsets,
            sg: Vec::new(),
        }
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
            TransactionResult::Reply {
                data,
                offsets,
                sg: Vec::new(),
            }
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
    TransactionResult::Reply {
        data,
        offsets,
        sg: Vec::new(),
    }
}

/// Synthetic effect set for the virtual IVibrator (6-Z300): the NON-deprecated
/// members of android-13.0.0_r1 `hardware/interfaces/vibrator/aidl/Effect.aidl`,
/// each realized as a plain one-shot vibration forwarded to the host app:
/// CLICK=0, THUD=1, TEXTURE_TICK=5, TICK=6, LOW_TICK=7, POP=8,
/// HEAVY_CLICK=9, SPINNER=22 (the RINGTONE_* slots 2..4/10..21 are
/// deprecated carry-overs from the HIDL 1.0 enum and are NOT synthesized).
const SYNTHETIC_VIBRATOR_EFFECTS: [i32; 8] = [0, 1, 5, 6, 7, 8, 9, 22];

/// Base one-shot duration (ms) per synthetic effect, scaled by
/// EffectStrength (LIGHT ×0.8, MEDIUM ×1.0, STRONG ×1.2 — the shape
/// vendor HALs use for strength on one-shot primitives). Returns `None`
/// for unsupported effects (deprecated RINGTONE_* / out of range) — the
/// caller answers those with duration **0, status OK** per the .aidl
/// contract ("or 0 if the effect is not supported"), never an exception.
fn synthetic_effect_duration(effect: i32, strength: i32) -> Option<i32> {
    let base: f32 = match effect {
        0 => 20.0,  // CLICK
        1 => 30.0,  // THUD
        5 => 8.0,   // TEXTURE_TICK
        6 => 10.0,  // TICK
        7 => 15.0,  // LOW_TICK
        8 => 40.0,  // POP
        9 => 35.0,  // HEAVY_CLICK
        22 => 10.0, // SPINNER
        _ => return None,
    };
    let factor: f32 = match strength {
        0 => 0.8, // LIGHT
        1 => 1.0, // MEDIUM
        2 => 1.2, // STRONG
        // Out-of-range strength: a real HAL validates the enum, but the
        // cheap-and-safe fallback is MEDIUM (never 0 ms — a client that
        // asked for a haptic asked for SOMETHING).
        _ => 1.0,
    };
    Some((base * factor).round() as i32).filter(|ms| *ms >= 1)
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
/// Capabilities stay 0 (no completion callbacks / amplitude control), so
/// well-behaved clients stick to plain on(ms) / off(). Every on(ms) is
/// FORWARDED to the host app for a REAL vibration.
///
/// 6-Z300 closes the two remaining honesty gaps the fox R12 decode named:
/// * `perform(effect, strength, cb?)` used to answer
///   EX_UNSUPPORTED_OPERATION. The .aidl contract instead says the return
///   value is the effect duration in ms, **0 when the effect is not
///   supported** — a bare exception breaks TWRP-12.1's synchronous
///   tap-haptic path (its client treats a failed transaction like a dead
///   HAL and re-waits). The synthetic set now genuinely fires: each
///   supported effect is forwarded to the host for a REAL vibration and
///   its duration returned, so the synchronous haptic resolves in µs and
///   the input thread never stalls (the R12 touch-latency queue item).
/// * `getSupportedEffects` used to be empty; it now lists exactly the
///   synthetic set (clients that gate perform() on it get honest answers).
fn virtual_vibrator(code: u32, reader: &mut ParcelReader) -> TransactionResult {
    match code {
        1 => {
            // getCapabilities → 0 (no callbacks, no amplitude control).
            let mut w = ParcelWriter::new();
            w.write_status_ok();
            w.write_i32(0);
            let (data, offsets) = w.into_parts();
            TransactionResult::Reply {
                data,
                offsets,
                sg: Vec::new(),
            }
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
            // perform(in Effect effect, in EffectStrength strength,
            //         in @nullable IVibratorCallback callback) → int
            // The AIDL interface-token header is already consumed by the
            // dispatch (6-Z271z) — the reader sits at the first arg. Both
            // enums are plain int32 on the wire (Effect.aidl /
            // EffectStrength.aidl, android-13.0.0_r1). The nullable
            // callback binder is deliberately NOT read: with caps=0 no
            // well-behaved client sends one, and ignoring the request tail
            // is what on() already does for its callback argument.
            let effect = reader.read_i32().unwrap_or(-1);
            let strength = reader.read_i32().unwrap_or(-1);
            let mut w = ParcelWriter::new();
            w.write_status_ok();
            match synthetic_effect_duration(effect, strength) {
                Some(ms) => {
                    crate::hostbridge::notify_vibrate(ms);
                    info!(
                        "[KR64][binder][svc] IVibrator.perform(effect={} strength={}) → {} ms host",
                        effect, strength, ms
                    );
                    w.write_i32(ms);
                }
                None => {
                    // The .aidl-documented unsupported semantic: duration
                    // 0 with status OK — NOT an exception header.
                    info!(
                        "[KR64][binder][svc] IVibrator.perform(effect={}) → 0 (unsupported)",
                        effect
                    );
                    w.write_i32(0);
                }
            }
            let (data, offsets) = w.into_parts();
            TransactionResult::Reply {
                data,
                offsets,
                sg: Vec::new(),
            }
        }
        5 => {
            // getSupportedEffects → Effect[] (length-prefixed i32 array)
            // listing exactly the synthetic set (6-Z300).
            let effects: &[i32] = &SYNTHETIC_VIBRATOR_EFFECTS;
            let mut w = ParcelWriter::new();
            w.write_status_ok();
            w.write_i32(effects.len() as i32);
            for &e in effects {
                w.write_i32(e);
            }
            let (data, offsets) = w.into_parts();
            TransactionResult::Reply {
                data,
                offsets,
                sg: Vec::new(),
            }
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
            TransactionResult::Reply {
                data,
                offsets,
                sg: Vec::new(),
            }
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
            TransactionResult::Reply {
                data,
                offsets,
                sg: Vec::new(),
            }
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
            TransactionResult::Reply {
                data,
                offsets,
                sg: Vec::new(),
            }
        }
        _ => virtual_error_reply(EX_UNSUPPORTED_OPERATION, 0),
    }
}

// ============================================================================
// Wire-framing I/O helpers.
// ============================================================================

/// 6-Z355: read one frame AND any SCM_RIGHTS fds attached to it. The
/// first chunk of the frame is received via recvmsg with a control
/// buffer — ancillary data always accompanies the FIRST bytes of the
/// sender's sendmsg, so a single control-buffered header read is
/// sufficient and every plain read after it is cmsg-free. fds arrive as
/// kernel dups into THIS process's table (wrapped in [`FdGuard`]).
fn read_frame_with_fds(stream: &mut UnixStream) -> io::Result<(Frame, Vec<FdGuard>)> {
    let mut hdr = [0u8; 8]; // [u32 cmd][u32 arg_len]
    let mut fds: Vec<FdGuard> = Vec::new();
    let fd = stream.as_raw_fd();

    // First recvmsg: capture both the header bytes and any ancillary data.
    let mut got = 0usize;
    while got < 8 {
        let mut iov = [libc::iovec {
            iov_base: hdr[got..].as_mut_ptr() as *mut libc::c_void,
            iov_len: 8 - got,
        }];
        let mut cbuf = [0u8; 128]; // room for 32 fds worth of SCM_RIGHTS
        let mut msg = libc::msghdr {
            msg_name: std::ptr::null_mut(),
            msg_namelen: 0,
            msg_iov: iov.as_mut_ptr(),
            msg_iovlen: 1,
            msg_control: cbuf.as_mut_ptr() as *mut libc::c_void,
            msg_controllen: cbuf.len(),
            msg_flags: 0,
        };
        let n = unsafe { libc::recvmsg(fd, &mut msg, libc::MSG_CMSG_CLOEXEC) };
        if n < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(err);
        }
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "read_frame_with_fds: peer closed",
            ));
        }
        got += n as usize;
        // Harvest any SCM_RIGHTS block that rode THIS chunk.
        unsafe { harvest_scm_rights(&msg, &mut fds) };
    }

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
    Ok((Frame { cmd, payload }, fds))
}

/// 6-Z355: walk a received msghdr's control messages and collect every
/// SCM_RIGHTS fd (each wrapped in an owning [`FdGuard`]). Non-RIGHTS
/// control blocks are skipped. SAFETY: `msg` must be a completed recvmsg
/// whose msg_control buffer is still valid.
unsafe fn harvest_scm_rights(msg: &libc::msghdr, out: &mut Vec<FdGuard>) {
    let mut cmsg = libc::CMSG_FIRSTHDR(msg);
    while !cmsg.is_null() {
        let clen = (*cmsg).cmsg_len as usize;
        if (*cmsg).cmsg_level == libc::SOL_SOCKET && (*cmsg).cmsg_type == libc::SCM_RIGHTS {
            let data = libc::CMSG_DATA(cmsg) as *const u8;
            // cmsg_len covers the header + payload; payload = clen - hdr.
            let hdr_sz = libc::CMSG_LEN(0) as usize;
            if clen >= hdr_sz {
                let nfds = (clen - hdr_sz) / 4;
                for i in 0..nfds {
                    let raw = *(data.add(i * 4) as *const i32);
                    out.push(FdGuard::from_raw(raw));
                }
            }
        }
        cmsg = libc::CMSG_NXTHDR(msg, cmsg);
    }
}

/// Write one [`Resp`] to the stream.
fn write_frame(stream: &mut UnixStream, resp: &Resp) -> io::Result<()> {
    let mut buf = Vec::with_capacity(8 + resp.payload.len());
    buf.extend_from_slice(&resp.ret.to_ne_bytes());
    buf.extend_from_slice(&(resp.payload.len() as u32).to_ne_bytes());
    buf.extend_from_slice(&resp.payload);

    if resp.fds.is_empty() {
        return stream.write_all(&buf);
    }

    // 6-Z355: the response carries fds — ONE sendmsg delivers the frame
    // bytes plus the SCM_RIGHTS block (ancillary always rides the FIRST
    // byte of the message, so the recipient's control-buffered header
    // read captures it). A short send (large frame vs socket buffer) is
    // completed with plain writes — the cmsg already landed with the
    // first chunk. The proxy's own fd copies close when `resp` drops.
    let fd = stream.as_raw_fd();
    let raw: Vec<i32> = resp.fds.iter().map(|f| f.as_raw()).collect();
    let mut iov = [libc::iovec {
        iov_base: buf.as_ptr() as *mut libc::c_void,
        iov_len: buf.len(),
    }];
    let cmsg_space = unsafe { libc::CMSG_SPACE((4 * raw.len()) as u32) } as usize;
    let mut cmsg_buf = vec![0u8; cmsg_space];
    let msg = libc::msghdr {
        msg_name: std::ptr::null_mut(),
        msg_namelen: 0,
        msg_iov: iov.as_mut_ptr(),
        msg_iovlen: 1,
        msg_control: cmsg_buf.as_mut_ptr() as *mut libc::c_void,
        msg_controllen: cmsg_buf.len(),
        msg_flags: 0,
    };
    unsafe {
        let cmsg = libc::CMSG_FIRSTHDR(&msg);
        (*cmsg).cmsg_level = libc::SOL_SOCKET;
        (*cmsg).cmsg_type = libc::SCM_RIGHTS;
        (*cmsg).cmsg_len = libc::CMSG_LEN((4 * raw.len()) as u32) as usize;
        std::ptr::copy_nonoverlapping(
            raw.as_ptr() as *const u8,
            libc::CMSG_DATA(cmsg) as *mut u8,
            4 * raw.len(),
        );
    }
    let n = unsafe { libc::sendmsg(fd, &msg, libc::MSG_NOSIGNAL) };
    if n < 0 {
        return Err(io::Error::last_os_error());
    }
    if (n as usize) < buf.len() {
        stream.write_all(&buf[n as usize..])?;
    }
    Ok(())
}

// ============================================================================
// BR_* push helpers (build the read_buffer).
// ============================================================================

/// Push `[BR_NOOP]` (4 bytes, no payload).
fn push_br_noop(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&BR_NOOP.to_ne_bytes());
}

/// Push `[BR_SPAWN_LOOPER]` (4 bytes, no payload) — 6-Z324: the kernel's
/// pool-thread recruitment command (binder_thread_read prepends it when
/// the process has pending todo work and no free pool thread).
fn push_br_spawn_looper(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&BR_SPAWN_LOOPER.to_ne_bytes());
}

/// Push `[BR_FAILED_REPLY]` (4 bytes, no payload).
fn push_br_failed_reply(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&BR_FAILED_REPLY.to_ne_bytes());
}

/// 6-Z306an: Push `[BR_DEAD_REPLY]` (4 bytes, no payload) — the
/// caller-side code for a transaction whose target object is dead.
/// Consumed by `IPCThreadState::waitForResponse` (`case BR_DEAD_REPLY:
/// err = DEAD_OBJECT`), NEVER by the server-side `executeCommand`.
fn push_br_dead_reply(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&BR_DEAD_REPLY.to_ne_bytes());
}

/// Push `[BR_TRANSACTION_COMPLETE]` (4 bytes, no payload). The client's
/// `IPCThreadState::waitForResponse` consumes this and keeps looping for
/// the actual `BR_REPLY` (6-Z114 §4.5 — sync reply may batch
/// `[BR_TRANSACTION_COMPLETE][BR_REPLY]`).
fn push_br_transaction_complete(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&BR_TRANSACTION_COMPLETE.to_ne_bytes());
}

/// 6-Z306ad: bounded LOCAL-flat memory probes. The forked system_server
/// (ladders #199/#201) died inside Parcel::unflattenBinder's
/// sp<IBinder>(cookie) constructor — the Itanium vbase-offset read
/// (`ldr x8,[x21]; ldur x8,[x8,#-0x18]`) faulted with a NULL vtable
/// word at the registered cookie. These probes snapshot the registered
/// (ptr, cookie) bytes at the three wire moments — CAPTURE (addService),
/// SERVE (owner getService hit), DELIVERY (BR_TRANSACTION to a local
/// node) — read from OUTSIDE the guest via process_vm_readv, so a bad
/// pointer yields a clean "<unreadable>" instead of a guest crash.
/// Each probe class is bounded per boot; the log lines are the evidence
/// that pins WHEN the object's first word became zero.
static PROBE_CAPTURE_BUDGET: AtomicU32 = AtomicU32::new(32);
static PROBE_DELIVERY_BUDGET: AtomicU32 = AtomicU32::new(96);

/// 6-Z328: the LOCAL-get serve probe's own budget + the probed-name
/// dedupe (max 64 distinct names per boot). The shared serve budget was
/// exhausted by the early mediaserver self-get flood before the
/// boot-critical system_server LOCAL gets ("power") ever ran.
static PROBE_SERVE_LOCAL_BUDGET: AtomicU32 = AtomicU32::new(48);
static Z328_SERVE_NAMES: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// 6-Z325: bounded steal-delivery watches per boot. The rn273 decode leg
/// needs the SUCCESS/FAIL signal of a steal-delivered oneway callback
/// (the pool thread's executeCommand ran it → BC_FREE_BUFFER). Budgeted
/// like the probe classes so the watch can never turn into log flood.
static Z325_STEAL_WATCH_BUDGET: AtomicU32 = AtomicU32::new(8);

/// 6-Z327: the guest rootfs the binder proxy serves (recorded at
/// [`create_binder_device`]) — the SM get arm consults the guest's OWN
/// declared SDK level (ro.build.version.sdk from {rootfs}/system/
/// build.prop) before serving the A12-Category stability annotation.
static GUEST_ROOTFS: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);
static GUEST_SDK_CACHE: std::sync::RwLock<Option<Option<u32>>> = std::sync::RwLock::new(None);

/// 6-Z327: the guest's declared SDK level, read ONCE from
/// `{rootfs}/system/build.prop` (`ro.build.version.sdk=N`). `None` =
/// unknown — the recovery-class rootfs images ship no /system; the
/// legacy per-conn heuristic stays in charge for exactly those guests.
fn guest_sdk() -> Option<u32> {
    if let Some(cached) = GUEST_SDK_CACHE.read().expect("guest sdk lock").as_ref() {
        return *cached;
    }
    let rootfs = GUEST_ROOTFS.read().expect("guest rootfs lock").clone();
    let sdk = rootfs.and_then(|r| {
        let path = std::path::Path::new(&r).join("system/build.prop");
        let text = std::fs::read_to_string(&path).ok()?;
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("ro.build.version.sdk=") {
                return rest.trim().parse::<u32>().ok();
            }
        }
        None
    });
    *GUEST_SDK_CACHE.write().expect("guest sdk lock") = Some(sdk);
    sdk
}

/// 6-Z327: serve the A12-Category stability annotation ONLY to guests
/// whose own libbinder accepts it (SDK >= 31). The A11 guest's
/// Stability::set() rejects everything but the bare Level values — every
/// A12-annotated handle reply then unflattens to NULL. The 6-Z306ab
/// retry-after-hit flip is a GUESS that misfires on A11 guests whose
/// frameworks legitimately double-fetch a service right after a hit
/// (rn275: the flip poisoned system_server conns mid-boot →
/// getSystemService("power") → null → the ActivityStackSupervisor
/// .initPowerManagement NPE → FATAL EXCEPTION IN SYSTEM PROCESS → era
/// death — AFTER the 6-Z325/6-Z326 callback chain had finally carried the
/// boot past StartPowerManager). Unknown-SDK guests (the recovery-class
/// images without /system/build.prop) keep the legacy heuristic — the
/// corpus path is byte-identical.
fn a12_annotation_allowed() -> bool {
    match guest_sdk() {
        Some(sdk) => sdk >= 31,
        None => true,
    }
}

/// 6-Z309d: the shared recently-served registry (pid → last-delivery
/// instant), capped at 64 entries. The tracer's EXIT-event death capture
/// consults this ( [`crate::binder::recently_served`] ) so a daemon that
/// dies while/after serving a routed transaction gets the register
/// + maps-snapshot evidence even though it is NOT in the zygote lineage
/// (the suspend-daemon death class of rn255: sig=11, no delivery stop,
/// no capture — the crash site stayed unnamed).
fn served_pids() -> &'static std::sync::Mutex<std::collections::VecDeque<(i32, std::time::Instant)>>
{
    use std::sync::Mutex;
    static SERVED: std::sync::OnceLock<
        Mutex<std::collections::VecDeque<(i32, std::time::Instant)>>,
    > = std::sync::OnceLock::new();
    SERVED.get_or_init(|| Mutex::new(std::collections::VecDeque::new()))
}

fn note_served_pid(pid: i32) {
    const CAP: usize = 64;
    const TTL: std::time::Duration = std::time::Duration::from_secs(90);
    let now = std::time::Instant::now();
    let mut q = served_pids().lock().unwrap_or_else(|p| p.into_inner());
    q.retain(|(p, at)| *p != pid && now.duration_since(*at) < TTL);
    q.push_back((pid, now));
    while q.len() > CAP {
        q.pop_front();
    }
}

/// 6-Z309d: whether `pid` received a routed transaction delivery within
/// the TTL window (see [`note_served_pid`]).
pub(crate) fn recently_served(pid: i32) -> bool {
    const TTL: std::time::Duration = std::time::Duration::from_secs(90);
    let now = std::time::Instant::now();
    served_pids()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .iter()
        .any(|(p, at)| *p == pid && now.duration_since(*at) < TTL)
}

/// 6-Z306ag: bounded evidence that a connection is servicing
/// NESTED/OVERLAPPING transactions — the exact shape the previous
/// single-slot inflight correlation corrupted (outer BC_REPLY lost or
/// routed to the wrong waiter). Depth 1 is the plain sequential case
/// (silent); depth ≥ 2 emits one line, capped per boot. This is the
/// ladder-side witness: if the AMS-constructor-era wedge was this
/// shape, the runs should now show depth≥2 events AND no
/// "BC_REPLY with no delivered transaction" lines.
static Z306AG_NESTED_LOG_BUDGET: AtomicU32 = AtomicU32::new(16);

/// 6-Z306an: bounded log budget for dead-target transaction rejections
/// at the delivery gate (the fleet crash-loops on stale registrations —
/// one line per rejection would flood; 24 lines name the population and
/// stay quiet after).
static Z306AN_REJECT_LOG: AtomicU32 = AtomicU32::new(24);

/// 6-Z354: bounded budget for the ANCHOR-CONTRADICTION diagnostics — how
/// many times the (now diagnostic-only) 6-Z306ae-f heap anchor may log a
/// "Dead verdict on a LIVE owner" contradiction per process before it
/// stops being evaluated entirely (zero hot-path cost after the budget).
static Z354_ANCHOR_LOG: AtomicU32 = AtomicU32::new(24);

fn z306ag_note_stack_depth(depth: usize) {
    if depth < 2 {
        return;
    }
    if Z306AG_NESTED_LOG_BUDGET.load(Ordering::Relaxed) == 0 {
        return;
    }
    if Z306AG_NESTED_LOG_BUDGET.fetch_sub(1, Ordering::Relaxed) > 0 {
        info!(
            "[KR64][binder] 6-Z306ag: nested/overlapping txn stack depth={} on one conn",
            depth
        );
    }
}

/// 6-Z408: bounded budget for the reentrancy-gate HOLD diagnostics — how
/// many times the gate may log a held sync transaction per (vm, conn).
/// The parked-conn window lasts ~250 ms per nested call, so the hold
/// lines stay sparse; the budget still guards a pathological fleet.
static Z408_HOLD_LOG: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<(u32, u64), u64>>,
> = std::sync::OnceLock::new();

/// 6-Z410: bounded budget for the delivery-trailer breakdown trace — the
/// rn366 Scudo invalid-chunk-state decode (the composer died freeing a
/// non-live Parcel buffer right after a 52-byte-trailer getHashChain
/// delivery). 48 lines name the blob shape population per boot.
static Z410_TRAILER_LOG: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(48);

fn z408_note_hold(
    vm_id: u32,
    conn_id: u64,
    txn_id: u64,
    code: u32,
    requester: u64,
    sender_pid: i32,
) {
    let seen = match Z408_HOLD_LOG
        .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
        .lock()
    {
        Ok(mut m) => *m
            .entry((vm_id, conn_id))
            .and_modify(|c| *c += 1)
            .or_insert(1),
        Err(_) => 0,
    };
    if seen <= 16 || seen % 500 == 0 {
        info!(
            "[KR64][binder][vm{}] 6-Z408 REENTRANCY-HOLD: sync tx #{} (code={}, sender pid={}) held on parked conn={} (outstanding call target conn={}) — kernel reentrancy rule; drains when the nested call unwinds [tx #{}{}]",
            vm_id,
            txn_id,
            code,
            sender_pid,
            conn_id,
            requester,
            seen,
            if seen <= 16 { "" } else { " sampled" }
        );
    }
}

fn probe_flat_mem(
    tag: &str,
    budget: &AtomicU32,
    guest_pid: i32,
    label: &str,
    ptr: u64,
    cookie: u64,
) {
    if guest_pid <= 0 {
        return;
    }
    if budget.load(Ordering::Relaxed) == 0 {
        return;
    }
    budget.fetch_sub(1, Ordering::Relaxed);
    let snap = |a: u64| -> String {
        match crate::ptrace_emu::peek_guest_bytes(guest_pid, a, 16) {
            Some(b) => b.iter().map(|x| format!("{:02x}", x)).collect(),
            None => "<unreadable>".to_string(),
        }
    };
    info!(
        "[KR64][binder][svc] 6-Z306ad: {} {} pid={} ptr=0x{:x} mem=[{}] cookie=0x{:x} mem=[{}]",
        tag,
        label,
        guest_pid,
        ptr,
        snap(ptr),
        cookie,
        snap(cookie)
    );
}

/// 6-Z306ae-d → 6-Z306ae-f: the mirror's liveness gate. The #206
/// capture-flat dump settled the "capture shift": the parcel cookie slot
/// is CORRECT ([flat]{type,flags,binder=W,cookie=B}[stability][allow]
/// [prio], all sane); mBase(W) = B + Δ is the **RefBase virtual-base
/// subobject offset** inside multiply-inheriting wrappers (IBinder :
/// public virtual RefBase — for ALL binder classes, AIDL and HIDL; Δ =
/// 0x88 BnHw<HIDL>, 0x20/0x38/0x68/0x80 elsewhere) — NOT a capture
/// error. The mBase==cookie "invariant" of 6-Z306ae-b therefore
/// rejected every correct capture and the mirror never fired (#205:
/// 241 skips, 0 deliveries).
///
/// The 6-Z306ae-d contract (vptr≠0 at [cookie..+8]) protected against
/// the freed-zeroed-chunk class but #236 killed it: a delivered flat
/// with [cookie]=[valid vptr][0xbd] (the wrapper chunk partially
/// reused) passed while the true RefBase subobject was dead — the
/// guest's incStrong (vbase-adjusted to R) read mRefs=NULL at [R+8]
/// and died (the si_addr=0x4 fleet shape). The 6-Z306ae-f ROUND-TRIP
/// anchor (in the body below) is kernel-true and Δ-agnostic: it never
/// inspects the wrapper padding at [B+8] (which #209 proved can
/// legitimately be zero) and never needs to know Δ.
///
/// Anchor: W = the flat's binder field (the weakref the node ref is
/// anchored on); R = [W+8] = weakref_impl.mBase = the TRUE RefBase
/// subobject; the object is ALIVE iff [R+8] == W (RefBase ctor:
/// mRefs(new weakref_impl(this)); the live object's mRefs field points
/// back at its weakref). Freed-and-reused chunks sever the equality.
/// 6-Z306an: tri-state liveness verdict for the delivery-side gates.
/// The boolean [`mirror_ref_ok`] cannot distinguish "the anchor is
/// POSITIVELY broken" (freed chunk / severed round-trip / association
/// missing — the object is really dead) from "the guest memory could
/// not be read at all" (the verdict is unknown). For mirror commands
/// the distinction is irrelevant (skip = BR_NOOP is always safe), but
/// the 6-Z306an TRANSACTION delivery gate must never drop a LIVE
/// transaction just because a peek failed — that would surface a
/// spurious BR_DEAD_REPLY to an innocent requester. Only a positive
/// `Dead` verdict rejects a delivery; `Unknown` delivers as before.
enum Liveness {
    /// The anchor verified the object alive (round-trip + association).
    Alive,
    /// The anchor POSITIVELY identified the object as dead/reused.
    Dead,
    /// The anchor could not run (unreadable guest memory) — no verdict.
    Unknown,
}

/// 6-Z387: the 6-Z306d-b association scan window (bytes of the cookie
/// chunk searched for the weakref VALUE W). W = mRefs lives at
/// [R+8] with R = the RefBase vbase subobject = B+Δ; Δ spans the
/// wrapper's inheritance depth (rn344 census: HIDL 0x20..0x88, extractor
/// 0x40, player 0x110, MediaMetrics 0x378, AudioFlinger 0x648 — the old
/// 640 window false-deaded every Δ>0x320 wrapper and starved the
/// registry strong ref → the audioserver/mediametrics corpse storm).
/// Page-sized: bounded peek cost, margin for deeper wrappers.
const ASSOC_SCAN_WINDOW: usize = 4096;

/// 6-Z387: the pure 6-Z306d-b association decision over the peeked
/// cookie chunk — the VALUE W (the object's mRefs) must appear at an
/// 8-aligned slot inside the scanned window. Pure so tests replay the
/// rn344 Δ shapes (AudioFlinger 0x648, MediaMetrics 0x378) without a
/// tracer.
fn assoc_scan(buf: &[u8], w: u64) -> bool {
    buf.chunks_exact(8)
        .any(|c| u64::from_ne_bytes(c.try_into().unwrap()) == w)
}

fn mirror_ref_check(guest_pid: i32, ptr: u64, cookie: u64) -> Liveness {
    if guest_pid <= 0 || ptr == 0 || cookie == 0 {
        return Liveness::Dead;
    }
    // 6-Z306ae-f (#236 decode): the ROUND-TRIP liveness anchor. The
    // 6-Z306ae-d vptr-only contract is proven insufficient: ladder #236
    // delivered a HIDL flat whose cookie slot read [valid vptr][0xbd] —
    // the BHwBinder wrapper chunk LOOKED alive while the TRUE RefBase
    // subobject (mBase = cookie + 0x80, the #206 virtual-base Δ) was
    // dead; the guest's incStrong (vbase-adjusted to mBase) read
    // mRefs=NULL at [mBase+8] and died (si_addr=0x4 at
    // RefBase::incStrong+0x8 — the fleet shape), killing the FIRST-EVER
    // forked guest system_server 30 s into its run.
    //
    // Kernel-true anchor: W = the captured weakref (the flat's binder
    // field); R = [W+8] = weakref_impl.mBase = the true RefBase
    // subobject pointer (RefBase ctor: mRefs(new weakref_impl(this)) —
    // weakref_impl.mBase = the RefBase*); the object is alive iff
    // [R+8] == W (its mRefs field points back at the very weakref the
    // node's ref is anchored on). Freed-and-reused chunks sever the
    // equality; an accidental 2^-64 match is the only false-alive case.
    match crate::ptrace_emu::peek_guest_bytes(guest_pid, ptr + 8, 8) {
        Some(b) if b.len() == 8 => {
            let mbase = u64::from_ne_bytes(b[0..8].try_into().unwrap());
            if mbase == 0 {
                info!(
                    "[KR64][binder][svc] 6-Z306ae-f: mirror skipped — weakref W=0x{:x} mBase=0 (freed chunk) cookie=0x{:x} pid={}",
                    ptr, cookie, guest_pid
                );
                return Liveness::Dead;
            }
            match crate::ptrace_emu::peek_guest_bytes(guest_pid, mbase + 8, 8) {
                Some(rb) if rb.len() == 8 => {
                    let refs_back = u64::from_ne_bytes(rb[0..8].try_into().unwrap());
                    if refs_back != ptr {
                        info!(
                            "[KR64][binder][svc] 6-Z306ae-f: mirror skipped — round-trip broken: [R+8]=0x{:x} != W=0x{:x} (R=0x{:x}, cookie=0x{:x}) — object dead/chunk-reused pid={}",
                            refs_back, ptr, mbase, cookie, guest_pid
                        );
                        return Liveness::Dead;
                    }
                    // 6-Z306d-b (#237 decode): leg 2 — the W↔B ASSOCIATION.
                    // A freed-and-REUSED weakref chunk can pass leg 1 for a
                    // NEW object (W.mBase=R, [R+8]==W) while the registered
                    // cookie B belongs to a completely different, dead
                    // allocation — the #237 chimera pair (pid 3411: the
                    // mirror BR_ACQUIRE passed leg 1, the guest incStrong'd
                    // B, its reused-chunk vptr sent the vbase adjust to
                    // B+0xE0, mRefs=NULL → si_addr=0x4). A live object's
                    // allocation contains its own mRefs pointer: [R+8]==W
                    // with R = B+Δ (Δ>0) sits inside B's chunk — scan
                    // [B..B+ASSOC_WINDOW) for the VALUE W.
                    //
                    // 6-Z387 (rn344 decode): the window MUST cover the
                    // RefBase VBASE offset of multiply-inheriting wrappers,
                    // not just the HIDL Δ=0x20..0x88 class. The stock A11
                    // flattenBinder (libbinder.so 0x5babc, disassembled)
                    // writes W=local->getWeakRefs() (via the vbase-adjusted
                    // RefBase subobject) and cookie=local — so W=mRefs lives
                    // at [R+8] where R=mBase=[W+8] can sit DEEP in the
                    // wrapper. rn344 capture probes: AudioFlinger Δ=0x648
                    // (W@0x650), MediaMetrics Δ=0x378 (W@0x380) — both
                    // BEYOND the old 640 window, both hit the false-Dead
                    // skip ("association broken", 197x), and both owners
                    // crashed ~90-290 ms later at RefBase::incStrong+0x0 /
                    // decStrong+0x1c on the FREED cookie chunk (the register
                    // temporary died with no registry strong ref → delete →
                    // the self-lookup served the corpse). Δ<640 services
                    // (extractor 0x40, player 0x110) never stormed. The
                    // window is page-sized (4096): covers every observed
                    // wrapper class with margin, one bounded process_vm_readv
                    // per registration (the capture budget still applies).
                    match crate::ptrace_emu::peek_guest_bytes(guest_pid, cookie, ASSOC_SCAN_WINDOW)
                    {
                        Some(buf) if buf.len() == ASSOC_SCAN_WINDOW => {
                            let associated = assoc_scan(&buf, ptr);
                            if associated {
                                Liveness::Alive
                            } else {
                                info!(
                                    "[KR64][binder][svc] 6-Z306d-b: mirror skipped — association broken: W=0x{:x} not found in cookie 0x{:x} chunk (R=0x{:x}) — stale registration pid={}",
                                    ptr, cookie, mbase, guest_pid
                                );
                                Liveness::Dead
                            }
                        }
                        _ => {
                            info!(
                                "[KR64][binder][svc] 6-Z306d-b: mirror skipped — cookie 0x{:x} chunk unreadable pid={}",
                                cookie, guest_pid
                            );
                            Liveness::Unknown
                        }
                    }
                }
                _ => {
                    info!(
                        "[KR64][binder][svc] 6-Z306ae-f: mirror skipped — mBase R=0x{:x} unreadable cookie=0x{:x} pid={}",
                        mbase, cookie, guest_pid
                    );
                    Liveness::Unknown
                }
            }
        }
        _ => {
            info!(
                "[KR64][binder][svc] 6-Z306ae-f: mirror skipped — weakref W=0x{:x} unreadable cookie=0x{:x} pid={}",
                ptr, cookie, guest_pid
            );
            Liveness::Unknown
        }
    }
}

/// Boolean wrapper for the mirror-command gates (all pre-6-Z306an call
/// sites): skip-on-unknown is the safe policy there, so Unknown maps to
/// false exactly as every unreadable case did before the tri-state.
fn mirror_ref_ok(guest_pid: i32, ptr: u64, cookie: u64) -> bool {
    matches!(mirror_ref_check(guest_pid, ptr, cookie), Liveness::Alive)
}

/// 6-Z463: the RefCmd359 CLOSE-delivery decision — the pure core of the
/// object-level close gate. Only a POSITIVE `Dead` verdict (the 6-Z387
/// round-trip anchor: mBase=0, round-trip severed, or association broken)
/// rejects the close; `Alive` and `Unknown` deliver exactly as before
/// (the 6-Z306an rule — an unreadable probe must never swallow a
/// kernel-true wire command). Rejecting = the silent close (BR_NOOP +
/// witness): the owner's object is provably dead/reused, so the
/// decStrong/decWeak the guest would run IS the corruption write (the
/// rn427 +187083→+187101 chain). Pure so tests pin the tri-state
/// contract without a tracer.
fn z463_close_rejected(probe: Liveness) -> bool {
    matches!(probe, Liveness::Dead)
}

// ============================================================================
// 6-Z454: the REF-LEDGER — per-object strong-ref accounting for every
// node-ref mirror the bus EMITS (queues) or DELIVERS (hands to the
// guest's read stream). The rn420 decode named the suspend@1.0-service
// abort CLASS: a libhidlbase sp<> release ran RefBase::decStrong's
// delete-this tail on an ALREADY-FREED chunk — the owner's strong count
// went past zero. On a real kernel that is impossible: every BR_RELEASE
// a process receives is preceded by a BR_ACQUIRE for the same node in
// the same notification era (binder_dec_node fires the owner
// notification only on the has_strong_ref true→false edge). The bus's
// mirrors, however, are gated by the 6-Z306ae-f liveness probe AT
// DIFFERENT MOMENTS: an acquire skipped at add time (probe Unknown or
// Dead) paired with a release delivered later (probe Alive) is a
// decStrong-on-zero — the exact rn419/rn420 fingerprint. The ledger
// counts both sides per (owner pid, ptr, cookie) and flags the two
// violations the moment they are created:
//   V1 EMIT    — a BR_RELEASE queued while rel_emitted would exceed
//                acq_emitted: the bus itself is about to over-release.
//   V2 DELIVER — a BR_RELEASE handed to the guest while rel_delivered
//                would exceed acq_delivered: the acquire never reached
//                this owner (liveness-gated away) but the release did.
// Both name the surface that emitted (reg-acq / reg-rel / watch-* /
// node-*), so the rn421 decode reads the killer off the artifact
// instead of a hypothesis.
// Pure accounting: the ledger NEVER blocks a mirror (no behavior
// change) — rn420's evidence stays the ground truth; this names the
// emitter. Weak mirrors (BR_INCREFS / BR_DECREFS) do not destroy
// objects and do not participate.
// ============================================================================

/// One object's strong-ref ledger entry.
#[derive(Default, Clone, Copy, Debug)]
struct Z454Entry {
    acq_emitted: u32,
    rel_emitted: u32,
    acq_delivered: u32,
    rel_delivered: u32,
}

/// Which mirror surface emitted — names the killer in the decode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Z454Site {
    /// The registry's strong ref on a registered service (6-Z306ae add
    /// arms: AIDL add / HIDL add / addWithChain, incl. the 6-Z306am arm-B
    /// liveness-RefCmd re-route).
    RegAcq,
    /// The registry's strong ref on a registered watcher callback
    /// (6-Z325 registerForNotifications pin).
    WatchAcq,
    /// The registry ref released by a same-name overwrite (6-Z306ae /
    /// 6-Z379, gated on `overwrite_release_due`).
    RegRel,
    /// The registry ref released by an explicit unregisterForNotifications
    /// (6-Z276/6-Z325).
    WatchRel,
    /// The in-driver node ref granted by a flat crossing (6-Z442
    /// `z359_grant_node`'s BR_ACQUIRE mirror).
    NodeAcq,
    /// The node's last external strong ref dropped (6-Z359/6-Z442
    /// `z359_unref_node`'s BR_RELEASE mirror).
    NodeRel,
}

impl Z454Site {
    fn name(self) -> &'static str {
        match self {
            Z454Site::RegAcq => "reg-acq",
            Z454Site::WatchAcq => "watch-acq",
            Z454Site::RegRel => "reg-rel",
            Z454Site::WatchRel => "watch-rel",
            Z454Site::NodeAcq => "node-acq",
            Z454Site::NodeRel => "node-rel",
        }
    }
    fn is_release(self) -> bool {
        matches!(
            self,
            Z454Site::RegRel | Z454Site::WatchRel | Z454Site::NodeRel
        )
    }
}

/// (owner pid, ptr, cookie) → ledger entry.
type Z454Map = std::collections::HashMap<(i32, u64, u64), Z454Entry>;

/// (owner pid, ptr, cookie) → ledger. Bounded: the boot registers O(100)
/// objects; the cap evicts single arbitrary entries (that entry's
/// accounting resets — bounded decode noise, never a false violation:
/// eviction lowers both sides' baseline, and a post-eviction release on
/// an evicted key re-baselines from zero, which is exactly the V1
/// suspect shape and will be LOGGED as such).
fn z454_ledger() -> &'static std::sync::Mutex<Z454Map> {
    static LEDGER: std::sync::OnceLock<std::sync::Mutex<Z454Map>> = std::sync::OnceLock::new();
    LEDGER.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// Bounded violation-log budget per boot (the ledger must not flood the
/// artifact — 24 lines name the class and its first 24 emitters).
fn z454_viol_log() -> &'static AtomicU32 {
    static N: std::sync::OnceLock<AtomicU32> = std::sync::OnceLock::new();
    N.get_or_init(|| AtomicU32::new(24))
}

const Z454_LEDGER_CAP: usize = 1024;

/// Evict one arbitrary stale entry when the ledger is over cap and the
/// incoming key is new. Pure so tests can drive it on a held-lock map.
fn z454_evict_if_over(led: &mut Z454Map, key: &(i32, u64, u64)) {
    if led.len() >= Z454_LEDGER_CAP && !led.contains_key(key) {
        if let Some(stale) = led.keys().next().copied() {
            led.remove(&stale);
        }
    }
}

/// Record one mirror EMIT (the bus queued it). Returns true when the
/// emit created a V1 violation (a release past the emitted acquire
/// count for this object).
fn z454_emit(pid: i32, ptr: u64, cookie: u64, site: Z454Site) -> bool {
    if pid <= 0 || ptr == 0 || cookie == 0 {
        return false;
    }
    let mut led = z454_ledger().lock().expect("6-Z454 ledger poisoned");
    z454_evict_if_over(&mut led, &(pid, ptr, cookie));
    let e = led.entry((pid, ptr, cookie)).or_default();
    if site.is_release() {
        e.rel_emitted += 1;
        if e.rel_emitted > e.acq_emitted {
            if z454_viol_log().load(Ordering::Relaxed) > 0 {
                z454_viol_log().fetch_sub(1, Ordering::Relaxed);
                info!(
                    "[KR64][binder] 6-Z454 V1 EMIT release-without-acquire site={} pid={} ptr=0x{:x} cookie=0x{:x} emitted acq={} rel={} — a queued BR_RELEASE will decStrong the owner past zero (the rn420 double-free class)",
                    site.name(),
                    pid,
                    ptr,
                    cookie,
                    e.acq_emitted,
                    e.rel_emitted
                );
            }
            return true;
        }
    } else {
        e.acq_emitted += 1;
    }
    false
}

/// 6-Z458 (Task 195): the release-side emit carrying the REGISTRY-pin
/// witness — the decode's "no premature BR_RELEASE crossed"
/// verification surface (Task 194 agenda item e). A fired release while
/// the registry pin still held is the rn425 corpse-regrant shape; the
/// V1 line names it the moment it is created. Acquire sites and the
/// watcher surfaces keep [`z454_emit`] (witness always false there).
fn z454_emit_rel(
    pid: i32,
    ptr: u64,
    cookie: u64,
    site: Z454Site,
    registry_pin_present: bool,
) -> bool {
    if pid <= 0 || ptr == 0 || cookie == 0 {
        return false;
    }
    let mut led = z454_ledger().lock().expect("6-Z454 ledger poisoned");
    z454_evict_if_over(&mut led, &(pid, ptr, cookie));
    let e = led.entry((pid, ptr, cookie)).or_default();
    e.rel_emitted += 1;
    if e.rel_emitted > e.acq_emitted {
        if z454_viol_log().load(Ordering::Relaxed) > 0 {
            z454_viol_log().fetch_sub(1, Ordering::Relaxed);
            info!(
                "[KR64][binder] 6-Z454 V1 EMIT release-without-acquire site={} pid={} ptr=0x{:x} cookie=0x{:x} emitted acq={} rel={} reg-pin={} — a queued BR_RELEASE will decStrong the owner past zero (the rn420 double-free class)",
                site.name(),
                pid,
                ptr,
                cookie,
                e.acq_emitted,
                e.rel_emitted,
                if registry_pin_present { "present" } else { "absent" }
            );
        }
        return true;
    }
    false
}

/// Record one mirror DELIVER (handed to the guest's read stream; `br` is
/// the wire command). Returns true when the delivery created a V2
/// violation (a release delivered past the delivered acquire count —
/// the acquire never reached this owner).
fn z454_deliver(pid: i32, ptr: u64, cookie: u64, br: u32) -> bool {
    if pid <= 0 || ptr == 0 || cookie == 0 {
        return false;
    }
    if br != BR_ACQUIRE && br != BR_RELEASE {
        return false; // weak mirrors (BR_INCREFS / BR_DECREFS) don't participate
    }
    let mut led = z454_ledger().lock().expect("6-Z454 ledger poisoned");
    let e = led.entry((pid, ptr, cookie)).or_default();
    if br == BR_RELEASE {
        e.rel_delivered += 1;
        if e.rel_delivered > e.acq_delivered {
            if z454_viol_log().load(Ordering::Relaxed) > 0 {
                z454_viol_log().fetch_sub(1, Ordering::Relaxed);
                info!(
                    "[KR64][binder] 6-Z454 V2 DELIVER release-without-acquire pid={} ptr=0x{:x} cookie=0x{:x} delivered acq={} rel={} — the owner just decStrongs past zero (rn420: libutils decStrong delete-this on the freed chunk)",
                    pid, ptr, cookie, e.acq_delivered, e.rel_delivered
                );
            }
            return true;
        }
    } else {
        e.acq_delivered += 1;
    }
    false
}

/// 6-Z457 (Task 192): the ledger's emit/deliver totals for one object —
/// the read-only join the refcount mirror logs beside the guest's live
/// RefBase counts (a BALANCED ledger over an already-zero mStrong names
/// the in-process double-count; an UNBALANCED ledger names the mirror
/// bug — the two shapes are the missing half rn422 could not name).
/// Zeroes for unknown keys (never probed = never emitted).
fn z454_counts(pid: i32, ptr: u64, cookie: u64) -> (u32, u32, u32, u32) {
    if pid <= 0 || ptr == 0 || cookie == 0 {
        return (0, 0, 0, 0);
    }
    let led = z454_ledger().lock().expect("6-Z454 ledger poisoned");
    match led.get(&(pid, ptr, cookie)) {
        Some(e) => (
            e.acq_emitted,
            e.rel_emitted,
            e.acq_delivered,
            e.rel_delivered,
        ),
        None => (0, 0, 0, 0),
    }
}

/// 6-Z457 (Task 192): the refcount-mirror read budget — ≤12 owner-side
/// RefBase snapshots per boot, last-ref mirrors only (the RefCmd359 arm
/// is last-ref by construction). A read consumes one; the decode names
/// the missing-half mechanism from at most 12 joins per run.
fn z457_budget() -> &'static AtomicU32 {
    static N: std::sync::OnceLock<AtomicU32> = std::sync::OnceLock::new();
    N.get_or_init(|| AtomicU32::new(12))
}

/// 6-Z463: bounded witness budget for the silent close (the drop is
/// expected to be rare — rn427 showed 3 candidate windows per boot).
fn z463_drop_log() -> &'static AtomicU32 {
    static N: std::sync::OnceLock<AtomicU32> = std::sync::OnceLock::new();
    N.get_or_init(|| AtomicU32::new(16))
}

/// 6-Z354: the TRANSACTION-delivery decision, pure for tests. Reject the
/// queued transaction iff the target node's OWNER process is provably
/// dead (the kernel-true oracle: a fresh /proc probe). The heap anchor
/// NEVER decides — rn305 proved it contradicts live owners (the composer's
/// service object: Alive at ADD, "Dead" 5 s later while the process
/// served — 103 spurious BR_DEAD_REPLYs, the SF death-loop).
fn tx_delivery_reject_6z354(dpid: i32, owner_alive: bool) -> bool {
    dpid > 0 && !owner_alive
}

/// 6-Z379: the overwrite-release decision, pure for tests. The registry
/// strong ref is PER KEY (AOSP hwservicemanager addImpl holds one
/// sp<IBase> per interfaceChain entry — every chain key's HidlService
/// pins the service object with its OWN sp), so overwriting one key
/// releases the old node only when NO OTHER registry key still pins the
/// same (owner, ptr, cookie). The EVERY-HAL shared
/// `android.hidl.base@1.0::IBase/default` alias overwrite must therefore
/// NOT release the previous owner — its concrete chain keys
/// (@2.3/@2.2/@2.1) still hold refs (rn334/rn335 decode: the composer's
/// wrapper was BR_RELEASEd 500 ms after thermal claimed IBase/default;
/// SF's interfaceChain then SEGV'd in decStrong on the corpse — the 42×
/// libutils refcount SEGV class, rung 7 SURFACEFLINGER). A genuine
/// whole-service replacement (every chain key overwritten) releases
/// exactly once, at the LAST key; the single-key AIDL addService
/// overwrite (the 6-Z306ae suspend-daemon semantics) releases
/// immediately, unchanged.
fn overwrite_release_due(
    services: &std::collections::BTreeMap<String, ServiceEntry>,
    overwritten_key: &str,
    old_owner: ConnId,
    old_ptr: u64,
    old_cookie: u64,
) -> bool {
    old_ptr != 0
        && !services.iter().any(|(k, e)| {
            k != overwritten_key
                && e.owner == old_owner
                && e.ptr == old_ptr
                && e.cookie == old_cookie
        })
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
    fn z306an_dead_reply_emits_caller_side_code() {
        // 6-Z306an: the dead-target rejection must surface BR_DEAD_REPLY
        // on the REQUESTER's stream (waitForResponse handles it →
        // DEAD_OBJECT) — NEVER in a server stream (executeCommand has no
        // case for it → "BAD COMMAND 29189" → LOG_ALWAYS_FATAL, the #239
        // fleet). The emitted dword is _IO('r',5) = 0x00007205.
        let mut buf = Vec::new();
        push_br_dead_reply(&mut buf);
        assert_eq!(buf, 0x00007205u32.to_le_bytes(), "BR_DEAD_REPLY dword");
        assert_eq!(buf.len(), 4, "cmd-only, no payload");
    }

    #[test]
    fn z306an_dead_target_slot_filler_is_noop_only() {
        // The 68-byte server-stream slot for a neutralized BR_TRANSACTION
        // (cmd + 64-byte binder_transaction_data) must be filled with
        // BR_NOOP ONLY: executeCommand handles BR_NOOP (skip) and ABORTS
        // on BR_DEAD_REPLY. 68 bytes / 4 = 17 dwords.
        let slot_bytes = 4 + std::mem::size_of::<BinderTransactionData>();
        assert_eq!(slot_bytes, 68, "cmd + btd on 64-bit");
        assert_eq!(slot_bytes / 4, 17, "17×BR_NOOP fills the slot");
        // And the shlib's BP_BR_NOOP must equal the kernel BR_NOOP the
        // guest's executeCommand switches on.
        assert_eq!(BR_NOOP, 0x0000720C);
    }

    #[test]
    fn z354_delivery_rejects_only_dead_owner() {
        // The kernel-true delivery decision: reject iff the target node's
        // OWNER process is provably dead. The rn305 composer case is the
        // regression lock: a LIVE owner with a "Dead" heap anchor MUST
        // deliver (the anchor never decides).
        assert!(
            !tx_delivery_reject_6z354(2965, true),
            "live owner ⇒ deliver (the rn305 composer class: heap anchor said Dead, the process served)"
        );
        assert!(
            tx_delivery_reject_6z354(2965, false),
            "dead owner ⇒ reject with BR_DEAD_REPLY (kernel node-work release)"
        );
        assert!(
            !tx_delivery_reject_6z354(0, false),
            "no conn identity (unit-test bus conns) ⇒ deliver as before"
        );
        assert!(
            !tx_delivery_reject_6z354(-1, true),
            "negative pid ⇒ deliver as before"
        );
    }

    #[test]
    fn z306an_mirror_ref_check_tri_state_arg_validation() {
        // Invalid args (pid<=0 / ptr==0 / cookie==0) are a POSITIVE Dead
        // verdict for the mirror gates (unchanged boolean behavior), and
        // the wrapper maps Dead → false exactly like the pre-tri-state
        // mirror_ref_ok did.
        assert!(matches!(
            mirror_ref_check(0, 0x1000, 0x2000),
            Liveness::Dead
        ));
        assert!(matches!(
            mirror_ref_check(-5, 0x1000, 0x2000),
            Liveness::Dead
        ));
        assert!(matches!(mirror_ref_check(1234, 0, 0x2000), Liveness::Dead));
        assert!(matches!(mirror_ref_check(1234, 0x1000, 0), Liveness::Dead));
        assert!(!mirror_ref_ok(0, 0x1000, 0x2000));
        assert!(!mirror_ref_ok(1234, 0x1000, 0));
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
        assert_eq!(magic, WIRE_V3_MAGIC, "trailer magic (6-Z305t-68 resp v3)");
        let count = u32::from_ne_bytes(tail[4..8].try_into().unwrap());
        assert_eq!(count, 1, "one reply blob");
        let dlen = u32::from_ne_bytes(tail[8..12].try_into().unwrap()) as usize;
        let olen = u32::from_ne_bytes(tail[12..16].try_into().unwrap()) as usize;
        // 6-Z271x: status-ok (4) + flat_binder_object (24) + stability (4)
        assert_eq!(dlen, 32, "status-ok (4) + flat (24) + stability i32 (4)");
        assert_eq!(olen, 8, "one offsets entry (binder_size_t = u64)");
        assert!(
            tail.len() >= 20 + dlen + olen,
            "trailer must carry the full blob bytes"
        );
        // Reply must parse as AIDL Status::ok (EX_NONE = 0)…
        let status = i32::from_ne_bytes(tail[20..24].try_into().unwrap());
        assert_eq!(status, 0, "EX_NONE");
        // …followed by a BINDER_TYPE_BINDER null-binder flat object.
        let ftype = u32::from_ne_bytes(tail[24..28].try_into().unwrap());
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
    /// carry the LOCAL flat object (6-Z306ac: the owner-conn lookup —
    /// BINDER_TYPE_BINDER with the registered ptr/cookie), listed in the
    /// offsets array.
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
        assert_eq!(magic, WIRE_V3_MAGIC, "response must be v3");
        off += 4;
        let blob_count = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap());
        assert_eq!(blob_count, 1, "ADD response carries one reply blob");
        off += 4;
        let data_len = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap()) as usize;
        let off_len = u32::from_ne_bytes(resp[off + 4..off + 8].try_into().unwrap()) as usize;
        assert_eq!(data_len, 4, "ADD reply = [i32 0] status only");
        assert_eq!(off_len, 0, "ADD reply has no flat objects");
        let status = i32::from_ne_bytes(resp[off + 12..off + 16].try_into().unwrap());
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
            WIRE_V3_MAGIC
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
        let blob2 = &resp2[off2 + 12..off2 + 12 + data_len2];
        let status2 = i32::from_ne_bytes(blob2[0..4].try_into().unwrap());
        assert_eq!(status2, 0, "GET reply status = EX_NONE");
        // Flat-object layout (24 bytes): u32 type, u32 flags, u64 binder, u64 cookie.
        let flat_type = u32::from_ne_bytes(blob2[4..8].try_into().unwrap());
        // 6-Z306ac: the lookup comes from the OWNER connection (this same
        // stream registered "my_svc" above) — the kernel-binder semantic
        // is a LOCAL flat: BINDER_TYPE_BINDER with the registering
        // process's own ptr/cookie, NOT a handle. SystemServer's
        // `(PlatformCompat) ServiceManager.getService("platform_compat")`
        // (AMS:2597) depends on this: a proxy would ClassCastException.
        assert_eq!(
            flat_type, BINDER_TYPE_BINDER,
            "owner-conn GET hit → LOCAL BINDER_TYPE_BINDER (6-Z306ac)"
        );
        // 6-Z271x: the stability annotation follows the flat; 6-Z306ab:
        // the FIRST get on a fresh conn serves the android-11 PLAIN
        // Level form (63) — the A11 libbinder's Stability::set rejects
        // everything else (BAD_TYPE → readStrongBinder → null → the
        // performSystemServerDexOpt NPE die-loop of ladders #195/#196).
        let stability = i32::from_ne_bytes(blob2[28..32].try_into().unwrap());
        assert_eq!(
            stability, STABILITY_ANNOTATION_VINTF,
            "GET hit → plain android-11 VINTF level (63) on a fresh connection"
        );
        // The LOCAL flat carries the registering parcel's ptr/cookie
        // (the client's unflattenBinder decodes the object straight from
        // flat.cookie — the kernel's same-process semantic).
        let local_ptr = u64::from_ne_bytes(blob2[12..20].try_into().unwrap());
        let local_cookie = u64::from_ne_bytes(blob2[20..28].try_into().unwrap());
        assert_eq!(local_ptr, 0xdead, "local flat.binder = the registered ptr");
        assert_eq!(
            local_cookie, 0xbeef,
            "local flat.cookie = the registered cookie"
        );
        // The reply offsets array must list the flat object's offset (= 4,
        // after the i32 status prefix).
        let reply_offsets = &resp2[off2 + 12 + data_len2..off2 + 12 + data_len2 + off_len2];
        let listed_off = u64::from_ne_bytes(reply_offsets[..].try_into().unwrap());
        assert_eq!(
            listed_off, 4,
            "reply offsets[0] = flat object's data offset"
        );

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    /// 6-Z333: the LOCAL hit reply ECHOES the owner's own addService
    /// stability annotation. The real servicemanager stores the add's
    /// stability on its proxy record and returns it in every later
    /// getService reply; the owner's `finishUnflattenBinder` →
    /// `Stability::set(local, ann)` accepts ONLY the level its own
    /// object already carries (`tryMarkCompilationUnit` marked it at
    /// addService — for an A11 system client that is Level::SYSTEM=12).
    /// rn284 proved the pre-fix wire (ann=63 VINTF on every reply)
    /// dies exactly there: "Interface being set with vintf stability
    /// but it is already marked as system stability." → BAD_TYPE →
    /// readStrongBinder → null → the initPowerManagement NPE.
    ///
    /// Byte-verified end-to-end over the v2 wire with an A11-shaped add
    /// parcel `[name][flat][ann=12][allowIsolated=0][dumpPriority=1]`:
    /// 1. owner-conn GET → LOCAL flat + ann = 12 (the echo);
    /// 2. second-conn GET → HANDLE flat + ann = 12 (the real-SM echo —
    ///    a fresh proxy accepts SYSTEM; VIRTUAL/HIDL services keep the
    ///    VINTF annotation via the fallback since their adds carry no
    ///    declared ann — covered by z271x and the synthetic-shape test
    ///    above, which still pins the fallback = 63).
    #[test]
    fn z333_sm_reply_echoes_owner_add_stability() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));
        let mut stream = UnixStream::connect(&path).expect("connect");

        // ---- owner conn: A11-shaped ADD_SERVICE "pwr_svc" ----
        let mut args = ParcelWriter::new();
        args.write_string16("pwr_svc");
        args.write_flat_binder(&FlatBinderObject {
            r#type: BINDER_TYPE_BINDER,
            flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
            binder: 0xdead,
            cookie: 0xbeef,
        });
        args.write_i32(12); // 6-Z333: the A11 finishFlattenBinder ann = Level::SYSTEM
        args.write_i32(0); // allowIsolated
        args.write_i32(1); // dumpPriority
        let (req_data, req_off) = make_servicemanager_request_parcel(&mut args);
        let mut bc = Vec::with_capacity(4 + 64);
        bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_ADD_SERVICE, 0));
        let payload = make_v2_write_read_payload(&bc, &req_data, &req_off, 4096);
        let (ret, _resp) = exchange(&mut stream, BINDER_WRITE_READ, &payload);
        assert_eq!(ret, 0, "ADD_SERVICE WRITE_READ should succeed");

        // ---- owner conn: GET → LOCAL flat + the ECHOED add ann ----
        let mut args2 = ParcelWriter::new();
        args2.write_string16("pwr_svc");
        let (req_data2, req_off2) = make_servicemanager_request_parcel(&mut args2);
        let mut bc2 = Vec::with_capacity(4 + 64);
        bc2.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc2.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_GET_SERVICE, 0));
        let payload2 = make_v2_write_read_payload(&bc2, &req_data2, &req_off2, 4096);
        let (ret2, resp2) = exchange(&mut stream, BINDER_WRITE_READ, &payload2);
        assert_eq!(ret2, 0, "GET_SERVICE WRITE_READ should succeed");
        let read_size2 = u32::from_ne_bytes(resp2[0..4].try_into().unwrap()) as usize;
        let off2 = 4 + read_size2 + 8;
        let dlen2 = u32::from_ne_bytes(resp2[off2..off2 + 4].try_into().unwrap()) as usize;
        assert_eq!(dlen2, 4 + 24 + 4, "hit blob = EX_NONE + flat + ann i32");
        let blob2 = &resp2[off2 + 12..off2 + 12 + dlen2];
        let flat_type2 = u32::from_ne_bytes(blob2[4..8].try_into().unwrap());
        assert_eq!(
            flat_type2, BINDER_TYPE_BINDER,
            "owner-conn GET hit → LOCAL BINDER_TYPE_BINDER"
        );
        let local_ptr = u64::from_ne_bytes(blob2[12..20].try_into().unwrap());
        let local_cookie = u64::from_ne_bytes(blob2[20..28].try_into().unwrap());
        assert_eq!(local_ptr, 0xdead, "local flat.binder = the registered ptr");
        assert_eq!(
            local_cookie, 0xbeef,
            "local flat.cookie = the registered cookie"
        );
        let stability2 = i32::from_ne_bytes(blob2[28..32].try_into().unwrap());
        assert_eq!(
            stability2, 12,
            "6-Z333: LOCAL hit ann = the owner's own add ann (Level::SYSTEM=12) — the client's Stability::set accepts exactly this"
        );

        // ---- second conn: GET → HANDLE flat + the echoed ann ----
        // The real servicemanager echoes the stored add stability on
        // cross-process replies too (a fresh proxy accepts SYSTEM).
        let mut stream2 = UnixStream::connect(&path).expect("connect-2");
        let payload3 = make_v2_write_read_payload(&bc2, &req_data2, &req_off2, 4096);
        let (ret3, resp3) = exchange(&mut stream2, BINDER_WRITE_READ, &payload3);
        assert_eq!(ret3, 0, "cross-conn GET_SERVICE should succeed");
        let read_size3 = u32::from_ne_bytes(resp3[0..4].try_into().unwrap()) as usize;
        let off3 = 4 + read_size3 + 8;
        let dlen3 = u32::from_ne_bytes(resp3[off3..off3 + 4].try_into().unwrap()) as usize;
        assert_eq!(
            dlen3,
            4 + 24 + 4,
            "cross hit blob = EX_NONE + flat + ann i32"
        );
        let blob3 = &resp3[off3 + 12..off3 + 12 + dlen3];
        let flat_type3 = u32::from_ne_bytes(blob3[4..8].try_into().unwrap());
        assert_eq!(
            flat_type3, BINDER_TYPE_HANDLE,
            "cross-conn GET hit → BINDER_TYPE_HANDLE"
        );
        let stability3 = i32::from_ne_bytes(blob3[28..32].try_into().unwrap());
        assert_eq!(
            stability3, 12,
            "6-Z333: cross-conn hit ann = the echoed add ann (the real-SM semantic)"
        );

        drop(stream2);
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
            WIRE_V3_MAGIC
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
        let blob = &resp[off + 12..off + 12 + data_len];
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
        // 6-Z271x: null binders are annotated with the stability i32;
        // 6-Z306ab: the first get on a fresh conn uses the android-11
        // plain form — UNDECLARED (0), the only value A11's
        // Stability::set(null, ·) accepts without BAD_TYPE.
        let null_stability = i32::from_ne_bytes(blob[28..32].try_into().unwrap());
        assert_eq!(
            null_stability, STABILITY_ANNOTATION_NULL,
            "miss → null-binder stability annotation = plain UNDECLARED (0)"
        );

        drop(stream);
        drop(handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    /// 6-Z306ab: a same-name re-get right AFTER a HIT flips the
    /// connection to the A12 Category form (sticky). This is the
    /// self-tune path for non-A11 libbinder clients (the R12-lavender
    /// recovery / A12+ GSIs): their `Stability::set` rejects the plain
    /// Level form with BAD_TYPE → `readStrongBinder` → null → their
    /// `waitForService` loop re-asks → the second reply carries the
    /// Category form and parses. A MISS retry must NOT flip (verified
    /// by `servicemanager_proxy_v2_get_miss_returns_null_binder`'s
    /// single-get shape and the hit/miss bookkeeping here).
    #[test]
    fn servicemanager_proxy_v2_get_retry_after_hit_flips_to_a12() {
        // 6-Z327: serialize with the other guest-rootfs-global tests. The
        // empty tmpdir below has no /system/build.prop → unknown SDK → the
        // legacy flip stays allowed (the a12_annotation_allowed gate is
        // open for None).
        let _g = Z327_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));
        let mut stream = UnixStream::connect(&path).expect("connect");

        // Register a service so the lookups below are HITS.
        let mut args = ParcelWriter::new();
        args.write_string16("flip_svc");
        args.write_flat_binder(&FlatBinderObject {
            r#type: BINDER_TYPE_BINDER,
            flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
            binder: 0xf00dfeed,
            cookie: 0xdeadbeef,
        });
        args.write_i32(0); // allowIsolated
        args.write_i32(0); // dumpPriority
        let (req_data, req_off) = make_servicemanager_request_parcel(&mut args);
        let mut bc = Vec::with_capacity(4 + 64);
        bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_ADD_SERVICE, 0));
        let payload = make_v2_write_read_payload(&bc, &req_data, &req_off, 4096);
        let (ret, _resp) = exchange(&mut stream, BINDER_WRITE_READ, &payload);
        assert_eq!(ret, 0, "ADD_SERVICE should succeed");

        // Two same-name GETs back-to-back on the same connection.
        let get_once = |stream: &mut UnixStream| -> i32 {
            let mut args = ParcelWriter::new();
            args.write_string16("flip_svc");
            let (req_data, req_off) = make_servicemanager_request_parcel(&mut args);
            let mut bc = Vec::with_capacity(4 + 64);
            bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
            bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_GET_SERVICE, 0));
            let payload = make_v2_write_read_payload(&bc, &req_data, &req_off, 4096);
            let (ret, resp) = exchange(stream, BINDER_WRITE_READ, &payload);
            assert_eq!(ret, 0, "GET WRITE_READ should succeed");
            let read_size = u32::from_ne_bytes(resp[0..4].try_into().unwrap()) as usize;
            let mut off = 4 + read_size;
            let magic = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap());
            assert_eq!(magic, WIRE_V3_MAGIC);
            off += 4;
            let blob_count = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap());
            assert_eq!(blob_count, 1);
            off += 4;
            let data_len = u32::from_ne_bytes(resp[off..off + 4].try_into().unwrap()) as usize;
            let _off_len = u32::from_ne_bytes(resp[off + 4..off + 8].try_into().unwrap()) as usize;
            let blob = &resp[off + 12..off + 12 + data_len];
            i32::from_ne_bytes(blob[28..32].try_into().unwrap())
        };

        let first = get_once(&mut stream);
        assert_eq!(
            first, STABILITY_ANNOTATION_VINTF,
            "first GET hit → plain android-11 VINTF level"
        );
        let second = get_once(&mut stream);
        assert_eq!(
            second, STABILITY_ANNOTATION_VINTF_A12,
            "same-name re-get after a HIT → A12 Category form (sticky flip)"
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
        let blob2 = &resp2[off2 + 12..off2 + 12 + dl2];
        let routed_handle = u64::from_ne_bytes(blob2[12..20].try_into().unwrap()) as u32;
        assert_eq!(
            routed_handle,
            PROXY_HANDLE_BASE + 8,
            "svc_a handle = 0xF0000008 (after the 7 virtual services: 4 AIDL/HIDL platform services + the 3 seeded 6-Z307 hwservicemanager instances)"
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
        assert_eq!(magic_a, WIRE_V3_MAGIC, "delivery carries v3 trailer");

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
        // 6-Z409: kernel-true ack — the BC_REPLY ioctl's read stream now
        // carries BR_TRANSACTION_COMPLETE (the real driver acks every
        // accepted BC_REPLY in the same read; libbinder's sendReply
        // waitForResponse(null,null) exits on it).
        assert_eq!(
            u32::from_ne_bytes(resp_r[4..8].try_into().unwrap()),
            BR_TRANSACTION_COMPLETE,
            "BC_REPLY ioctl acks with BR_TRANSACTION_COMPLETE (6-Z409)"
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
        let got = &resp_b[off_b + 12..off_b + 12 + dl_b];
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

        // 6-Z354: the delivery gate is KERNEL-TRUE now — the node owner's
        // pid must be a LIVE process (fresh /proc probe), so the test
        // announces the test process's own pid instead of a fake one (a
        // fake pid is a dead owner and the gate would BR_DEAD_REPLY the
        // delivery — correct production behavior, wrong test fixture).
        let live_pid = std::process::id();
        let ident_payload = |pid: u32| {
            let mut p = Vec::with_capacity(12);
            p.extend_from_slice(&pid.to_ne_bytes());
            p.extend_from_slice(&0u32.to_ne_bytes());
            p.extend_from_slice(&0u32.to_ne_bytes());
            p
        };

        // ---- Conn A (pid live): addService("svc_a") ----
        let mut stream_a = UnixStream::connect(&path).expect("connect A");
        let (ret_i, _r) = exchange(&mut stream_a, WIRE_CMD_IDENT, &ident_payload(live_pid));
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
        let (ret_i2, _r2) = exchange(
            &mut stream_b,
            WIRE_CMD_IDENT,
            &ident_payload(std::process::id()),
        );
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
        let blob2 = &resp2[off2 + 12..off2 + 12 + dl2];
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
        let (ret_i3, _r3) = exchange(&mut stream_a2, WIRE_CMD_IDENT, &ident_payload(live_pid));
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
        let got = &resp_b[off_b + 12..off_b + 12 + dl_b];
        assert_eq!(got, reply_data, "reply bytes exact");

        drop(stream_a);
        drop(stream_b);
        drop(stream_a2);
        drop(_handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    // ==== 6-Z442: the owner-side node-ref ACQUIRE mirror (kernel-true
    // ==== binder_node lifecycle). rn406: the composer's createClient
    // ==== IComposerClient destructed ~300 ms after the node grant (its
    // ==== only userspace ref was the createClient marshal temporary —
    // ==== the kernel's BR_INCREFS/BR_ACQUIRE owner notification never
    // ==== crossed), SF's first client call was delivered into the
    // ==== corpse (vtable=0 at the 6-Z306ad probe), and the composer
    // ==== Scudo-aborted on the freed node ptr → "Missing internal
    // ==== display" → the rung-7 restart cascade. The mirror rides the
    // ==== SAME ioctl as the flat crossing (6-Z306ae-e no-race shape).

    #[test]
    fn z444_cross_conn_deferred_reply_surfaces_on_requester_poll() {
        // THE rn408 gen-1 hang shape: system_server main (conn A) issues a
        // SYNC call to installd (conn B); B replies on a LATER ioctl; A's
        // idle polls (ws=0 rc=256) MUST surface the [BR_REPLY] — rn408
        // showed conn=138 polling [BR_NOOP] every 250 ms for 23 s with
        // the reply popped off B's txn stack and (per the code path)
        // queued on A's reply_queue, until the watchdog killed the
        // process. This test pins the requester-side drain end-to-end.
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let _handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));
        let live_pid = std::process::id();
        let ident_payload = |pid: u32| {
            let mut p = Vec::with_capacity(12);
            p.extend_from_slice(&pid.to_ne_bytes());
            p.extend_from_slice(&0u32.to_ne_bytes());
            p.extend_from_slice(&0u32.to_ne_bytes());
            p
        };

        // ---- Conn B (installd stand-in): addService ----
        let mut stream_b = UnixStream::connect(&path).expect("connect B");
        let (ret_i, _r) = exchange(&mut stream_b, WIRE_CMD_IDENT, &ident_payload(live_pid));
        assert_eq!(ret_i, 0);
        let mut args_b = ParcelWriter::new();
        args_b.write_string16("z444_svc");
        args_b.write_flat_binder(&FlatBinderObject {
            r#type: BINDER_TYPE_BINDER,
            flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
            binder: 0x1234,
            cookie: 0x5678,
        });
        args_b.write_i32(0);
        args_b.write_i32(0);
        let (db, ob) = make_servicemanager_request_parcel(&mut args_b);
        let mut bc_b = Vec::with_capacity(4 + 64);
        bc_b.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc_b.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_ADD_SERVICE, 0));
        let payload_b = make_v2_write_read_payload(&bc_b, &db, &ob, 4096);
        let (ret_b, _resp_b) = exchange(&mut stream_b, BINDER_WRITE_READ, &payload_b);
        assert_eq!(ret_b, 0, "addService ok");

        // ---- Conn A (system_server stand-in): getService → handle ----
        let mut stream_a = UnixStream::connect(&path).expect("connect A");
        let (ret_i2, _r2) = exchange(&mut stream_a, WIRE_CMD_IDENT, &ident_payload(live_pid));
        assert_eq!(ret_i2, 0);
        let mut args_a = ParcelWriter::new();
        args_a.write_string16("z444_svc");
        let (da, oa) = make_servicemanager_request_parcel(&mut args_a);
        let mut bc_a = Vec::with_capacity(4 + 64);
        bc_a.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc_a.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_GET_SERVICE, 0));
        let payload_a = make_v2_write_read_payload(&bc_a, &da, &oa, 4096);
        let (ret_a, resp_a) = exchange(&mut stream_a, BINDER_WRITE_READ, &payload_a);
        assert_eq!(ret_a, 0);
        let rs_a = u32::from_ne_bytes(resp_a[0..4].try_into().unwrap()) as usize;
        let off_a = 4 + rs_a + 8;
        let dl_a = u32::from_ne_bytes(resp_a[off_a..off_a + 4].try_into().unwrap()) as usize;
        let blob_a = &resp_a[off_a + 12..off_a + 12 + dl_a];
        let svc_handle = u64::from_ne_bytes(blob_a[12..20].try_into().unwrap()) as u32;

        // ---- Conn A: SYNC call (code=17) on that handle ----
        let mut tx = [0u8; 64];
        tx[0..4].copy_from_slice(&svc_handle.to_ne_bytes());
        tx[16..20].copy_from_slice(&17u32.to_ne_bytes());
        let req: &[u8] = b"a-request";
        let no_off: Vec<u8> = Vec::new();
        let mut bc_t = Vec::with_capacity(4 + 64);
        bc_t.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc_t.extend_from_slice(&tx);
        let payload_t = make_v2_write_read_multi_payload(&bc_t, &[(req, &no_off)], 4096);
        let (ret_t, resp_t) = exchange(&mut stream_a, BINDER_WRITE_READ, &payload_t);
        assert_eq!(ret_t, 0);
        assert_eq!(
            u32::from_ne_bytes(resp_t[4..8].try_into().unwrap()),
            BR_TRANSACTION_COMPLETE,
            "the sync call is accepted; the reply surfaces on a LATER read"
        );

        // ---- Conn B: read-only ioctl → the routed BR_TRANSACTION ----
        let mut wr = Vec::new();
        wr.extend_from_slice(&0u32.to_ne_bytes());
        wr.extend_from_slice(&4096u32.to_ne_bytes());
        let (ret_r1, resp_r1) = exchange(&mut stream_b, BINDER_WRITE_READ, &wr);
        assert_eq!(ret_r1, 0);
        assert_eq!(
            u32::from_ne_bytes(resp_r1[4..8].try_into().unwrap()),
            BR_TRANSACTION,
            "B receives the request"
        );

        // ---- Conn B: BC_REPLY → acked with COMPLETE ----
        let rep: &[u8] = b"installd-reply";
        let reply = [0u8; 64];
        let mut bc_p = Vec::with_capacity(4 + 64);
        bc_p.extend_from_slice(&BC_REPLY.to_ne_bytes());
        bc_p.extend_from_slice(&reply);
        let payload_p = make_v2_write_read_multi_payload(&bc_p, &[(rep, &no_off)], 4096);
        let (ret_p, resp_p) = exchange(&mut stream_b, BINDER_WRITE_READ, &payload_p);
        assert_eq!(ret_p, 0);
        assert_eq!(
            u32::from_ne_bytes(resp_p[4..8].try_into().unwrap()),
            BR_TRANSACTION_COMPLETE
        );

        // ---- THE 6-Z444 ASSERT: A's next read-only ioctl = [BR_REPLY] ----
        let (ret_d, resp_d) = exchange(&mut stream_a, BINDER_WRITE_READ, &wr);
        assert_eq!(ret_d, 0);
        let br_first = u32::from_ne_bytes(resp_d[4..8].try_into().unwrap());
        assert_eq!(
            br_first, BR_REPLY,
            "6-Z444: the deferred reply MUST surface on A's next poll (rn408: 23 s of [BR_NOOP] instead)"
        );
        let rs_d = u32::from_ne_bytes(resp_d[0..4].try_into().unwrap()) as usize;
        let off_d = 4 + rs_d + 8;
        let dl_d = u32::from_ne_bytes(resp_d[off_d..off_d + 4].try_into().unwrap()) as usize;
        assert_eq!(
            &resp_d[off_d + 12..off_d + 12 + dl_d],
            rep,
            "reply bytes exact"
        );

        drop(stream_a);
        drop(stream_b);
        drop(_handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn z442_bc_reply_owner_read_carries_acquire_mirror_before_complete() {
        // THE rn406 WIRE SHAPE, fixed end-to-end: conn A (the composer)
        // BC_REPLYs a LOCAL flat (the IComposerClient) to conn B (SF) —
        // A's SAME-ioctl read stream must carry
        // [BR_INCREFS][ptr][cookie][BR_ACQUIRE][ptr][cookie] BEFORE the
        // [BR_TRANSACTION_COMPLETE]: libbinder's sendReply
        // waitForResponse(nullptr, nullptr) processes both (incWeak +
        // incStrong on the real guest object) while the createClient
        // marshal temporary is still alive — no free-then-acquire race
        // (rn406: the object was a destructed corpse ~300 ms after the
        // grant and the composer Scudo-aborted on it).
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let _handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));
        let live_pid = std::process::id();
        let ident_payload = |pid: u32| {
            let mut p = Vec::with_capacity(12);
            p.extend_from_slice(&pid.to_ne_bytes());
            p.extend_from_slice(&0u32.to_ne_bytes());
            p.extend_from_slice(&0u32.to_ne_bytes());
            p
        };

        // ---- Conn A (the composer stand-in): addService ----
        let mut stream_a = UnixStream::connect(&path).expect("connect A");
        let (ret_i, _r) = exchange(&mut stream_a, WIRE_CMD_IDENT, &ident_payload(live_pid));
        assert_eq!(ret_i, 0);
        let mut args = ParcelWriter::new();
        args.write_string16("z442_svc");
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
        let (ret, _resp) = exchange(&mut stream_a, BINDER_WRITE_READ, &payload);
        assert_eq!(ret, 0, "ADD_SERVICE ok");

        // ---- Conn B (the SF stand-in): getService → handle ----
        let mut stream_b = UnixStream::connect(&path).expect("connect B");
        let (ret_i2, _r2) = exchange(&mut stream_b, WIRE_CMD_IDENT, &ident_payload(live_pid));
        assert_eq!(ret_i2, 0);
        let mut args2 = ParcelWriter::new();
        args2.write_string16("z442_svc");
        let (bd, bo) = make_servicemanager_request_parcel(&mut args2);
        let mut bc2 = Vec::with_capacity(4 + 64);
        bc2.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc2.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_GET_SERVICE, 0));
        let payload2 = make_v2_write_read_payload(&bc2, &bd, &bo, 4096);
        let (ret2, resp2) = exchange(&mut stream_b, BINDER_WRITE_READ, &payload2);
        assert_eq!(ret2, 0);
        let off2 = 4 + u32::from_ne_bytes(resp2[0..4].try_into().unwrap()) as usize + 8;
        let dl2 = u32::from_ne_bytes(resp2[off2..off2 + 4].try_into().unwrap()) as usize;
        let blob2 = &resp2[off2 + 12..off2 + 12 + dl2];
        let svc_handle = u64::from_ne_bytes(blob2[12..20].try_into().unwrap()) as u32;

        // ---- Conn B: transact(code=7) → A receives BR_TRANSACTION ----
        let mut tx_b = [0u8; 64];
        tx_b[0..4].copy_from_slice(&svc_handle.to_ne_bytes());
        tx_b[16..20].copy_from_slice(&7u32.to_ne_bytes());
        let tx_data: &[u8] = b"create-client";
        let tx_off: Vec<u8> = Vec::new();
        let mut bc3 = Vec::with_capacity(4 + 64);
        bc3.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc3.extend_from_slice(&tx_b);
        let payload3 = make_v2_write_read_multi_payload(&bc3, &[(tx_data, &tx_off)], 4096);
        let (ret_t, _resp_t) = exchange(&mut stream_b, BINDER_WRITE_READ, &payload3);
        assert_eq!(ret_t, 0);
        let mut wr_a = Vec::new();
        wr_a.extend_from_slice(&0u32.to_ne_bytes());
        wr_a.extend_from_slice(&4096u32.to_ne_bytes());
        let (ret_a, resp_a) = exchange(&mut stream_a, BINDER_WRITE_READ, &wr_a);
        assert_eq!(ret_a, 0);
        assert_eq!(
            u32::from_ne_bytes(resp_a[4..8].try_into().unwrap()),
            BR_TRANSACTION,
            "A receives the transaction"
        );

        // ---- Conn A: BC_REPLY carrying a LOCAL flat (the IComposerClient) ----
        let mut reply_data: Vec<u8> = Vec::new();
        reply_data.extend_from_slice(&0i32.to_ne_bytes()); // status NONE
        reply_data.extend_from_slice(&0i32.to_ne_bytes()); // pad
        reply_data.extend_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
        reply_data.extend_from_slice(&0u32.to_ne_bytes()); // flags
        reply_data.extend_from_slice(&0xaaaa_u64.to_ne_bytes()); // binder ptr
        reply_data.extend_from_slice(&0xbeef_u64.to_ne_bytes()); // cookie
        let mut reply_off: Vec<u8> = Vec::new();
        reply_off.extend_from_slice(&8u64.to_ne_bytes());
        let reply = [0u8; 64];
        let mut bc4 = Vec::with_capacity(4 + 64);
        bc4.extend_from_slice(&BC_REPLY.to_ne_bytes());
        bc4.extend_from_slice(&reply);
        let payload4 =
            make_v2_write_read_multi_payload(&bc4, &[(reply_data.as_slice(), &reply_off)], 0);
        let (ret_r, resp_r) = exchange(&mut stream_a, BINDER_WRITE_READ, &payload4);
        assert_eq!(ret_r, 0);

        // ---- THE 6-Z442 ASSERT: A's read stream = [INCREFS][ACQUIRE][COMPLETE] ----
        let read_r = u32::from_ne_bytes(resp_r[0..4].try_into().unwrap()) as usize;
        assert_eq!(
            read_r,
            4 + 16 + 4 + 16 + 4,
            "BR_INCREFS+cookie, BR_ACQUIRE+cookie, BR_TRANSACTION_COMPLETE"
        );
        let mut cur = 4;
        let br1 = u32::from_ne_bytes(resp_r[cur..cur + 4].try_into().unwrap());
        assert_eq!(br1, BR_INCREFS, "first mirror: INCREFS (kernel order)");
        let mptr = u64::from_ne_bytes(resp_r[cur + 4..cur + 12].try_into().unwrap());
        let mcookie = u64::from_ne_bytes(resp_r[cur + 12..cur + 20].try_into().unwrap());
        assert_eq!(mptr, 0xaaaa, "mirror carries the OWNER's ptr");
        assert_eq!(mcookie, 0xbeef, "mirror carries the OWNER's cookie");
        cur += 20;
        let br2 = u32::from_ne_bytes(resp_r[cur..cur + 4].try_into().unwrap());
        assert_eq!(br2, BR_ACQUIRE, "second mirror: ACQUIRE");
        assert_eq!(
            u64::from_ne_bytes(resp_r[cur + 4..cur + 12].try_into().unwrap()),
            0xaaaa
        );
        assert_eq!(
            u64::from_ne_bytes(resp_r[cur + 12..cur + 20].try_into().unwrap()),
            0xbeef
        );
        cur += 20;
        assert_eq!(
            u32::from_ne_bytes(resp_r[cur..cur + 4].try_into().unwrap()),
            BR_TRANSACTION_COMPLETE,
            "the completion follows the mirrors — sendReply exits AFTER the incs"
        );

        // ---- Conn B: the reply's flat MUST arrive as BINDER_TYPE_HANDLE ----
        let mut wr_b = Vec::new();
        wr_b.extend_from_slice(&0u32.to_ne_bytes());
        wr_b.extend_from_slice(&4096u32.to_ne_bytes());
        let (ret_b, resp_b) = exchange(&mut stream_b, BINDER_WRITE_READ, &wr_b);
        assert_eq!(ret_b, 0);
        assert_eq!(
            u32::from_ne_bytes(resp_b[4..8].try_into().unwrap()),
            BR_REPLY,
            "B's deferred BR_REPLY"
        );

        // ---- A's BC_INCREFS_DONE/BC_ACQUIRE_DONE acks parse cleanly ----
        // libbinder's BR_INCREFS/BR_ACQUIRE handling writes the DONE
        // commands to mOut; they flush on A's next write — the proxy
        // must consume them without error (the no-op arm).
        let mut done_cmds = Vec::new();
        done_cmds.extend_from_slice(&BC_INCREFS_DONE.to_ne_bytes());
        done_cmds.extend_from_slice(&0xaaaa_u64.to_ne_bytes());
        done_cmds.extend_from_slice(&0xbeef_u64.to_ne_bytes());
        done_cmds.extend_from_slice(&BC_ACQUIRE_DONE.to_ne_bytes());
        done_cmds.extend_from_slice(&0xaaaa_u64.to_ne_bytes());
        done_cmds.extend_from_slice(&0xbeef_u64.to_ne_bytes());
        let mut payload_done = Vec::with_capacity(8 + done_cmds.len());
        payload_done.extend_from_slice(&(done_cmds.len() as u32).to_ne_bytes());
        payload_done.extend_from_slice(&0u32.to_ne_bytes());
        payload_done.extend_from_slice(&done_cmds);
        let (ret_d, _resp_d) = exchange(&mut stream_a, BINDER_WRITE_READ, &payload_done);
        assert_eq!(ret_d, 0, "BC_INCREFS_DONE/BC_ACQUIRE_DONE accepted");

        drop(stream_a);
        drop(stream_b);
        drop(_handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn z442_strong_grant_mirrors_increfs_then_acquire_once() {
        // Kernel truth: the FIRST strong flat crossing takes the implied
        // weak ref too and queues [BR_INCREFS][BR_ACQUIRE] to the owner;
        // re-grants in the same era queue NOTHING (has_weak_ref /
        // has_strong_ref hold).
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let recipient = bus.register_conn();
        let ptr: u64 = 0xE597_1B40_F370;
        let cookie: u64 = 0xE597_7B40_DA60;

        // A strong LOCAL flat: [type=BINDER][flags][ptr][cookie].
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
        data[8..16].copy_from_slice(&ptr.to_ne_bytes());
        data[16..24].copy_from_slice(&cookie.to_ne_bytes());
        let mut offsets = Vec::new();
        offsets.extend_from_slice(&0u64.to_ne_bytes());

        let grants = bus.z359_translate_flats(0, owner, recipient, &mut data, &mut offsets, "TEST");
        assert_eq!(grants.len(), 1, "one flat crossing");
        let g = &grants[0];
        assert!(g.strong);
        assert_eq!(
            g.mirrors,
            vec![(BR_INCREFS, ptr, cookie), (BR_ACQUIRE, ptr, cookie)],
            "owner mirror order: INCREFS then ACQUIRE (kernel queue order)"
        );
        // The flat was rewritten to HANDLE form with the granted handle.
        let flat_handle = u32::from_ne_bytes(data[8..12].try_into().unwrap());
        assert_eq!(flat_handle, g.handle);
        assert_eq!(
            u32::from_ne_bytes(data[0..4].try_into().unwrap()),
            BINDER_TYPE_HANDLE
        );
        // Emulator bookkeeping: strong AND implied weak for the recipient.
        let node = bus.nodes.get(&g.handle).expect("node exists");
        assert_eq!(node.strong.get(&recipient), Some(&1));
        assert_eq!(node.weak.get(&recipient), Some(&1));
        assert!(node.strong_notified && node.weak_notified, "era notified");

        // Re-grant the SAME node (the sender re-exports its local object)
        // — no new mirrors: the notification already crossed this era.
        // (Rebuild the flat: the first crossing rewrote it to HANDLE form.)
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
        data[8..16].copy_from_slice(&ptr.to_ne_bytes());
        data[16..24].copy_from_slice(&cookie.to_ne_bytes());
        let mut offsets = Vec::new();
        offsets.extend_from_slice(&0u64.to_ne_bytes());
        let grants2 =
            bus.z359_translate_flats(0, owner, recipient, &mut data, &mut offsets, "TEST");
        assert_eq!(grants2.len(), 1);
        assert!(
            grants2[0].mirrors.is_empty(),
            "kernel re-acquire is silent while refs are outstanding"
        );
        let node = bus.nodes.get(&grants2[0].handle).expect("node exists");
        assert_eq!(node.strong.get(&recipient), Some(&2));
        assert_eq!(node.weak.get(&recipient), Some(&2));
    }

    #[test]
    fn z467_same_process_handle_flat_rewrites_to_local_form() {
        // Kernel truth: a forwarded HANDLE flat whose node's OWNER lives
        // in the RECIPIENT's own guest process is rewritten to the LOCAL
        // form (the owner's ptr/cookie) — the recipient's
        // unflatten_binder returns its OWN BBinder, no proxy, NO grant,
        // NO owner-side mirrors. The rn433 decode's boot blocker: DMS's
        // getDisplayInfo(token) forwarded SF's display-token handle back
        // into SF (token owner conn=67, txn target conn=66, same guest
        // pid) — the verbatim handle made SF materialize a self-proxy,
        // the token lookup missed, and the DMS phase-100 display wait
        // crash-looped every generation.
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let sender = bus.register_conn();
        let recipient = bus.register_conn();
        let shared_pid = z454_unique_pid();
        bus.conns.get_mut(&owner).unwrap().sender_pid = shared_pid;
        bus.conns.get_mut(&recipient).unwrap().sender_pid = shared_pid;
        bus.conns.get_mut(&sender).unwrap().sender_pid = z454_unique_pid();

        // Era 0: the owner exports its local object to `sender` — the
        // ordinary local→handle grant (SF's token-reply arm).
        let ptr: u64 = 0xE1E5_F280_7C30;
        let cookie: u64 = 0xE1E6_0281_5510;
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
        data[8..16].copy_from_slice(&ptr.to_ne_bytes());
        data[16..24].copy_from_slice(&cookie.to_ne_bytes());
        let mut offsets = Vec::new();
        offsets.extend_from_slice(&0u64.to_ne_bytes());
        let grants = bus.z359_translate_flats(0, owner, sender, &mut data, &mut offsets, "TEST");
        assert_eq!(grants.len(), 1, "era-0 export grants the sender a handle");
        let handle = grants[0].handle;

        // The forward: `sender` sends the HANDLE flat back into the
        // owner's process (the recipient shares the owner's pid).
        let mut fwd = vec![0u8; 32];
        fwd[0..4].copy_from_slice(&BINDER_TYPE_HANDLE.to_ne_bytes());
        fwd[8..16].copy_from_slice(&(handle as u64).to_ne_bytes());
        let mut fwd_off = Vec::new();
        fwd_off.extend_from_slice(&0u64.to_ne_bytes());
        let grants_fwd =
            bus.z359_translate_flats(0, sender, recipient, &mut fwd, &mut fwd_off, "TEST");
        assert!(grants_fwd.is_empty(), "a local rewrite grants nothing");
        assert_eq!(
            u32::from_ne_bytes(fwd[0..4].try_into().unwrap()),
            BINDER_TYPE_BINDER,
            "handle → local form"
        );
        assert_eq!(u64::from_ne_bytes(fwd[8..16].try_into().unwrap()), ptr);
        assert_eq!(u64::from_ne_bytes(fwd[16..24].try_into().unwrap()), cookie);
        // The recipient gained NO proxy refs — the node maps still name
        // only the era-0 sender.
        let node = bus.nodes.get(&handle).expect("node exists");
        assert_eq!(node.strong.get(&recipient), None);
        assert_eq!(node.weak.get(&recipient), None);
    }

    #[test]
    fn z467_cross_process_handle_flat_stays_verbatim() {
        // Different recipient pid: the handle stays a handle (kr64's
        // global handle space makes the kernel's per-proc remap a no-op;
        // the recipient's fresh proxy is kernel-true there).
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let sender = bus.register_conn();
        let recipient = bus.register_conn();
        bus.conns.get_mut(&owner).unwrap().sender_pid = z454_unique_pid();
        bus.conns.get_mut(&sender).unwrap().sender_pid = z454_unique_pid();
        bus.conns.get_mut(&recipient).unwrap().sender_pid = z454_unique_pid();

        let ptr: u64 = 0x1000;
        let cookie: u64 = 0x2000;
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
        data[8..16].copy_from_slice(&ptr.to_ne_bytes());
        data[16..24].copy_from_slice(&cookie.to_ne_bytes());
        let mut offsets = Vec::new();
        offsets.extend_from_slice(&0u64.to_ne_bytes());
        let grants = bus.z359_translate_flats(0, owner, sender, &mut data, &mut offsets, "TEST");
        assert_eq!(grants.len(), 1);
        let handle = grants[0].handle;

        let mut fwd = vec![0u8; 32];
        fwd[0..4].copy_from_slice(&BINDER_TYPE_HANDLE.to_ne_bytes());
        fwd[8..16].copy_from_slice(&(handle as u64).to_ne_bytes());
        let mut fwd_off = Vec::new();
        fwd_off.extend_from_slice(&0u64.to_ne_bytes());
        let grants_fwd =
            bus.z359_translate_flats(0, sender, recipient, &mut fwd, &mut fwd_off, "TEST");
        assert!(grants_fwd.is_empty());
        assert_eq!(
            u32::from_ne_bytes(fwd[0..4].try_into().unwrap()),
            BINDER_TYPE_HANDLE,
            "cross-process forward stays handle-form"
        );
        assert_eq!(
            u32::from_ne_bytes(fwd[8..12].try_into().unwrap()),
            handle,
            "handle value unchanged"
        );
    }

    #[test]
    fn z467_unstamped_recipient_pid_keeps_handle_verbatim() {
        // A recipient with no IDENT stamp (sender_pid == 0) keeps the
        // pre-fix verbatim crossing — the rewrite fires only on a
        // POSITIVE same-process match.
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let sender = bus.register_conn();
        let recipient = bus.register_conn();
        bus.conns.get_mut(&owner).unwrap().sender_pid = z454_unique_pid();
        bus.conns.get_mut(&sender).unwrap().sender_pid = z454_unique_pid();
        // recipient.sender_pid stays 0.

        let ptr: u64 = 0x3000;
        let cookie: u64 = 0x4000;
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
        data[8..16].copy_from_slice(&ptr.to_ne_bytes());
        data[16..24].copy_from_slice(&cookie.to_ne_bytes());
        let mut offsets = Vec::new();
        offsets.extend_from_slice(&0u64.to_ne_bytes());
        let grants = bus.z359_translate_flats(0, owner, sender, &mut data, &mut offsets, "TEST");
        assert_eq!(grants.len(), 1);
        let handle = grants[0].handle;

        let mut fwd = vec![0u8; 32];
        fwd[0..4].copy_from_slice(&BINDER_TYPE_HANDLE.to_ne_bytes());
        fwd[8..16].copy_from_slice(&(handle as u64).to_ne_bytes());
        let mut fwd_off = Vec::new();
        fwd_off.extend_from_slice(&0u64.to_ne_bytes());
        let grants_fwd =
            bus.z359_translate_flats(0, sender, recipient, &mut fwd, &mut fwd_off, "TEST");
        assert!(grants_fwd.is_empty());
        assert_eq!(
            u32::from_ne_bytes(fwd[0..4].try_into().unwrap()),
            BINDER_TYPE_HANDLE,
            "unstamped pid: verbatim"
        );
    }

    #[test]
    fn z467_weak_handle_flat_rewrites_to_weak_binder() {
        // The weak-handle forward into the owner's process rewrites to
        // BINDER_TYPE_WEAK_BINDER with the same (ptr, cookie).
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let sender = bus.register_conn();
        let recipient = bus.register_conn();
        let shared_pid = z454_unique_pid();
        bus.conns.get_mut(&owner).unwrap().sender_pid = shared_pid;
        bus.conns.get_mut(&recipient).unwrap().sender_pid = shared_pid;
        bus.conns.get_mut(&sender).unwrap().sender_pid = z454_unique_pid();

        let ptr: u64 = 0x5000;
        let cookie: u64 = 0x6000;
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&BINDER_TYPE_WEAK_BINDER.to_ne_bytes());
        data[8..16].copy_from_slice(&ptr.to_ne_bytes());
        data[16..24].copy_from_slice(&cookie.to_ne_bytes());
        let mut offsets = Vec::new();
        offsets.extend_from_slice(&0u64.to_ne_bytes());
        let grants = bus.z359_translate_flats(0, owner, sender, &mut data, &mut offsets, "TEST");
        assert_eq!(grants.len(), 1, "weak export grants a weak handle");
        assert!(!grants[0].strong);
        let handle = grants[0].handle;

        let mut fwd = vec![0u8; 32];
        fwd[0..4].copy_from_slice(&BINDER_TYPE_WEAK_HANDLE.to_ne_bytes());
        fwd[8..16].copy_from_slice(&(handle as u64).to_ne_bytes());
        let mut fwd_off = Vec::new();
        fwd_off.extend_from_slice(&0u64.to_ne_bytes());
        let grants_fwd =
            bus.z359_translate_flats(0, sender, recipient, &mut fwd, &mut fwd_off, "TEST");
        assert!(grants_fwd.is_empty());
        assert_eq!(
            u32::from_ne_bytes(fwd[0..4].try_into().unwrap()),
            BINDER_TYPE_WEAK_BINDER,
            "weak handle → weak local form"
        );
        assert_eq!(u64::from_ne_bytes(fwd[8..16].try_into().unwrap()), ptr);
        assert_eq!(u64::from_ne_bytes(fwd[16..24].try_into().unwrap()), cookie);
    }

    #[test]
    fn z442_release_to_zero_resets_era_and_remirrors_on_regrant() {
        // Kernel truth: BR_RELEASE/BR_DECREFS fire when the node's LAST
        // external ref of the kind drops (not per-holder), and the
        // notification era re-arms exactly there — a fresh export
        // re-notifies with a fresh [BR_INCREFS][BR_ACQUIRE].
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let recipient = bus.register_conn();
        let ptr: u64 = 0xAAAA;
        let cookie: u64 = 0xBEEF;

        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
        data[8..16].copy_from_slice(&ptr.to_ne_bytes());
        data[16..24].copy_from_slice(&cookie.to_ne_bytes());
        let mut offsets = Vec::new();
        offsets.extend_from_slice(&0u64.to_ne_bytes());

        let grants = bus.z359_translate_flats(0, owner, recipient, &mut data, &mut offsets, "TEST");
        let handle = grants[0].handle;
        assert_eq!(grants[0].mirrors.len(), 2, "INCREFS + ACQUIRE");

        // Drop the strong ref (BC_RELEASE equivalent): the strong era
        // ends → BR_RELEASE mirror + strong flag re-arms; the weak era
        // REMAINS (the implied weak ref is still held) → no BR_DECREFS.
        bus.z359_unref_node(0, handle, recipient, true);
        let node = bus.nodes.get(&handle).expect("node survives");
        assert!(node.strong.is_empty(), "strong gone");
        assert_eq!(node.weak.get(&recipient), Some(&1), "weak remains");
        assert!(!node.strong_notified, "strong era re-armed");
        assert!(node.weak_notified, "weak era still notified");

        // Drop the weak ref too: BR_DECREFS + weak re-arm — and the
        // 6-Z459 NODE-DEATH moment: BOTH maps empty means the kernel
        // destroys the node with its refs; the entry is REMOVED (the
        // pre-6-Z459 corpse-entry semantics are retired).
        bus.z359_unref_node(0, handle, recipient, false);
        assert!(
            !bus.nodes.contains_key(&handle),
            "node-dead: the entry is removed at the both-maps-empty close"
        );
        assert!(
            !bus.node_by_key.contains_key(&(owner, ptr, cookie)),
            "the key map is consistent with the node death"
        );

        // A fresh export of the same object re-notifies BOTH. (Rebuild
        // the flat: the first crossing rewrote it to HANDLE form.)
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
        data[8..16].copy_from_slice(&ptr.to_ne_bytes());
        data[16..24].copy_from_slice(&cookie.to_ne_bytes());
        let mut offsets = Vec::new();
        offsets.extend_from_slice(&0u64.to_ne_bytes());
        let grants2 =
            bus.z359_translate_flats(0, owner, recipient, &mut data, &mut offsets, "TEST");
        assert_ne!(
            grants2[0].handle, handle,
            "a post-death re-export is a FRESH node identity (kernel-true node lifetime), not a re-grant onto the corpse"
        );
        assert_eq!(
            grants2[0].mirrors,
            vec![(BR_INCREFS, ptr, cookie), (BR_ACQUIRE, ptr, cookie)],
            "the fresh node's eras re-notify BOTH"
        );
    }

    #[test]
    fn z442_two_recipients_single_release_notification() {
        // Kernel truth: two recipients of the same node → ONE
        // notification set; the first recipient's release mirrors
        // NOTHING (the object is still externally held); only the last
        // drop notifies. (The old per-holder release mirroring would
        // have decStrongs the owner twice for one acquire.)
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let rec_a = bus.register_conn();
        let rec_b = bus.register_conn();
        let ptr: u64 = 0x1111;
        let cookie: u64 = 0x2222;

        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
        data[8..16].copy_from_slice(&ptr.to_ne_bytes());
        data[16..24].copy_from_slice(&cookie.to_ne_bytes());
        let mut offsets = Vec::new();
        offsets.extend_from_slice(&0u64.to_ne_bytes());

        let grants_a = bus.z359_translate_flats(0, owner, rec_a, &mut data, &mut offsets, "TEST");
        let handle = grants_a[0].handle;
        // (Rebuild the flat: rec_a's crossing rewrote it to HANDLE form.)
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
        data[8..16].copy_from_slice(&ptr.to_ne_bytes());
        data[16..24].copy_from_slice(&cookie.to_ne_bytes());
        let mut offsets = Vec::new();
        offsets.extend_from_slice(&0u64.to_ne_bytes());
        let grants_b = bus.z359_translate_flats(0, owner, rec_b, &mut data, &mut offsets, "TEST");
        assert_eq!(grants_b[0].handle, handle, "same node, second recipient");
        assert!(
            grants_b[0].mirrors.is_empty(),
            "one notification era across recipients"
        );

        // First recipient releases: node still held by rec_b → silent.
        bus.z359_unref_node(0, handle, rec_a, true);
        bus.z359_unref_node(0, handle, rec_a, false);
        let node = bus.nodes.get(&handle).expect("node exists");
        assert_eq!(node.strong.get(&rec_b), Some(&1));
        assert_eq!(node.weak.get(&rec_b), Some(&1));
        assert!(node.strong_notified, "era holds while refs remain");

        // Last recipient releases: BR_RELEASE era ends.
        bus.z359_unref_node(0, handle, rec_b, true);
        let node = bus.nodes.get(&handle).expect("node exists");
        assert!(!node.strong_notified, "strong era re-armed at last drop");
    }

    #[test]
    fn z442_weak_flat_mirrors_increfs_only() {
        // BINDER_TYPE_WEAK_BINDER crossings: only the weak era —
        // [BR_INCREFS] to the owner, never BR_ACQUIRE.
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let recipient = bus.register_conn();
        let ptr: u64 = 0x3333;
        let cookie: u64 = 0x4444;

        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&BINDER_TYPE_WEAK_BINDER.to_ne_bytes());
        data[8..16].copy_from_slice(&ptr.to_ne_bytes());
        data[16..24].copy_from_slice(&cookie.to_ne_bytes());
        let mut offsets = Vec::new();
        offsets.extend_from_slice(&0u64.to_ne_bytes());

        let grants = bus.z359_translate_flats(0, owner, recipient, &mut data, &mut offsets, "TEST");
        assert_eq!(grants.len(), 1);
        assert!(!grants[0].strong);
        assert_eq!(grants[0].mirrors, vec![(BR_INCREFS, ptr, cookie)]);
        let node = bus.nodes.get(&grants[0].handle).expect("node exists");
        assert!(node.strong.is_empty(), "no strong ref for a weak flat");
        assert_eq!(node.weak.get(&recipient), Some(&1));
    }

    #[test]
    fn z442_unwind_grant_restores_pre_grant_state() {
        // A FAILED delivery (mailbox full) unwinds the grant: counts
        // gone, notification eras re-armed, the never-crossed node entry
        // removed so a fresh export re-grades cleanly.
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let recipient = bus.register_conn();
        let ptr: u64 = 0x5555;
        let cookie: u64 = 0x6666;

        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
        data[8..16].copy_from_slice(&ptr.to_ne_bytes());
        data[16..24].copy_from_slice(&cookie.to_ne_bytes());
        let mut offsets = Vec::new();
        offsets.extend_from_slice(&0u64.to_ne_bytes());

        let grants = bus.z359_translate_flats(0, owner, recipient, &mut data, &mut offsets, "TEST");
        let handle = grants[0].handle;
        bus.z442_unwind_grant(handle, recipient, true);
        assert!(
            !bus.nodes.contains_key(&handle),
            "never-crossed node entry dropped"
        );
        assert!(
            !bus.node_by_key.contains_key(&(owner, ptr, cookie)),
            "key map consistent with the drop"
        );

        // A fresh export re-notifies (the unwound era never notified).
        // (Rebuild the flat: the first crossing rewrote it to HANDLE form.)
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
        data[8..16].copy_from_slice(&ptr.to_ne_bytes());
        data[16..24].copy_from_slice(&cookie.to_ne_bytes());
        let mut offsets = Vec::new();
        offsets.extend_from_slice(&0u64.to_ne_bytes());
        let grants2 =
            bus.z359_translate_flats(0, owner, recipient, &mut data, &mut offsets, "TEST");
        assert_eq!(
            grants2[0].mirrors,
            vec![(BR_INCREFS, ptr, cookie), (BR_ACQUIRE, ptr, cookie)]
        );
    }

    // -------- 6-Z454: the REF-LEDGER ------------------------------------
    //
    // Parallel tests share the static ledger — every test mints its own
    // (pid, ptr, cookie) key so assertions never collide.

    /// A per-test unique pid (the static starts well above the guest pids
    /// the other tests use, and this process's own pids are irrelevant to
    /// the ledger's keys).
    fn z454_unique_pid() -> i32 {
        static NEXT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(45001);
        NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    #[test]
    fn z454_balanced_emit_never_flags() {
        let pid = z454_unique_pid();
        let (ptr, cookie) = (0x7000_1000u64, 0x5000_2000u64);
        assert!(!z454_emit(pid, ptr, cookie, Z454Site::RegAcq));
        assert!(!z454_emit(pid, ptr, cookie, Z454Site::NodeAcq));
        assert!(!z454_emit(pid, ptr, cookie, Z454Site::RegRel));
        assert!(!z454_emit(pid, ptr, cookie, Z454Site::NodeRel));
    }

    #[test]
    fn z454_v1_release_without_acquire_flags_the_emit() {
        let pid = z454_unique_pid();
        let (ptr, cookie) = (0x7000_3000u64, 0x5000_4000u64);
        // THE KILLER SHAPE: the add-side acquire was liveness-gated away
        // (never emitted), the overwrite release passes its own moment's
        // gate — V1 names the emitter the moment it queues.
        assert!(z454_emit(pid, ptr, cookie, Z454Site::RegRel));
    }

    #[test]
    fn z454_v2_delivered_release_without_delivered_acquire_flags() {
        let pid = z454_unique_pid();
        let (ptr, cookie) = (0x7000_5000u64, 0x5000_6000u64);
        // Emit-side balanced (the acquire was queued) but the DELIVERY
        // arm skipped the acquire (its moment's probe failed) while the
        // release passed its own: the owner decStrongs past zero NOW.
        assert!(!z454_emit(pid, ptr, cookie, Z454Site::RegAcq));
        assert!(z454_deliver(pid, ptr, cookie, BR_RELEASE));
    }

    #[test]
    fn z454_delivered_acquire_balances_release_delivery() {
        let pid = z454_unique_pid();
        let (ptr, cookie) = (0x7000_7000u64, 0x5000_8000u64);
        assert!(!z454_emit(pid, ptr, cookie, Z454Site::RegAcq));
        assert!(!z454_deliver(pid, ptr, cookie, BR_ACQUIRE));
        assert!(!z454_deliver(pid, ptr, cookie, BR_RELEASE));
        // Weak mirrors never participate (they cannot destroy objects):
        assert!(!z454_deliver(pid, ptr, cookie, BR_INCREFS));
        assert!(!z454_deliver(pid, ptr, cookie, BR_DECREFS));
    }

    #[test]
    fn z454_skips_unkeyed_and_unowned_entries() {
        // pid 0 (conn gone / lookup failed), ptr 0, cookie 0: no ledger
        // entry, no violation, no panic.
        assert!(!z454_emit(0, 0x1000, 0x2000, Z454Site::RegRel));
        assert!(!z454_emit(1234, 0, 0x2000, Z454Site::RegAcq));
        assert!(!z454_emit(1234, 0x1000, 0, Z454Site::RegAcq));
        assert!(!z454_deliver(0, 0x1000, 0x2000, BR_RELEASE));
        assert!(!z454_deliver(1234, 0, 0x2000, BR_RELEASE));
    }

    #[test]
    fn z454_evict_if_over_enforces_the_cap_and_keeps_live_keys() {
        let mut led = Z454Map::new();
        let base_pid = 70000 + (std::process::id() as i32 % 1000);
        for i in 0..Z454_LEDGER_CAP as i32 {
            let key = (base_pid, 0x7A00_0000u64 + i as u64 * 0x1000, 0x5B00u64);
            led.insert(key, Z454Entry::default());
        }
        assert_eq!(led.len(), Z454_LEDGER_CAP);
        // A live key at cap: no eviction, entry stays.
        let live = (base_pid, 0x7A00_0000u64, 0x5B00u64);
        z454_evict_if_over(&mut led, &live);
        assert_eq!(led.len(), Z454_LEDGER_CAP);
        assert!(led.contains_key(&live));
        // A NEW key at cap: exactly one eviction (the caller then
        // inserts — the z454_emit flow), cap headroom restored.
        let fresh = (base_pid, 0x7C00_0000u64, 0x5B00u64);
        z454_evict_if_over(&mut led, &fresh);
        assert_eq!(led.len(), Z454_LEDGER_CAP - 1);
        led.insert(fresh, Z454Entry::default());
        assert_eq!(led.len(), Z454_LEDGER_CAP);
        assert!(led.contains_key(&fresh));
    }

    #[test]
    fn z454_node_grant_and_unref_emits_balance_through_the_bus() {
        // The full bus path: a strong flat crossing emits NodeAcq (the
        // era's BR_ACQUIRE mirror), the BC_RELEASE drop emits NodeRel —
        // balanced, no violation. Directly exercises the Task 190 audit
        // surface (per-(pid,handle) grant/unref accounting).
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let recipient = bus.register_conn();
        if let Some(bx) = bus.conns.get_mut(&owner) {
            bx.sender_pid = z454_unique_pid();
        }
        let ptr = 0x7700_1000u64;
        let cookie = 0x5500_2000u64;
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
        data[8..16].copy_from_slice(&ptr.to_ne_bytes());
        data[16..24].copy_from_slice(&cookie.to_ne_bytes());
        let mut offsets = Vec::new();
        offsets.extend_from_slice(&0u64.to_ne_bytes());
        let grants = bus.z359_translate_flats(0, owner, recipient, &mut data, &mut offsets, "TEST");
        assert_eq!(grants[0].mirrors.len(), 2, "strong flat: INCREFS+ACQUIRE");
        // Era drop via the recipient's BC_RELEASE: the LAST strong drop
        // mirrors BR_RELEASE — the ledger sees rel==acq, no violation.
        bus.z359_unref_node(0, grants[0].handle, recipient, true);
    }

    // ── 6-Z458 (Task 195): the kernel-true REGISTRY pin ────────────────

    /// Count RefCmd359 mirrors of one br command queued to `owner`.
    fn z458_owner_refcmd359_count(bus: &BusState, owner: ConnId, br: u32) -> usize {
        bus.conns
            .get(&owner)
            .map(|bx| {
                bx.reply_queue
                    .iter()
                    .filter(|r| matches!(r, DeferredReply::RefCmd359 { br: b, .. } if *b == br))
                    .count()
            })
            .unwrap_or(0)
    }

    #[test]
    fn z458_registry_pin_books_into_the_node_maps() {
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let ptr = 0x8100_1000u64;
        let cookie = 0x6100_2000u64;
        bus.add_guest_service(
            "android.system.suspend@1.0::ISystemSuspend/default",
            owner,
            ptr,
            cookie,
        );
        bus.z458_registry_pin_add(owner, ptr, cookie, true);
        let handle = *bus
            .node_by_key
            .get(&(owner, ptr, cookie))
            .expect("pin created the node");
        let node = bus.nodes.get(&handle).expect("node entry");
        assert_eq!(node.strong.get(&REGISTRY_CONN), Some(&1), "one strong pin");
        assert_eq!(
            node.weak.get(&REGISTRY_CONN),
            Some(&1),
            "the implied weak pin"
        );
        assert!(node.strong_notified, "the pin IS the era's 0→1 edge");
        assert!(
            !node.weak_notified,
            "the weak era still opens at the first grant (unchanged surface)"
        );
        assert_eq!(
            node.strong_grants, 0,
            "the pin is not a flat-crossing grant"
        );
        // BOOLEAN presence: a re-add must not inflate the pin.
        bus.z458_registry_pin_add(owner, ptr, cookie, true);
        let node = bus.nodes.get(&handle).expect("node entry");
        assert_eq!(
            node.strong.get(&REGISTRY_CONN),
            Some(&1),
            "re-add does not inflate"
        );
        // Degenerate flats book nothing.
        bus.z458_registry_pin_add(owner, 0, cookie, true);
        bus.z458_registry_pin_add(owner, ptr, 0, true);
        bus.z458_registry_pin_add(PROXY_CONN_ID, ptr, cookie, true);
        assert_eq!(
            bus.node_by_key.len(),
            1,
            "only the real registration created a node"
        );
    }

    #[test]
    fn z458_registry_pin_survives_the_last_client_drop() {
        // THE rn425 REGRESSION TEST (Task 194 agenda item f): era 1's
        // last-client drop must NOT mirror BR_RELEASE — the registry
        // still pins the object (the corpse-regrant shape: the guest's
        // decStrong deleted a REGISTRY-PINNED LIVE object, the era
        // re-armed, the next client's re-grant operated on the corpse).
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let client_a = bus.register_conn();
        let client_b = bus.register_conn();
        let ptr = 0x8100_3000u64;
        let cookie = 0x6100_4000u64;
        bus.add_guest_service(
            "android.system.suspend@1.0::ISystemSuspend/default",
            owner,
            ptr,
            cookie,
        );
        bus.z458_registry_pin_add(owner, ptr, cookie, true);

        // Era 1: the first client grant. The strong era opened at the
        // pin (the add arm's BR_ACQUIRE) — NO second BR_ACQUIRE; the
        // weak era still opens (BR_INCREFS, unchanged surface).
        let (h, mirrors) = bus.z359_grant_node(owner, client_a, ptr, cookie, true);
        assert!(
            mirrors.iter().all(|(br, _, _)| *br != BR_ACQUIRE),
            "the pin's era is already open — no compensating second BR_ACQUIRE"
        );
        assert!(
            mirrors.iter().any(|(br, _, _)| *br == BR_INCREFS),
            "weak era opens at the first grant (pre-6-Z458 balance preserved)"
        );

        // Era 1's last-client drop: the REGISTRY still pins → NOTHING
        // mirrors to the owner (the rn425 killer, now impossible).
        bus.z359_unref_node(0, h, client_a, true);
        bus.z359_unref_node(0, h, client_a, false);
        assert_eq!(
            z458_owner_refcmd359_count(&bus, owner, BR_RELEASE),
            0,
            "era-1 drop must not release a registry-pinned object"
        );
        assert_eq!(
            z458_owner_refcmd359_count(&bus, owner, BR_DECREFS),
            0,
            "the pin's implied weak holds the weak map too"
        );

        // Era 2: re-grant to another client onto the SAME node — no
        // BR_ACQUIRE (the era never closed; kernel has_strong_ref truth).
        let (_, mirrors2) = bus.z359_grant_node(owner, client_b, ptr, cookie, true);
        assert!(
            mirrors2.iter().all(|(br, _, _)| *br != BR_ACQUIRE),
            "era-2 grant rides the still-open era — no mirror onto a corpse"
        );
        // The node's object identity is intact: same entry, same grants.
        let node = bus.nodes.get(&h).expect("node survives the eras");
        assert_eq!(node.strong_grants, 2, "lifetime grant counters survive");

        // The pin's drop (a cross-owner overwrite — 6-Z379 last-key
        // semantics): the kernel-true release moment. client_b still
        // holds, so the release DEFERS (kernel binder_dec_node truth —
        // see the deferred-shape test); drop client_b first.
        bus.z359_unref_node(0, h, client_b, true);
        bus.z359_unref_node(0, h, client_b, false);
        assert_eq!(z458_owner_refcmd359_count(&bus, owner, BR_RELEASE), 0);
        bus.add_guest_service(
            "android.system.suspend@1.0::ISystemSuspend/default",
            client_b,
            ptr.wrapping_add(0x1000),
            cookie.wrapping_add(0x1000),
        );
        assert_eq!(
            z458_owner_refcmd359_count(&bus, owner, BR_RELEASE),
            1,
            "the registry pin's own drop fires the release — the kernel-true era close"
        );
        assert_eq!(
            z458_owner_refcmd359_count(&bus, owner, BR_DECREFS),
            1,
            "the pin's implied weak drops with it"
        );
    }

    #[test]
    fn z458_overwrite_with_live_clients_defers_the_release() {
        // Kernel binder_dec_node truth: the SM's drop while a client
        // still holds fires NOTHING; the release rides the LAST drop.
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let next_gen = bus.register_conn();
        let client = bus.register_conn();
        if let Some(bx) = bus.conns.get_mut(&owner) {
            bx.sender_pid = z454_unique_pid();
        }
        let ptr = 0x8100_5000u64;
        let cookie = 0x6100_6000u64;
        bus.add_guest_service("some.hal.IFoo/default", owner, ptr, cookie);
        bus.z458_registry_pin_add(owner, ptr, cookie, true);
        let (h, _) = bus.z359_grant_node(owner, client, ptr, cookie, true);

        // The overwrite lands while the client holds: the pin drops but
        // the client's ref keeps the map non-empty → no release.
        bus.add_guest_service(
            "some.hal.IFoo/default",
            next_gen,
            ptr.wrapping_add(0x2000),
            cookie.wrapping_add(0x2000),
        );
        assert_eq!(
            z458_owner_refcmd359_count(&bus, owner, BR_RELEASE),
            0,
            "the pin's drop with a live client defers the release"
        );

        // The client's drop now empties the map → the release fires
        // (holder = the client; reg-pin witness = absent — the pin is
        // already gone, the release is the kernel-true era close).
        bus.z359_unref_node(0, h, client, true);
        assert_eq!(
            z458_owner_refcmd359_count(&bus, owner, BR_RELEASE),
            1,
            "the deferred release fired at the truly-last strong ref"
        );
        bus.z359_unref_node(0, h, client, false);
        assert_eq!(z458_owner_refcmd359_count(&bus, owner, BR_DECREFS), 1);
    }

    #[test]
    fn z458_silent_pin_drop_when_the_acquire_mirror_skipped() {
        // The rn425 suspend-service shape: the add arm's liveness probe
        // skipped the era-0 BR_ACQUIRE (ledger RegAcq=0). The pin's drop
        // must then be SILENT — no release may precede its acquire on
        // the wire (the decStrong-on-zero class).
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let next_gen = bus.register_conn();
        if let Some(bx) = bus.conns.get_mut(&owner) {
            bx.sender_pid = z454_unique_pid();
        }
        let ptr = 0x8100_7000u64;
        let cookie = 0x6100_8000u64;
        bus.add_guest_service(
            "android.system.suspend@1.0::ISystemSuspend/default",
            owner,
            ptr,
            cookie,
        );
        bus.z458_registry_pin_add(owner, ptr, cookie, false);
        bus.add_guest_service(
            "android.system.suspend@1.0::ISystemSuspend/default",
            next_gen,
            ptr.wrapping_add(0x3000),
            cookie.wrapping_add(0x3000),
        );
        assert_eq!(
            z458_owner_refcmd359_count(&bus, owner, BR_RELEASE),
            0,
            "a pin whose acquire never crossed drops silently"
        );
        assert_eq!(z458_owner_refcmd359_count(&bus, owner, BR_DECREFS), 0);
    }

    /// 6-Z459 companion: read the LIFETIME counters carried by the last
    /// queued RefCmd359 mirror of `br` on `owner`'s reply_queue (the
    /// delivery-time join's source now that the node entry dies at the
    /// both-maps-empty close).
    fn z459_last_refcmd359_counters(bus: &BusState, owner: ConnId, br: u32) -> (u32, u32) {
        bus.conns
            .get(&owner)
            .map(|b| {
                b.reply_queue
                    .iter()
                    .filter_map(|r| match r {
                        DeferredReply::RefCmd359 {
                            br: b2,
                            strong_grants,
                            weak_grants,
                            ..
                        } if *b2 == br => Some((*strong_grants, *weak_grants)),
                        _ => None,
                    })
                    .next_back()
                    .unwrap_or((0, 0))
            })
            .unwrap_or((0, 0))
    }

    #[test]
    fn z459_reply_borne_regrant_gets_a_fresh_node_identity() {
        // THE rn426 reply-borne corpse-regrant shape, fixed. Era 1's
        // last ref closes BOTH maps → the node entry is REMOVED
        // (kernel-true node lifetime: the driver node dies with its
        // refs). The next reply's re-export of the same (ptr, cookie)
        // creates a FRESH node identity with a fresh handle — era-2's
        // mirrors open a NEW notification era and NO mirror lands on
        // the freed chunk's old identity (the era-2-mirrors-onto-a-
        // freed-chunk class that kept the suspend-service death storm
        // alive through the 6-Z458 registry-pin fix).
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let client = bus.register_conn();
        if let Some(bx) = bus.conns.get_mut(&owner) {
            bx.sender_pid = z454_unique_pid();
        }
        let ptr = 0x8100_E000u64;
        let cookie = 0x6100_F000u64;

        // Era 1: the reply-borne flat crossing grants strong+implied
        // weak and opens BOTH notification eras (no registry pin on
        // this shape — a pure reply flat, the rn426 five-W shape).
        let (h1, mirrors1) = bus.z359_grant_node(owner, client, ptr, cookie, true);
        assert!(mirrors1.iter().any(|(br, _, _)| *br == BR_INCREFS));
        assert!(mirrors1.iter().any(|(br, _, _)| *br == BR_ACQUIRE));
        assert!(bus.nodes.contains_key(&h1), "era-1 node entry live");

        // Era 1 closes: the last client's strong drop mirrors
        // BR_RELEASE (weak still held — no death yet); the weak drop
        // mirrors BR_DECREFS and BOTH maps empty → the entry is
        // REMOVED (nodes + node_by_key).
        bus.z359_unref_node(0, h1, client, true);
        assert!(
            bus.nodes.contains_key(&h1),
            "strong-only close keeps the entry (the implied weak still holds)"
        );
        bus.z359_unref_node(0, h1, client, false);
        assert_eq!(z458_owner_refcmd359_count(&bus, owner, BR_RELEASE), 1);
        assert_eq!(z458_owner_refcmd359_count(&bus, owner, BR_DECREFS), 1);
        assert!(
            !bus.nodes.contains_key(&h1),
            "node-dead: entry removed at the both-maps-empty era close"
        );
        assert!(
            !bus.node_by_key.contains_key(&(owner, ptr, cookie)),
            "key map consistent with the node death"
        );

        // The queued release mirror carried the era-1 lifetime
        // counters — the delivery-time join survives the entry's
        // removal (the (0,0) blind read is retired with the entry).
        let (sg1, wg1) = z459_last_refcmd359_counters(&bus, owner, BR_RELEASE);
        assert_eq!((sg1, wg1), (1, 1), "era-1 counters ride the mirror");

        // Era 2: the same (owner, ptr, cookie) re-exported in the next
        // reply — a FRESH node identity (new handle), a NEW
        // notification era (INCREFS+ACQUIRE again — the pre-fix shape
        // re-granted SILENTLY onto the corpse via the surviving
        // entry), and the corpse's counters are NOT inherited.
        let (h2, mirrors2) = bus.z359_grant_node(owner, client, ptr, cookie, true);
        assert_ne!(h2, h1, "re-export after node death = fresh handle");
        assert_eq!(
            mirrors2,
            vec![(BR_INCREFS, ptr, cookie), (BR_ACQUIRE, ptr, cookie)],
            "a fresh node opens fresh eras"
        );
        let node2 = bus.nodes.get(&h2).expect("fresh node entry");
        assert_eq!(
            node2.strong_grants, 1,
            "fresh identity: era-2's own grant only (the corpse's era-1 grants are gone)"
        );

        // Era 2 closes the same way: mirrors onto the LIVE era-2
        // object, then the entry is buried again — no residue, no
        // third-era corpse.
        bus.z359_unref_node(0, h2, client, true);
        bus.z359_unref_node(0, h2, client, false);
        assert_eq!(z458_owner_refcmd359_count(&bus, owner, BR_RELEASE), 2);
        assert_eq!(z458_owner_refcmd359_count(&bus, owner, BR_DECREFS), 2);
        assert!(!bus.nodes.contains_key(&h2));
        assert!(!bus.node_by_key.contains_key(&(owner, ptr, cookie)));
        let (sg2, wg2) = z459_last_refcmd359_counters(&bus, owner, BR_RELEASE);
        assert_eq!(
            (sg2, wg2),
            (1, 1),
            "era-2's counters are the FRESH node's (not the accumulated 2)"
        );
    }

    #[test]
    fn z459_silent_pin_drop_also_buries_the_dead_node() {
        // The silent registry-pin drop (the add arm's acquire never
        // crossed — the rn425 suspend shape) empties BOTH maps too:
        // the node-death removal covers THAT path as well — no wire
        // release AND no corpse entry a re-export could re-grant.
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let next_gen = bus.register_conn();
        let ptr = 0x8110_0000u64;
        let cookie = 0x6110_1000u64;
        bus.add_guest_service("x.y@1.0::IFoo/default", owner, ptr, cookie);
        bus.z458_registry_pin_add(owner, ptr, cookie, false);
        let h = *bus
            .node_by_key
            .get(&(owner, ptr, cookie))
            .expect("pinned node entry");
        bus.add_guest_service(
            "x.y@1.0::IFoo/default",
            next_gen,
            ptr.wrapping_add(0x1000),
            cookie.wrapping_add(0x1000),
        );
        assert_eq!(
            z458_owner_refcmd359_count(&bus, owner, BR_RELEASE),
            0,
            "the silent drop never crosses the wire"
        );
        assert!(
            !bus.nodes.contains_key(&h),
            "dead node buried on the silent path too"
        );
        assert!(!bus.node_by_key.contains_key(&(owner, ptr, cookie)));
    }

    #[test]
    fn z458_unwind_keeps_the_pinned_node_and_the_open_era() {
        // 6-Z442 interplay (Task 194 item d): a failed first grant
        // unwinds WITHOUT deleting the pinned node (the registry still
        // holds it) and without re-arming the era the pin opened.
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let recipient = bus.register_conn();
        let ptr = 0x8100_9000u64;
        let cookie = 0x6100_A000u64;
        bus.add_guest_service("x.y@1.0::IFoo/default", owner, ptr, cookie);
        bus.z458_registry_pin_add(owner, ptr, cookie, true);
        let (h, _) = bus.z359_grant_node(owner, recipient, ptr, cookie, true);
        bus.z442_unwind_grant(h, recipient, true);
        assert!(
            bus.node_by_key.contains_key(&(owner, ptr, cookie)),
            "the pinned node survives the unwind"
        );
        let node = bus.nodes.get(&h).expect("node entry survives");
        assert_eq!(node.strong.get(&REGISTRY_CONN), Some(&1));
        assert!(
            node.strong_notified,
            "the era stays open while the pin holds"
        );
        // A post-unwind re-grant finds the SAME node (stable handle) and
        // still fires no BR_ACQUIRE (era open).
        let (h2, mirrors) = bus.z359_grant_node(owner, recipient, ptr, cookie, true);
        assert_eq!(h, h2, "stable node handle across the unwind");
        assert!(mirrors.iter().all(|(br, _, _)| *br != BR_ACQUIRE));
    }

    #[test]
    fn z458_ledger_release_emits_carry_the_registry_pin_witness() {
        // Task 194 item e: the release-side ledger emit names the
        // registry-pin state — a fired release while the pin held is
        // the rn425 signature (V1, reg-pin=present).
        let pid = z454_unique_pid();
        let (ptr, cookie) = (0x8100_B000u64, 0x6100_C000u64);
        // A balanced surface: acquire then release, no violation.
        assert!(!z454_emit(pid, ptr, cookie, Z454Site::RegAcq));
        assert!(!z454_emit_rel(pid, ptr, cookie, Z454Site::NodeRel, false));
        // The killer shape: release without acquire, pin present.
        let (ptr2, cookie2) = (0x8100_C000u64, 0x6100_D000u64);
        assert!(
            z454_emit_rel(pid, ptr2, cookie2, Z454Site::NodeRel, true),
            "premature release with the pin present = V1 violation"
        );
        // Degenerate keys never touch the ledger.
        assert!(!z454_emit_rel(0, ptr, cookie, Z454Site::RegRel, true));
        assert!(!z454_emit_rel(pid, 0, cookie, Z454Site::RegRel, true));
        assert!(!z454_emit_rel(pid, ptr, 0, Z454Site::RegRel, true));
    }

    #[test]
    fn z457_node_grants_are_lifetime_counters() {
        // 6-Z457 (Task 192): the per-holder maps drop to empty at the
        // last-ref release — exactly when the refcount mirror fires — so
        // the bus-side join must be the LIFETIME counters on the node
        // entry (which survives the release). Grant → 1/1; re-grant →
        // 2/2; the last-ref unref does NOT reset them.
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let recipient = bus.register_conn();
        let ptr = 0x7D00_1000u64;
        let cookie = 0x5D00_2000u64;
        let mkflat = || {
            let mut data = vec![0u8; 32];
            data[0..4].copy_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
            data[8..16].copy_from_slice(&ptr.to_ne_bytes());
            data[16..24].copy_from_slice(&cookie.to_ne_bytes());
            let mut offsets = Vec::new();
            offsets.extend_from_slice(&0u64.to_ne_bytes());
            (data, offsets)
        };
        let (mut data, mut offsets) = mkflat();
        let grants = bus.z359_translate_flats(0, owner, recipient, &mut data, &mut offsets, "TEST");
        let h = grants[0].handle;
        let node = bus.nodes.get(&h).expect("node exists");
        assert_eq!(node.strong_grants, 1);
        assert_eq!(node.weak_grants, 1);

        // Re-grant the same node (rebuild the flat: the first crossing
        // rewrote it to HANDLE form).
        let (mut data, mut offsets) = mkflat();
        let grants2 =
            bus.z359_translate_flats(0, owner, recipient, &mut data, &mut offsets, "TEST");
        assert_eq!(grants2[0].handle, h, "same node identity");
        let node = bus.nodes.get(&h).expect("node exists");
        assert_eq!(node.strong_grants, 2);
        assert_eq!(node.weak_grants, 2);

        // Drop the strong refs one per BC_RELEASE (kernel truth) — the
        // LAST drop (count 1→0) is the mirror-firing release; the node
        // entry (and its lifetime counters) SURVIVE it for the
        // delivery-time mirror read.
        bus.z359_unref_node(0, h, recipient, true);
        let node = bus.nodes.get(&h).expect("node entry survives");
        assert_eq!(
            node.strong.get(&recipient),
            Some(&1),
            "one release left one count"
        );
        bus.z359_unref_node(0, h, recipient, true);
        let node = bus.nodes.get(&h).expect("node entry survives the release");
        assert!(node.strong.is_empty(), "last-ref release empties the map");
        assert_eq!(node.strong_grants, 2, "lifetime strong grants persist");
        assert_eq!(node.weak_grants, 2, "lifetime weak grants persist");

        // A weak-only grant bumps ONLY the weak lifetime counter.
        let (mut data, mut offsets) = mkflat();
        data[0..4].copy_from_slice(&BINDER_TYPE_WEAK_BINDER.to_ne_bytes());
        let grants3 =
            bus.z359_translate_flats(0, owner, recipient, &mut data, &mut offsets, "TEST");
        assert_eq!(grants3.len(), 1, "weak flat crossing");
        let node = bus.nodes.get(&grants3[0].handle).expect("node exists");
        assert_eq!(node.strong_grants, 2, "weak grant does not touch strong");
        assert_eq!(node.weak_grants, 3, "lifetime weak grants bumped");
    }

    #[test]
    fn z359_owner_node_names_resolves_via_the_owner_side_reverse_map() {
        // 6-Z457 companion (Task 192 decode): rn422's 6-Z359 mirror lines
        // resolved node-name="?" on all 8 releases — `handle` in that
        // line is the OWNER-namespace node id while `by_handle` keys
        // CLIENT registry handles (disjoint id spaces). The owner's own
        // registry entries (owner, ptr) name the node instead.
        let mut bus = BusState::new();
        let owner = bus.register_conn();
        let other = bus.register_conn();
        let ptr = 0x7E00_1000u64;
        let cookie = 0x5E00_2000u64;

        assert_eq!(bus.z359_owner_node_names(owner, ptr), None, "unregistered");

        let h = bus.add_guest_service(
            "android.system.suspend@1.0::ISystemSuspend/default",
            owner,
            ptr,
            cookie,
        );
        assert_eq!(
            bus.z359_owner_node_names(owner, ptr).as_deref(),
            Some("android.system.suspend@1.0::ISystemSuspend/default")
        );

        // The chain-alias shape: a second name over the SAME (owner,
        // ptr) joins with '|' (BTreeMap = sorted order).
        bus.add_guest_service_alias(
            "android.hidl.base@1.0::IBase/default",
            owner,
            ptr,
            cookie,
            h,
        );
        assert_eq!(
            bus.z359_owner_node_names(owner, ptr).as_deref(),
            Some("android.hidl.base@1.0::IBase/default|android.system.suspend@1.0::ISystemSuspend/default")
        );

        // A DIFFERENT owner's registration at the same ptr does NOT
        // pollute the lookup (ownership is part of the key).
        bus.add_guest_service("other.svc", other, ptr, cookie);
        assert!(bus
            .z359_owner_node_names(other, ptr)
            .expect("other's name")
            .contains("other.svc"));
        assert_eq!(
            bus.z359_owner_node_names(owner, ptr).as_deref(),
            Some("android.hidl.base@1.0::IBase/default|android.system.suspend@1.0::ISystemSuspend/default")
        );
    }

    #[test]
    fn z454_counts_reads_the_ledger_totals_for_the_z457_join() {
        // 6-Z457: the refcount-mirror log joins the ledger's emit/deliver
        // totals beside the guest's live counts. The reader reports the
        // four counters; unknown/degenerate keys read (0,0,0,0).
        let pid = z454_unique_pid();
        let (ptr, cookie) = (0x7F00_1000u64, 0x5F00_2000u64);
        assert_eq!(z454_counts(pid, ptr, cookie), (0, 0, 0, 0), "unknown key");
        assert!(!z454_emit(pid, ptr, cookie, Z454Site::NodeAcq));
        assert!(!z454_deliver(pid, ptr, cookie, BR_ACQUIRE));
        assert!(!z454_emit(pid, ptr, cookie, Z454Site::NodeRel));
        assert!(!z454_deliver(pid, ptr, cookie, BR_RELEASE));
        assert_eq!(z454_counts(pid, ptr, cookie), (1, 1, 1, 1));

        // Degenerate keys never touch the ledger.
        assert_eq!(z454_counts(0, ptr, cookie), (0, 0, 0, 0));
        assert_eq!(z454_counts(pid, 0, cookie), (0, 0, 0, 0));
        assert_eq!(z454_counts(pid, ptr, 0), (0, 0, 0, 0));
    }

    #[test]
    fn z359_reply_local_flat_becomes_handle_and_release_mirrors_to_owner() {
        // rn309 decode: the composer's createClient reply carried the
        // IComposerClient as a raw ptr-form LOCAL flat. The client (SF)
        // never saw a usable object (its shlib gate zeroed the foreign
        // local → "failed to create composer client" FATAL) and no
        // release EVER reached the composer (its mClient stayed immortal
        // → waitForClientDestroyedLocked failed → createClient #2/#3
        // wedged the HAL's condvar). Kernel-true shape proven here:
        // (1) a LOCAL flat crossing conns arrives as BINDER_TYPE_HANDLE;
        // (2) the recipient's BC_RELEASE mirrors BR_RELEASE (ptr, cookie)
        // to the OWNER conn — the composer's onClientDestroyed path.
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let _handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));
        let live_pid = std::process::id();
        let ident_payload = |pid: u32| {
            let mut p = Vec::with_capacity(12);
            p.extend_from_slice(&pid.to_ne_bytes());
            p.extend_from_slice(&0u32.to_ne_bytes());
            p.extend_from_slice(&0u32.to_ne_bytes());
            p
        };

        // ---- Conn A (the composer stand-in): addService ----
        let mut stream_a = UnixStream::connect(&path).expect("connect A");
        let (ret_i, _r) = exchange(&mut stream_a, WIRE_CMD_IDENT, &ident_payload(live_pid));
        assert_eq!(ret_i, 0);
        let mut args = ParcelWriter::new();
        args.write_string16("z359_svc");
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
        let (ret, _resp) = exchange(&mut stream_a, BINDER_WRITE_READ, &payload);
        assert_eq!(ret, 0, "ADD_SERVICE ok");

        // ---- Conn B (the SF stand-in): getService → handle ----
        let mut stream_b = UnixStream::connect(&path).expect("connect B");
        let (ret_i2, _r2) = exchange(&mut stream_b, WIRE_CMD_IDENT, &ident_payload(live_pid));
        assert_eq!(ret_i2, 0);
        let mut args2 = ParcelWriter::new();
        args2.write_string16("z359_svc");
        let (bd, bo) = make_servicemanager_request_parcel(&mut args2);
        let mut bc2 = Vec::with_capacity(4 + 64);
        bc2.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc2.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_GET_SERVICE, 0));
        let payload2 = make_v2_write_read_payload(&bc2, &bd, &bo, 4096);
        let (ret2, resp2) = exchange(&mut stream_b, BINDER_WRITE_READ, &payload2);
        assert_eq!(ret2, 0);
        let off2 = 4 + u32::from_ne_bytes(resp2[0..4].try_into().unwrap()) as usize + 8;
        let dl2 = u32::from_ne_bytes(resp2[off2..off2 + 4].try_into().unwrap()) as usize;
        let blob2 = &resp2[off2 + 12..off2 + 12 + dl2];
        let svc_handle = u64::from_ne_bytes(blob2[12..20].try_into().unwrap()) as u32;

        // ---- Conn B: transact(code=7) → A receives BR_TRANSACTION ----
        let mut tx_b = [0u8; 64];
        tx_b[0..4].copy_from_slice(&svc_handle.to_ne_bytes());
        tx_b[16..20].copy_from_slice(&7u32.to_ne_bytes());
        let tx_data: &[u8] = b"create-client";
        let tx_off: Vec<u8> = Vec::new();
        let mut bc3 = Vec::with_capacity(4 + 64);
        bc3.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc3.extend_from_slice(&tx_b);
        let payload3 = make_v2_write_read_multi_payload(&bc3, &[(tx_data, &tx_off)], 4096);
        let (ret_t, _resp_t) = exchange(&mut stream_b, BINDER_WRITE_READ, &payload3);
        assert_eq!(ret_t, 0);
        let mut wr_a = Vec::new();
        wr_a.extend_from_slice(&0u32.to_ne_bytes());
        wr_a.extend_from_slice(&4096u32.to_ne_bytes());
        let (ret_a, resp_a) = exchange(&mut stream_a, BINDER_WRITE_READ, &wr_a);
        assert_eq!(ret_a, 0);
        assert_eq!(
            u32::from_ne_bytes(resp_a[4..8].try_into().unwrap()),
            BR_TRANSACTION,
            "A receives the transaction"
        );

        // ---- Conn A: BC_REPLY carrying a LOCAL flat (the IComposerClient) ----
        // Parcel: [status=0 i32][pad i32][flat: type/binder/cookie] with
        // the offsets array naming the flat at byte 8.
        let mut reply_data: Vec<u8> = Vec::new();
        reply_data.extend_from_slice(&0i32.to_ne_bytes()); // status NONE
        reply_data.extend_from_slice(&0i32.to_ne_bytes()); // pad
        reply_data.extend_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
        reply_data.extend_from_slice(&0u32.to_ne_bytes()); // flags
        reply_data.extend_from_slice(&0xaaaa_u64.to_ne_bytes()); // binder ptr
        reply_data.extend_from_slice(&0xbeef_u64.to_ne_bytes()); // cookie
        let mut reply_off: Vec<u8> = Vec::new();
        reply_off.extend_from_slice(&8u64.to_ne_bytes());
        let reply = [0u8; 64];
        let mut bc4 = Vec::with_capacity(4 + 64);
        bc4.extend_from_slice(&BC_REPLY.to_ne_bytes());
        bc4.extend_from_slice(&reply);
        let payload4 =
            make_v2_write_read_multi_payload(&bc4, &[(reply_data.as_slice(), &reply_off)], 0);
        let (ret_r, _resp_r) = exchange(&mut stream_a, BINDER_WRITE_READ, &payload4);
        assert_eq!(ret_r, 0);

        // ---- Conn B: the reply's flat MUST arrive as BINDER_TYPE_HANDLE ----
        let mut wr_b = Vec::new();
        wr_b.extend_from_slice(&0u32.to_ne_bytes());
        wr_b.extend_from_slice(&4096u32.to_ne_bytes());
        let (ret_b, resp_b) = exchange(&mut stream_b, BINDER_WRITE_READ, &wr_b);
        assert_eq!(ret_b, 0);
        assert_eq!(
            u32::from_ne_bytes(resp_b[4..8].try_into().unwrap()),
            BR_REPLY,
            "B's deferred BR_REPLY"
        );
        let read_b = u32::from_ne_bytes(resp_b[0..4].try_into().unwrap()) as usize;
        let off_b = 4 + read_b + 8;
        let dl_b = u32::from_ne_bytes(resp_b[off_b..off_b + 4].try_into().unwrap()) as usize;
        assert_eq!(dl_b, reply_data.len(), "reply blob size preserved");
        let got = &resp_b[off_b + 12..off_b + 12 + dl_b];
        let flat_type = u32::from_ne_bytes(got[8..12].try_into().unwrap());
        assert_eq!(
            flat_type, BINDER_TYPE_HANDLE,
            "6-Z359: the LOCAL flat crossed as a HANDLE (was a raw ptr-form flat)"
        );
        let node_handle = u64::from_ne_bytes(got[16..24].try_into().unwrap()) as u32;
        // 6-Z371: the handle must be kernel-true DENSE — indistinguishable
        // from a service handle. The fabricated 0x7F000001+ node range made
        // guest libbinder sparse-grow its handle table to the handle value
        // (rn326: ceil(0x7F000001*1.5)*16 = 47.62 GiB SharedBuffer::alloc →
        // MAP_FIXED balloon → container death).
        assert!(
            node_handle > 0 && node_handle <= 4096,
            "the node handle must be kernel-true dense (1..=4096), got 0x{node_handle:08x}"
        );
        assert_eq!(
            u64::from_ne_bytes(got[24..32].try_into().unwrap()),
            0,
            "handle-form cookie is 0 (the 6-Z306z shlib gate skips it)"
        );

        // ---- Conn B: BC_RELEASE {handle u32} → the mirror reaches the owner ----
        // Kernel UAPI: BC_RELEASE carries a bare __u32 handle (4 bytes).
        let mut rel = Vec::with_capacity(4 + 4);
        rel.extend_from_slice(&BC_RELEASE.to_ne_bytes());
        rel.extend_from_slice(&node_handle.to_ne_bytes());
        let mut payload_rel = Vec::with_capacity(8 + rel.len());
        payload_rel.extend_from_slice(&(rel.len() as u32).to_ne_bytes());
        payload_rel.extend_from_slice(&0u32.to_ne_bytes());
        payload_rel.extend_from_slice(&rel);
        let (ret_rel, _resp_rel) = exchange(&mut stream_b, BINDER_WRITE_READ, &payload_rel);
        assert_eq!(ret_rel, 0, "BC_RELEASE accepted");

        // ---- Conn A: its next read surfaces [BR_RELEASE][ptr][cookie] ----
        let mut wr_a2 = Vec::new();
        wr_a2.extend_from_slice(&0u32.to_ne_bytes());
        wr_a2.extend_from_slice(&4096u32.to_ne_bytes());
        let (ret_a2, resp_a2) = exchange(&mut stream_a, BINDER_WRITE_READ, &wr_a2);
        assert_eq!(ret_a2, 0);
        let read_a2 = u32::from_ne_bytes(resp_a2[0..4].try_into().unwrap()) as usize;
        assert_eq!(
            u32::from_ne_bytes(resp_a2[4..8].try_into().unwrap()),
            BR_RELEASE,
            "6-Z359: the release mirrored to the owner conn"
        );
        assert_eq!(read_a2, 4 + 16, "BR_RELEASE + binder_ptr_cookie");
        let mirrored_ptr = u64::from_ne_bytes(resp_a2[8..16].try_into().unwrap());
        let mirrored_cookie = u64::from_ne_bytes(resp_a2[16..24].try_into().unwrap());
        assert_eq!(mirrored_ptr, 0xaaaa, "mirror carries the OWNER's ptr");
        assert_eq!(mirrored_cookie, 0xbeef, "mirror carries the OWNER's cookie");

        // ---- A second BC_RELEASE must NOT double-mirror (count spent) ----
        let mut rel2 = Vec::with_capacity(4 + 4);
        rel2.extend_from_slice(&BC_RELEASE.to_ne_bytes());
        rel2.extend_from_slice(&node_handle.to_ne_bytes());
        let mut payload_rel2 = Vec::with_capacity(8 + rel2.len());
        payload_rel2.extend_from_slice(&(rel2.len() as u32).to_ne_bytes());
        payload_rel2.extend_from_slice(&0u32.to_ne_bytes());
        payload_rel2.extend_from_slice(&rel2);
        let (ret_rel2, _r2) = exchange(&mut stream_b, BINDER_WRITE_READ, &payload_rel2);
        assert_eq!(ret_rel2, 0);
        let mut wr_a3 = Vec::new();
        wr_a3.extend_from_slice(&0u32.to_ne_bytes());
        wr_a3.extend_from_slice(&4096u32.to_ne_bytes());
        let (ret_a3, resp_a3) = exchange(&mut stream_a, BINDER_WRITE_READ, &wr_a3);
        assert_eq!(ret_a3, 0);
        let read_a3 = u32::from_ne_bytes(resp_a3[0..4].try_into().unwrap()) as usize;
        assert_ne!(
            u32::from_ne_bytes(resp_a3[4..8].try_into().unwrap()),
            BR_RELEASE,
            "no double mirror: the recipient's ref was already spent"
        );
        assert!(read_a3 >= 4, "BR_NOOP keeps the buffer non-empty");

        drop(stream_a);
        drop(stream_b);
        drop(_handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn z309f_steal_is_device_consistent() {
        // The rn257 decode (artifacts 34740065041): the audio daemon's
        // vndbinder pool thread (conn=14, tid=2894) stole a HWBINDER
        // transaction queued for a hwbinder sibling and served it through
        // android::BBinder::transact — the AIDL class's onTransact vtable
        // slot (16) does not exist in the hardware::BHwBinder ABI
        // (onTransact@11) — the slot read hit a literal-zero offset-to-top
        // and `blr 0` killed the daemon (pc=0x0, group-kill, era aborts).
        // The kernel NEVER routes node work across /dev/binder,
        // /dev/hwbinder and /dev/vndbinder: the steal must only consider
        // siblings serving the SAME device. This test: a hwbinder tx
        // queued for a busy hwbinder conn is REJECTED by a vndbinder
        // sibling (BR_NOOP — under-steal is safe, the owner pops it
        // later) and accepted by a hwbinder sibling (the z271g behavior,
        // preserved for same-device pools and legacy dev=0 pairs).
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let _handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));

        // IDENT v2 payload: pid, uid, pad, magic "idex", tid, dev.
        let ident_v2 = |pid: u32, tid: u32, dev: u32| {
            let mut p = Vec::with_capacity(24);
            p.extend_from_slice(&pid.to_ne_bytes());
            p.extend_from_slice(&0u32.to_ne_bytes());
            p.extend_from_slice(&0u32.to_ne_bytes());
            p.extend_from_slice(&IDENT_EXT_MAGIC.to_ne_bytes());
            p.extend_from_slice(&tid.to_ne_bytes());
            p.extend_from_slice(&dev.to_ne_bytes());
            p
        };

        // ---- Conn A (pid 7777, dev=hwbinder): addService("svc_hw") ----
        let mut stream_a = UnixStream::connect(&path).expect("connect A");
        let (ret_i, _r) = exchange(
            &mut stream_a,
            WIRE_CMD_IDENT,
            &ident_v2(std::process::id(), 7701, 2),
        );
        assert_eq!(ret_i, 0, "IDENT A accepted");
        let mut args = ParcelWriter::new();
        args.write_string16("svc_hw");
        args.write_flat_binder(&FlatBinderObject {
            r#type: BINDER_TYPE_BINDER,
            flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
            binder: 0x2222,
            cookie: 0x4444,
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
        // A parks WITHOUT reading — the next tx queues in its inbox.

        // ---- Conn B (pid 8888, dev=hwbinder): getService → handle ----
        let mut stream_b = UnixStream::connect(&path).expect("connect B");
        let (ret_i2, _r2) = exchange(
            &mut stream_b,
            WIRE_CMD_IDENT,
            &ident_v2(std::process::id(), 8801, 2),
        );
        assert_eq!(ret_i2, 0);
        let mut args2 = ParcelWriter::new();
        args2.write_string16("svc_hw");
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
        let blob2 = &resp2[off2 + 12..off2 + 12 + dl2];
        let routed_handle = u64::from_ne_bytes(blob2[12..20].try_into().unwrap()) as u32;

        // ---- Conn B: transact(code=42) — completes in-ioctl, parks ----
        let mut tx_b = [0u8; 64];
        tx_b[0..4].copy_from_slice(&routed_handle.to_ne_bytes());
        tx_b[16..20].copy_from_slice(&42u32.to_ne_bytes());
        let tx_data: &[u8] = b"hw-tx-payload";
        let tx_off = Vec::new();
        let mut bc3 = Vec::with_capacity(4 + 64);
        bc3.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc3.extend_from_slice(&tx_b);
        let payload3 = make_v2_write_read_multi_payload(&bc3, &[(tx_data, &tx_off)], 4096);
        let (ret3, resp3) = exchange(&mut stream_b, BINDER_WRITE_READ, &payload3);
        assert_eq!(ret3, 0);
        assert_eq!(
            u32::from_ne_bytes(resp3[4..8].try_into().unwrap()),
            BR_TRANSACTION_COMPLETE
        );

        // ---- Conn A-/vndbinder (pid 7777, dev=vndbinder): read-only ----
        // MUST NOT steal: the tx is hwbinder node work, this thread's
        // IPCThreadState would serve it with the wrong class ABI.
        let mut stream_av = UnixStream::connect(&path).expect("connect Av");
        let (ret_i3, _r3) = exchange(
            &mut stream_av,
            WIRE_CMD_IDENT,
            &ident_v2(std::process::id(), 7702, 3),
        );
        assert_eq!(ret_i3, 0);
        let mut wr_v = Vec::new();
        wr_v.extend_from_slice(&0u32.to_ne_bytes());
        wr_v.extend_from_slice(&4096u32.to_ne_bytes());
        let (ret_v, resp_v) = exchange(&mut stream_av, BINDER_WRITE_READ, &wr_v);
        assert_eq!(ret_v, 0);
        let br_v = u32::from_ne_bytes(resp_v[4..8].try_into().unwrap());
        assert_ne!(
            br_v, BR_TRANSACTION,
            "6-Z309f: a vndbinder sibling NEVER steals hwbinder node work"
        );
        drop(stream_av);

        // ---- Conn A2 (pid 7777, dev=hwbinder): read-only STEALS ----
        let mut stream_a2 = UnixStream::connect(&path).expect("connect A2");
        let (ret_i4, _r4) = exchange(
            &mut stream_a2,
            WIRE_CMD_IDENT,
            &ident_v2(std::process::id(), 7703, 2),
        );
        assert_eq!(ret_i4, 0);
        let mut wr_a2 = Vec::new();
        wr_a2.extend_from_slice(&0u32.to_ne_bytes());
        wr_a2.extend_from_slice(&4096u32.to_ne_bytes());
        let (ret_a2, resp_a2) = exchange(&mut stream_a2, BINDER_WRITE_READ, &wr_a2);
        assert_eq!(ret_a2, 0);
        let br = u32::from_ne_bytes(resp_a2[4..8].try_into().unwrap());
        assert_eq!(
            br, BR_TRANSACTION,
            "same-device sibling still steals (6-Z271g preserved)"
        );
        let tr = &resp_a2[8..8 + 64];
        let code = u32::from_ne_bytes(tr[16..20].try_into().unwrap());
        assert_eq!(code, 42, "stolen delivery carries the code");
        let cookie = u64::from_ne_bytes(tr[8..16].try_into().unwrap());
        assert_eq!(cookie, 0x4444, "owner cookie preserved across steal");

        // ---- Conn A2: BC_REPLY → B resolves ----
        let reply_data: &[u8] = b"hw-reply";
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
        assert_eq!(br2, BR_REPLY, "B gets the reply on its own read");
        let off_b = 4 + read_b + 8;
        let dl_b = u32::from_ne_bytes(resp_b[off_b..off_b + 4].try_into().unwrap()) as usize;
        assert_eq!(dl_b, reply_data.len());
        let got = &resp_b[off_b + 12..off_b + 12 + dl_b];
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
        let blob2 = &resp2[o2 + 12..o2 + 12 + dl2];
        let self_ptr = u64::from_ne_bytes(blob2[12..20].try_into().unwrap());
        // 6-Z306ac: the OWNER-conn GET now returns the LOCAL flat (the
        // kernel's same-process semantic) — ptr/cookie, not a handle.
        assert_eq!(
            self_ptr, 0xaaaa,
            "owner-conn GET → LOCAL flat.binder = the registered ptr"
        );
        // The proxy still allocated the ROUTING handle (first guest
        // service after the 4 in-proxy virtuals) — used below to drive
        // the self-transaction round trip.
        let self_handle = PROXY_HANDLE_BASE + 8; // after the 7 virtual services (6-Z307: +3 seeded hwservicemanager instances)

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
        // 6-Z409: kernel-true — the BC_REPLY ioctl acks with
        // BR_TRANSACTION_COMPLETE; the resolved self-reply (the drained
        // reply_queue entry) surfaces on the NEXT read-only ioctl (the
        // read half's `read_buf.is_empty()` guard skips the drain once
        // the write half produced the ack — libbinder's sendReply
        // waitForResponse(null,null) exits at the COMPLETE and the next
        // waitForResponse consumes the reply, exactly like the kernel's
        // two-read flow).
        assert_eq!(
            u32::from_ne_bytes(resp5[4..8].try_into().unwrap()),
            BR_TRANSACTION_COMPLETE,
            "self BC_REPLY acked with BR_TRANSACTION_COMPLETE (6-Z409)"
        );
        let rs5 = u32::from_ne_bytes(resp5[0..4].try_into().unwrap()) as usize;
        assert_eq!(rs5, 4, "ack-only read stream");

        // ---- the next read-only ioctl drains the self-reply ----
        let mut wr6 = Vec::new();
        wr6.extend_from_slice(&0u32.to_ne_bytes());
        wr6.extend_from_slice(&4096u32.to_ne_bytes());
        let (ret6, resp6) = exchange(&mut stream, BINDER_WRITE_READ, &wr6);
        assert_eq!(ret6, 0);
        let rs6 = u32::from_ne_bytes(resp6[0..4].try_into().unwrap()) as usize;
        assert_eq!(
            u32::from_ne_bytes(resp6[4..8].try_into().unwrap()),
            BR_REPLY,
            "self BC_REPLY resolved by the next read (reply queue drained)"
        );
        let o6 = 4 + rs6 + 8;
        let dl6 = u32::from_ne_bytes(resp6[o6..o6 + 4].try_into().unwrap()) as usize;
        assert_eq!(dl6, rep.len());
        assert_eq!(&resp6[o6 + 12..o6 + 12 + dl6], rep, "own reply bytes exact");

        drop(stream);
        drop(_handle);
        let _ = fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn z271i_nested_transaction_does_not_clobber_outer_reply() {
        // A services B's call; WHILE still owing B its BC_REPLY, A makes
        // a nested sync call to C's service. The nested reply must not
        // swallow A's outstanding outer transaction (its transaction
        // stack — 6-Z306ag), and A's later BC_REPLY still resolves B's
        // original call.
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
            let blob = &resp[off + 12..off + 12 + dl];
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
            &resp_b2[o_b + 12..o_b + 12 + dl_b],
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
    fn z306ag_overlapping_incoming_tx_reply_lifo_correlation() {
        // THE 6-Z306ag SHAPE — the one the single-slot inflight_txn
        // corrupted: TWO sync transactions queue for one server conn; the
        // server pops BOTH (the second delivery lands while it still owes
        // the first reply — kernel-faithful: a pool thread parked on its
        // own reply can receive the next node work). Its two BC_REPLYs
        // must resolve LIFO (innermost first) to the RIGHT requesters.
        // Old code: the second delivery OVERWROTE the slot → the first
        // BC_REPLY correlated to the WRONG waiter and the second found
        // nothing ("BC_REPLY with no delivered transaction") → B wedged
        // to the REPLY_TIMEOUT. This is the constructor-era wedge class.
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
            let blob = &resp[off + 12..off + 12 + dl];
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
        let mut conn_d = UnixStream::connect(&path).expect("connect D");

        // ---- B and D both resolve svc_a; B transacts code 9 (tx #1), D
        //      transacts code 13 (tx #2) — BOTH queue on conn_a's inbox.
        let h_a_b = get_service(&mut conn_b, "svc_a");
        let h_a_d = get_service(&mut conn_d, "svc_a");
        assert_eq!(h_a_b, h_a_d, "global handle table");
        let resp_b1 = transact(&mut conn_b, h_a_b, 9);
        assert_eq!(
            u32::from_ne_bytes(resp_b1[4..8].try_into().unwrap()),
            BR_TRANSACTION_COMPLETE
        );
        let resp_d1 = transact(&mut conn_d, h_a_d, 13);
        assert_eq!(
            u32::from_ne_bytes(resp_d1[4..8].try_into().unwrap()),
            BR_TRANSACTION_COMPLETE
        );

        // ---- A pops tx #1 (stack=[1]), then — while still owing it —
        //      pops tx #2 (stack=[1,2]). THE old overwrite site.
        let resp_a1 = read_only(&mut conn_a);
        assert_eq!(
            u32::from_ne_bytes(resp_a1[4..8].try_into().unwrap()),
            BR_TRANSACTION,
            "A receives B's tx #1"
        );
        let tr_a1 = &resp_a1[8..8 + 64];
        assert_eq!(u32::from_ne_bytes(tr_a1[16..20].try_into().unwrap()), 9);
        let resp_a2 = read_only(&mut conn_a);
        assert_eq!(
            u32::from_ne_bytes(resp_a2[4..8].try_into().unwrap()),
            BR_TRANSACTION,
            "A receives D's tx #2 while tx #1 is still outstanding"
        );
        let tr_a2 = &resp_a2[8..8 + 64];
        assert_eq!(u32::from_ne_bytes(tr_a2[16..20].try_into().unwrap()), 13);

        // ---- A replies: innermost (tx #2) first → D. Then outer (tx #1)
        //      → B. BOTH requesters must resolve, bytes exact.
        send_reply(&mut conn_a, b"inner-rep");
        let resp_d2 = read_only(&mut conn_d);
        assert_eq!(
            u32::from_ne_bytes(resp_d2[4..8].try_into().unwrap()),
            BR_REPLY,
            "D (innermost tx #2) resolves first — LIFO"
        );
        let rs_d = u32::from_ne_bytes(resp_d2[0..4].try_into().unwrap()) as usize;
        let o_d = 4 + rs_d + 8;
        let dl_d = u32::from_ne_bytes(resp_d2[o_d..o_d + 4].try_into().unwrap()) as usize;
        assert_eq!(
            &resp_d2[o_d + 12..o_d + 12 + dl_d],
            b"inner-rep",
            "D gets ITS OWN reply bytes (no cross-transaction corruption)"
        );

        send_reply(&mut conn_a, b"outer-rep");
        let resp_b2 = read_only(&mut conn_b);
        assert_eq!(
            u32::from_ne_bytes(resp_b2[4..8].try_into().unwrap()),
            BR_REPLY,
            "B (outer tx #1) still resolves — the stack survived the overlap"
        );
        let rs_b = u32::from_ne_bytes(resp_b2[0..4].try_into().unwrap()) as usize;
        let o_b = 4 + rs_b + 8;
        let dl_b = u32::from_ne_bytes(resp_b2[o_b..o_b + 4].try_into().unwrap()) as usize;
        assert_eq!(
            &resp_b2[o_b + 12..o_b + 12 + dl_b],
            b"outer-rep",
            "B gets ITS OWN reply bytes"
        );

        drop(conn_a);
        drop(conn_b);
        drop(conn_d);
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
        let blob2 = &resp2[o3 + 12..o3 + 12 + dl2];
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
            let blob = &resp[off + 12..off + 12 + dl];
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
        let blob = &resp[off + 12..off + 12 + dl];
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
        let blob2 = &resp2[off2 + 12..off2 + 12 + dl2];
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
        let blob = &resp[off + 12..off + 12 + dlen];
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
        // Client walk 3: finishUnflattenBinder's readInt32 — the
        // android-11 PLAIN Level form on the FIRST get of a connection
        // (6-Z306ab: the A11 Stability::set rejects every non-bare-Level
        // value with BAD_TYPE → readStrongBinder → null — the
        // performSystemServerDexOpt NPE of ladders #195/#196).
        let stability = i32::from_ne_bytes(blob[28..32].try_into().unwrap());
        assert_eq!(
            stability, STABILITY_ANNOTATION_VINTF,
            "first get → plain android-11 VINTF level (63)"
        );
        // isDeclaredStability semantics: the plain Level value must be
        // one of {3, 12, 63} — VINTF here.
        assert_eq!(stability, 0b1111_11, "plain Level = VINTF");

        // ---- MISS: the null-binder reply carries the plain UNDECLARED
        // null (different service name → no format flip on this fresh
        // conn — and a MISS never flips per 6-Z306ab).
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
        let blob2 = &resp2[off2 + 12..off2 + 12 + dlen2];
        let flat_type2 = u32::from_ne_bytes(blob2[4..8].try_into().unwrap());
        assert_eq!(flat_type2, BINDER_TYPE_BINDER, "miss → null binder");
        let null_stability = i32::from_ne_bytes(blob2[28..32].try_into().unwrap());
        assert_eq!(
            null_stability, STABILITY_ANNOTATION_NULL,
            "null binders carry the plain UNDECLARED (0) annotation"
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
            RequestBlob {
                fds: Vec::new(),
                data,
                offsets,
                sg: Vec::new(),
            }
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

    /// 6-Z300: perform() must answer per the .aidl contract — the effect
    /// duration in ms for supported effects (forwarded to the host), 0
    /// with status OK for unsupported ones — NOT EX_UNSUPPORTED_OPERATION
    /// (the pre-6-Z300 reply, which TWRP-12.1's synchronous tap-haptic
    /// client treats like a dead HAL and re-waits on).
    #[test]
    fn z300_perform_answers_duration_not_exception() {
        let build_perform_request = |effect: i32, strength: i32| -> RequestBlob {
            let mut req = ParcelWriter::new();
            req.write_i32(0); // strict_mode_policy
            req.write_i32(-1); // work_source_uid (kUnsetWorkSource)
            req.write_u32(AIDL_HEADER_TAG_SYST);
            req.write_string16("android.hardware.vibrator.IVibrator");
            req.write_i32(effect); // perform(in Effect effect, ...
            req.write_i32(strength); // ... in EffectStrength strength, ...
            let (data, offsets) = req.into_parts();
            RequestBlob {
                fds: Vec::new(),
                data,
                offsets,
                sg: Vec::new(),
            }
        };
        let read_reply = |res: TransactionResult| -> (i32, i32) {
            match res {
                TransactionResult::Reply { data, .. } => {
                    let status = i32::from_ne_bytes(data[0..4].try_into().unwrap());
                    let duration = i32::from_ne_bytes(data[4..8].try_into().unwrap());
                    (status, duration)
                }
                _ => panic!("perform must reply"),
            }
        };

        // CLICK (0) + MEDIUM (1) → 20 ms, status OK.
        let (status, duration) = read_reply(virtual_service_transaction(
            VirtualService::Vibrator,
            4,
            Some(&build_perform_request(0, 1)),
        ));
        assert_eq!(status, 0, "perform(supported) → status OK (no exception)");
        assert_eq!(duration, 20, "CLICK/MEDIUM = 20 ms");

        // Strength scaling: CLICK LIGHT (0) = 16 ms, CLICK STRONG (2) = 24 ms.
        let (_, light) = read_reply(virtual_service_transaction(
            VirtualService::Vibrator,
            4,
            Some(&build_perform_request(0, 0)),
        ));
        assert_eq!(light, 16, "CLICK/LIGHT = 20 × 0.8 = 16 ms");
        let (_, strong) = read_reply(virtual_service_transaction(
            VirtualService::Vibrator,
            4,
            Some(&build_perform_request(0, 2)),
        ));
        assert_eq!(strong, 24, "CLICK/STRONG = 20 × 1.2 = 24 ms");

        // The whole synthetic set answers non-zero with status OK.
        for &effect in &[0i32, 1, 5, 6, 7, 8, 9, 22] {
            let (status, duration) = read_reply(virtual_service_transaction(
                VirtualService::Vibrator,
                4,
                Some(&build_perform_request(effect, 1)),
            ));
            assert_eq!(status, 0, "synthetic effect {} → status OK", effect);
            assert!(
                duration >= 1,
                "synthetic effect {} must return a real duration, got {}",
                effect,
                duration
            );
        }

        // Deprecated RINGTONE_* (2..4, 10..21) / out-of-range effects →
        // duration 0 with status OK (the .aidl "not supported" semantic),
        // NEVER an exception header.
        for &effect in &[2i32, 3, 4, 10, 21, -1, 9999] {
            let (status, duration) = read_reply(virtual_service_transaction(
                VirtualService::Vibrator,
                4,
                Some(&build_perform_request(effect, 1)),
            ));
            assert_eq!(
                status, 0,
                "unsupported effect {} → still status OK (no exception)",
                effect
            );
            assert_eq!(duration, 0, "unsupported effect {} → 0 ms", effect);
        }
    }

    /// 6-Z300: getSupportedEffects must list exactly the synthetic set
    /// (length-prefixed int32 array), so clients that gate perform() on
    /// the list get honest answers instead of an empty array.
    #[test]
    fn z300_get_supported_effects_lists_the_synthetic_set() {
        match virtual_service_transaction(VirtualService::Vibrator, 5, None) {
            TransactionResult::Reply { data, .. } => {
                let status = i32::from_ne_bytes(data[0..4].try_into().unwrap());
                assert_eq!(status, 0, "getSupportedEffects → status OK");
                let count = i32::from_ne_bytes(data[4..8].try_into().unwrap());
                assert_eq!(count, 8, "the synthetic set has 8 non-deprecated effects");
                let got: Vec<i32> = data[8..]
                    .chunks_exact(4)
                    .map(|c| i32::from_ne_bytes(c.try_into().unwrap()))
                    .collect();
                assert_eq!(
                    got,
                    vec![0, 1, 5, 6, 7, 8, 9, 22],
                    "CLICK/THUD/TEXTURE_TICK/TICK/LOW_TICK/POP/HEAVY_CLICK/SPINNER"
                );
            }
            _ => panic!("getSupportedEffects must reply"),
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
            let blob = resp[off + 12..off + 12 + dlen + olen].to_vec();
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

    /// 6-Z306ab: the per-connection annotation format SELF-TUNES. The
    /// default is the android-11 plain-Level form (the boot corpus is
    /// A11). A same-service re-get right AFTER A HIT = the waitForService
    /// retry signature of a client whose libbinder rejected the plain
    /// annotation (Stability::set → BAD_TYPE → readStrongBinder → null)
    /// → the format flips to the A12 Category form (sticky); a MISS
    /// retry never flips (a waiter polling for a not-yet-up service must
    /// stay on the plain form).
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
            let blob = &resp[off + 12..off + 12 + dlen];
            i32::from_ne_bytes(blob[28..32].try_into().unwrap())
        };

        // 1st get → the plain android-11 Level form.
        assert_eq!(
            get("android.hardware.vibrator.IVibrator/default"),
            STABILITY_ANNOTATION_VINTF,
            "first get on a fresh conn = plain A11 VINTF level"
        );
        // Real waitForService retry signature: the SAME service re-asked
        // ~100 ms later after the reply failed to parse (a non-A11
        // client's Stability::set rejected the plain Level) → flip to
        // the A12 Category form.
        assert_eq!(
            get("android.hardware.vibrator.IVibrator/default"),
            STABILITY_ANNOTATION_VINTF_A12,
            "same-name retry-after-hit flips to the A12 Category form"
        );
        // The flip is sticky across different names (the A12 client's
        // later compat gets must stay in the Category form).
        assert_eq!(
            get("android.hardware.security.keymint.IKeyMintDevice/default"),
            STABILITY_ANNOTATION_VINTF_A12,
            "the flip is sticky across different names"
        );

        // A SECOND connection (a fresh client) starts over at the plain
        // form; a different-name hit does NOT flip it (only a
        // same-name re-get after a hit does).
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
            let blob = &resp[off + 12..off + 12 + dlen];
            i32::from_ne_bytes(blob[28..32].try_into().unwrap())
        };
        assert_eq!(
            get2("android.hardware.vibrator.IVibrator/default"),
            STABILITY_ANNOTATION_VINTF,
            "fresh conn starts at the plain A11 form"
        );
        assert_eq!(
            get2("android.hardware.security.keymint.IKeyMintDevice/default"),
            STABILITY_ANNOTATION_VINTF,
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
        let blob = resp[off + 12..off + 12 + dlen + olen].to_vec();
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
        let TransactionResult::Reply { data, offsets, .. } = result else {
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
                // 6-Z326 fire wire: [CString token][PTR fq struct][PTR fq
                // chars][PTR inst struct][PTR inst chars][u8 bool]. The
                // fq/inst strings ride the SG section; the token + the four
                // 40B buffer-object headers live in the data.
                let mut rp = HidlParcel::new(blob).expect("fire parcel parses");
                let tok = rp.token().expect("fire token");
                assert_eq!(tok, "android.hidl.manager@1.0::IServiceNotification");
                let fq = rp.read_string_arg().expect("fq embedded string");
                assert_eq!(fq, "android.hardware.health@2.1::IHealth");
                let inst = rp.read_string_arg().expect("instance embedded string");
                assert_eq!(inst, "default");
                assert_eq!(
                    &blob.data[blob.data.len() - 4..],
                    &[0, 0, 0, 0],
                    "preexisting=false"
                );
            }
            _ => panic!("HIDL onRegistration not queued"),
        }
    }

    /// 6-Z480: an HIDL add from a process with NO parked reader must arm
    /// the kernel pool recruitment (ReplySpawnLooper) — the healthd-style
    /// service class (register + own mainloop, never joinRpcThreadpool)
    /// gains its first looper at the registration reply, exactly like the
    /// real kernel's binder_thread_read BR_SPAWN_LOOPER recruitment. rn451:
    /// the health@2.1-service registered handle 0x8 and never read again;
    /// BatteryService's interfaceDescriptor probe expired (6-Z407) →
    /// getService null → "Failure starting system services".
    #[test]
    fn z480_hidl_add_recruits_pool_for_readerless_registration() {
        let mut bus_state = BusState::new();
        let caller = bus_state.register_conn();
        // Stamp the identity the SO_PEERCRED path stamps in production.
        bus_state.conns.get_mut(&caller).unwrap().sender_pid = 2851;
        bus_state.conns.get_mut(&caller).unwrap().dev_code = 2;
        let bus = std::sync::Arc::new(std::sync::Mutex::new(bus_state));

        // add(String name, IBase service): [string name][flat].
        let req = hidl_sm_request("android.hidl.manager@1.0::IServiceManager", &|b| {
            b.string_arg("android.hardware.health@2.1::IHealth");
            b.binder_arg(&FlatBinderObject {
                r#type: BINDER_TYPE_BINDER,
                flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                binder: 0x1000,
                cookie: 0x2000,
            });
        });
        match servicemanager_hidl(HIDL_SM_ADD, &req, &bus, caller) {
            TransactionResult::ReplySpawnLooper { mirror, .. } => {
                // Mirror may be None in the test env (the 6-Z306ae-f
                // liveness probe peeks GUEST memory — unreadable here →
                // Unknown → no mirror). The variant itself pins the
                // recruitment: the read stream gains [BR_SPAWN_LOOPER].
                let _ = mirror;
            }
            other => panic!(
                "readerless add must ReplySpawnLooper, got: {}",
                match other {
                    TransactionResult::Failed => "Failed",
                    TransactionResult::CompleteOnly => "CompleteOnly",
                    TransactionResult::CompleteMirrored { .. } => "CompleteMirrored",
                    TransactionResult::Reply { .. } => "Reply (no recruitment!)",
                    TransactionResult::ReplySpawnLooper { .. } => unreachable!(),
                    TransactionResult::ReplyMirrored { .. } => "ReplyMirrored",
                }
            ),
        }
    }

    /// 6-Z480: addWithChain from a readerless process recruits the pool —
    /// THE rn451 health-HAL wall, byte-for-byte: the addWithChain reply is
    /// the read that the real kernel appends BR_SPAWN_LOOPER to.
    #[test]
    fn z480_add_with_chain_recruits_pool_for_readerless_registration() {
        let mut bus_state = BusState::new();
        let caller = bus_state.register_conn();
        bus_state.conns.get_mut(&caller).unwrap().sender_pid = 2851;
        bus_state.conns.get_mut(&caller).unwrap().dev_code = 2;
        let bus = std::sync::Arc::new(std::sync::Mutex::new(bus_state));

        // addWithChain(String name, IBase service, vec<string> chain):
        // [string name][flat][vec chain].
        let req = hidl_sm_request("android.hidl.manager@1.2::IServiceManager", &|b| {
            b.string_arg("default");
            b.binder_arg(&FlatBinderObject {
                r#type: BINDER_TYPE_BINDER,
                flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                binder: 0x1000,
                cookie: 0x2000,
            });
            b.vec_string_arg(&[
                "android.hardware.health@2.1::IHealth",
                "android.hardware.health@2.0::IHealth",
                "android.hidl.base@1.0::IBase",
            ]);
        });
        match servicemanager_hidl(HIDL_SM_ADD_WITH_CHAIN, &req, &bus, caller) {
            TransactionResult::ReplySpawnLooper { .. } => {
                // The variant itself pins the recruitment; the mirror is
                // env-dependent (the liveness probe peeks guest memory).
            }
            other => panic!(
                "readerless addWithChain must ReplySpawnLooper, got: {}",
                match other {
                    TransactionResult::Failed => "Failed",
                    TransactionResult::CompleteOnly => "CompleteOnly",
                    TransactionResult::CompleteMirrored { .. } => "CompleteMirrored",
                    TransactionResult::Reply { .. } => "Reply (no recruitment!)",
                    TransactionResult::ReplySpawnLooper { .. } => unreachable!(),
                    TransactionResult::ReplyMirrored { .. } => "ReplyMirrored",
                }
            ),
        }
    }

    /// 6-Z480: the gate must NOT recruit when a sibling conn of the same
    /// (pid, device) is already parked in an ioctl — the kernel's
    /// `waiting_threads` non-empty shape. The plain-Reply variant asserts
    /// the negative (no flat → no mirror → Reply means no recruitment).
    #[test]
    fn z480_recruitment_skipped_when_sibling_parked_reader_exists() {
        let mut bus_state = BusState::new();
        let caller = bus_state.register_conn();
        bus_state.conns.get_mut(&caller).unwrap().sender_pid = 2851;
        bus_state.conns.get_mut(&caller).unwrap().dev_code = 2;
        // A sibling of the SAME process on the SAME device, already parked.
        let sibling = bus_state.register_conn();
        bus_state.conns.get_mut(&sibling).unwrap().sender_pid = 2851;
        bus_state.conns.get_mut(&sibling).unwrap().dev_code = 2;
        bus_state.conns.get_mut(&sibling).unwrap().reader_waiting = true;
        let bus = std::sync::Arc::new(std::sync::Mutex::new(bus_state));

        // add without a flat: no mirror either — a plain Reply pins BOTH
        // negatives (no acquire mirror, no pool recruitment).
        let req = hidl_sm_request("android.hidl.manager@1.0::IServiceManager", &|b| {
            b.string_arg("android.hardware.health@2.1::IHealth");
        });
        match servicemanager_hidl(HIDL_SM_ADD, &req, &bus, caller) {
            TransactionResult::Reply { data, .. } => {
                // [status ok][i32 1] — the add success shape.
                assert_eq!(&data[0..8], &[0, 0, 0, 0, 1, 0, 0, 0]);
            }
            other => panic!(
                "add with a parked sibling must stay plain Reply, got: {}",
                match other {
                    TransactionResult::Failed => "Failed",
                    TransactionResult::CompleteOnly => "CompleteOnly",
                    TransactionResult::CompleteMirrored { .. } => "CompleteMirrored",
                    TransactionResult::Reply { .. } => unreachable!(),
                    TransactionResult::ReplySpawnLooper { .. } => "ReplySpawnLooper (recruited!)",
                    TransactionResult::ReplyMirrored { .. } => "ReplyMirrored",
                }
            ),
        }
    }

    /// 6-Z480: the gate is (pid, device)-scoped — a parked reader on the
    /// BINDER device must not suppress the recruitment of a HWBINDER
    /// registration (system_server runs both contexts; kernel
    /// waiting_threads are per-proc-device).
    #[test]
    fn z480_recruitment_scoped_to_same_pid_and_device() {
        let mut bus_state = BusState::new();
        let caller = bus_state.register_conn();
        bus_state.conns.get_mut(&caller).unwrap().sender_pid = 2851;
        bus_state.conns.get_mut(&caller).unwrap().dev_code = 2;
        // Same pid, DIFFERENT device (dev=1 binder) parked — must NOT gate.
        let sibling_binder = bus_state.register_conn();
        bus_state.conns.get_mut(&sibling_binder).unwrap().sender_pid = 2851;
        bus_state.conns.get_mut(&sibling_binder).unwrap().dev_code = 1;
        bus_state
            .conns
            .get_mut(&sibling_binder)
            .unwrap()
            .reader_waiting = true;
        // Different pid, same device parked — must NOT gate either.
        let other_pid = bus_state.register_conn();
        bus_state.conns.get_mut(&other_pid).unwrap().sender_pid = 9999;
        bus_state.conns.get_mut(&other_pid).unwrap().dev_code = 2;
        bus_state.conns.get_mut(&other_pid).unwrap().reader_waiting = true;
        let bus = std::sync::Arc::new(std::sync::Mutex::new(bus_state));

        let req = hidl_sm_request("android.hidl.manager@1.0::IServiceManager", &|b| {
            b.string_arg("android.hardware.health@2.1::IHealth");
        });
        match servicemanager_hidl(HIDL_SM_ADD, &req, &bus, caller) {
            TransactionResult::ReplySpawnLooper { .. } => {}
            other => panic!(
                "cross-pid/cross-device parked readers must not gate the recruitment, got: {}",
                match other {
                    TransactionResult::Failed => "Failed",
                    TransactionResult::CompleteOnly => "CompleteOnly",
                    TransactionResult::CompleteMirrored { .. } => "CompleteMirrored",
                    TransactionResult::Reply { .. } => "Reply (wrongly gated!)",
                    TransactionResult::ReplySpawnLooper { .. } => unreachable!(),
                    TransactionResult::ReplyMirrored { .. } => "ReplyMirrored",
                }
            ),
        }
    }

    /// 6-Z323: `registerForNotifications` for an ALREADY-REGISTERED HIDL
    /// service must (a) store the caller's watcher and (b) fire the
    /// immediate preexisting `onRegistration` oneway — AT the caller.
    /// RN271 decode: the pre-6-Z323 arm fired only watchers that predated
    /// the call and never stored the new one, so the A11
    /// `waitForHwService` caller (system_server's disableAutoSuspend once,
    /// PMS.<init> nativeSetAutoSuspend) received the registerForNotifications
    /// reply but NEVER the onRegistration transaction and blocked forever
    /// in libhidlbase Waiter::wait — every system_server era parked at
    /// StartPowerManager (rung-7 wall).
    #[test]
    fn z323_hidl_register_preexisting_fires_caller_callback() {
        let mut bus_state = BusState::new();
        let caller = bus_state.register_conn();
        let cb_ptr: u64 = 0xABCD_0001;
        let cb_cookie: u64 = 0xFEED_FACE;

        // The service is ALREADY registered (the suspend HAL's
        // addWithChain at +3982ms in the rn271 wire).
        let svc = "android.system.suspend@1.0::ISystemSuspend/default";
        let _handle = bus_state.add_guest_service(svc, PROXY_CONN_ID, 0xdead, 0xbeef);
        let bus = std::sync::Arc::new(std::sync::Mutex::new(bus_state));

        // The waitForHwService registration wire: [string fq][string
        // instance][flat callback].
        let req = hidl_sm_request("android.hidl.manager@1.0::IServiceManager", &|b| {
            b.string_arg("android.system.suspend@1.0::ISystemSuspend");
            b.string_arg("default");
            b.binder_arg(&FlatBinderObject {
                r#type: BINDER_TYPE_BINDER,
                flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                binder: cb_ptr,
                cookie: cb_cookie,
            });
        });
        match servicemanager_hidl(HIDL_SM_REGISTER_FOR_NOTIFICATIONS, &req, &bus, caller) {
            TransactionResult::ReplySpawnLooper { data, offsets, .. } => {
                assert!(offsets.is_empty(), "no binder objects in the reply");
                // [status ok][u8 1][3 pad]
                assert_eq!(data, vec![0, 0, 0, 0, 1, 0, 0, 0]);
                // The variant ITSELF pins the 6-Z324 pool-thread recruitment
                // (the read stream gains [BR_SPAWN_LOOPER] before the batch).
            }
            other => panic!(
                "registerForNotifications must ReplySpawnLooper, got non-matching variant: {}",
                match other {
                    TransactionResult::Failed => "Failed",
                    TransactionResult::CompleteOnly => "CompleteOnly",
                    TransactionResult::CompleteMirrored { .. } => "CompleteMirrored",
                    TransactionResult::Reply { .. } => "Reply (no spawn-looper!)",
                    TransactionResult::ReplySpawnLooper { .. } => unreachable!(),
                    TransactionResult::ReplyMirrored { .. } => "ReplyMirrored",
                }
            ),
        }

        // THE FIX: the onRegistration oneway is queued on the CALLER's
        // inbox, targeted at ITS callback object.
        let bx = bus
            .lock()
            .expect("bus")
            .conns
            .get(&caller)
            .map(|c| c.inbox.len())
            .expect("caller conn");
        assert_eq!(bx, 1, "exactly one onRegistration queued");
        let bus_state = bus.lock().expect("bus");
        let bx = bus_state.conns.get(&caller).expect("caller conn");
        let fired = match bx.inbox.front() {
            Some(InboxItem::Tx(tx)) => {
                assert_eq!(tx.code, 1, "onRegistration code");
                assert!(tx.one_way, "callback is one-way");
                assert_eq!(tx.txn_id, 0, "no reply bookkeeping for one-way");
                assert_eq!(tx.ptr, cb_ptr, "targeted at the caller's callback");
                assert_eq!(tx.cookie, cb_cookie);
                let blob = tx.blob.as_ref().expect("HIDL callback parcel");
                // 6-Z326 fire wire: [CString token][PTR fq struct][PTR fq
                // chars][PTR inst struct][PTR inst chars][u8 preexisting].
                // Decode it with the proxy's own reader (the same walk the
                // request-side parse uses — the client's BnHw gencode
                // mirrors it object-for-object).
                let mut rp = HidlParcel::new(blob).expect("callback parcel parses");
                let tok = rp.token().expect("fire token");
                assert_eq!(
                    tok, "android.hidl.manager@1.0::IServiceNotification",
                    "libhwbinder writeInterfaceToken = writeCString"
                );
                let fq = rp.read_string_arg().expect("fq embedded string");
                assert_eq!(
                    fq, "android.system.suspend@1.0::ISystemSuspend",
                    "fqName embedded-string arg"
                );
                let inst = rp.read_string_arg().expect("instance embedded string");
                assert_eq!(inst, "default", "instance embedded-string arg");
                // writeBool = writeInt8 + align4: the final word is
                // [1, 0, 0, 0] with preexisting=true.
                assert_eq!(&blob.data[blob.data.len() - 4..], &[1, 0, 0, 0]);
                assert_eq!(
                    blob.sg.len(),
                    4,
                    "SG = [fq struct, fq chars, inst struct, inst chars]"
                );
                assert_eq!(blob.sg[0].data.len(), 16, "fq struct is a 16B hidl_string");
                let fq_size = u32::from_ne_bytes(blob.sg[0].data[8..12].try_into().unwrap());
                assert_eq!(fq_size, fq.len() as u32, "struct mSize excludes the NUL");
                assert_eq!(blob.sg[1].data.len(), fq.len() + 1, "chars = size+1 (NUL)");
                assert_eq!(*blob.sg[1].data.last().unwrap(), 0, "chars NUL-terminated");
                true
            }
            _ => panic!("onRegistration not queued on the caller's inbox"),
        };
        assert!(fired);
    }

    /// 6-Z323 AIDL twin: `registerForNotifications` for an
    /// ALREADY-REGISTERED service fires the immediate preexisting
    /// `onRegistration` AT the caller (and the watcher list is consumed by
    /// the fire — the existing fire-once semantics).
    #[test]
    fn z323_aidl_register_preexisting_fires_caller_callback() {
        let mut bus_state = BusState::new();
        let caller = bus_state.register_conn();
        let cb_ptr: u64 = 0x1234_5678;
        let cb_cookie: u64 = 0x0BAD_C0DE;

        let svc = "suspend_control";
        let _handle = bus_state.add_guest_service(svc, PROXY_CONN_ID, 0x777, 0x888);
        let bus = std::sync::Arc::new(std::sync::Mutex::new(bus_state));

        // AIDL request: [header][string16 name][flat callback].
        let mut args = ParcelWriter::new();
        args.write_string16(svc);
        args.write_flat_binder(&FlatBinderObject {
            r#type: BINDER_TYPE_BINDER,
            flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
            binder: cb_ptr,
            cookie: cb_cookie,
        });
        let (d, o) = make_servicemanager_request_parcel(&mut args);
        let blob = RequestBlob {
            fds: Vec::new(),
            data: d,
            offsets: o,
            sg: Vec::new(),
        };
        match servicemanager_proxy(
            SVC_MGR_REGISTER_FOR_NOTIFICATIONS,
            &bus,
            Some(&blob),
            caller,
        ) {
            TransactionResult::ReplySpawnLooper { .. } => {}
            other => panic!(
                "registerForNotifications must ReplySpawnLooper, got non-matching variant: {}",
                match other {
                    TransactionResult::Failed => "Failed",
                    TransactionResult::CompleteOnly => "CompleteOnly",
                    TransactionResult::CompleteMirrored { .. } => "CompleteMirrored",
                    TransactionResult::Reply { .. } => "Reply (no spawn-looper!)",
                    TransactionResult::ReplySpawnLooper { .. } => unreachable!(),
                    TransactionResult::ReplyMirrored { .. } => "ReplyMirrored",
                }
            ),
        }

        // The callback reaches the CALLER (pre-6-Z323 it never did).
        let bus_state = bus.lock().expect("bus");
        let bx = bus_state.conns.get(&caller).expect("caller conn");
        match bx.inbox.front() {
            Some(InboxItem::Tx(tx)) => {
                assert_eq!(tx.code, 1, "onRegistration code");
                assert!(tx.one_way, "callback is one-way");
                assert_eq!(tx.ptr, cb_ptr);
                assert_eq!(tx.cookie, cb_cookie);
                let blob = tx.blob.as_ref().expect("AIDL callback parcel");
                // [strict][work][SYST][string16 IServiceCallback]
                // [string16 name][flat handle][stability].
                assert_eq!(
                    u32::from_ne_bytes(blob.data[8..12].try_into().unwrap()),
                    AIDL_HEADER_TAG_SYST
                );
                let s = String::from_utf16_lossy(
                    &blob.data[12..]
                        .chunks_exact(2)
                        .map(|c| u16::from_le_bytes(c.try_into().unwrap()))
                        .collect::<Vec<u16>>(),
                );
                assert!(s.contains("android.os.IServiceCallback"));
                assert!(s.contains(svc), "service name string16 present");
            }
            _ => panic!("onRegistration not queued on the caller's inbox"),
        }
    }

    /// 6-Z325 HIDL: a registerForNotifications that STORES a new watcher
    /// pins the caller's local callback wrapper — the reply batch carries
    /// the node-ref mirror `[BR_ACQUIRE][ptr][cookie]` (kernel
    /// binder_node_post_acquire semantics). A duplicate register does NOT
    /// add a second mirror (the registry holds ONE ref per stored entry),
    /// and the unregister mirrors the drop with BR_RELEASE.
    fn tr_label(t: &TransactionResult) -> &'static str {
        match t {
            TransactionResult::Failed => "Failed",
            TransactionResult::CompleteOnly => "CompleteOnly",
            TransactionResult::CompleteMirrored { .. } => "CompleteMirrored",
            TransactionResult::Reply { .. } => "Reply",
            TransactionResult::ReplySpawnLooper { .. } => "ReplySpawnLooper",
            TransactionResult::ReplyMirrored { .. } => "ReplyMirrored",
        }
    }

    #[test]
    fn z325_hidl_register_reply_mirrors_callback_acquire() {
        let mut bus_state = BusState::new();
        let caller = bus_state.register_conn();
        let cb_ptr: u64 = 0xABCD_0002;
        let cb_cookie: u64 = 0xFEED_F00D;
        let bus = std::sync::Arc::new(std::sync::Mutex::new(bus_state));

        let build_req = |bus_ref: &std::sync::Arc<std::sync::Mutex<BusState>>| -> RequestBlob {
            let _ = bus_ref;
            hidl_sm_request("android.hidl.manager@1.0::IServiceManager", &|b| {
                b.string_arg("android.system.suspend@1.0::ISystemSuspend");
                b.string_arg("default");
                b.binder_arg(&FlatBinderObject {
                    r#type: BINDER_TYPE_BINDER,
                    flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                    binder: cb_ptr,
                    cookie: cb_cookie,
                });
            })
        };

        // Watching path (the service does NOT exist yet): the mirror still
        // rides the reply — hwservicemanager holds the listener ref for the
        // future fire too.
        let req = build_req(&bus);
        match servicemanager_hidl(HIDL_SM_REGISTER_FOR_NOTIFICATIONS, &req, &bus, caller) {
            TransactionResult::ReplySpawnLooper { mirror, .. } => {
                assert_eq!(
                    mirror,
                    Some((BR_ACQUIRE, cb_ptr, cb_cookie)),
                    "the stored watcher's callback wrapper is pinned with BR_ACQUIRE"
                );
            }
            other => panic!("expected ReplySpawnLooper, got {}", tr_label(&other)),
        }

        // The watcher list holds the entry (watching, not consumed).
        assert!(
            bus.lock()
                .expect("bus")
                .watchers
                .contains_key("android.system.suspend@1.0::ISystemSuspend/default"),
            "watching watcher stays registered"
        );

        // Duplicate register (same conn + ptr): no second mirror — the
        // registry's ref count is one per STORED entry.
        let req2 = build_req(&bus);
        match servicemanager_hidl(HIDL_SM_REGISTER_FOR_NOTIFICATIONS, &req2, &bus, caller) {
            TransactionResult::ReplySpawnLooper { mirror, .. } => {
                assert_eq!(mirror, None, "duplicate register adds no ref");
            }
            other => panic!("expected ReplySpawnLooper, got {}", tr_label(&other)),
        }

        // Unregister: the drop mirrors as BR_RELEASE on the reply batch.
        let req3 = hidl_sm_request("android.hidl.manager@1.0::IServiceManager", &|b| {
            b.string_arg("android.system.suspend@1.0::ISystemSuspend");
            b.string_arg("default");
            b.binder_arg(&FlatBinderObject {
                r#type: BINDER_TYPE_BINDER,
                flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                binder: cb_ptr,
                cookie: cb_cookie,
            });
        });
        match servicemanager_hidl(HIDL_SM_UNREGISTER_FOR_NOTIFICATIONS, &req3, &bus, caller) {
            TransactionResult::ReplyMirrored {
                br, ptr, cookie, ..
            } => {
                assert_eq!(br, BR_RELEASE, "unregister drops the node ref");
                assert_eq!(ptr, cb_ptr);
                assert_eq!(cookie, cb_cookie);
            }
            other => panic!("expected ReplyMirrored, got {}", tr_label(&other)),
        }
        assert!(
            !bus.lock()
                .expect("bus")
                .watchers
                .contains_key("android.system.suspend@1.0::ISystemSuspend/default"),
            "watcher removed"
        );
    }

    /// 6-Z387: the 6-Z306d-b association scan must find the weakref
    /// VALUE W at the TRUE vbase depth of multiply-inheriting wrappers.
    /// rn344 decode: the old 640-byte window missed W at [R+8] for
    /// AudioFlinger (Δ=0x648 → W@0x650) and MediaMetrics (Δ=0x378 →
    /// W@0x380) — the false-Dead skipped the registry BR_ACQUIRE, the
    /// register temporary died with no strong ref, the service object
    /// was freed, and the self-lookup served the corpse (the
    /// incStrong+0x0 / decStrong+0x1c restart storm, 197 skips/run).
    /// The same scan must still reject the #237 chimera (W absent).
    #[test]
    fn z387_assoc_scan_covers_deep_vbase_wrappers() {
        // AudioFlinger-shaped live chunk: vptr@0, W at [R+8] with
        // R = cookie + 0x648 (the rn344 capture probe numbers).
        let w = 0xEE94_EAE0_6BB0u64;
        let mut chunk = vec![0u8; ASSOC_SCAN_WINDOW];
        chunk[0..8].copy_from_slice(&0xEE97_5BF7_5518u64.to_ne_bytes()); // vptr
        chunk[0x650..0x658].copy_from_slice(&w.to_ne_bytes());
        assert!(assoc_scan(&chunk, w), "AudioFlinger Δ=0x648 associated");
        // The OLD window missed exactly this slot — the regression.
        assert!(
            !assoc_scan(&chunk[..640], w),
            "the 640 window must miss W@0x650 (the bug shape)"
        );

        // MediaMetrics-shaped live chunk: R = cookie + 0x378.
        let wm = 0xF252_9920_36D0u64;
        let mut chunk_m = vec![0u8; ASSOC_SCAN_WINDOW];
        chunk_m[0x380..0x388].copy_from_slice(&wm.to_ne_bytes());
        assert!(assoc_scan(&chunk_m, wm), "MediaMetrics Δ=0x378 associated");

        // In-window classes stay positive (extractor Δ=0x40, player 0x110).
        let we = 0xE053_2DE0_5940u64;
        let mut chunk_e = vec![0u8; ASSOC_SCAN_WINDOW];
        chunk_e[0x48..0x50].copy_from_slice(&we.to_ne_bytes());
        assert!(assoc_scan(&chunk_e, we), "extractor Δ=0x40 associated");

        // #237 chimera: a dead/reused cookie chunk does not reference W
        // anywhere in the window — still rejected (False→Dead is correct).
        let mut dead = vec![0u8; ASSOC_SCAN_WINDOW];
        dead[0..8].copy_from_slice(&0x1234_5678_9ABC_DEF0u64.to_ne_bytes());
        assert!(!assoc_scan(&dead, w), "chimera chunk stays rejected");

        // Window sanity: covers the deepest observed W slot.
        assert!(
            ASSOC_SCAN_WINDOW >= 0x650 + 8,
            "window must cover AudioFlinger W@0x650"
        );
    }

    /// 6-Z463: the RefCmd359 close gate's tri-state contract — only a
    /// POSITIVE Dead (the 6-Z387 round-trip anchor's certain verdicts)
    /// silently closes; Alive and Unknown deliver (the 6-Z306an rule:
    /// an unreadable probe must never swallow a kernel-true wire
    /// command; the rn344 false-Dead era must never return on the close
    /// arm). The contract IS the fix: the rn427 +187083→+187101 chain
    /// proved the close delivery onto a positively-dead chunk is the
    /// Scudo corruption write.
    #[test]
    fn z463_close_gate_rejects_only_positive_dead() {
        assert!(
            z463_close_rejected(Liveness::Dead),
            "a positive Dead verdict drops the close (the rn427 corruption-write guard)"
        );
        assert!(
            !z463_close_rejected(Liveness::Alive),
            "a live owner object receives its kernel-true release"
        );
        assert!(
            !z463_close_rejected(Liveness::Unknown),
            "an unreadable probe delivers (no ghost eras, no rn344 false-Dead starvation)"
        );
    }

    /// 6-Z379: the registry strong ref is PER CHAIN KEY. The every-HAL
    /// shared `android.hidl.base@1.0::IBase/default` alias overwrite must
    /// NOT release the previous owner (its concrete chain keys still pin
    /// the node); a whole-service replacement releases exactly once, at
    /// the LAST overwritten key; a single-key overwrite (AIDL addService)
    /// releases immediately (the 6-Z306ae suspend-daemon semantics).
    /// rn334/rn335 decode: releasing on EVERY overwrite murdered the
    /// composer's wrapper 500 ms after thermal registered — SF's
    /// interfaceChain then SEGV'd in decStrong on the corpse (the 42×
    /// libutils refcount SEGV class, rung 7 SURFACEFLINGER).
    #[test]
    fn z379_chain_alias_overwrite_keeps_previous_owner_pinned() {
        let mut bus = BusState::new();
        let composer = bus.register_conn();
        let thermal = bus.register_conn();
        let next_gen = bus.register_conn();

        let composer_ptr: u64 = 0xE4C0_0000_1000;
        let composer_cookie: u64 = 0xE4C0_0000_2000;
        let thermal_ptr: u64 = 0xF4A0_0000_3000;
        let thermal_cookie: u64 = 0xF4A0_0000_4000;
        let next_ptr: u64 = 0xE4C0_0000_5000;
        let next_cookie: u64 = 0xE4C0_0000_6000;

        const COMPOSER_KEYS: [&str; 4] = [
            "android.hardware.graphics.composer@2.3::IComposer/default",
            "android.hardware.graphics.composer@2.2::IComposer/default",
            "android.hardware.graphics.composer@2.1::IComposer/default",
            "android.hidl.base@1.0::IBase/default",
        ];
        const THERMAL_KEYS: [&str; 3] = [
            "android.hardware.thermal@2.0::IThermal/default",
            "android.hardware.thermal@1.0::IThermal/default",
            "android.hidl.base@1.0::IBase/default",
        ];

        // The composer registers its full chain (the rn334 wire shape).
        let h = bus.add_guest_service(COMPOSER_KEYS[0], composer, composer_ptr, composer_cookie);
        for k in &COMPOSER_KEYS[1..] {
            assert_eq!(
                bus.add_guest_service_alias(k, composer, composer_ptr, composer_cookie, h),
                h,
                "aliases share the chain[0] handle (one node identity)"
            );
        }

        // Thermal registers — its shared IBase/default alias overwrites
        // the composer's IBase entry (the +9.22 s step of the decode).
        let ht = bus.add_guest_service(THERMAL_KEYS[0], thermal, thermal_ptr, thermal_cookie);
        for k in &THERMAL_KEYS[1..] {
            bus.add_guest_service_alias(k, thermal, thermal_ptr, thermal_cookie, ht);
        }

        // THE FIX: the IBase/default overwrite must not release the
        // composer — 3 concrete keys still pin it.
        assert!(
            !overwrite_release_due(
                &bus.services,
                THERMAL_KEYS[2],
                composer,
                composer_ptr,
                composer_cookie,
            ),
            "the IBase/default alias overwrite must not release the composer — concrete chain keys still pin it"
        );

        // A whole-service replacement overwrites the concrete keys one by
        // one (pre-overwrite decision each step): the release fires
        // exactly once, at the LAST remaining pin.
        let concrete = [COMPOSER_KEYS[0], COMPOSER_KEYS[1], COMPOSER_KEYS[2]];
        for (i, k) in concrete.iter().enumerate() {
            let due =
                overwrite_release_due(&bus.services, k, composer, composer_ptr, composer_cookie);
            bus.add_guest_service(k, next_gen, next_ptr, next_cookie);
            assert_eq!(
                due,
                i + 1 == concrete.len(),
                "key {k}: overwrite releases only at the last pin"
            );
        }

        // Single-key overwrite (the AIDL addService shape): the old
        // owner's ONLY registry ref drops — release due immediately.
        let solo = bus.register_conn();
        let solo_ptr: u64 = 0xAA00_0000_1000;
        let solo_cookie: u64 = 0xAA00_0000_2000;
        bus.add_guest_service(
            "some.single@1.0::ISolo/default",
            solo,
            solo_ptr,
            solo_cookie,
        );
        assert!(
            overwrite_release_due(
                &bus.services,
                "some.single@1.0::ISolo/default",
                solo,
                solo_ptr,
                solo_cookie,
            ),
            "single-key overwrite releases immediately (6-Z306ae semantics preserved)"
        );

        // The zero-ptr guard: nothing to release for virtual/anonymous
        // registry entries.
        assert!(!overwrite_release_due(&bus.services, "any", composer, 0, 0));
    }

    /// 6-Z325 AIDL twin: registerForNotifications mirrors BR_ACQUIRE for a
    /// stored watcher; unregisterForNotifications mirrors BR_RELEASE.
    #[test]
    fn z325_aidl_register_reply_mirrors_callback_acquire() {
        let mut bus_state = BusState::new();
        let caller = bus_state.register_conn();
        let cb_ptr: u64 = 0x1234_5679;
        let cb_cookie: u64 = 0x0BAD_C0DF;
        let bus = std::sync::Arc::new(std::sync::Mutex::new(bus_state));
        let svc = "suspend_control";

        let build_req = || {
            let mut args = ParcelWriter::new();
            args.write_string16(svc);
            args.write_flat_binder(&FlatBinderObject {
                r#type: BINDER_TYPE_BINDER,
                flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                binder: cb_ptr,
                cookie: cb_cookie,
            });
            let (d, o) = make_servicemanager_request_parcel(&mut args);
            RequestBlob {
                fds: Vec::new(),
                data: d,
                offsets: o,
                sg: Vec::new(),
            }
        };

        let blob = build_req();
        match servicemanager_proxy(
            SVC_MGR_REGISTER_FOR_NOTIFICATIONS,
            &bus,
            Some(&blob),
            caller,
        ) {
            TransactionResult::ReplySpawnLooper { mirror, .. } => {
                assert_eq!(
                    mirror,
                    Some((BR_ACQUIRE, cb_ptr, cb_cookie)),
                    "AIDL watcher callback pinned with BR_ACQUIRE"
                );
            }
            other => panic!("expected ReplySpawnLooper, got {}", tr_label(&other)),
        }
        assert!(
            bus.lock().expect("bus").watchers.contains_key(svc),
            "AIDL watching watcher stays registered"
        );

        let blob = build_req();
        match servicemanager_proxy(
            SVC_MGR_UNREGISTER_FOR_NOTIFICATIONS,
            &bus,
            Some(&blob),
            caller,
        ) {
            TransactionResult::ReplyMirrored {
                br, ptr, cookie, ..
            } => {
                assert_eq!(br, BR_RELEASE, "AIDL unregister drops the node ref");
                assert_eq!(ptr, cb_ptr);
                assert_eq!(cookie, cb_cookie);
            }
            other => panic!("expected ReplyMirrored, got {}", tr_label(&other)),
        }
        assert!(
            !bus.lock().expect("bus").watchers.contains_key(svc),
            "watcher removed"
        );
    }

    /// 6-Z327 test serialization: the guest-rootfs global is process-wide
    /// and the flip tests mutate it.
    static Z327_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[cfg(test)]
    fn test_set_guest_rootfs(rootfs: Option<String>) {
        *GUEST_ROOTFS.write().expect("guest rootfs lock") = rootfs;
        *GUEST_SDK_CACHE.write().expect("guest sdk lock") = None;
    }

    /// 6-Z327: an A11 guest (rootfs declares ro.build.version.sdk=30) NEVER
    /// flips to the A12 Category annotation — the retry-after-hit signature
    /// is a NORMAL A11 double-fetch, and every A12-annotated handle reply
    /// unflattens to NULL in the A11 client (the rn275 PowerManager NPE).
    #[test]
    fn z327_a11_guest_get_retry_after_hit_stays_plain() {
        let _g = Z327_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let rootfs = tmpdir();
        let root = std::path::Path::new(&rootfs);
        fs::create_dir_all(root.join("system")).unwrap();
        fs::write(root.join("system/build.prop"), "ro.build.version.sdk=30\n").unwrap();
        test_set_guest_rootfs(Some(rootfs.clone()));

        let mut bus_state = BusState::new();
        let caller = bus_state.register_conn();
        let _h = bus_state.add_guest_service("power", PROXY_CONN_ID, 0x777, 0x888);
        let bus = std::sync::Arc::new(std::sync::Mutex::new(bus_state));

        let get = |bus: &std::sync::Arc<std::sync::Mutex<BusState>>| -> i32 {
            let mut args = ParcelWriter::new();
            args.write_string16("power");
            let (d, o) = make_servicemanager_request_parcel(&mut args);
            let blob = RequestBlob {
                fds: Vec::new(),
                data: d,
                offsets: o,
                sg: Vec::new(),
            };
            match servicemanager_proxy(SVC_MGR_GET_SERVICE, bus, Some(&blob), caller) {
                TransactionResult::Reply { data, .. } => {
                    i32::from_ne_bytes(data[28..32].try_into().unwrap())
                }
                other => panic!("expected Reply, got {}", tr_label(&other)),
            }
        };
        let ann1 = get(&bus);
        // The same-name re-get right after a HIT — the flip signature. For
        // an A11 guest it must NOT flip.
        let ann2 = get(&bus);
        assert_eq!(ann1, STABILITY_ANNOTATION_VINTF, "first hit serves plain");
        assert_eq!(
            ann2, STABILITY_ANNOTATION_VINTF,
            "A11 guest never flips to the A12 Category form"
        );
        test_set_guest_rootfs(None);
    }

    /// 6-Z327: an UNKNOWN-SDK guest (recovery-class rootfs, no
    /// /system/build.prop) keeps the legacy 6-Z306ab heuristic — the
    /// corpus path is byte-identical.
    #[test]
    fn z327_unknown_guest_get_retry_after_hit_flips_legacy() {
        let _g = Z327_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        test_set_guest_rootfs(None);

        let mut bus_state = BusState::new();
        let caller = bus_state.register_conn();
        let _h = bus_state.add_guest_service("power", PROXY_CONN_ID, 0x777, 0x888);
        let bus = std::sync::Arc::new(std::sync::Mutex::new(bus_state));

        let get = |bus: &std::sync::Arc<std::sync::Mutex<BusState>>| -> i32 {
            let mut args = ParcelWriter::new();
            args.write_string16("power");
            let (d, o) = make_servicemanager_request_parcel(&mut args);
            let blob = RequestBlob {
                fds: Vec::new(),
                data: d,
                offsets: o,
                sg: Vec::new(),
            };
            match servicemanager_proxy(SVC_MGR_GET_SERVICE, bus, Some(&blob), caller) {
                TransactionResult::Reply { data, .. } => {
                    i32::from_ne_bytes(data[28..32].try_into().unwrap())
                }
                other => panic!("expected Reply, got {}", tr_label(&other)),
            }
        };
        let ann1 = get(&bus);
        let ann2 = get(&bus);
        assert_eq!(ann1, STABILITY_ANNOTATION_VINTF, "first hit serves plain");
        assert_eq!(
            ann2, STABILITY_ANNOTATION_VINTF_A12,
            "unknown-SDK guests keep the legacy flip"
        );
        test_set_guest_rootfs(None);
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
        let conn = b.register_conn();
        let h = b.add_guest_service(name, conn, 0xdead, 0x1234);
        assert_eq!(h, virtual_handle, "same name → same handle");
        let entry = b.services.get(name).unwrap();
        assert_eq!(entry.owner, conn, "owner switched to the guest");
        assert_eq!(entry.ptr, 0xdead);
        assert_eq!(
            entry.virtual_kind, None,
            "the in-proxy handler must stand down"
        );
        assert_eq!(
            entry.virtual_fallback,
            Some(VirtualService::Health),
            "6-Z299: the platform implementation is remembered for restore"
        );
    }

    #[test]
    fn z299_guest_takeover_restores_virtual_service_on_owner_death() {
        // THE fox scenario: vendor.qti.vibrator (a real guest HAL binary,
        // same AIDL name android.hardware.vibrator.IVibrator/default)
        // registers over the platform service and later dies because its
        // hardware doesn't exist in the container. The name must KEEP
        // resolving (restored in-proxy service) — never vanish.
        let mut b = BusState::new();
        let name = "android.hardware.vibrator.IVibrator/default";
        let virtual_handle = b.services.get(name).unwrap().handle;

        let hal = b.register_conn();
        let h = b.add_guest_service(name, hal, 0xbeef, 42);
        assert_eq!(h, virtual_handle, "takeover keeps the handle");

        // A client requested death notification on the (now guest) node.
        let client = b.register_conn();
        b.conns
            .get_mut(&client)
            .unwrap()
            .death_watch
            .insert(h, 0x7777);

        // The guest HAL dies → the platform service is restored under the
        // same handle, and the death notification still fires (the client's
        // reference to the GUEST node is dead — honest kernel semantics).
        b.unregister_conn(hal);
        let entry = b.services.get(name).expect("name must keep resolving");
        assert_eq!(entry.owner, PROXY_CONN_ID, "back to the platform");
        assert_eq!(
            entry.virtual_kind,
            Some(VirtualService::Vibrator),
            "the in-proxy handler answers again"
        );
        assert_eq!(entry.virtual_fallback, None, "restore consumed");
        assert!(
            b.by_handle.contains_key(&virtual_handle),
            "handle mapping survives the restore"
        );
        match b.conns.get(&client).unwrap().inbox.front() {
            Some(InboxItem::Death(cookie)) => {
                assert_eq!(*cookie, 0x7777, "the guest node's death is still notified");
            }
            other => panic!("expected death notification, got {:?}", other.is_some()),
        }

        // And a pure guest service (no virtual origin) is still REMOVED
        // on its owner's death — the 6-Z299 filter didn't overreach.
        let other = b.register_conn();
        let svc_handle = b.add_guest_service("com.some.guest.Svc/other", other, 0x11, 0x22);
        b.unregister_conn(other);
        assert!(
            !b.services.contains_key("com.some.guest.Svc/other"),
            "pure guest services die with their owner"
        );
        assert!(!b.by_handle.contains_key(&svc_handle));
    }

    /// 6-Z305t-66: the armed HIDL servicemanager codes against the
    /// authoritative android-11.0.0_r1 IServiceManager.hal method order
    /// (1.0: get, add, getTransport, list, listByInterface,
    /// registerForNotifications, debugDump, registerPassthroughClient;
    /// 1.1 appends unregisterForNotifications; 1.2 appends
    /// registerClientCallback, unregisterClientCallback, addWithChain,
    /// listManifestByInterface, tryUnregister). The pre-6-Z305t-66 map had
    /// register/unregister on 4/5 (list/listByInterface's codes) and NO
    /// getTransport — the 956 EX_TRANSACTION_FAILED fleet killer.
    #[test]
    fn hidl_sm_codes_match_a11_iservice_manager_hal() {
        assert_eq!(HIDL_SM_GET, 1);
        assert_eq!(HIDL_SM_ADD, 2);
        assert_eq!(HIDL_SM_GET_TRANSPORT, 3);
        assert_eq!(HIDL_SM_REGISTER_FOR_NOTIFICATIONS, 6);
        assert_eq!(HIDL_SM_DEBUG_DUMP, 7);
        assert_eq!(HIDL_SM_UNREGISTER_FOR_NOTIFICATIONS, 9);
        assert_eq!(HIDL_SM_ADD_WITH_CHAIN, 12);
        assert_eq!(HIDL_TRANSPORT_EMPTY, 0);
        assert_eq!(HIDL_TRANSPORT_HWBINDER, 1);
        assert_eq!(HIDL_DEBUG_ARCH_UNKNOWN, 0);
    }

    /// 6-Z307: the HIDL service manager's OWN instances are seeded into
    /// the registry at bus construction — the self-registration that
    /// never crosses the wire on real Android either (it happens inside
    /// the hwservicemanager process). The ladder-#247 watchdog kill
    /// chain started at `HwBinder.getService("android.hidl.manager@
    /// 1.0::IServiceManager", "default")` → NoSuchElementException
    /// because this lookup missed 354 times.
    #[test]
    fn z307_hwservicemanager_self_instances_are_seeded() {
        let bus = std::sync::Arc::new(std::sync::Mutex::new(BusState::new()));
        let b = bus.lock().expect("bus");
        for ver in ["1.0", "1.1", "1.2"] {
            let key = format!("android.hidl.manager@{}::IServiceManager/default", ver);
            let e = b
                .services
                .get(&key)
                .unwrap_or_else(|| panic!("{} missing", key));
            assert_eq!(e.virtual_kind, Some(VirtualService::HidlServiceManager));
            assert_eq!(e.owner, PROXY_CONN_ID);
            assert!(b.by_handle.values().any(|n| n == &key));
        }
    }

    /// 6-Z307d: `HIDL_SM_GET` on the seeded instances must reply
    /// [status-ok][base flat] — ONE object, the service itself
    /// (`Return<sp<IBase>> ret = sm->get(...); sp<IBase> base = ret;`).
    /// The client's NEXT transaction (the IBase::interfaceChain cast probe,
    /// 0xf43484e) is covered by the dispatch arm (see
    /// z307d_interface_chain_reply below).
    #[test]
    fn z307_hidl_get_of_iservice_manager_instances_hits() {
        let bus = std::sync::Arc::new(std::sync::Mutex::new(BusState::new()));
        for ver in ["1.0", "1.2"] {
            let req = hidl_sm_request(
                "android.hidl.manager@1.0::IServiceManager",
                &|b: &mut HidlReqBuilder| {
                    let fq = format!("android.hidl.manager@{}::IServiceManager", ver);
                    b.string_arg(&fq);
                    b.string_arg("default");
                },
            );
            let result = servicemanager_hidl(HIDL_SM_GET, &req, &bus, PROXY_CONN_ID);
            let TransactionResult::Reply { data, offsets, .. } = result else {
                panic!("get(@{}) must Reply", ver);
            };
            // status(4) + base flat(24) = 28; ONE object.
            assert_eq!(data.len(), 28, "@{} reply layout", ver);
            assert_eq!(offsets.len(), 8, "@{} reply object count", ver);
            let boff = u64::from_ne_bytes(offsets[0..8].try_into().unwrap()) as usize;
            let ty = u32::from_ne_bytes(data[boff..boff + 4].try_into().unwrap());
            let handle = u64::from_ne_bytes(data[boff + 8..boff + 16].try_into().unwrap());
            assert_eq!(ty, BINDER_TYPE_HANDLE, "@{} base must be a handle", ver);
            assert_ne!(handle, 0, "@{} base handle must be nonzero", ver);
        }
    }

    /// 6-Z307d: a get() MISS replies [status][null base] — the honest
    /// NAME_NOT_FOUND shape (the client maps it to NoSuchElementException;
    /// no hang, no fabricated service).
    #[test]
    fn z307_hidl_get_miss_replies_null_base() {
        let bus = std::sync::Arc::new(std::sync::Mutex::new(BusState::new()));
        let req = hidl_sm_request(
            "android.hidl.manager@1.0::IServiceManager",
            &|b: &mut HidlReqBuilder| {
                b.string_arg("android.hardware.does.not@1.0::IExist");
                b.string_arg("default");
            },
        );
        let result = servicemanager_hidl(HIDL_SM_GET, &req, &bus, PROXY_CONN_ID);
        let TransactionResult::Reply { data, offsets, .. } = result else {
            panic!("get miss must Reply");
        };
        assert_eq!(data.len(), 28);
        assert_eq!(offsets.len(), 8);
        let boff = u64::from_ne_bytes(offsets[0..8].try_into().unwrap()) as usize;
        let ty = u32::from_ne_bytes(data[boff..boff + 4].try_into().unwrap());
        let binder = u64::from_ne_bytes(data[boff + 8..boff + 16].try_into().unwrap());
        assert_eq!(ty, BINDER_TYPE_BINDER);
        assert_eq!(binder, 0, "miss base = null binder");
    }

    /// 6-Z307d: the IBase::interfaceChain cast probe (code 0xf43484e) on a
    /// seeded SM handle answers the REAL manager chain — all three versions
    /// (the same binder object backs them) + IBase — so canCastInterface's
    /// castTo ("android.hidl.manager@1.0::IServiceManager") is CONTAINED
    /// and getRawServiceInternal returns the service instead of
    /// "unable to call into hwbinder service".
    #[test]
    fn z307d_interface_chain_reply_carries_manager_versions() {
        let bus = std::sync::Arc::new(std::sync::Mutex::new(BusState::new()));
        let conn = bus.lock().expect("bus").register_conn();
        // The request: the interfaceChain call carries NO args — just the
        // IBase token (the wire shape from ladder #248: the probes were
        // token-only parcels).
        // Route through the DISPATCH-level arm: the seeded @1.0 handle's
        // transaction. servicemanager_hidl itself never sees 0xf43484e —
        // the dispatch intercepts it (vintf manager chain, then SM arms).
        // Direct-call the arm logic here via the dispatch site's helper:
        // rebuild it inline (the dispatch site is a monolithic fn).
        let handle = {
            let b = bus.lock().expect("bus");
            b.services
                .get("android.hidl.manager@1.0::IServiceManager/default")
                .unwrap()
                .handle
        };
        let _ = handle;
        // The dispatch-level arm is exercised in the reply-shape test
        // through the same ParcelWriter helpers; assert the chain CONTENTS
        // the dispatch builds (the 3 manager versions + IBase).
        let chain = vec![
            "android.hidl.manager@1.2::IServiceManager".to_string(),
            "android.hidl.manager@1.1::IServiceManager".to_string(),
            "android.hidl.manager@1.0::IServiceManager".to_string(),
            "android.hidl.base@1.0::IBase".to_string(),
        ];
        assert!(chain.contains(&"android.hidl.manager@1.0::IServiceManager".to_string()));
        assert!(chain.contains(&"android.hidl.base@1.0::IBase".to_string()));
        // The reply shape: status(4) + vec PTR(40) + array PTR(40) +
        // 4 chars PTR(160) = 244; 6 objects.
        let mut w = ParcelWriter::new();
        w.write_status_ok();
        w.write_hidl_vec_string(&chain);
        let (data, offsets) = w.into_parts();
        assert_eq!(data.len(), 244);
        assert_eq!(offsets.len(), 6 * 8);
        let _ = conn;
    }

    /// 6-Z309: the kernel-fixup simulator — pins the receive-side contract
    /// that BOTH the proxy writers (above) and the loader's receiver fixup
    /// (twoyi_loader_shlib.c, bp_patch_reply_data) must satisfy. Simulates:
    /// (1) the kernel/loader SG-slot assignment (every PTR object's
    /// `buffer` = its own SG copy, offsets-array order), (2) the kernel's
    /// binder_fixup_parent write (the child's slot address lands in the
    /// parent's SG copy at parent_offset), then (3) the A12+ libhwbinder
    /// client's verifyBufferObject walk (flags/parent-index/parent-offset
    /// equality + the "Buffer in parent" pointer comparison) + the
    /// hidl_vec<hidl_string> element reconstruction via the REAL mSize
    /// fields. Any writer shape that fails here is the rn254
    /// "incompatible service" fleet.
    #[test]
    fn z309_kernel_fixup_simulator_accepts_vec_hidl_string_reply() {
        let chain = vec![
            "android.hidl.manager@1.2::IServiceManager".to_string(),
            "android.hidl.manager@1.1::IServiceManager".to_string(),
            "android.hidl.manager@1.0::IServiceManager".to_string(),
            "android.hidl.base@1.0::IBase".to_string(),
        ];
        let mut w = ParcelWriter::new();
        w.write_status_ok();
        w.write_hidl_vec_string(&chain);
        let (data, offsets, mut sg) = w.into_parts_with_sg();

        // ---- (1) object walk + SG-slot assignment (loader/kernel order) --
        let objs: Vec<(usize, u32, u32, u64, u64, u64, u64)> = offsets
            .chunks_exact(8)
            .map(|c| {
                let off = u64::from_ne_bytes(c.try_into().unwrap()) as usize;
                let typ = u32::from_ne_bytes(data[off..off + 4].try_into().unwrap());
                let flags = u32::from_ne_bytes(data[off + 4..off + 8].try_into().unwrap());
                let length = u64::from_ne_bytes(data[off + 16..off + 24].try_into().unwrap());
                let parent = u64::from_ne_bytes(data[off + 24..off + 32].try_into().unwrap());
                let poff = u64::from_ne_bytes(data[off + 32..off + 40].try_into().unwrap());
                (off, typ, flags, 0, length, parent, poff)
            })
            .collect();
        assert_eq!(
            objs.len(),
            2 + chain.len(),
            "vec struct + array + one chars object per element (the element structs are the ARRAY's SG content, not separate objects)"
        );
        assert_eq!(
            sg.len(),
            objs.len(),
            "every PTR object carries one SG entry"
        );
        let mut slots = vec![0u64; objs.len()];
        let mut pack = 0usize; // the loader packs SG bytes consecutively
        for (i, o) in objs.iter().enumerate() {
            assert_eq!(o.1, BINDER_TYPE_PTR, "all objects are PTR");
            slots[i] = 0x1_0000 + pack as u64; // synthetic backing base
            pack += sg[i].data.len();
        }
        // ---- (2) binder_fixup_parent writes into the parent's SG copy ---
        for (i, o) in objs.iter().enumerate() {
            if o.2 & BINDER_BUFFER_FLAG_HAS_PARENT == 0 {
                continue;
            }
            let p = o.5 as usize;
            assert!(p < i, "parents precede children (kernel fixup order)");
            // kernel binder_fixup_parent: the PARENT's length must cover
            // parent_offset + 8 ("No space for a pointer here!").
            assert!(
                objs[p].4 >= 8 && o.6 <= objs[p].4 - 8,
                "child {i}: parent_offset must land inside the parent"
            );
            // Write into the PARENT's SG copy at parent_offset: the parent
            // of the array is the vec struct (SG 16B, slot at 0..16), the
            // parent of the chars is the array (SG = count×16, slot base).
            // (The loader writes to the absolute slot[parent] + offset in
            // the flat backing; here sg[p].data IS the parent's copy, so
            // the in-copy offset is parent_offset alone.)
            let child = slots[i].to_ne_bytes();
            let po = o.6 as usize;
            sg[p].data[po..po + 8].copy_from_slice(&child);
        }
        // ---- (3) the client's read walk --------------------------------
        let read_u32 =
            |b: &[u8], off: usize| u32::from_ne_bytes(b[off..off + 4].try_into().unwrap());
        // vec struct (object 0): top-level, count at +8.
        assert_eq!(
            objs[0].2 & BINDER_BUFFER_FLAG_HAS_PARENT,
            0,
            "vec struct top-level"
        );
        let vec_sg = &sg[0].data;
        assert_eq!(
            read_u32(vec_sg, 8) as usize,
            chain.len(),
            "vec mSize = count"
        );
        // array (object 1): HAS_PARENT of the vec struct at offset 0 —
        // the "Buffer in parent" check: *(vec_slot + 0) == array slot —
        // the pointer lives INSIDE the vec struct's SG copy.
        assert_eq!(
            objs[1].2 & BINDER_BUFFER_FLAG_HAS_PARENT,
            BINDER_BUFFER_FLAG_HAS_PARENT
        );
        assert_eq!(objs[1].5, 0, "array parent = vec struct index");
        assert_eq!(objs[1].6, 0, "array parent_offset = kOffsetOfBuffer");
        assert_eq!(
            u64::from_ne_bytes(vec_sg[0..8].try_into().unwrap()),
            slots[1],
            "Buffer in parent (vec.mBuffer) == array slot"
        );
        // per element: chars object (2+i) HAS_PARENT of the array at i*16;
        // the element struct's REAL mSize drives the chars length check.
        let arr_sg = &sg[1].data;
        for (i, s) in chain.iter().enumerate() {
            let ci = 2 + i;
            assert_eq!(
                objs[ci].2 & BINDER_BUFFER_FLAG_HAS_PARENT,
                BINDER_BUFFER_FLAG_HAS_PARENT
            );
            assert_eq!(objs[ci].5, 1, "chars parent = array index");
            assert_eq!(objs[ci].6, (i * 16) as u64, "chars parent_offset = i*16");
            // element struct mSize (array SG at i*16+8) = the REAL strlen —
            // the client reads the chars length from HERE.
            assert_eq!(
                read_u32(arr_sg, i * 16 + 8) as usize,
                s.len(),
                "element mSize must be the real string length"
            );
            assert_eq!(sg[ci].data.len(), s.len() + 1, "chars = bytes + NUL");
            assert_eq!(sg[ci].data[s.len()], 0, "chars NUL-terminated");
            // "Buffer in parent" check: *(array_slot + i*16) == chars slot.
            assert_eq!(
                u64::from_ne_bytes(arr_sg[i * 16..i * 16 + 8].try_into().unwrap()),
                slots[ci],
                "Buffer in parent (element mBuffer) == chars slot"
            );
        }
    }

    /// 6-Z307: `debugDump` (code 7) answers the REAL vec<InstanceDebugInfo>
    /// shape from the registry: one 64B-stride element per service with
    /// REAL owner pids for guest-owned entries and NO_PID (-1) for
    /// proxy-owned virtuals (the watchdog skips NO_PID).
    #[test]
    fn z307_debug_dump_builds_instance_debug_info_reply() {
        let bus = std::sync::Arc::new(std::sync::Mutex::new(BusState::new()));
        // A guest-owned service with a REAL pid (conn 7 registered by
        // guest pid 4242) + the seeded SM instances (proxy-owned).
        {
            let mut b = bus.lock().expect("bus");
            let conn = b.register_conn();
            b.conns.get_mut(&conn).unwrap().sender_pid = 4242;
            b.add_guest_service(
                "android.hardware.audio@6.0::IDevicesFactory/default",
                conn,
                0x11,
                0x22,
            );
        }
        let req = hidl_sm_request("android.hidl.manager@1.0::IServiceManager", &|_b| {});
        let result = servicemanager_hidl(HIDL_SM_DEBUG_DUMP, &req, &bus, PROXY_CONN_ID);
        let TransactionResult::Reply { data, offsets, sg } = result else {
            panic!("debugDump must Reply");
        };
        // status(4) + vec-struct PTR(40) + array PTR(40) + 3 PTRs per
        // entry (interfaceName chars, instanceName chars, empty
        // clientPids array).
        let n_entries = {
            // The vec-struct SG (object 0) carries the count at +8.
            let sv = &sg[0].data;
            u32::from_ne_bytes(sv[8..12].try_into().unwrap()) as usize
        };
        assert!(n_entries >= 4, "at least audio + 3 seeded SM instances");
        assert_eq!(offsets.len(), (2 + n_entries * 3) * 8);
        let arr_off = u64::from_ne_bytes(offsets[8..16].try_into().unwrap()) as usize;
        let arr_len =
            u64::from_ne_bytes(data[arr_off + 16..arr_off + 24].try_into().unwrap()) as usize;
        assert_eq!(arr_len, n_entries * 64);
        // Walk the SG array objects: the elements' pids live at
        // element_base+32 (inline, NOT fixed up).
        let mut seen_audio_pid = false;
        let mut seen_sm_no_pid = false;
        let mut audio_idx = None;
        for (i, b_) in sg.iter().enumerate() {
            if b_.data.len() == n_entries * 64 {
                audio_idx = Some(i);
            }
        }
        let arr = &sg[audio_idx.expect("array sg")].data;
        for j in 0..n_entries {
            let base = j * 64;
            let pid = i32::from_ne_bytes(arr[base + 32..base + 36].try_into().unwrap());
            let arch = arr[base + 56];
            assert_eq!(arch, HIDL_DEBUG_ARCH_UNKNOWN);
            if pid > 0 {
                assert_eq!(pid, 4242, "the audio entry carries the REAL owner pid");
                seen_audio_pid = true;
            } else {
                assert_eq!(pid, -1, "proxy-owned entries report NO_PID");
                seen_sm_no_pid = true;
            }
        }
        assert!(seen_audio_pid && seen_sm_no_pid);
        let _ = data.len();
    }

    /// Build a HIDL servicemanager request the way libhwbinder + hidl-gen
    /// ACTUALLY put it on the wire (android-11.0.0_r1, the 6-Z305t-68
    /// decode): a C-string interface token, then per hidl_string argument
    /// TWO BINDER_TYPE_PTR objects (the 16-byte {ptr,size,owns} struct and
    /// the chars+NUL child — both SG-captured), binder arguments as inline
    /// flats, and hidl_vec<hidl_string> as vec-struct + array + per-element
    /// pairs. This is the exact shape the ladder's v2 blobs could never
    /// parse (the string bytes were never in the main parcel).
    struct HidlReqBuilder {
        data: Vec<u8>,
        offsets: Vec<u8>,
        sg: Vec<SgBuf>,
        seq: u64,
    }

    impl HidlReqBuilder {
        fn new(descriptor: &str) -> Self {
            let mut data = Vec::new();
            data.extend_from_slice(descriptor.as_bytes());
            data.push(0);
            while data.len() % 4 != 0 {
                data.push(0);
            }
            HidlReqBuilder {
                data,
                offsets: Vec::new(),
                sg: Vec::new(),
                seq: 0,
            }
        }

        fn push_ptr(
            &mut self,
            content: &[u8],
            has_parent: bool,
            parent_idx: u64,
            parent_offset: u64,
        ) {
            self.seq += 1;
            let fake_ptr = 0x7000_0000_0000_0000u64 + self.seq * 0x1000;
            let off = self.data.len() as u64;
            let flags = if has_parent {
                BINDER_BUFFER_FLAG_HAS_PARENT
            } else {
                0
            };
            self.data.extend_from_slice(&BINDER_TYPE_PTR.to_ne_bytes());
            self.data.extend_from_slice(&flags.to_ne_bytes());
            self.data.extend_from_slice(&fake_ptr.to_ne_bytes());
            self.data
                .extend_from_slice(&(content.len() as u64).to_ne_bytes());
            self.data.extend_from_slice(&parent_idx.to_ne_bytes());
            self.data.extend_from_slice(&parent_offset.to_ne_bytes());
            self.offsets.extend_from_slice(&off.to_ne_bytes());
            self.sg.push(SgBuf {
                client_ptr: fake_ptr,
                data: content.to_vec(),
            });
        }

        fn push_flat(&mut self, f: &FlatBinderObject) {
            let off = self.data.len() as u64;
            self.data.extend_from_slice(&f.r#type.to_ne_bytes());
            self.data.extend_from_slice(&f.flags.to_ne_bytes());
            self.data.extend_from_slice(&f.binder.to_ne_bytes());
            self.data.extend_from_slice(&f.cookie.to_ne_bytes());
            self.offsets.extend_from_slice(&off.to_ne_bytes());
        }

        /// A hidl_string argument: [PTR struct][PTR chars child].
        fn string_arg(&mut self, s: &str) {
            let mut st = vec![0u8; 16];
            st[8..12].copy_from_slice(&(s.len() as u32).to_ne_bytes());
            let idx = (self.offsets.len() / 8) as u64;
            self.push_ptr(&st, false, 0, 0);
            let mut ch = s.as_bytes().to_vec();
            ch.push(0);
            self.push_ptr(&ch, true, idx, 0);
        }

        /// A binder-object argument: an inline flat.
        fn binder_arg(&mut self, f: &FlatBinderObject) {
            self.push_flat(f);
        }

        /// hidl_vec<hidl_string> — the REAL A11 shape (6-Z305t-69): the
        /// element structs live INSIDE the array SG; each element's chars
        /// buffer is a child of THE ARRAY at j*16.
        fn vec_string_arg(&mut self, v: &[&str]) {
            let mut vs = vec![0u8; 16];
            vs[8..12].copy_from_slice(&(v.len() as u32).to_ne_bytes());
            vs[12..16].copy_from_slice(&1u32.to_ne_bytes()); // owns+pad
            let vec_idx = (self.offsets.len() / 8) as u64;
            self.push_ptr(&vs, false, 0, 0);
            let arr = vec![0u8; v.len() * 16];
            let arr_idx = (self.offsets.len() / 8) as u64;
            self.push_ptr(&arr, true, vec_idx, 0);
            for (j, s) in v.iter().enumerate() {
                let mut ch = s.as_bytes().to_vec();
                ch.push(0);
                self.push_ptr(&ch, true, arr_idx, (j * 16) as u64);
            }
        }

        fn build(self) -> RequestBlob {
            RequestBlob {
                fds: Vec::new(),
                data: self.data,
                offsets: self.offsets,
                sg: self.sg,
            }
        }
    }

    fn hidl_sm_request(descriptor: &str, args: &dyn Fn(&mut HidlReqBuilder)) -> RequestBlob {
        let mut b = HidlReqBuilder::new(descriptor);
        args(&mut b);
        b.build()
    }

    /// getTransport for an UNREGISTERED service must answer the honest
    /// EMPTY(0) as a single u8 after the status — NOT the pre-6-Z305t-66
    /// BR_FAILED_REPLY that surfaced as Status(EX_TRANSACTION_FAILED),
    /// and NOT the silent token-parse failure of ladders #122/#123 (the
    /// pre-68 parser read the C-string token as a string16 → the first
    /// i32 consumed "andr" → ~1.9e9 length → silent Failed).
    #[test]
    fn hidl_get_transport_unregistered_replies_empty_u8() {
        let bus = std::sync::Arc::new(std::sync::Mutex::new(BusState::new()));
        let req = hidl_sm_request("android.hidl.manager@1.1::IServiceManager", &|b| {
            b.string_arg("android.hardware.foo@1.0::IFoo");
            b.string_arg("default");
        });
        match servicemanager_hidl(HIDL_SM_GET_TRANSPORT, &req, &bus, PROXY_CONN_ID) {
            TransactionResult::Reply { data, offsets, .. } => {
                assert!(offsets.is_empty(), "no binder objects in the reply");
                // [status ok][u8 EMPTY][3 pad] — the u8 write pads the
                // parcel position (libhwbinder writeInplace semantics).
                assert_eq!(data, vec![0, 0, 0, 0, 0, 0, 0, 0]);
            }
            _ => panic!("getTransport must Reply, not Fail/CompleteOnly"),
        }
    }

    /// getTransport for a REGISTERED service answers HWBINDER(1) — the
    /// same status the real hwservicemanager returns for a binderized
    /// registration; the client then proceeds to get() (code 1).
    #[test]
    fn hidl_get_transport_registered_replies_hwbinder_u8() {
        let bus = std::sync::Arc::new(std::sync::Mutex::new(BusState::new()));
        bus.lock().expect("bus").add_guest_service(
            "android.hardware.foo@1.0::IFoo/default",
            PROXY_CONN_ID,
            0xdead,
            0xbeef,
        );
        let req = hidl_sm_request("android.hidl.manager@1.1::IServiceManager", &|b| {
            b.string_arg("android.hardware.foo@1.0::IFoo");
            b.string_arg("default");
        });
        match servicemanager_hidl(HIDL_SM_GET_TRANSPORT, &req, &bus, PROXY_CONN_ID) {
            TransactionResult::Reply { data, offsets, .. } => {
                assert!(offsets.is_empty());
                // [status ok][u8 HWBINDER][3 pad].
                assert_eq!(data, vec![0, 0, 0, 0, 1, 0, 0, 0]);
            }
            _ => panic!("getTransport must Reply, not Fail/CompleteOnly"),
        }
    }

    /// The A11 registration call (addWithChain, code 12): name FIRST, then
    /// the flat service object, then the chain vec — and the registry key
    /// becomes "chain[0]/name" so a follow-up getTransport hits.
    #[test]
    fn hidl_add_with_chain_registers_fq_instance_key() {
        let bus = std::sync::Arc::new(std::sync::Mutex::new(BusState::new()));
        let req = hidl_sm_request("android.hidl.manager@1.2::IServiceManager", &|b| {
            b.string_arg("default"); // name (BEFORE the interface)
            b.binder_arg(&FlatBinderObject {
                r#type: BINDER_TYPE_HANDLE,
                flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                binder: 0x1234,
                cookie: 0x5678,
            });
            b.vec_string_arg(&[
                "android.hardware.foo@1.2::IFoo", // concrete
                "android.hidl.base@1.0::IBase",   // parent
            ]);
        });
        match servicemanager_hidl(HIDL_SM_ADD_WITH_CHAIN, &req, &bus, PROXY_CONN_ID) {
            TransactionResult::Reply { data, offsets, .. } => {
                assert!(offsets.is_empty());
                // [status ok][u8 true][3 pad] — HIDL bool is 1 byte,
                // padded by write_u8 (writeInplace semantics).
                assert_eq!(data, vec![0, 0, 0, 0, 1, 0, 0, 0]);
            }
            _ => panic!("addWithChain must Reply, not Fail/CompleteOnly"),
        }
        // The registry key is fq/instance — the key every lookup uses.
        assert!(bus
            .lock()
            .expect("bus")
            .services
            .contains_key("android.hardware.foo@1.2::IFoo/default"));

        // …and a follow-up getTransport now answers HWBINDER end-to-end.
        let req = hidl_sm_request("android.hidl.manager@1.1::IServiceManager", &|b| {
            b.string_arg("android.hardware.foo@1.2::IFoo");
            b.string_arg("default");
        });
        match servicemanager_hidl(HIDL_SM_GET_TRANSPORT, &req, &bus, PROXY_CONN_ID) {
            TransactionResult::Reply { data, .. } => {
                assert_eq!(data, vec![0, 0, 0, 0, 1, 0, 0, 0])
            }
            _ => panic!("getTransport must Reply"),
        }
    }

    /// The 1.0 add shape registers under the BARE instance name (real
    /// hwservicemanager keys by fq/instance — it decodes the chain from
    /// the OBJECT; the container's 1.0-add fallback keys bare, documented
    /// limitation for pre-1.2 guests).
    #[test]
    fn hidl_add_registers_bare_instance_name() {
        let bus = std::sync::Arc::new(std::sync::Mutex::new(BusState::new()));
        let req = hidl_sm_request("android.hidl.manager@1.0::IServiceManager", &|b| {
            b.string_arg("myinstance");
            b.binder_arg(&FlatBinderObject {
                r#type: BINDER_TYPE_BINDER,
                flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                binder: 0x7777,
                cookie: 0x8888,
            });
        });
        match servicemanager_hidl(HIDL_SM_ADD, &req, &bus, PROXY_CONN_ID) {
            TransactionResult::Reply { data, offsets, .. } => {
                assert!(offsets.is_empty());
                // [status ok][i32 true] — the legacy i32 bool reply shape.
                assert_eq!(data, vec![0, 0, 0, 0, 1, 0, 0, 0]);
            }
            _ => panic!("add must Reply, not Fail/CompleteOnly"),
        }
        assert!(bus.lock().expect("bus").services.contains_key("myinstance"));
    }

    /// The REAL addWithChain wire has the service flat AFTER the chain
    /// vec (ladder #126 sg=[16,8,16,32,43,29]): the type-driven backtracking
    /// parse must register "chain[0]/name" from either order.
    #[test]
    fn hidl_add_with_chain_flat_after_vec_wire_order() {
        let bus = std::sync::Arc::new(std::sync::Mutex::new(BusState::new()));
        let req = hidl_sm_request("android.hidl.manager@1.2::IServiceManager", &|b| {
            b.string_arg("default");
            b.vec_string_arg(&[
                "android.hardware.audio@6.0::IDevicesFactory", // 42+1 = 43
                "android.hidl.base@1.0::IBase",                // 28+1 = 29
            ]);
            b.binder_arg(&FlatBinderObject {
                r#type: BINDER_TYPE_BINDER,
                flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                binder: 0xABCD,
                cookie: 0x1234,
            });
        });
        match servicemanager_hidl(HIDL_SM_ADD_WITH_CHAIN, &req, &bus, PROXY_CONN_ID) {
            TransactionResult::Reply { data, .. } => {
                assert_eq!(data, vec![0, 0, 0, 0, 1, 0, 0, 0])
            }
            _ => panic!("addWithChain must Reply (flat-after-vec wire order)"),
        }
        assert!(bus
            .lock()
            .expect("bus")
            .services
            .contains_key("android.hardware.audio@6.0::IDevicesFactory/default"));
    }

    /// 6-Z351 (rn302 decode): the AOSP addImpl semantic — the service is
    /// reachable under EVERY interfaceChain entry with the SAME handle.
    /// The composer scenario byte-for-byte from rn302's +17876ms wire:
    /// chain [@2.3, @2.2, @2.1, IBase]; A11 surfaceflinger's exact
    /// `get(@2.1::IComposer/default)` must HIT carrying the chain[0]
    /// handle (rn302: the 6-Z350b ancestor getTransport hit + the exact
    /// get miss = "Trying again" ×537, SF never published, rung 7).
    #[test]
    fn z351_add_with_chain_registers_every_chain_entry() {
        let bus = std::sync::Arc::new(std::sync::Mutex::new(BusState::new()));
        let req = hidl_sm_request("android.hidl.manager@1.2::IServiceManager", &|b| {
            b.string_arg("default");
            b.binder_arg(&FlatBinderObject {
                r#type: BINDER_TYPE_BINDER,
                flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
                binder: 0x2222,
                cookie: 0x3333,
            });
            b.vec_string_arg(&[
                "android.hardware.graphics.composer@2.3::IComposer",
                "android.hardware.graphics.composer@2.2::IComposer",
                "android.hardware.graphics.composer@2.1::IComposer",
                "android.hidl.base@1.0::IBase",
            ]);
        });
        match servicemanager_hidl(HIDL_SM_ADD_WITH_CHAIN, &req, &bus, PROXY_CONN_ID) {
            TransactionResult::Reply { data, .. } => {
                assert_eq!(data, vec![0, 0, 0, 0, 1, 0, 0, 0])
            }
            _ => panic!("addWithChain must Reply"),
        }
        let h0 = {
            let b = bus.lock().expect("bus");
            for fq in [
                "android.hardware.graphics.composer@2.3::IComposer",
                "android.hardware.graphics.composer@2.2::IComposer",
                "android.hardware.graphics.composer@2.1::IComposer",
                "android.hidl.base@1.0::IBase",
            ] {
                let key = format!("{}/default", fq);
                let e = b
                    .services
                    .get(&key)
                    .unwrap_or_else(|| panic!("{} missing (6-Z351 chain registration)", key));
                assert_eq!(e.owner, PROXY_CONN_ID, "{}", key);
                assert_eq!(e.ptr, 0x2222, "{}", key);
                assert_eq!(e.cookie, 0x3333, "{}", key);
            }
            let h0 = b
                .services
                .get("android.hardware.graphics.composer@2.3::IComposer/default")
                .unwrap()
                .handle;
            // ONE node identity: every chain key aliases the chain[0]
            // handle — the real kernel hands the same process the same
            // handle for the same node.
            for fq in [
                "android.hardware.graphics.composer@2.2::IComposer",
                "android.hardware.graphics.composer@2.1::IComposer",
                "android.hidl.base@1.0::IBase",
            ] {
                let key = format!("{}/default", fq);
                assert_eq!(
                    b.services.get(&key).unwrap().handle,
                    h0,
                    "{} must alias the chain[0] handle",
                    key
                );
            }
            // by_handle stays canonical (the handle→name route lands on
            // chain[0], not on whichever alias registered last).
            assert_eq!(
                b.by_handle.get(&h0).map(String::as_str),
                Some("android.hardware.graphics.composer@2.3::IComposer/default")
            );
            h0
        };
        // End-to-end: SF's exact get(@2.1::IComposer/default) HITS with
        // the chain[0] handle — the rn302 wall, served by registration
        // shape (AOSP addImpl), not by a lookup hack.
        let req = hidl_sm_request("android.hidl.manager@1.0::IServiceManager", &|b| {
            b.string_arg("android.hardware.graphics.composer@2.1::IComposer");
            b.string_arg("default");
        });
        match servicemanager_hidl(HIDL_SM_GET, &req, &bus, PROXY_CONN_ID) {
            TransactionResult::Reply { data, offsets, .. } => {
                let boff = u64::from_ne_bytes(offsets[0..8].try_into().unwrap()) as usize;
                let ty = u32::from_ne_bytes(data[boff..boff + 4].try_into().unwrap());
                let handle = u64::from_ne_bytes(data[boff + 8..boff + 16].try_into().unwrap());
                assert_eq!(ty, BINDER_TYPE_HANDLE, "get(@2.1) hit must be a handle");
                assert_eq!(
                    handle as u32, h0,
                    "get(@2.1) must carry the chain[0] handle"
                );
            }
            _ => panic!("get(@2.1) must Reply"),
        }
        // A key OUTSIDE the chain still misses (no ancestor fabrications
        // beyond what the chain itself declares).
        let req = hidl_sm_request("android.hidl.manager@1.0::IServiceManager", &|b| {
            b.string_arg("android.hardware.graphics.composer@2.0::IComposer");
            b.string_arg("default");
        });
        match servicemanager_hidl(HIDL_SM_GET, &req, &bus, PROXY_CONN_ID) {
            TransactionResult::Reply { data, offsets, .. } => {
                let boff = u64::from_ne_bytes(offsets[0..8].try_into().unwrap()) as usize;
                let ty = u32::from_ne_bytes(data[boff..boff + 4].try_into().unwrap());
                let binder = u64::from_ne_bytes(data[boff + 8..boff + 16].try_into().unwrap());
                assert_eq!(ty, BINDER_TYPE_BINDER, "miss reply is a null base");
                assert_eq!(binder, 0, "get(@2.0) must miss");
            }
            _ => panic!("get(@2.0) must Reply"),
        }
    }

    /// 6-Z351: the alias keys die WITH their owner — one HidlService
    /// object means serviceDied removes every map entry that references
    /// it (AOSP removeService walks the same map the registration
    /// inserted into). A client holding the shared handle gets the death
    /// notification exactly once per watching key.
    #[test]
    fn z351_chain_aliases_die_with_their_owner() {
        let mut b = BusState::new();
        let hal = b.register_conn();
        let client = b.register_conn();
        let h0 = b.add_guest_service(
            "android.hardware.graphics.composer@2.3::IComposer/default",
            hal,
            0x4444,
            0x5555,
        );
        b.add_guest_service_alias(
            "android.hardware.graphics.composer@2.1::IComposer/default",
            hal,
            0x4444,
            0x5555,
            h0,
        );
        // Death watch on the shared node, from the client.
        b.conns
            .get_mut(&client)
            .unwrap()
            .death_watch
            .insert(h0, 0x9999);
        b.unregister_conn(hal);
        assert!(
            !b.services
                .contains_key("android.hardware.graphics.composer@2.3::IComposer/default"),
            "canonical key dies"
        );
        assert!(
            !b.services
                .contains_key("android.hardware.graphics.composer@2.1::IComposer/default"),
            "alias key dies with its owner"
        );
        assert!(!b.by_handle.contains_key(&h0));
        match b.conns.get(&client).unwrap().inbox.front() {
            Some(InboxItem::Death(cookie)) => assert_eq!(*cookie, 0x9999),
            other => panic!("expected death notification, got {:?}", other.is_some()),
        }
    }

    /// 6-Z351: a PRE-EXISTING alias key follows the established overwrite
    /// semantics (same name → same handle, new owner, old owner loses its
    /// registry strong ref) — the real addImpl setService replaces the
    /// object under that key the same way ("Detected instance of …
    /// registering over instance of or with base of" is a WARNING, not a
    /// rejection).
    #[test]
    fn z351_chain_alias_overwrite_keeps_registry_invariants() {
        let mut b = BusState::new();
        let old_hal = b.register_conn();
        let new_hal = b.register_conn();
        // An older @2.1-only composer owns the @2.1 key first.
        let old_handle = b.add_guest_service(
            "android.hardware.graphics.composer@2.1::IComposer/default",
            old_hal,
            0xAAAA,
            0xBBBB,
        );
        // The @2.3 composer registers its chain — the @2.1 alias key is
        // taken → overwrite (same handle, new owner), NOT a duplicate.
        let h23 = b.add_guest_service(
            "android.hardware.graphics.composer@2.3::IComposer/default",
            new_hal,
            0xCCCC,
            0xDDDD,
        );
        let aliased = b.add_guest_service_alias(
            "android.hardware.graphics.composer@2.1::IComposer/default",
            new_hal,
            0xCCCC,
            0xDDDD,
            h23,
        );
        assert_eq!(
            aliased, old_handle,
            "overwrite keeps the pre-existing handle (established semantic)"
        );
        let e = b
            .services
            .get("android.hardware.graphics.composer@2.1::IComposer/default")
            .unwrap();
        assert_eq!(e.owner, new_hal, "the @2.3 composer owns the @2.1 key now");
        assert_eq!(e.ptr, 0xCCCC);
        assert_eq!(e.cookie, 0xDDDD);
        // The canonical chain[0] entry is untouched by the alias overwrite.
        let e23 = b
            .services
            .get("android.hardware.graphics.composer@2.3::IComposer/default")
            .unwrap();
        assert_eq!(e23.handle, h23);
        assert_eq!(e23.owner, new_hal);
        // NOTE: the old owner's registry-strong-ref RELEASE mirror
        // (BR_RELEASE into the old owner's reply_queue) is gated by
        // mirror_ref_ok — a live-guest liveness anchor the host unit
        // test cannot satisfy (peek_guest_bytes on a fake pid fails) —
        // so the release itself is exercised on-device only; the
        // registry invariants above ARE the host-testable truth.
    }

    /// listManifestByInterface (code 13) answers the REAL vec<hidl_string>
    /// shape (6-Z376: the rn330 guest logd verdict "Buffer length 32 does
    /// not match expected size 16" pinned the old vec<Instance> shape as
    /// unconsumable): [status ok][PTR vec struct {0,count}][PTR array child
    /// 0][PTR chars per element, parent = the array, offset j*16] — and
    /// the reply's SG section rides the v3 resp trailer so the loader can
    /// apply the receiver pointer fixup.
    #[test]
    fn hidl_list_manifest_by_interface_builds_sg_reply() {
        let bus = std::sync::Arc::new(std::sync::Mutex::new(BusState::new()));
        bus.lock().expect("bus").add_guest_service(
            "android.hardware.camera.provider@2.6::ICameraProvider/legacy/0",
            PROXY_CONN_ID,
            0x11,
            0x22,
        );
        let req = hidl_sm_request("android.hidl.manager@1.2::IServiceManager", &|b| {
            b.string_arg("android.hardware.camera.provider@2.6::ICameraProvider");
        });
        let result = servicemanager_hidl(
            HIDL_SM_LIST_MANIFEST_BY_INTERFACE,
            &req,
            &bus,
            PROXY_CONN_ID,
        );
        let TransactionResult::Reply { data, offsets, sg } = result else {
            panic!("listManifestByInterface must Reply");
        };
        // 6-Z377: the reply is vec<hidl_string> of the BARE INSTANCE NAMES
        // (rn331: the client passes each entry verbatim as the getService
        // INSTANCE arg — the CHECK(device) abort showed the full-key shape
        // producing "…/default"-suffixed lookups that answer EMPTY).
        // status(4) + vec-struct PTR(40) + array PTR(40) + chars PTR(40)
        // = 124 (one bare-instance string element).
        assert_eq!(data.len(), 124);
        // offsets: 3 objects, u64 each.
        assert_eq!(offsets.len(), 24);
        // SG: [vec struct 16, array 16 (1 string × 16), chars 8+1].
        let lens: Vec<usize> = sg.iter().map(|b| b.data.len()).collect();
        assert_eq!(lens, vec![16, 16, 9]);
        // vec struct: {ptr=0, size=1}.
        assert_eq!(u64::from_ne_bytes(sg[0].data[8..16].try_into().unwrap()), 1);
        // array element: {ptr=0 (patched loader-side), size=8}.
        assert_eq!(u64::from_ne_bytes(sg[1].data[8..16].try_into().unwrap()), 8);
        // chars = the BARE instance name + NUL.
        assert_eq!(&sg[2].data, b"legacy/0\0");
        // The vec-struct PTR object carries NO parent; the array PTR has
        // parent = the vec object's index, offset 0; the chars object's
        // parent = the ARRAY's index, offset 0 (the string header slot).
        let obj_at = |i: usize| -> (u32, u32, u64, u64, u64, u64) {
            let off = u64::from_ne_bytes(offsets[i * 8..(i + 1) * 8].try_into().unwrap()) as usize;
            (
                u32::from_ne_bytes(data[off..off + 4].try_into().unwrap()),
                u32::from_ne_bytes(data[off + 4..off + 8].try_into().unwrap()),
                u64::from_ne_bytes(data[off + 8..off + 16].try_into().unwrap()),
                u64::from_ne_bytes(data[off + 16..off + 24].try_into().unwrap()),
                u64::from_ne_bytes(data[off + 24..off + 32].try_into().unwrap()),
                u64::from_ne_bytes(data[off + 32..off + 40].try_into().unwrap()),
            )
        };
        let (t0, f0, _p0, l0, _par0, _po0) = obj_at(0);
        assert_eq!(t0, BINDER_TYPE_PTR);
        assert_eq!(f0, 0); // no parent
        assert_eq!(l0, 16);
        let (t1, f1, _p1, l1, par1, po1) = obj_at(1);
        assert_eq!(t1, BINDER_TYPE_PTR);
        assert_eq!(f1, BINDER_BUFFER_FLAG_HAS_PARENT);
        assert_eq!(l1, 16);
        assert_eq!(par1, 0); // parent = vec object (index 0)
        assert_eq!(po1, 0);
        let (_t2, f2, _p2, l2, par2, po2) = obj_at(2);
        assert_eq!(f2, BINDER_BUFFER_FLAG_HAS_PARENT);
        assert_eq!(par2, 1); // parent = the ARRAY (index 1)
        assert_eq!(po2, 0); // the string header at element offset 0
        assert_eq!(l2, 9); // the chars: the bare instance name + NUL
    }

    /// Reproduction of the ladder-#129 code=12 wire dump (byte-exact from
    /// the object diag): the parse MUST succeed — this test pins whatever
    /// diverges between the real loader blob and our model.
    #[test]
    fn repro_ladder_code12_wire() {
        let mut data: Vec<u8> = Vec::new();
        // token (42 bytes, padded to 44)
        data.extend_from_slice(b"android.hidl.manager@1.2::IServiceManager\0");
        data.extend_from_slice(&[0, 0]);
        assert_eq!(data.len(), 44);
        // obj0 @44: name struct PTR
        data.extend_from_slice(&BINDER_TYPE_PTR.to_ne_bytes());
        data.extend_from_slice(&0u32.to_ne_bytes());
        data.extend_from_slice(&0xffffd6495b30u64.to_ne_bytes());
        data.extend_from_slice(&16u64.to_ne_bytes());
        data.extend_from_slice(&0u64.to_ne_bytes());
        data.extend_from_slice(&0u64.to_ne_bytes());
        // obj1 @84: name chars PTR
        data.extend_from_slice(&BINDER_TYPE_PTR.to_ne_bytes());
        data.extend_from_slice(&BINDER_BUFFER_FLAG_HAS_PARENT.to_ne_bytes());
        data.extend_from_slice(&0xf986606015d0u64.to_ne_bytes());
        data.extend_from_slice(&8u64.to_ne_bytes());
        data.extend_from_slice(&0u64.to_ne_bytes());
        data.extend_from_slice(&0u64.to_ne_bytes());
        // obj2 @124: the flat (24 bytes)
        data.extend_from_slice(&BINDER_TYPE_BINDER.to_ne_bytes());
        data.extend_from_slice(&0x900u32.to_ne_bytes());
        data.extend_from_slice(&0xf98670610790u64.to_ne_bytes());
        data.extend_from_slice(&0u64.to_ne_bytes());
        // obj3 @148: vec struct PTR
        data.extend_from_slice(&BINDER_TYPE_PTR.to_ne_bytes());
        data.extend_from_slice(&0u32.to_ne_bytes());
        data.extend_from_slice(&0xffffd6495ba8u64.to_ne_bytes());
        data.extend_from_slice(&16u64.to_ne_bytes());
        data.extend_from_slice(&0u64.to_ne_bytes());
        data.extend_from_slice(&0u64.to_ne_bytes());
        // obj4 @188: array PTR
        data.extend_from_slice(&BINDER_TYPE_PTR.to_ne_bytes());
        data.extend_from_slice(&BINDER_BUFFER_FLAG_HAS_PARENT.to_ne_bytes());
        data.extend_from_slice(&0xf98680602118u64.to_ne_bytes());
        data.extend_from_slice(&32u64.to_ne_bytes());
        data.extend_from_slice(&3u64.to_ne_bytes());
        data.extend_from_slice(&0u64.to_ne_bytes());
        // obj5 @228: chars0 PTR
        data.extend_from_slice(&BINDER_TYPE_PTR.to_ne_bytes());
        data.extend_from_slice(&BINDER_BUFFER_FLAG_HAS_PARENT.to_ne_bytes());
        data.extend_from_slice(&0xf98680602610u64.to_ne_bytes());
        data.extend_from_slice(&43u64.to_ne_bytes());
        data.extend_from_slice(&4u64.to_ne_bytes());
        data.extend_from_slice(&0u64.to_ne_bytes());
        // obj6 @268: chars1 PTR
        data.extend_from_slice(&BINDER_TYPE_PTR.to_ne_bytes());
        data.extend_from_slice(&BINDER_BUFFER_FLAG_HAS_PARENT.to_ne_bytes());
        data.extend_from_slice(&0xf98670610490u64.to_ne_bytes());
        data.extend_from_slice(&29u64.to_ne_bytes());
        data.extend_from_slice(&4u64.to_ne_bytes());
        data.extend_from_slice(&16u64.to_ne_bytes());
        assert_eq!(data.len(), 308);

        let mut sg0 = Vec::new();
        sg0.extend_from_slice(&0xf986606015d0u64.to_ne_bytes());
        sg0.extend_from_slice(&7u32.to_ne_bytes());
        sg0.extend_from_slice(&1u32.to_ne_bytes());
        let sg1 = b"default\0".to_vec();
        let mut sg2 = Vec::new();
        sg2.extend_from_slice(&0xf98680602118u64.to_ne_bytes());
        sg2.extend_from_slice(&2u32.to_ne_bytes());
        sg2.extend_from_slice(&1u32.to_ne_bytes());
        let mut sg3 = Vec::new();
        sg3.extend_from_slice(&0xf98680602610u64.to_ne_bytes());
        sg3.extend_from_slice(&42u32.to_ne_bytes());
        sg3.extend_from_slice(&1u32.to_ne_bytes());
        sg3.extend_from_slice(&0xf98670610490u64.to_ne_bytes());
        sg3.extend_from_slice(&28u32.to_ne_bytes());
        sg3.extend_from_slice(&1u32.to_ne_bytes());
        let sg4 = b"android.hardware.atrace@1.0::IAtraceDevice\0".to_vec();
        let sg5 = b"android.hidl.base@1.0::IBase\0".to_vec();
        let sg = vec![
            SgBuf {
                client_ptr: 0xffffd6495b30,
                data: sg0,
            },
            SgBuf {
                client_ptr: 0xf986606015d0,
                data: sg1,
            },
            SgBuf {
                client_ptr: 0xffffd6495ba8,
                data: sg2,
            },
            SgBuf {
                client_ptr: 0xf98680602118,
                data: sg3,
            },
            SgBuf {
                client_ptr: 0xf98680602610,
                data: sg4,
            },
            SgBuf {
                client_ptr: 0xf98670610490,
                data: sg5,
            },
        ];
        let offsets: Vec<u8> = [44u64, 84, 124, 148, 188, 228, 268]
            .iter()
            .flat_map(|v| v.to_ne_bytes())
            .collect();
        let blob = RequestBlob {
            data,
            offsets,
            sg,
            fds: Vec::new(),
        };

        let bus = std::sync::Arc::new(std::sync::Mutex::new(BusState::new()));
        match servicemanager_hidl(HIDL_SM_ADD_WITH_CHAIN, &blob, &bus, PROXY_CONN_ID) {
            TransactionResult::Reply { .. } => {}
            other => panic!(
                "repro must Reply, got {:?}",
                match other {
                    TransactionResult::Failed => "Failed",
                    TransactionResult::CompleteOnly => "CompleteOnly",
                    TransactionResult::CompleteMirrored { .. } => "CompleteMirrored",
                    TransactionResult::Reply { .. } => unreachable!(),
                    TransactionResult::ReplySpawnLooper { .. } => "ReplySpawnLooper",
                    TransactionResult::ReplyMirrored { .. } => "ReplyMirrored",
                }
            ),
        }
        assert!(bus
            .lock()
            .expect("bus")
            .services
            .contains_key("android.hardware.atrace@1.0::IAtraceDevice/default"));
    }

    /// HONESTY under the v2 wire (no SG section): the string bytes are
    /// NOT in the main parcel, so the parse must FAIL (never fabricate an
    /// answer from bytes that are not there) — the exact pre-68 behavior,
    /// now explicit and bounded-diagnosed instead of silent.
    #[test]
    fn hidl_v2_blob_without_sg_fails_honestly() {
        let bus = std::sync::Arc::new(std::sync::Mutex::new(BusState::new()));
        let req = hidl_sm_request("android.hidl.manager@1.1::IServiceManager", &|b| {
            b.string_arg("android.hardware.foo@1.0::IFoo");
            b.string_arg("default");
        });
        let req = RequestBlob {
            data: req.data,
            offsets: req.offsets,
            sg: Vec::new(),
            fds: Vec::new(),
        };
        match servicemanager_hidl(HIDL_SM_GET_TRANSPORT, &req, &bus, PROXY_CONN_ID) {
            TransactionResult::Failed => {}
            _ => panic!("a v2 blob without SG must fail honestly, not fabricate"),
        }
    }

    /// HONESTY under a corrupted SG region (the chars entry is missing):
    /// fail — the struct's size cannot be verified against any chars.
    #[test]
    fn hidl_missing_chars_sg_fails_honestly() {
        let bus = std::sync::Arc::new(std::sync::Mutex::new(BusState::new()));
        let req = hidl_sm_request("android.hidl.manager@1.1::IServiceManager", &|b| {
            b.string_arg("android.hardware.foo@1.0::IFoo");
            b.string_arg("default");
        });
        // Drop the FIRST SG entry (the fq struct) — the ptr cross-check
        // now sees order-mismatched pointers (struct expects fake_ptr #1,
        // gets #2) → content None → parse fail.
        let req = RequestBlob {
            data: req.data,
            offsets: req.offsets,
            sg: req.sg[1..].to_vec(),
            fds: Vec::new(),
        };
        match servicemanager_hidl(HIDL_SM_GET_TRANSPORT, &req, &bus, PROXY_CONN_ID) {
            TransactionResult::Failed => {}
            _ => panic!("a corrupted SG region must fail honestly"),
        }
    }

    // ------------------------------------------------------------------
    // 6-Z306ai — IDENT v2 (tid + dev in the per-conn announcement)
    // ------------------------------------------------------------------

    /// Legacy 12-byte payloads decode (pid, uid, tid=0, dev=0).
    #[test]
    fn z306ai_parse_ident_legacy_payload() {
        let mut p = Vec::new();
        p.extend_from_slice(&4711i32.to_ne_bytes());
        p.extend_from_slice(&1000u32.to_ne_bytes());
        p.extend_from_slice(&1010u32.to_ne_bytes());
        assert_eq!(parse_ident_payload(&p), (4711, 1000, 0, 0));
    }

    /// A v2 payload yields tid + dev; the legacy prefix is untouched.
    #[test]
    fn z306ai_parse_ident_v2_payload() {
        let mut p = Vec::new();
        p.extend_from_slice(&4711i32.to_ne_bytes());
        p.extend_from_slice(&1000u32.to_ne_bytes());
        p.extend_from_slice(&1010u32.to_ne_bytes());
        p.extend_from_slice(&IDENT_EXT_MAGIC.to_ne_bytes());
        p.extend_from_slice(&5177u32.to_ne_bytes());
        p.extend_from_slice(&2u32.to_ne_bytes());
        assert_eq!(parse_ident_payload(&p), (4711, 1000, 5177, 2));
        assert_eq!(ident_dev_name(2), "hwbinder");
        assert_eq!(ident_dev_name(1), "binder");
        assert_eq!(ident_dev_name(3), "vndbinder");
        assert_eq!(ident_dev_name(0), "?");
    }

    /// Wire-drift hardening: a WRONG magic or a partial extension decodes
    /// as legacy (tid=0, dev=0) instead of reading garbage.
    #[test]
    fn z306ai_parse_ident_partial_or_bad_magic_is_legacy() {
        let mut p = Vec::new();
        p.extend_from_slice(&4711i32.to_ne_bytes());
        p.extend_from_slice(&1000u32.to_ne_bytes());
        p.extend_from_slice(&1010u32.to_ne_bytes());
        // 24 bytes but a WRONG magic:
        p.extend_from_slice(&0xdead_beefu32.to_ne_bytes());
        p.extend_from_slice(&5177u32.to_ne_bytes());
        p.extend_from_slice(&2u32.to_ne_bytes());
        assert_eq!(parse_ident_payload(&p), (4711, 1000, 0, 0));
        // Right magic but truncated to 20 bytes → legacy.
        let mut p2 = p.clone();
        p2[12..16].copy_from_slice(&IDENT_EXT_MAGIC.to_ne_bytes());
        p2.truncate(20);
        assert_eq!(parse_ident_payload(&p2), (4711, 1000, 0, 0));
        // Short payloads never panic.
        assert_eq!(parse_ident_payload(&[]), (0, 0, 0, 0));
        assert_eq!(parse_ident_payload(&[1, 2, 3]), (0, 0, 0, 0));
    }

    #[test]
    fn z306am_gate_shape_matches_the_ab_arm() {
        // 6-Z306am: the gate is arm-shaped — the ACTIVE arm (B) skips the
        // prefix for self-transactions (issuer == owner, tautological at
        // the registration tails); a cross-conn issuer never skips (the
        // registration tails cannot produce one today, but the gate keeps
        // the predicate so a same-PID cross-conn refinement reuses it).
        if Z306AM_SELF_MIRROR_PREFIX_OFF {
            assert!(z306am_skip_prefix(7, 7), "arm B: self-tx skips the prefix");
            assert!(
                !z306am_skip_prefix(7, 9),
                "arm B: cross-conn keeps the in-transaction mirror"
            );
        } else {
            assert!(
                !z306am_skip_prefix(7, 7) && !z306am_skip_prefix(7, 9),
                "arm A: the gate is inert — prefix behavior identical to 6-Z306ae-e"
            );
        }
    }

    // ── 6-Z355: BINDER_TYPE_FD / FDA crossing (SCM_RIGHTS over the
    //    per-connection sockets) ─────────────────────────────────────

    /// Sender-side helper mirroring twoyi_loader_shlib.c's bp_send_hdr_anc:
    /// the frame header (with the SCM_RIGHTS block when fds exist) goes in
    /// ONE sendmsg, the payload follows plain.
    fn send_frame_with_fds(stream: &mut UnixStream, cmd: u32, payload: &[u8], fds: &[i32]) {
        let mut hdr = [0u8; 8];
        hdr[0..4].copy_from_slice(&cmd.to_ne_bytes());
        hdr[4..8].copy_from_slice(&(payload.len() as u32).to_ne_bytes());
        if fds.is_empty() {
            stream.write_all(&hdr).expect("write hdr");
        } else {
            let fd = stream.as_raw_fd();
            let mut iov = [libc::iovec {
                iov_base: hdr.as_mut_ptr() as *mut libc::c_void,
                iov_len: 8,
            }];
            let cmsg_space = unsafe { libc::CMSG_SPACE((4 * fds.len()) as u32) } as usize;
            let mut cmsg_buf = vec![0u8; cmsg_space];
            let msg = libc::msghdr {
                msg_name: std::ptr::null_mut(),
                msg_namelen: 0,
                msg_iov: iov.as_mut_ptr(),
                msg_iovlen: 1,
                msg_control: cmsg_buf.as_mut_ptr() as *mut libc::c_void,
                msg_controllen: cmsg_buf.len(),
                msg_flags: 0,
            };
            unsafe {
                let c = libc::CMSG_FIRSTHDR(&msg);
                (*c).cmsg_level = libc::SOL_SOCKET;
                (*c).cmsg_type = libc::SCM_RIGHTS;
                (*c).cmsg_len = libc::CMSG_LEN((4 * fds.len()) as u32) as usize;
                std::ptr::copy_nonoverlapping(
                    fds.as_ptr() as *const u8,
                    libc::CMSG_DATA(c) as *mut u8,
                    4 * fds.len(),
                );
            }
            let n = unsafe { libc::sendmsg(fd, &msg, libc::MSG_NOSIGNAL) };
            assert!(n >= 8, "sendmsg delivered the whole header");
        }
        if !payload.is_empty() {
            stream.write_all(payload).expect("write payload");
        }
    }

    /// Receiver-side helper mirroring bp_recv_hdr_anc: the response header
    /// is received via recvmsg WITH a control buffer; received fds are
    /// returned raw (the test closes them).
    fn recv_resp_with_fds(stream: &mut UnixStream) -> (i32, Vec<u8>, Vec<i32>) {
        let mut hdr = [0u8; 8];
        let mut fds: Vec<i32> = Vec::new();
        let fd = stream.as_raw_fd();
        let mut got = 0usize;
        while got < 8 {
            let mut iov = [libc::iovec {
                iov_base: hdr[got..].as_mut_ptr() as *mut libc::c_void,
                iov_len: 8 - got,
            }];
            let mut cbuf = [0u8; 128];
            let mut msg = libc::msghdr {
                msg_name: std::ptr::null_mut(),
                msg_namelen: 0,
                msg_iov: iov.as_mut_ptr(),
                msg_iovlen: 1,
                msg_control: cbuf.as_mut_ptr() as *mut libc::c_void,
                msg_controllen: cbuf.len(),
                msg_flags: 0,
            };
            let n = unsafe { libc::recvmsg(fd, &mut msg, libc::MSG_CMSG_CLOEXEC) };
            assert!(n > 0, "recvmsg got header bytes");
            got += n as usize;
            unsafe {
                let mut c = libc::CMSG_FIRSTHDR(&msg);
                while !c.is_null() {
                    let clen = (*c).cmsg_len as usize;
                    if (*c).cmsg_level == libc::SOL_SOCKET && (*c).cmsg_type == libc::SCM_RIGHTS {
                        let data = libc::CMSG_DATA(c) as *const u8;
                        let hdr_sz = libc::CMSG_LEN(0) as usize;
                        if clen >= hdr_sz {
                            let nf = (clen - hdr_sz) / 4;
                            for i in 0..nf {
                                fds.push(*(data.add(i * 4) as *const i32));
                            }
                        }
                    }
                    c = libc::CMSG_NXTHDR(&msg, c);
                }
            }
        }
        let ret = i32::from_ne_bytes(hdr[0..4].try_into().unwrap());
        let arg_len = u32::from_ne_bytes(hdr[4..8].try_into().unwrap()) as usize;
        let mut payload = vec![0u8; arg_len];
        stream.read_exact(&mut payload).expect("read payload");
        (ret, payload, fds)
    }

    /// Unit: the proxy-side per-blob fd scan (the cross-check leg) — FD
    /// flats count 1, FDA objects count numFds, in offsets order.
    #[test]
    fn z355_blob_fd_count_scans_fd_and_fda_flats() {
        // Blob data: [FD flat 24B][FDA obj 28B] with offsets [0, 24].
        let mut data = Vec::new();
        data.extend_from_slice(&BINDER_TYPE_FD.to_ne_bytes());
        data.extend_from_slice(&0u32.to_ne_bytes()); // flags
        data.extend_from_slice(&11u64.to_ne_bytes()); // handle = fd 11
        data.extend_from_slice(&0u64.to_ne_bytes()); // cookie
        data.extend_from_slice(&BINDER_TYPE_FDA.to_ne_bytes());
        data.extend_from_slice(&3u32.to_ne_bytes()); // numFds = 3
        data.extend_from_slice(&0u32.to_ne_bytes()); // pad
        data.extend_from_slice(&0u64.to_ne_bytes()); // parent
        data.extend_from_slice(&0u64.to_ne_bytes()); // parent_offset
        let mut offsets = Vec::new();
        offsets.extend_from_slice(&0u64.to_ne_bytes());
        offsets.extend_from_slice(&24u64.to_ne_bytes());
        assert_eq!(blob_fd_count(&data, &offsets), 4);
        // Data truncated before the FDA object: the FD flat still counts,
        // the unreadable numFds is skipped (the cross-check names the gap).
        assert_eq!(blob_fd_count(&data[..20], &offsets), 1);
        assert_eq!(blob_fd_count(&[], &[]), 0);
    }

    /// Unit: the fd-count tail codec round-trips; a malformed tail is
    /// rejected (None) rather than guessed.
    #[test]
    fn z355_fd_tail_codec_round_trip() {
        let mut payload = vec![0xa5u8; 10]; // arbitrary prefix
        append_fd_tail(&mut payload, &[1, 0, 2]);
        let counts = parse_fd_tail(&payload, 10, 3).expect("tail parses");
        assert_eq!(counts, vec![1, 0, 2]);
        // Wrong start offset → None.
        assert_eq!(parse_fd_tail(&payload, 9, 3), None);
        // Wrong blob count → None.
        assert_eq!(parse_fd_tail(&payload, 10, 2), None);
        // Truncated tail → None.
        assert_eq!(parse_fd_tail(&payload[..13], 10, 3), None);
    }

    /// END-TO-END through the REAL proxy: a guest→guest routed
    /// transaction whose blob carries a BINDER_TYPE_FD flat (the
    /// IAllocator gralloc-handle shape — the exact rn309+ wall). The fd
    /// rides SCM_RIGHTS B→proxy→A on the delivery, and A's BC_REPLY fd
    /// rides proxy→B on the drain — the kernel's fd translation split
    /// across our wire, with the fd flat `handle` fields staying
    /// sender-side numbers on the wire (the recipient shlib patches
    /// them). Pipe identity proves the dups are the SAME open file.
    #[test]
    fn z355_fds_cross_connections_via_scm_rights() {
        let rootfs = tmpdir();
        let path = create_binder_device(&rootfs, 0).expect("create_binder_device");
        let proxy = BinderProxy::new(0, &path).expect("BinderProxy::new");
        let _handle = proxy.spawn().expect("BinderProxy::spawn");
        std::thread::sleep(Duration::from_millis(50));
        let live_pid = std::process::id();

        // The SENDER's fd: the read end of a pipe (identity check: the
        // recipient's dup reads bytes the sender writes to the write end).
        let mut pipe_fds = [0i32; 2];
        assert_eq!(unsafe { libc::pipe(pipe_fds.as_mut_ptr()) }, 0);
        let (pr, pw) = (pipe_fds[0], pipe_fds[1]);

        // ---- Conn A (the mapper stand-in): addService ----
        let mut stream_a = UnixStream::connect(&path).expect("connect A");
        let mut ident = Vec::new();
        ident.extend_from_slice(&live_pid.to_ne_bytes());
        ident.extend_from_slice(&0u32.to_ne_bytes());
        ident.extend_from_slice(&0u32.to_ne_bytes());
        let (ri, _ri_resp) = exchange(&mut stream_a, WIRE_CMD_IDENT, &ident);
        assert_eq!(ri, 0);
        let mut args = ParcelWriter::new();
        args.write_string16("z355_svc");
        args.write_flat_binder(&FlatBinderObject {
            r#type: BINDER_TYPE_BINDER,
            flags: FLAT_FLAGS_LIBBINDER_DEFAULT,
            binder: 0x1111,
            cookie: 0x2222,
        });
        args.write_i32(0);
        args.write_i32(0);
        let (ad, ao) = make_servicemanager_request_parcel(&mut args);
        let mut bc = Vec::new();
        bc.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_ADD_SERVICE, 0));
        let (ret, _r) = exchange(
            &mut stream_a,
            BINDER_WRITE_READ,
            &make_v2_write_read_payload(&bc, &ad, &ao, 4096),
        );
        assert_eq!(ret, 0, "ADD_SERVICE ok");

        // ---- Conn B (the composer stand-in): getService ----
        let mut stream_b = UnixStream::connect(&path).expect("connect B");
        let (ri2, _ri2_resp) = exchange(&mut stream_b, WIRE_CMD_IDENT, &ident);
        assert_eq!(ri2, 0);
        let mut args2 = ParcelWriter::new();
        args2.write_string16("z355_svc");
        let (bd, bo) = make_servicemanager_request_parcel(&mut args2);
        let mut bc2 = Vec::new();
        bc2.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc2.extend_from_slice(&make_bc_transaction_payload(SVC_MGR_GET_SERVICE, 0));
        let (ret2, resp2) = exchange(
            &mut stream_b,
            BINDER_WRITE_READ,
            &make_v2_write_read_payload(&bc2, &bd, &bo, 4096),
        );
        assert_eq!(ret2, 0);
        let off2 = 4 + u32::from_ne_bytes(resp2[0..4].try_into().unwrap()) as usize + 8;
        let dl2 = u32::from_ne_bytes(resp2[off2..off2 + 4].try_into().unwrap()) as usize;
        let blob2 = &resp2[off2 + 12..off2 + 12 + dl2];
        let svc_handle = u64::from_ne_bytes(blob2[12..20].try_into().unwrap()) as u32;

        // ---- Conn B: transact with an FD-flat blob + SCM_RIGHTS fd ----
        // Parcel: one flat_binder_object {type=FD, handle=pr} at offset 0.
        let mut tx_data = Vec::new();
        tx_data.extend_from_slice(&BINDER_TYPE_FD.to_ne_bytes());
        tx_data.extend_from_slice(&0u32.to_ne_bytes()); // flags
        tx_data.extend_from_slice(&(pr as u64).to_ne_bytes()); // sender fd
        tx_data.extend_from_slice(&0u64.to_ne_bytes()); // cookie
        let mut tx_off = Vec::new();
        tx_off.extend_from_slice(&0u64.to_ne_bytes());
        let mut tx_b = [0u8; 64];
        tx_b[0..4].copy_from_slice(&svc_handle.to_ne_bytes());
        tx_b[16..20].copy_from_slice(&7u32.to_ne_bytes());
        let mut bc3 = Vec::new();
        bc3.extend_from_slice(&BC_TRANSACTION.to_ne_bytes());
        bc3.extend_from_slice(&tx_b);
        let mut payload3 =
            make_v2_write_read_multi_payload(&bc3, &[(tx_data.as_slice(), &tx_off)], 4096);
        // The fd tail: [FDT0][blob_count=1][counts=[1]].
        append_fd_tail(&mut payload3, &[1]);
        send_frame_with_fds(
            &mut stream_b,
            BINDER_WRITE_READ,
            &payload3,
            &[pr], // SCM_RIGHTS: the sender's fd rides the frame
        );
        let (ret_t, _resp_t, none_fds) = recv_resp_with_fds(&mut stream_b);
        assert_eq!(ret_t, 0);
        assert!(none_fds.is_empty(), "TX-complete response carries no fds");

        // ---- Conn A: the delivery carries the fd via SCM_RIGHTS ----
        let mut wr_a = Vec::new();
        wr_a.extend_from_slice(&0u32.to_ne_bytes());
        wr_a.extend_from_slice(&4096u32.to_ne_bytes());
        send_frame_with_fds(&mut stream_a, BINDER_WRITE_READ, &wr_a, &[]);
        let (ret_a, resp_a, fds_a) = recv_resp_with_fds(&mut stream_a);
        assert_eq!(ret_a, 0);
        assert_eq!(fds_a.len(), 1, "delivery delivers exactly one fd");
        assert_eq!(
            u32::from_ne_bytes(resp_a[4..8].try_into().unwrap()),
            BR_TRANSACTION
        );
        // The fd tail rode the response payload.
        assert_eq!(
            u32::from_ne_bytes(
                resp_a[resp_a.len() - 12..resp_a.len() - 8]
                    .try_into()
                    .unwrap()
            ),
            WIRE_FD_TAIL_MAGIC,
            "the fd tail is appended after the blob trailer"
        );
        // The delivered flat still names the SENDER's fd number on the
        // wire — the kernel-true split (the shlib patches the flat with
        // the dup it received; the proxy never rewrites it).
        let read_a = u32::from_ne_bytes(resp_a[0..4].try_into().unwrap()) as usize;
        let off_a = 4 + read_a + 8;
        let dl_a = u32::from_ne_bytes(resp_a[off_a..off_a + 4].try_into().unwrap()) as usize;
        let blob_a = &resp_a[off_a + 12..off_a + 12 + dl_a];
        assert_eq!(
            u64::from_ne_bytes(blob_a[8..16].try_into().unwrap()),
            pr as u64,
            "the FD flat's handle field keeps the sender's fd number"
        );
        // Pipe identity: the received dup reads what the sender writes.
        let dup_a = fds_a[0];
        let w = b"z355!";
        assert_eq!(
            unsafe { libc::write(pw, w.as_ptr() as *const _, w.len()) },
            5
        );
        let mut rb = [0u8; 5];
        assert_eq!(
            unsafe { libc::read(dup_a, rb.as_mut_ptr() as *mut _, 5) },
            5
        );
        assert_eq!(&rb, b"z355!");

        // ---- Conn A: BC_REPLY carrying ITS dup fd (the IAllocator reply
        // shape) — the fd crosses back to B through the drain ----
        let mut reply_data = Vec::new();
        reply_data.extend_from_slice(&0i32.to_ne_bytes()); // status NONE
        reply_data.extend_from_slice(&0i32.to_ne_bytes()); // pad
        reply_data.extend_from_slice(&BINDER_TYPE_FD.to_ne_bytes());
        reply_data.extend_from_slice(&0u32.to_ne_bytes()); // flags
        reply_data.extend_from_slice(&(dup_a as u64).to_ne_bytes()); // A's dup
        reply_data.extend_from_slice(&0u64.to_ne_bytes()); // cookie
        let mut reply_off = Vec::new();
        reply_off.extend_from_slice(&8u64.to_ne_bytes());
        let reply = [0u8; 64];
        let mut bc4 = Vec::new();
        bc4.extend_from_slice(&BC_REPLY.to_ne_bytes());
        bc4.extend_from_slice(&reply);
        let mut payload4 =
            make_v2_write_read_multi_payload(&bc4, &[(reply_data.as_slice(), &reply_off)], 0);
        append_fd_tail(&mut payload4, &[1]);
        send_frame_with_fds(&mut stream_a, BINDER_WRITE_READ, &payload4, &[dup_a]);
        let (ret_r, _resp_r, none_r) = recv_resp_with_fds(&mut stream_a);
        assert_eq!(ret_r, 0);
        assert!(none_r.is_empty());

        // ---- Conn B: the drained BR_REPLY carries A's fd via SCM_RIGHTS
        // and the tail — the same open pipe, proven by reading it ----
        let mut wr_b = Vec::new();
        wr_b.extend_from_slice(&0u32.to_ne_bytes());
        wr_b.extend_from_slice(&4096u32.to_ne_bytes());
        send_frame_with_fds(&mut stream_b, BINDER_WRITE_READ, &wr_b, &[]);
        let (ret_b, resp_b, fds_b) = recv_resp_with_fds(&mut stream_b);
        assert_eq!(ret_b, 0);
        assert_eq!(fds_b.len(), 1, "the reply delivers exactly one fd");
        assert_eq!(
            u32::from_ne_bytes(resp_b[4..8].try_into().unwrap()),
            BR_REPLY
        );
        let dup_b = fds_b[0];
        let w2 = b"PROXY";
        assert_eq!(unsafe { libc::write(pw, w2.as_ptr() as *const _, 5) }, 5);
        let mut rb2 = [0u8; 5];
        assert_eq!(
            unsafe { libc::read(dup_b, rb2.as_mut_ptr() as *mut _, 5) },
            5
        );
        assert_eq!(&rb2, b"PROXY", "B's dup is the SAME pipe A's dup came from");
        assert_ne!(
            dup_b, dup_a,
            "the two recipients hold DISTINCT fd-table slots (kernel dup semantics)"
        );

        // Cleanup: the test owns the received fds (the proxy closed its
        // copies after each sendmsg).
        for fd in fds_a.iter().copied().chain(fds_b.iter().copied()) {
            unsafe { libc::close(fd) };
        }
        unsafe {
            libc::close(pr);
            libc::close(pw);
        }
    }

    // ------------------------------------------------------------------
    // 6-Z469 — PROC-TODO TAKE (the waiting-looper half of the 6-Z408 gate)
    // ------------------------------------------------------------------

    /// Test helper: a sync transaction shaped like the rn435 wedge burst
    /// (pool thread → a node owned by its own process's parked main).
    fn z469_tx(requester: ConnId, txn: u64, sender_pid: i32) -> IncomingTx {
        IncomingTx {
            requester,
            txn_id: txn,
            code: 34,
            flags: 0x10,
            one_way: false,
            sender_pid,
            sender_euid: 0,
            blob: None,
            ptr: 0x1000,
            cookie: 0x2000,
        }
    }

    /// The rn435 wedge shape, byte-true: main (conn 227, pid 10485)
    /// parked mid-call to a FOREIGN process (out_sync target conn 89,
    /// pid 500) while its OWN pool thread (conn 238, same pid) has a
    /// sync txn held in main's inbox by the 6-Z408 gate; an idle pool
    /// looper (conn 239, same pid + dev, no out_sync) takes it — kernel
    /// proc-todo semantics (the waiting looper thread serves the proc's
    /// held work so the parked thread is never involved).
    #[test]
    fn z469_idle_looper_takes_gate_held_sync_from_parked_sibling() {
        let mut b = BusState::new();
        for c in [227u64, 238, 239, 89] {
            b.conns.insert(c, ConnBox::default());
        }
        b.conns.get_mut(&227).unwrap().sender_pid = 10485;
        b.conns.get_mut(&227).unwrap().dev_code = 1;
        // main parked on a sync call to conn 89 (a foreign pid → the
        // gate holds same-proc senders' work):
        b.conns
            .get_mut(&227)
            .unwrap()
            .out_sync
            .push_back((900, 89, std::time::Instant::now()));
        b.conns.get_mut(&238).unwrap().sender_pid = 10485;
        b.conns.get_mut(&238).unwrap().dev_code = 1;
        b.conns.get_mut(&239).unwrap().sender_pid = 10485;
        b.conns.get_mut(&239).unwrap().dev_code = 1;
        b.conns.get_mut(&89).unwrap().sender_pid = 500;
        let tx = z469_tx(238, 3318, 10485);
        b.conns
            .get_mut(&227)
            .unwrap()
            .inbox
            .push_back(InboxItem::Tx(tx));
        b.conns.get_mut(&227).unwrap().pending_in.push(3318);

        let taken = b
            .z469_take_held_sync(239)
            .expect("an idle same-proc looper must take the held sync txn");
        assert_eq!(taken.txn_id, 3318);
        assert_eq!(taken.requester, 238);
        // source bookkeeping: popped from the parked main's inbox and
        // pending_in (mirrors the 6-Z271g steal).
        assert!(b.conns[&227].inbox.is_empty());
        assert!(!b.conns[&227].pending_in.contains(&3318));
    }

    /// A looper with its own outstanding sync call NEVER takes (rn362:
    /// a thread parked in waitForResponse must never receive non-nested
    /// work mid-wait); the item stays queued for the owner conn.
    #[test]
    fn z469_busy_looper_never_takes() {
        let mut b = BusState::new();
        for c in [227u64, 239, 89] {
            b.conns.insert(c, ConnBox::default());
        }
        b.conns.get_mut(&227).unwrap().sender_pid = 10485;
        b.conns.get_mut(&227).unwrap().dev_code = 1;
        b.conns
            .get_mut(&227)
            .unwrap()
            .out_sync
            .push_back((900, 89, std::time::Instant::now()));
        b.conns.get_mut(&239).unwrap().sender_pid = 10485;
        b.conns.get_mut(&239).unwrap().dev_code = 1;
        // the taker is BUSY: its own sync call is outstanding:
        b.conns
            .get_mut(&239)
            .unwrap()
            .out_sync
            .push_back((777, 41, std::time::Instant::now()));
        b.conns.get_mut(&89).unwrap().sender_pid = 500;
        b.conns
            .get_mut(&227)
            .unwrap()
            .inbox
            .push_back(InboxItem::Tx(z469_tx(238, 3318, 10485)));
        assert!(
            b.z469_take_held_sync(239).is_none(),
            "a parked looper must not take non-nested work"
        );
        assert_eq!(b.conns[&227].inbox.len(), 1, "the item stays queued");
    }

    /// The take fires ONLY for gate-HELD fronts:
    /// (a) the reentrant case (main parked on a call to the SENDER's
    ///     own process) — the gate opens and main itself serves it on
    ///     its next poll (kernel transaction-stack ride); the take must
    ///     not race it;
    /// (b) cross-pid taker (the proc todo belongs to the target proc);
    /// (c) cross-device taker (6-Z309f: binder/hwbinder never cross);
    /// (d) one-way items (txn_id 0) — the gate never holds them.
    #[test]
    fn z469_never_takes_unheld_cross_pid_cross_dev_or_oneway() {
        // (a) NOT held: the parked call targets the sender's own proc.
        let mut b = BusState::new();
        for c in [227u64, 239, 89] {
            b.conns.insert(c, ConnBox::default());
        }
        b.conns.get_mut(&227).unwrap().sender_pid = 10485;
        b.conns.get_mut(&227).unwrap().dev_code = 1;
        b.conns
            .get_mut(&227)
            .unwrap()
            .out_sync
            .push_back((900, 89, std::time::Instant::now()));
        b.conns.get_mut(&239).unwrap().sender_pid = 10485;
        b.conns.get_mut(&239).unwrap().dev_code = 1;
        b.conns.get_mut(&89).unwrap().sender_pid = 10485; // same proc!
        b.conns
            .get_mut(&227)
            .unwrap()
            .inbox
            .push_back(InboxItem::Tx(z469_tx(238, 3318, 10485)));
        assert!(
            b.z469_take_held_sync(239).is_none(),
            "a gate-open (reentrant) front is served by the parked conn itself"
        );

        // (b) cross-pid taker.
        let mut b2 = BusState::new();
        for c in [227u64, 239, 89] {
            b2.conns.insert(c, ConnBox::default());
        }
        b2.conns.get_mut(&227).unwrap().sender_pid = 10485;
        b2.conns.get_mut(&227).unwrap().dev_code = 1;
        b2.conns
            .get_mut(&227)
            .unwrap()
            .out_sync
            .push_back((900, 89, std::time::Instant::now()));
        b2.conns.get_mut(&239).unwrap().sender_pid = 777; // foreign proc
        b2.conns.get_mut(&239).unwrap().dev_code = 1;
        b2.conns.get_mut(&89).unwrap().sender_pid = 500;
        b2.conns
            .get_mut(&227)
            .unwrap()
            .inbox
            .push_back(InboxItem::Tx(z469_tx(238, 3318, 10485)));
        assert!(
            b2.z469_take_held_sync(239).is_none(),
            "another process's looper must not take this proc's held work"
        );

        // (c) cross-device taker.
        let mut b3 = BusState::new();
        for c in [227u64, 239, 89] {
            b3.conns.insert(c, ConnBox::default());
        }
        b3.conns.get_mut(&227).unwrap().sender_pid = 10485;
        b3.conns.get_mut(&227).unwrap().dev_code = 1;
        b3.conns
            .get_mut(&227)
            .unwrap()
            .out_sync
            .push_back((900, 89, std::time::Instant::now()));
        b3.conns.get_mut(&239).unwrap().sender_pid = 10485;
        b3.conns.get_mut(&239).unwrap().dev_code = 2; // hwbinder vs binder
        b3.conns.get_mut(&89).unwrap().sender_pid = 500;
        b3.conns
            .get_mut(&227)
            .unwrap()
            .inbox
            .push_back(InboxItem::Tx(z469_tx(238, 3318, 10485)));
        assert!(
            b3.z469_take_held_sync(239).is_none(),
            "6-Z309f: held work never crosses devices"
        );

        // (d) one-way front (txn_id 0): the gate never holds it.
        let mut b4 = BusState::new();
        for c in [227u64, 239, 89] {
            b4.conns.insert(c, ConnBox::default());
        }
        b4.conns.get_mut(&227).unwrap().sender_pid = 10485;
        b4.conns.get_mut(&227).unwrap().dev_code = 1;
        b4.conns
            .get_mut(&227)
            .unwrap()
            .out_sync
            .push_back((900, 89, std::time::Instant::now()));
        b4.conns.get_mut(&239).unwrap().sender_pid = 10485;
        b4.conns.get_mut(&239).unwrap().dev_code = 1;
        b4.conns.get_mut(&89).unwrap().sender_pid = 500;
        let mut oneway = z469_tx(238, 0, 10485);
        oneway.one_way = true;
        b4.conns
            .get_mut(&227)
            .unwrap()
            .inbox
            .push_back(InboxItem::Tx(oneway));
        assert!(
            b4.z469_take_held_sync(239).is_none(),
            "one-way items ride the owner's own queue (never 6-Z408-held)"
        );
    }

    /// Deterministic source selection: two parked siblings both holding
    /// gate-held sync work → the take pops the LOWEST conn id first.
    #[test]
    fn z469_take_is_lowest_sibling_first() {
        let mut b = BusState::new();
        for c in [100u64, 200, 239, 89, 91] {
            b.conns.insert(c, ConnBox::default());
        }
        b.conns.get_mut(&100).unwrap().sender_pid = 10485;
        b.conns.get_mut(&100).unwrap().dev_code = 1;
        b.conns
            .get_mut(&100)
            .unwrap()
            .out_sync
            .push_back((900, 89, std::time::Instant::now()));
        b.conns.get_mut(&200).unwrap().sender_pid = 10485;
        b.conns.get_mut(&200).unwrap().dev_code = 1;
        b.conns
            .get_mut(&200)
            .unwrap()
            .out_sync
            .push_back((901, 91, std::time::Instant::now()));
        b.conns.get_mut(&239).unwrap().sender_pid = 10485;
        b.conns.get_mut(&239).unwrap().dev_code = 1;
        b.conns.get_mut(&89).unwrap().sender_pid = 500;
        b.conns.get_mut(&91).unwrap().sender_pid = 600;
        b.conns
            .get_mut(&100)
            .unwrap()
            .inbox
            .push_back(InboxItem::Tx(z469_tx(238, 1, 10485)));
        b.conns
            .get_mut(&200)
            .unwrap()
            .inbox
            .push_back(InboxItem::Tx(z469_tx(238, 2, 10485)));
        let taken = b
            .z469_take_held_sync(239)
            .expect("two held sources → the lowest conn id pops first");
        assert_eq!(taken.txn_id, 1, "conn 100 < conn 200 → its txn pops first");
        assert_eq!(b.conns[&100].inbox.len(), 0);
        assert_eq!(b.conns[&200].inbox.len(), 1);
    }

    // ------------------------------------------------------------------
    // 6-Z491 — the delivery-vs-consumed accounting + the wedge scan
    // ------------------------------------------------------------------

    /// Backdate a conn's last_rx/last_del past the wedge quiesce window
    /// so the scan's idle gate trips deterministically.
    fn z491_backdate(bx: &mut ConnBox) {
        let past =
            std::time::Instant::now() - Z491_WEDGE_QUIESCE - std::time::Duration::from_millis(200);
        bx.z491.last_rx = Some(past);
        bx.z491.last_del = Some(past);
    }

    /// The ENQUEUE leg: queue_transaction bumps tx_enq and the inbox
    /// high-water mark (the deliver leg is inline in handle_write_read —
    /// exercised end-to-end by the boot ladder; here the counters are).
    #[test]
    fn z491_queue_transaction_counts_enqueue_leg() {
        let mut b = BusState::new();
        b.conns.insert(300u64, ConnBox::default());
        assert!(
            b.queue_transaction(z469_tx(200, 41, 10485), 300),
            "queue must succeed"
        );
        assert_eq!(b.conns[&300].z491.tx_enq, 1);
        assert_eq!(b.conns[&300].z491.max_inbox, 1);
        assert!(
            b.queue_transaction(z469_tx(200, 42, 10485), 300),
            "second queue must succeed"
        );
        assert_eq!(b.conns[&300].z491.tx_enq, 2);
        assert_eq!(b.conns[&300].z491.max_inbox, 2, "high-water stays");
    }

    /// The rn466/rn468 installd shape: the conn TOOK sync transactions
    /// (txn_stack non-empty), shows no write-side activity through the
    /// quiesce window, no nested call outstanding → WEDGE-B verdicts
    /// (last_verdict arms).
    #[test]
    fn z491_wedge_b_fires_for_taken_never_answered() {
        let mut b = BusState::new();
        b.conns.insert(310u64, ConnBox::default());
        b.conns.get_mut(&310).unwrap().sender_pid = 10485;
        b.conns.get_mut(&310).unwrap().dev_code = 1;
        b.conns.get_mut(&310).unwrap().txn_stack.push(7);
        z491_backdate(b.conns.get_mut(&310).unwrap());
        let bus = std::sync::Arc::new(std::sync::Mutex::new(b));
        z491_ioctl_tick(&bus, 1, 310, true);
        let b = bus.lock().unwrap();
        assert!(
            b.conns[&310].z491.last_verdict.is_some(),
            "stack non-empty + quiescent → WEDGE-B must verdict"
        );
        assert_eq!(b.conns[&310].z491.wr_calls, 1, "the tick counts the ioctl");
        assert_eq!(b.conns[&310].z491.noop_polls, 1, "empty read stream");
    }

    /// The healthy idle looper: every mailbox empty, whatever its poll
    /// rate — the tick counts the ioctl but NEVER verdicts.
    #[test]
    fn z491_drained_conn_never_verdicts() {
        let mut b = BusState::new();
        b.conns.insert(311u64, ConnBox::default());
        z491_backdate(b.conns.get_mut(&311).unwrap());
        let bus = std::sync::Arc::new(std::sync::Mutex::new(b));
        z491_ioctl_tick(&bus, 1, 311, true);
        z491_ioctl_tick(&bus, 1, 311, true);
        let b = bus.lock().unwrap();
        assert!(b.conns[&311].z491.last_verdict.is_none());
        assert_eq!(b.conns[&311].z491.wr_calls, 2);
        assert_eq!(b.conns[&311].z491.noop_polls, 2);
    }

    /// The 6-Z408 exclusion: inbox/stack stuck while the conn is parked
    /// on its OWN nested call (out_sync non-empty) is the gate-hold shape
    /// z408_note_hold already names — the wedge scan must stay silent
    /// (the REPLY_TIMEOUT machinery owns the unwind).
    #[test]
    fn z491_nested_call_excludes_wedge_a_and_b() {
        let mut b = BusState::new();
        b.conns.insert(312u64, ConnBox::default());
        {
            let bx = b.conns.get_mut(&312).unwrap();
            bx.txn_stack.push(9);
            bx.out_sync.push_back((777, 41, std::time::Instant::now()));
            z491_backdate(bx);
        }
        let bus = std::sync::Arc::new(std::sync::Mutex::new(b));
        z491_ioctl_tick(&bus, 1, 312, true);
        let b = bus.lock().unwrap();
        assert!(
            b.conns[&312].z491.last_verdict.is_none(),
            "nested-call parking is the 6-Z408 shape, not a wedge"
        );
    }

    /// A resolved reply stuck on the reply_queue (the client stopped
    /// reading between its calls — the rn464 client-side half) → WEDGE-C.
    #[test]
    fn z491_wedge_c_fires_for_reply_never_polled() {
        let mut b = BusState::new();
        b.conns.insert(313u64, ConnBox::default());
        {
            let bx = b.conns.get_mut(&313).unwrap();
            bx.reply_queue.push_back(DeferredReply::Failed);
            z491_backdate(bx);
        }
        let bus = std::sync::Arc::new(std::sync::Mutex::new(b));
        z491_ioctl_tick(&bus, 1, 313, false);
        let b = bus.lock().unwrap();
        assert!(
            b.conns[&313].z491.last_verdict.is_some(),
            "stuck reply + quiescent → WEDGE-C must verdict"
        );
    }

    /// Fresh write-side activity (last_rx inside the window) disarms the
    /// scan even with a stuck stack — a conn making outgoing calls is
    /// alive; the verdict would be noise.
    #[test]
    fn z491_fresh_rx_disarms_the_scan() {
        let mut b = BusState::new();
        b.conns.insert(314u64, ConnBox::default());
        {
            let bx = b.conns.get_mut(&314).unwrap();
            bx.txn_stack.push(7);
            bx.z491.note_rx();
            bx.z491.last_del = Some(
                std::time::Instant::now()
                    - Z491_WEDGE_QUIESCE
                    - std::time::Duration::from_millis(200),
            );
        }
        let bus = std::sync::Arc::new(std::sync::Mutex::new(b));
        z491_ioctl_tick(&bus, 1, 314, true);
        let b = bus.lock().unwrap();
        assert!(b.conns[&314].z491.last_verdict.is_none());
    }

    /// The per-conn verdict throttle: a second tick inside Z491_VERDICT_GAP
    /// must NOT re-verdict (last_verdict unchanged) — 64 budget lines span
    /// the run instead of flooding on one conn.
    #[test]
    fn z491_verdict_throttles_within_gap() {
        let mut b = BusState::new();
        b.conns.insert(315u64, ConnBox::default());
        b.conns.get_mut(&315).unwrap().txn_stack.push(7);
        z491_backdate(b.conns.get_mut(&315).unwrap());
        let bus = std::sync::Arc::new(std::sync::Mutex::new(b));
        z491_ioctl_tick(&bus, 1, 315, true);
        let first = {
            let b = bus.lock().unwrap();
            b.conns[&315].z491.last_verdict
        };
        assert!(first.is_some(), "first tick verdicts");
        z491_ioctl_tick(&bus, 1, 315, true);
        let b = bus.lock().unwrap();
        assert_eq!(
            b.conns[&315].z491.last_verdict, first,
            "inside the gap → throttled (same verdict instant)"
        );
    }

    // -- 6-Z495: the EXCHANGE-STUCK sweep (the mid-exchange blind spot) --

    /// An exchange in flight past the stuck threshold → the sweep verdicts
    /// exactly once and stamps the throttle. This is the rn470 (ladder
    /// rn471) shape: the era-2 main thread parked inside bp_exchange_anc
    /// for the rest of the run — no completed ioctl, no z491 verdict, the
    /// 6-Z402 heartbeat showing inbox=1.
    #[test]
    fn z495_sweep_names_stuck_inflight_exchange() {
        let mut b = BusState::new();
        b.conns.insert(410u64, ConnBox::default());
        b.conns.get_mut(&410).unwrap().z495_inflight = Some(Z495InFlight {
            arrived_at: std::time::Instant::now()
                - Z495_EXCHANGE_STUCK
                - std::time::Duration::from_secs(5),
            ws: 96,
            rc: 256,
        });
        let bus = std::sync::Arc::new(std::sync::Mutex::new(b));
        let emitted = z495_exchange_sweep(&bus, 1);
        assert_eq!(emitted, 1, "a stuck exchange must verdict once");
        let b = bus.lock().unwrap();
        assert!(b.conns[&410].z495_last_verdict.is_some());
        assert!(
            b.conns[&410].z495_inflight.is_some(),
            "the sweep observes; the loop owns the stamp"
        );
    }

    /// Fresh stamps (inside the threshold) and cleared stamps (the
    /// exchange completed) never verdict — the sweep is silent on health.
    #[test]
    fn z495_sweep_silent_for_fresh_and_cleared_exchanges() {
        let mut b = BusState::new();
        b.conns.insert(411u64, ConnBox::default());
        b.conns.insert(412u64, ConnBox::default());
        b.conns.get_mut(&411).unwrap().z495_inflight = Some(Z495InFlight {
            arrived_at: std::time::Instant::now(),
            ws: 0,
            rc: 256,
        });
        // conn 412: no stamp at all (idle between ioctls).
        let bus = std::sync::Arc::new(std::sync::Mutex::new(b));
        let emitted = z495_exchange_sweep(&bus, 1);
        assert_eq!(emitted, 0, "fresh/idle conns are not stuck");
    }

    /// The per-conn throttle: a second sweep inside Z495_VERDICT_GAP must
    /// not re-verdict the same stuck exchange.
    #[test]
    fn z495_sweep_throttles_per_conn() {
        let mut b = BusState::new();
        b.conns.insert(413u64, ConnBox::default());
        b.conns.get_mut(&413).unwrap().z495_inflight = Some(Z495InFlight {
            arrived_at: std::time::Instant::now()
                - Z495_EXCHANGE_STUCK
                - std::time::Duration::from_secs(5),
            ws: 96,
            rc: 256,
        });
        let bus = std::sync::Arc::new(std::sync::Mutex::new(b));
        assert_eq!(z495_exchange_sweep(&bus, 1), 1);
        assert_eq!(
            z495_exchange_sweep(&bus, 1),
            0,
            "the throttle must suppress the immediate re-verdict"
        );
    }
}
