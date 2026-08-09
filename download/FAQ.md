# Twoyi FAQ

> Answers to the most common questions from new contributors and users.
> This document reflects the honest state of the project as of the
> `main` branch (round 68, 2026-08-08). Twoyi has a documented history of
> overclaims — when in doubt, trust [`TWOYI_HONEST_STATUS.md`](TWOYI_HONEST_STATUS.md)
> over any "it works" claim, including ones here.

---

## 1. Does twoyi work on x86_64?

**Partially.** The app no longer crashes (the SIGABRT in `renderer_reset_window`
was fixed in commit `7664c66`), and on an x86_64 host using the Android SDK's
`system-images;android-30;google_apis;x86_64` as the rootfs we have, for the
first time ever:

- Seen in logcat: `avc: granted { execute } for path=".../rootfs/init"` — the
  guest `init` binary actually ran.
- `[NEW_RENDERER] QEMU pipe device /dev/qemu_pipe availability: true`
- `[NEW_RENDERER] Successfully connected to QEMU pipe: /opengles3`
- `[NEW_RENDERER] GL context created successfully`

**But rendering does not work yet.** The very next line in logcat is:

```
[NEW_RENDERER] Failed to write to pipe: Invalid argument (os error 22)
```

See [`X86_64_BREAKTHROUGH.md`](X86_64_BREAKTHROUGH.md) for the full sequence.
The honest summary: the architecture-mismatch blocker that previously looked
like a 3–5 day AOSP build is solved; the remaining blocker is one well-defined
task (see question 3).

## 2. Can I boot a GSI?

**Not yet.** Booting a Generic System Image (GSI) is the headline goal of the
fork, and the design is fully specified in [`GSI_BOOT_PLAN.md`](GSI_BOOT_PLAN.md),
but none of its nine sub-projects are implemented beyond skeleton stage. The
single biggest missing piece is the `kr64` kernel-replacement daemon being
wired into the boot flow (see question 7). Until that lands, `RomManager`
cannot call `GsiExtractor.extract()` → `GsiInitPatcher.patch()` → `libkr64.so`
spawn → guest `init` exec → `BOOT_COMPLETED`.

If you want to work on this, Phase 3 of [`DEVELOPMENT_ROADMAP.md`](DEVELOPMENT_ROADMAP.md)
is the entry point — every protocol, device path, and constant has already
been decoded and documented.

## 3. Why doesn't the container render?

Because of a **QEMU pipe protocol mismatch**. Twoyi's renderer connects to
`/dev/qemu_pipe` and expects the guest's SurfaceFlinger to send GL commands
through it. In a stock Android emulator, `/dev/qemu_pipe` exists but it is
connected to the **emulator's own** goldfish GL renderer, which speaks a
protocol twoyi's renderer does not understand. Writes therefore fail with
`EINVAL` (`os error 22`).

The fix is for the `kr64` daemon to create its **own** `/dev/qemu_pipe` (the
`create_qemu_pipe()` stub already exists in `app/rs/kr64/src/devices.rs`) and
route it to twoyi's AOSP-built `libOpenglRender.so`. Once that is in place,
the guest's SurfaceFlinger will talk to twoyi's renderer instead of the
emulator's. This is the new top priority of the project.

## 4. How is this different from the original twoyi?

The original `twoyi/twoyi` repo is archived and was last touched in 2022. This
fork lives on the `main` branch of
[`Disable-OP/twoyi`](https://github.com/Disable-OP/twoyi) (the historical
`improvements/initial-cleanup` branch has been merged in and deleted) and
adds **84+ commits**
on top of upstream, including:

- An **open-source `libOpenglRender.so`** rebuilt from AOSP `emugl` source
  (Apache-2.0), replacing the 1.06 MB closed-source arm64-only blob. Builds
  for both `arm64-v8a` and `x86_64`.
- The **`kr64` kernel-replacement daemon** in Rust — ~11,554 lines, 165 unit
  tests, 8 feature modules (binder, sensors, audio, battery, seccomp,
  `proc_emu`, `mount_mgr`, devices).
- **Work profile support** — eight hardcoded `/data/data/io.twoyi` paths
  replaced with `Context.getDataDir()`-resolved runtime paths, so the app
  works inside a work profile or cloned-app context.
- A **devcontainer** with KVM, signed release APKs, CI on every push, and a
  complete contributor documentation set (`QUICK_START.md`,
  `CONTRIBUTING.md`, `DEVELOPMENT_ROADMAP.md`, `ARCHITECTURE.md`, plus 27
  analysis reports in `download/`).

See [`MIGRATION_GUIDE.md`](MIGRATION_GUIDE.md) if you are coming from the
original fork.

## 5. What is Virtual Master and why did you reverse-engineer it?

Virtual Master (`com.clone.android.dual.space`) is a closed-source
Android-on-Android container app that is functionally equivalent to twoyi but
noticeably more polished — it ships audio, sensor, camera, and battery HALs,
multi-VM support, and a cloud ROM distribution protocol. We reverse-engineered
it end-to-end (six analysis reports, ~4,000 lines, in `download/`) **to learn
how the binder, audio, and sensor HALs work** in a userspace-kernel-replacement
architecture. The `kr64` daemon's design is directly informed by VM's
`libkr64.so`.

We also recovered its AES-128-ECB key (`%z89aviCM0KkbEs9`) and confirmed it
does not bundle a ROM in the APK — its four `assets/plugins/*.zip` files are
encrypted add-on packs (GApps, Magisk, Xposed, Superuser), and ROM images are
downloaded from `api.virtualmaster.app` behind an auth flow. We do **not** ship
any of VM's code or proprietary assets.

## 6. Can I use this on a real device?

**Yes, on arm64.** The release APK is signed with an APK Signature Scheme v2
key and installs cleanly via `adb install -r`. On a physical arm64 phone, the
bundled arm64 rootfs extracts, the guest `init` executes natively, the QEMU
pipe is created by the guest's modified `init`, and the legacy renderer works.
This is the intended use case for the original twoyi and remains the most
reliable path today.

On x86_64 hardware (e.g. a Chromebook or an Intel tablet) the same caveats as
question 1 apply: the app launches and the renderer initializes, but the
container does not yet render. Pick up the kr64 pipe-creation work (question 3)
if you want to fix that.

## 7. What's the `kr64` daemon?

`kr64` is a **kernel replacement daemon** written in Rust. When twoyi boots a
guest Android, the guest thinks it is running on a real kernel — but it is
actually inside one host Android process. `kr64` fakes the kernel side: it
materialises a per-VM virtual `/dev/` tree (`qemu_pipe`, `touch`, `key0`,
`gb`, `gb2`, `event`, `binder`, `audio`, `sensors`), installs a seccomp BPF
filter (~60 syscalls allowed, ~15 blocked) with a `SIGSYS` handler, emulates a
static `/proc`, sets up a mount namespace via `pivot_root` + tmpfs, and finally
`exec`s the guest `init`.

It is the centrepiece of the fork. Today it compiles, all 165 tests pass, and
the design is complete — but its JNI surface is stubbed, the binder proxy is
unreachable without a guest-side LD_PRELOAD shim, and `create_qemu_pipe()` is
still a stub. Wiring kr64 into the actual boot flow is the single highest-
leverage piece of remaining work.

## 8. How do I contribute?

Start with [`QUICK_START.md`](QUICK_START.md) (5-minute path from `git clone`
to a picked task) and [`CONTRIBUTING.md`](../CONTRIBUTING.md) (dev environment,
code style, PR process). The full work plan, with file paths and acceptance
criteria, is in [`DEVELOPMENT_ROADMAP.md`](DEVELOPMENT_ROADMAP.md).

Three good first issues (each effort **S** — ≤1 week, no architectural
discussion needed):

1. Drop-in test the AOSP renderer on a real arm64 device (Phase 1 task 1.1,
   ~1 day). Requires a physical arm64 phone. Highest-leverage verification in
   the project.
2. Wire `kr64` into the boot flow (Phase 1 task 1.4, ~2 days). Goal: see
   `[KR64 INFO] created device /dev/qemu_pipe` in logcat on redroid x86_64.
3. Extend the `kr64` device tree to 20+ devices (Phase 3 task 3.1, ~3 days,
   ~30 min per device).

Branch from `main`, use Conventional Commits
(`feat:`, `fix:`, `docs:` …), and open a PR against the same branch. **Open
an issue first** for anything bigger than a typo.

## 9. What's the license?

- **Twoyi's own code** (Rust crates, Java, native glue) is **MPL-2.0**. See
  [`LICENSE`](../LICENSE) for the full text.
- **AOSP-derived code** — most notably the `libOpenglRender.so` source rebuilt
  from `platform/sdk` commit `7a712acc` — is **Apache-2.0**. Provenance is
  documented in [`AOSP_BUILD_RESULTS.md`](AOSP_BUILD_RESULTS.md).
- The still-shipped `libadb.so` blob (4.46 MB, arm64-only) is AOSP's `adb`
  binary renamed; it is Apache-2.0 upstream and slated for replacement in
  Phase 2 of the roadmap.

The MPL-2.0 choice is deliberate: it allows the code to be used in
proprietary apps while requiring that improvements to the MPL-covered files
themselves be contributed back.

## 10. Is this ready for production?

**No.** This is an active development fork with skeleton implementations. The
honest status, from [`TWOYI_HONEST_STATUS.md`](TWOYI_HONEST_STATUS.md):

- The app launches and does not crash on x86_64. ✅
- The new Rust renderer initializes and creates a GL context. ✅
- The guest `init` executes and the QEMU pipe connects on x86_64. ✅
- The container renders. ❌ — pipe protocol mismatch, see question 3.
- A GSI boots. ❌ — kr64 not wired in, see question 2.
- Audio, sensors, battery, binder virtualisation. 🟡 — skeletons exist, JNI
  is stubbed.
- The `libadb.so` blob is still closed-source. 🔴

If you want a working Android-on-Android container **today**, use the
original twoyi release on an arm64 device, or use Virtual Master. This fork is
for contributors who want to help build the open-source, GSI-booting,
multi-HAL future of twoyi. If that is you, start at question 8.

---

*Need more detail? Read [`SESSION_SUMMARY.md`](SESSION_SUMMARY.md) for the
overnight work log, [`PROJECT_SUMMARY.md`](PROJECT_SUMMARY.md) for the
definitive state-of-the-project write-up, and
[`TECHNICAL_BRIEFING.md`](TECHNICAL_BRIEFING.md) for a 15-minute
architectural briefing.*
