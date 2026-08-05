# MEMORY.md
Project: twoyi fork at Disable-OP/twoyi, branch improvements/initial-cleanup
Codespace: twoyi-dev-3-jr47xg6xvx7ghq6p (EastUs, AMD EPYC, KVM works)
SSH: install gh CLI v2.50.0 + openssh-client, use nohup pattern
Emulator: android-30 google_apis x86_64, KVM accelerated
Rootfs: extracted from SDK system image, Android 11 x86_64
Renderer: AOSP emugl (Apache 2.0) with real EGL, built from app/cpp/emugl/
All blobs removed, 100% open source
kr64 daemon: 9581 lines, 144 tests

## INIT BOOT STATUS (latest test 12:40 UTC)
- loader64 launches init (PID 6776, parent twoyi 6703)
- init executes (SELinux grants execute)
- init loads HOST bootstrap libs (/system/lib64/bootstrap/libc.so on dm-4)
- init becomes zombie immediately — exits silently, no output
- Problem: init uses HOST linker, loads HOST /system, tries PID 1 ops, fails

## NEXT STEPS
1. Make init use ROOTFS libraries not HOST libraries
2. Options: patchelf INTERP, rootfs linker, or chroot/unshare
3. patchelf to /system/bin/linker64 caused binder crash
4. rootfs linker produced no output — investigate
5. REAL fix: unshare + bind-mount rootfs/system over /system before exec init

## SSH SETUP
1. Download gh CLI v2.50.0 to /home/z/my-project/.local/bin/
2. apt-get download openssh-client; extract to /home/z/my-project/.local/openssh/
3. Symlink ssh into .local/bin/
4. Set GH_TOKEN env var (ask user)
5. SSH: nohup gh cs ssh -c CS "CMD" > /tmp/out.txt 2>&1 &

## GIT GOTCHAS
- Secret scanning blocks pushes with token — NEVER commit token
- .gitignore blocks libtwoyi.so — use git add -f
- Use filter-branch to remove secrets from history
