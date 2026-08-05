# Security Policy

Twoyi runs an entire second Android userland inside a single unprivileged
app process — please read this document before deploying, distributing,
or auditing the project.

---

## 1. Reporting a vulnerability

If you believe you have found a security issue in Twoyi, **do not open a
public GitHub issue**. Instead:

1. Open a **private security advisory** at
   `https://github.com/Disable-OP/twoyi/security/advisories/new`, or
2. Contact the maintainer via the email on the
   [`Disable-OP`](https://github.com/Disable-OP) GitHub profile, subject
   `Confidential — Twoyi security report`.

Do **not** disclose publicly until a fix is released and the coordinated
disclosure window (default 90 days) has elapsed. Acknowledgement is
expected within **5 business days**. Confirmed reports are credited in
release notes unless you ask to remain anonymous.

**Please include:** Twoyi version and commit SHA; host device, Android
version, and ABI; guest rootfs used; step-by-step reproduction (with
inputs, ROMs, APKs); impact and severity (host-app / guest-to-guest
escape, denial of service, data leak); suggested fix, if any.

---

## 2. Security considerations — running another Android userland

Twoyi runs a complete Android userland (`init`, `zygote`,
`system_server`, `SurfaceFlinger`, framework, HALs, ART) **inside the
host app's process tree, sharing the host kernel**. No hardware
hypervisor, no separate kernel:

- **Shared kernel attack surface.** A kernel exploit in the guest is a
  kernel exploit on the host. Twoyi provides no kernel-level isolation.
- **Same UID as the host app.** All guest processes run with the host
  app's UID and can read any file the host app can read. The mount
  namespace and seccomp filter (§5) are the primary barriers, not UID
  separation.
- **Bundled rootfs trust.** A rootfs is third-party code. Only boot
  images from sources you trust. The cyanmint `original` rootfs is
  community-built and not audited by this project.
- **Network reachability.** The guest has a network stack via the host
  app's socket permissions and can reach the network like any host app.
- **No verified boot.** The guest filesystem is writable, ordinary
  files under `/data/data/io.twoyi/` — no dm-verity, no attestation.

Twoyi is **not** a sandbox for untrusted Android code. For strong
isolation, use a real virtual machine (KVM, gVisor-style) instead.

---

## 3. The committed test keystore

The repository ships a self-signed RSA-2048 keystore at
[`app/twoyi-release.keystore`](app/twoyi-release.keystore) (also mirrored
at the repo root). Store, key, and alias passwords are all
`twoyi-release`. Gradle uses it to sign every release build from
`./gradlew assembleRelease`.

- Anyone with the repo can produce APKs signable with the same key as
  the official builds — they can impersonate the "official" Twoyi
  signature to Android's package manager.
- The key is **not trusted by Google Play** and is unsuitable for any
  production distribution.
- An attacker who obtains this key (trivial — it is in git) can craft an
  update Android will accept as from the same publisher, **overwriting
  a genuine Twoyi install** on any device that sideloaded the original
  signed build.

**Before publishing**, replace the keystore with your own RSA-2048 key
(e.g. `keytool -genkeypair -v -keystore app/twoyi-release.keystore
-alias twoyi-release -keyalg RSA -keysize 2048 -validity 10000`) and
keep the replacement **out of version control**. The committed key is
compromised-by-design — it exists only so CI and codespace builds can
produce installable APKs without manual setup.

---

## 4. SELinux — permissive mode required for testing

Twoyi runs as an unprivileged app and cannot relabel the guest
filesystem, load SELinux policies, or set enforcing mode. The guest
`init` is patched (or instructed via `androidboot.selinux=permissive`)
to leave SELinux in **permissive** mode — denials are logged but not
enforced.

- The guest's MAC layer is effectively disabled. Every domain can
  read/write every file the UID can reach.
- Android security boundaries that depend on SELinux (in-guest app
  sandboxing, SELinux-protected services) are weaker than on a real
  device.
- AVC denials in `dmesg` / logcat are expected, not bugs.
- Running untrusted guest code inside Twoyi is **more dangerous** than
  on a stock Android device with SELinux enforcing.

Reaching SELinux enforcing mode inside the container is roadmap item
5.10 and is not yet implemented. Treat the guest as having no MAC.

---

## 5. The `kr64` seccomp filter

The `kr64` daemon (`app/rs/kr64/`) installs a BPF seccomp filter on the
guest process tree before `execve(init)`. Every syscall is classified
into one of three buckets:

- **Allowed (~80 syscalls)** — passed through to the host kernel
  unchanged (`read`, `write`, `openat`, `mmap`, `futex`, …). Default
  for ordinary syscalls.
- **Trapped (`SECCOMP_RET_TRAP`)** — `mount`, `umount2`, `swapon`,
  `swapoff`, `reboot`, `acct`, `sethostname`. Caught by a `SIGSYS`
  handler and emulated in userspace (e.g. `mount` → in-namespace bind
  mount) or faked to return success.
- **Killed (`SECCOMP_RET_KILL_PROCESS`)** — `ptrace`, `kexec_load`,
  `init_module`, `finit_module`, `delete_module`, `pivot_root`. Any
  guest process invoking these is terminated immediately.
- The filter is a **containment** mechanism, not a security sandbox.
  Its goal is to stop the guest from disrupting the host (mounting over
  host paths, loading kernel modules, rebooting) — not to stop a
  determined attacker from breaking out.
- The default action for unknown syscalls is `ALLOW`, so future kernel
  syscalls pass through unfiltered. `PR_SET_NO_NEW_PRIVS` is set, so
  setuid binaries cannot elevate the guest.
- The filter is **architecture-checked**: a syscall from the wrong ABI
  is killed, blocking the seccomp-bypass via `int 0x80` on a 64-bit
  process.

---

## 6. Root access inside the container (Magisk)

The guest rootfs includes **Magisk** and a `su` binary. Guest apps (and
the guest shell via `adb`) can obtain **root inside the guest's
namespace**.

- Root is scoped to the guest. It does **not** grant root on the host
  device — the guest still runs under the host app's UID and is
  constrained by the host kernel and the seccomp filter (§5).
- However, root within the guest is unrestricted relative to the
  guest itself: a Magisk-rooted guest app can read every other guest
  app's data, install system-level hooks (Xposed modules), and modify
  the guest's `/system` overlay.
- If the host app's data directory is ever exposed (documents-provider
  leak, backup, or filesystem traversal bug), guest-root means that
  exposure covers **all guest app data**, not just the calling app's.
- Never run Twoyi inside a work profile that holds sensitive host data,
  and do not grant the host app broad storage or accessibility
  permissions it does not strictly need.
