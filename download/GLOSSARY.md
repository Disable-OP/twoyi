# Twoyi Glossary

> Reference for technical terms used across the twoyi codebase, architecture
> docs, and reverse-engineering notes. Aimed at new contributors. Alphabetical;
> each entry gives a one-line definition plus (where useful) how it shows up in
> twoyi.

## A

**ABI** — Application Binary Interface (calling conventions, struct layouts, syscall numbers). twoyi ships `arm64-v8a` and `x86_64`; each JNI library is built per-ABI.

**AOSP** — Android Open Source Project. twoyi builds `libOpenglRender_aosp.so` and bionic-style compat headers from AOSP source (commit `7a712ac…`, NDK r27c, clang 18).

**AVD** — Android Virtual Device (emulator config). The `system-images;android-30;google_apis;x86_64` AVD supplied twoyi's first bootable x86_64 rootfs.

## B

**binder** — Android's kernel IPC driver. The guest expects `/dev/binder`; twoyi virtualises it via `kr64/src/binder.rs` plus a Java `BinderService` proxy.

**BootLogTexture** — twoyi's `TextureView`/`SurfaceView` subclass showing guest boot logs on-screen before SurfaceFlinger composites a real frame.

## C

**ColorBuffer** — emugl/AOSP term for a host-side GPU buffer backing a guest gralloc allocation. Managed by `app/rs/src/renderer_new/gralloc.rs`.

## E

**EGL** — Khronos Embedded-system Graphics Library; window-system glue between OpenGL ES and the native surface. The AOSP renderer links system `libEGL.so` directly.

**emugl** — The Android emulator's legacy OpenGL-over-pipe bridge. twoyi's `libOpenglRender_aosp.so` is built from emugl-derived AOSP sources.

## F

**FrameBuffer** — In twoyi, the guest's composited display output flowing through the QEMU pipe to the host renderer (not Linux `/dev/fb0`).

## G

**goldfish** — The Android emulator's virtual platform (`goldfish_pipe`, goldfish events). On Android 11 the guest uses `/dev/goldfish_pipe` as an alias for `qemu_pipe`.

**gralloc** — Graphics Memory Allocator HAL. twoyi emulates the `/dev/gb` and `/dev/gb2` gralloc char devices from `kr64/src/devices.rs`.

**GSI** — Generic System Image. A Treble-compliant `system.img` that boots on any Treble device; twoyi's eventual target rootfs (today ships pre-Treble 8.1 `rootfs.7z`).

## H

**HAL** — Hardware Abstraction Layer. Vendor module (audio, sensors, gralloc, …) with a stable C ABI. twoyi implements `audio`, `battery`, `sensors` HALs in Rust.

**HWC** — Hardware Composer. The HAL SurfaceFlinger uses to push final composition to the display. Not yet virtualised by twoyi.

## I

**init** — PID 1 of the Android userland. twoyi `fork`+`exec`s the guest's `/system/bin/init` inside the chroot-style data dir (`app/rs/src/core.rs`).

**INTERP (PT_INTERP)** — ELF program header naming the dynamic linker. VM's `libkr64.so` sets `PT_INTERP` to a custom `libkrloader64.so`; twoyi mirrors via `kr64/interp.c`.

## J

**JNI** — Java Native Interface. The FFI between Java/Kotlin and native code. `renderer_bindings.rs` exposes 6 C-ABI symbols consumed from Java.

## K

**kr64** — "kernel-replacement, 64-bit". twoyi's Rust daemon (`app/rs/kr64/`, ~11.6 kLOC across 11 `.rs` files, 165 tests) that materialises the virtual `/dev` tree, installs seccomp, emulates `/proc`. Analogue of VM's `libkr64.so`.

**KVM** — Kernel Virtual Machine (hardware-assisted virtualisation). twoyi does **not** use KVM — it shares the host kernel directly.

## L

**LD_PRELOAD** — Env var forcing the dynamic linker to load a `.so` before all others; the classic hook-injection vector VM uses to override libc/syscalls in guest `init`.

**linker64** — Bionic's 64-bit dynamic linker (`/system/bin/linker64`). VM replaces it with `libkrloader64.so`; twoyi mirrors the same scheme via `kr64/interp.c`.

## M

**Magisk** — Popular Android root + systemless-modification suite. Virtual Master bundles `magisk.zip` (decrypted by twoyi's recovered AES-128-ECB key `%z89aviCM0KkbEs9`).

## N

**NDK** — Native Development Kit. twoyi builds `libOpenglRender_aosp.so` with NDK r27c, clang 18.0.3, cmake 3.22.1.

## O

**opengles pipe** — The QEMU-pipe service selected by writing `/opengles3` (or `/opengles`, `/opengles2`) to `/dev/qemu_pipe`. SurfaceFlinger opens the pipe, writes this name, then streams GLES commands. Bridging it into the `RenderServer` is twoyi's critical next step.

## P

**PIE** — Position-Independent Executable (required by Android since API 21). `libkr64.so` is built as PIE but invoked as a standalone executable through its custom `PT_INTERP`.

**QEMU pipe** — The Android emulator's high-bandwidth guest↔host channel (`/dev/qemu_pipe`). twoyi repurposes it as the GL command transport; kr64 owns the listener socket.

## R

**Render2Activity** — twoyi's `Activity` hosting the rendering `SurfaceView`/`TextureView` plus the boot-log overlay.

**RenderServer** — Server side of AOSP `libOpenglRender.so`; accepts GL pipe clients and produces frames onto the `ANativeWindow`. The kr64 accept loop must hand accepted fds to it (see `TECHNICAL_BRIEFING.md` §5).

**rootfs** — Root filesystem the guest `init` is chrooted into. twoyi ships a pre-Treble 8.1 `rootfs.7z` today; GSI extraction (`system.img`, `product.img`, APEX) is planned.

## S

**seccomp** — Linux secure-computing BPF filter restricting syscalls. `kr64/src/seccomp.rs` installs a filter plus a `SIGSYS` handler that emulates forbidden calls (fake success) or kills the guest.

**shadowhook** — ByteDance's inline-hook library. Used by VM's `libkr64.so` to intercept libc calls without relinking; twoyi ports the same pattern in Rust.

**SurfaceFlinger** — Android's display compositor system service. Opens `/dev/qemu_pipe` and emits the GL stream twoyi's host renderer consumes.

**SurfaceView** — Android `View` creating a separate surface composited independently of the view hierarchy. Hosts twoyi's rendered frame.

**TextureView** — Android `View` rendering a GPU texture inside the normal view hierarchy. `BootLogTexture` uses it for the boot-log overlay.

## T

**Treble** — Project Treble (Android 8+) separates vendor HALs from the framework via HIDL/AIDL so a single GSI boots on any compliant device. twoyi targets a Treble GSI on x86_64.

## V

**vndbinder** — Vendor-scoped binder instance (`/dev/vndbinder`) introduced by Treble for vendor↔vendor IPC. Currently out of scope for twoyi (only `/dev/binder` is proxied).

## X

**Xposed** — Framework hooking the Android runtime at the zygote level. Virtual Master bundles `xposed.zip`; twoyi recovered its AES key alongside Magisk's during ROM analysis.

## Z

**zygote** — Android's forking process that templates all app processes. Started by `init` from `init.rc`; runs inside the twoyi guest after `init` execs.

---

*For deeper context see `ARCHITECTURE.md`, `download/TECHNICAL_BRIEFING.md`,
`download/GSI_BOOT_PLAN.md`, and the `VM_*_ANALYSIS.md` reverse-engineering
notes.*
