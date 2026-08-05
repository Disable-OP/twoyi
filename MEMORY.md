# MEMORY.md
Project: twoyi fork at Disable-OP/twoyi, branch improvements/initial-cleanup
Codespace: twoyi-dev-3-jr47xg6xvx7ghq6p (EastUs, AMD EPYC, KVM works)
SSH: install gh CLI + openssh-client, use nohup pattern
Emulator: android-30 google_apis x86_64, KVM accelerated
Rootfs: extracted from SDK system image, Android 11 x86_64
Renderer: AOSP emugl (Apache 2.0) with real EGL, built from app/cpp/emugl/
All blobs removed, 100% open source
kr64 daemon: 9581 lines, 144 tests
Init problem: INTERP points to host linker, need loader64 approach
Next: fix core.rs (add init_path var), rebuild, test
