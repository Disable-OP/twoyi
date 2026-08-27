# Why is there an entire AOSP `init` in this repo?

This directory is a **read-only reference copy** of AOSP's `system/core/init`
(Android 12 era). It is **not compiled, not shipped, and not executed** —
nothing in `app/cpp/build.sh` (the only C/C++ build entry point) touches it.

## Why it exists

Twoyi's tracer (`app/rs/kr64`) impersonates the Linux kernel for a guest
Android system. To do that faithfully, the tracer must reproduce the exact
kernel behavior that AOSP `init` depends on — property-area layouts, .prop
load order, capability dropping, socket/binder device setup, service
reaping, and so on. When we emulate one of those behaviors, the Rust code
cites the exact AOSP source file and line it was derived from, e.g.:

- `app/rs/kr64/src/proc_emu.rs` cites `property_service.cpp:891`
  (init's fixed `.prop` load list — which files `PropertyLoadBootDefaults()`
  reads, so we know where to drop `ro.hardware=goldfish`).
- `app/rs/kr64/src/ptrace_emu.rs` cites `capabilities.cpp`
  (init's capability handling during service fork).

Having the tree vendored means those citations can be verified with a local
grep instead of cloning all of AOSP. It previously lived under `app/cpp/`,
which wrongly suggested it was part of the app build — hence the move here.

## Origin & license

Copied from AOSP `system/core/init`; Apache 2.0 (see `NOTICE`,
`MODULE_LICENSE_APACHE2`). Do not build or modify it; if a citation rots
after an AOSP upgrade, re-vendor the new tree in one commit.
