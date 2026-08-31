// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 6-Z271: host HAL bridge — tracer → app notifications for REAL
//! host-backed hardware effects.
//!
//! The in-proxy virtual binder services (binder.rs) forward guest HAL
//! requests here; this module delivers them to the host app over the
//! existing `@TWOYI_SOCK` abstract-namespace SEQPACKET channel (the same
//! socket the 6-Z48/6-Z62 BOOT_COMPLETED synthesis uses — the app's
//! `TwoyiSocketServer` accepts it). Message protocol: one packet per
//! request, ASCII, `TWOYI_VIBRATE:<ms>` / `TWOYI_VIBRATE_OFF`. The Java
//! side maps them onto the real host `Vibrator` API.
//!
//! Design constraints honored here:
//! * no uncontrolled vibration: every request carries an explicit ms cap
//!   (60 s) and `off` cancels; the host side additionally clamps;
//! * fire-and-forget with bounded retry: a failed delivery is logged and
//!   dropped (haptics are best-effort — a missing host bridge must never
//!   wedge the guest's binder thread);
//! * connect-per-message (the BOOT_COMPLETED recipe): no persistent fd to
//!   leak, no reconnect state machine. Local UDS connect+write+close is
//!   sub-millisecond.

use std::time::Duration;

/// Cap on a single vibration request (ms) — mirrors the virtual HAL's
/// validation in binder.rs.
const MAX_VIBRATE_MS: i32 = 60_000;

/// Connect + send timeout: the app's LocalServerSocket accept loop is
/// fast; if it isn't there (app dead), fail fast and drop the request.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(250);

/// Send one text packet to the app's `@TWOYI_SOCK` (and its twin
/// `@TWOYI_BOOT_SOCK`) abstract socket. Best-effort: returns false when
/// no listener accepted.
fn send_to_app_sock(msg: &str) -> bool {
    const SOCK_NAMES: [&str; 2] = ["TWOYI_SOCK", "TWOYI_BOOT_SOCK"];
    for sock_name in SOCK_NAMES {
        let sock =
            unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC, 0) };
        if sock < 0 {
            continue;
        }
        // Bind a connect timeout via the socket's SNDTIMEO so a wedged
        // listener can't stall the caller (the proxy's per-connection
        // thread runs this inline).
        let tv = libc::timeval {
            tv_sec: CONNECT_TIMEOUT.as_secs() as libc::time_t,
            tv_usec: CONNECT_TIMEOUT.subsec_micros() as libc::suseconds_t,
        };
        unsafe {
            libc::setsockopt(
                sock,
                libc::SOL_SOCKET,
                libc::SO_SNDTIMEO,
                &tv as *const libc::timeval as *const libc::c_void,
                std::mem::size_of::<libc::timeval>() as libc::socklen_t,
            );
        }
        let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        addr.sun_family = libc::AF_UNIX as u16;
        let name_bytes = sock_name.as_bytes();
        let copy_len = name_bytes.len().min(addr.sun_path.len() - 1);
        for (i, &b) in name_bytes[..copy_len].iter().enumerate() {
            addr.sun_path[i + 1] = b as libc::c_char;
        }
        let addr_len = (std::mem::size_of::<u16>() + 1 + copy_len) as u32;
        let connected =
            unsafe { libc::connect(sock, &addr as *const _ as *const libc::sockaddr, addr_len) }
                == 0;
        if connected {
            let bytes = msg.as_bytes();
            let written =
                unsafe { libc::write(sock, bytes.as_ptr() as *const libc::c_void, bytes.len()) };
            unsafe { libc::close(sock) };
            if written == bytes.len() as isize {
                return true;
            }
        } else {
            unsafe { libc::close(sock) };
        }
    }
    false
}

/// Forward a guest `IVibrator.on(timeoutMs)` to the host app → the real
/// phone vibrator.
pub fn notify_vibrate(timeout_ms: i32) {
    let ms = timeout_ms.clamp(1, MAX_VIBRATE_MS);
    let msg = format!("TWOYI_VIBRATE:{}", ms);
    if !send_to_app_sock(&msg) {
        // Best-effort: haptics must never wedge the binder thread. A
        // bounded single retry after a short backoff covers the app-side
        // accept-loop race during app startup.
        std::thread::sleep(Duration::from_millis(20));
        let _ = send_to_app_sock(&msg);
    }
}

/// Forward a guest `IVibrator.off()` to the host app → cancel vibration.
pub fn notify_vibrator_off() {
    let _ = send_to_app_sock("TWOYI_VIBRATE_OFF");
}
