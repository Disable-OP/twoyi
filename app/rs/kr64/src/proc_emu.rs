// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of
// merchantability, fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://www.mozilla.org/MPL/2.0/.

//! `/proc` emulator — synthesises the per-VM `/proc` tree.
//!
//! This mirrors what VM's `libkr64.so` does at the `/dev/vmproc` entry
//! point (see `VM_KR64_ANALYSIS.md` §2.8 and §4.2 — the `vmproc` device
//! decoded with keys 0x47/0x64/0x07):
//!
//! * Intercept `open("/proc/…")` calls (via shadowhook on `open` /
//!   `openat`).
//! * When the path matches one of the synthesised patterns, return a
//!   fake file descriptor that yields the synthesised content.
//!
//! For the MVP we don't yet shadowhook `open` — instead we just write
//! the synthesised files into a tmpfs mounted at `{rootfs}/proc` so
//! the guest's `open("/proc/…")` reads them directly. This works for
//! the static files (`/proc/version`, `/proc/cpuinfo`, `/proc/meminfo`,
//! `/proc/cmdline`). Dynamic files (`/proc/self/maps`,
//! `/proc/self/status`, `/proc/<pid>/…`) require the shadowhook
//! interception and will be added in a follow-up task.
//!
//! # Synthesised content (MVP)
//!
//! | Path                  | Content                                                    |
//! |-----------------------|------------------------------------------------------------|
//! | `/proc/version`       | `Linux version 4.14.x …` (matches GSI's expected kernel)   |
//! | `/proc/cpuinfo`       | 8× "processor : N" blocks with reasonable ARMv8 / x86-64 fields |
//! | `/proc/meminfo`       | `MemTotal: 4 GB`, `MemFree: 1 GB`, etc.                    |
//! | `/proc/cmdline`       | `androidboot.hardware=… androidboot.bootdevice=…`          |
//! | `/proc/self/`         | Symlinks to `/proc/<current_pid>/`                         |
//! | `/proc/mounts`        | Symlink to `/proc/self/mounts` (which we synthesise)       |
//! | `/proc/self/mounts`   | Static list of the guest's mounts                          |
//!
//! # What's NOT here yet
//!
//! The full VM `/proc` emulator synthesises ~20 paths (see
//! `GSI_BOOT_PLAN.md` §3.5 and `VM_KR64_ANALYSIS.md` §2.8):
//! `/proc/self/maps`, `/proc/self/status`, `/proc/self/exe`,
//! `/proc/self/fd/%d`, `/proc/%d/%s`, `/proc/exe_%d`, `/proc/mnt_points`,
//! `/proc/net/if_inet6/`, `/proc/sys/kernel/kptr_restrict`,
//! `/proc/sys/vm/mmap_rnd_bits`, etc. These will be added in
//! follow-up tasks (KR64-IMPL-2 etc.).

use std::fs;
use std::io::Write;
use std::path::Path;

// Crate-local logging macros (defined in lib.rs) — no external `log` crate.
use crate::info;
use crate::warning;

/// Synthesise the MVP `/proc` tree at `{rootfs}/proc`.
///
/// `cpu_count` is the number of CPU entries to fake in `/proc/cpuinfo`
/// (default 8 — matches a typical modern Android device). `mem_mb` is
/// the total memory in MB to fake in `/proc/meminfo` (default 4096).
pub fn populate_proc(rootfs: &str, cpu_count: u32, mem_mb: u64) -> std::io::Result<()> {
    let proc_dir = format!("{}/proc", rootfs);
    fs::create_dir_all(&proc_dir)?;

    // Idempotency: a previous populate_proc() call leaves /proc at mode
    // 0o555 (read-only — see the chmod at the end of this function). On
    // re-run that would make every write_file() below fail with EACCES
    // (you can't create/replace files in a read-only directory). Restore
    // the writable mode up-front so re-runs behave like first runs.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&proc_dir, fs::Permissions::from_mode(0o755));
    }

    // NOTE: the chmod to 0o555 (read-only) MUST happen AFTER all the
    // files are written, otherwise the write_file() calls below fail
    // with EACCES (you can't create files in a read-only directory).
    // We do the chmod at the very end of this function.

    write_proc_version(&proc_dir)?;
    write_proc_cpuinfo(&proc_dir, cpu_count)?;
    write_proc_meminfo(&proc_dir, mem_mb)?;
    write_proc_cmdline(&proc_dir)?;
    write_proc_mounts(&proc_dir)?;
    write_proc_self(&proc_dir)?;
    write_proc_sys(&proc_dir)?;

    // ro.vm.* system properties — written to a dedicated prop file under
    // the rootfs's /system/etc so the guest's property loader picks them
    // up. Lives in proc_emu.rs (alongside the other synthesised metadata)
    // because it's part of the same "fake the VM environment" pass. See
    // `write_proc_vm_properties` below.
    if let Err(e) = write_proc_vm_properties(rootfs, cpu_count, mem_mb) {
        // Non-fatal: a missing ro.vm.* prop file doesn't block boot —
        // guest apps that read these just see "undefined" and fall back
        // to defaults. Log so the operator knows why the props are absent.
        info!(
            "[KR64][proc_emu] warning: failed to write ro.vm.* props: {}",
            e
        );
    }

    // Boot-critical preset properties — appended to /system/build.prop so
    // init's PropertyLoadBootDefaults() actually loads them. The key
    // property is apexd.status=activated, which unblocks the
    // `wait_for_prop apexd.status activated` at init.rc:763. Without
    // this, init busy-loops forever on that wait (because the loader's
    // __system_property_wait_any hook returns immediately, so a missing
    // property = infinite spin) and never reaches the zygote-start
    // trigger. See `write_boot_preset_properties` above for full details.
    if let Err(e) = write_boot_preset_properties(rootfs) {
        // Non-fatal in the sense that kr64 can still fork init, but init
        // will almost certainly busy-loop at wait_for_prop. Log loudly.
        warning!(
            "[KR64][proc_emu] warning: failed to write boot preset properties: {} -- \
             init will likely stall at wait_for_prop apexd.status activated",
            e
        );
    }

    // vendor/default.prop — read by init's PropertyLoadBootDefaults()
    // (app/cpp/init/property_service.cpp:891 — /vendor/default.prop is
    // one of the FIXED list of .prop files init loads, alongside
    // /system/build.prop, /vendor/build.prop, etc.). The CRITICAL key
    // is ro.hardware=goldfish which tells init to load AOSP Goldfish
    // virtualization HALs (audio.primary.goldfish.so, gralloc.goldfish.so,
    // etc.) that tolerate missing real hardware — essential for a
    // containerized guest that has no real device drivers.
    //
    // Adopted from Nogitsune's ensureVendorDefaultProp (BootHelper.kt:191-206).
    //
    // This runs for BOTH TWRP and Android modes (no boot_recovery gating)
    // because populate_proc itself is called for both modes (lib.rs:2206
    // has no TWRP-mode skip — TWRP's init.rc reads /proc/* just like
    // Android's, and TWRP's recovery service probes for HALs via
    // libminuitwrp.so where ro.hardware=goldfish is also useful).
    //
    // TODO: wire lcd_density from cfg.dpi (currently hardcoded 320 —
    // matching Config::default().dpi). The locale/timezone defaults
    // ("en"/"US"/"America/New_York") match Nogitsune's defaults; a
    // follow-up should detect them from the host Locale/TimeZone.
    // populate_proc's signature would need to change to accept these
    // (or write_vendor_default_prop would need to be called separately
    // from lib.rs with cfg.dpi — but lib.rs is owned by another sub-agent
    // and is off-limits per the ground rules, so we hardcode here for now).
    if let Err(e) = write_vendor_default_prop(
        rootfs,
        /* lcd_density   */ 320,
        /* language      */ "en",
        /* country       */ "US",
        /* timezone      */ "America/New_York",
    ) {
        // Non-fatal in the sense that kr64 can still fork init, but the
        // guest will fall back to the build-time ro.hardware (e.g. ranchu)
        // whose HALs may not exist in the container's /vendor/lib64/hw/,
        // causing HAL-load failures during boot. Log loudly.
        warning!(
            "[KR64][proc_emu] warning: failed to write vendor/default.prop: {} -- \
             init may fail to load goldfish virtualization HALs",
            e
        );
    }

    // Now that all files are in place, mark /proc read-only (matching
    // the kernel's behaviour — `/proc` is mounted with mode 0555).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&proc_dir, fs::Permissions::from_mode(0o555));
    }

    info!(
        "[KR64][proc_emu] populated {}/proc with synthesised files (cpu_count={}, mem_mb={})",
        proc_dir, cpu_count, mem_mb
    );
    Ok(())
}

/// `/proc/version` — `Linux version 4.14.x …` matching the GSI's
/// expected kernel.
///
/// The string format is documented in `man 5 proc`:
/// ```text
/// Linux version <release> (<who@host>) (<compiler>) <version>
/// ```
///
/// We pick `4.14.190-g45619c7d3dc8-ab7891234` because:
///   - 4.14 is the Android 11 kernel ABI (per GSI_BOOT_PLAN.md §1.3)
///   - The `g<sha1>` suffix is the AOSP git commit
///   - The `-ab<digits>` suffix is the Android Build number
fn write_proc_version(proc_dir: &str) -> std::io::Result<()> {
    let content = concat!(
        "Linux version 4.14.190-g45619c7d3dc8-ab7891234 ",
        "(build-user@build-host) (Android clang 11.0.5) ",
        "4.14.190-g45619c7d3dc8-ab7891234 #1 SMP PREEMPT ",
        "Mon Jan 01 00:00:00 UTC 2026 (aarch64)\n",
    );
    write_file(proc_dir, "version", content)
}

/// `/proc/cpuinfo` — `cpu_count` blocks of "processor : N" entries
/// with reasonable ARMv8 / x86-64 fields.
///
/// We synthesise architecture-appropriate content via cfg():
///   - aarch64 → ARMv8 fields (CPU implementer, architecture, variant,
///     part, revision, BogoMIPS, Features).
///   - x86_64  → x86-64 fields (vendor_id, cpu family, model, model
///     name, stepping, cpu MHz, cache size, flags).
fn write_proc_cpuinfo(proc_dir: &str, cpu_count: u32) -> std::io::Result<()> {
    let mut content = String::new();
    for i in 0..cpu_count {
        #[cfg(target_arch = "aarch64")]
        {
            content.push_str(&format!(
                "processor\t: {}\n\
                 BogoMIPS\t: 200.00\n\
                 Features\t: fp asimd evtstrm aes pmull sha1 sha2 crc32 atomics fphp asimdhp cpuid asimdrdm jscvt fcma lrcpc dcpop sha3 sm3 sm4 asimddp sha512 sve asimdfhm dit uscat ilrcpc flagm ssbs sb paca pacg dcpodp flagm2 frint\n\
                 CPU implementer\t: 0x51\n\
                 CPU architecture: 8\n\
                 CPU variant\t: 0xc\n\
                 CPU part\t: 0x805\n\
                 CPU revision\t: 14\n\n",
                i,
            ));
        }
        #[cfg(target_arch = "x86_64")]
        {
            content.push_str(&format!(
                "processor\t: {}\n\
                 vendor_id\t: GenuineIntel\n\
                 cpu family\t: 6\n\
                 model\t\t: 85\n\
                 model name\t: Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz\n\
                 stepping\t: 7\n\
                 microcode\t: 0x1\n\
                 cpu MHz\t\t: 2793.438\n\
                 cache size\t: 49152 KB\n\
                 physical id\t: 0\n\
                 siblings\t: {}\n\
                 core id\t\t: {}\n\
                 cpu cores\t: {}\n\
                 apicid\t\t: {}\n\
                 initial apicid\t: {}\n\
                 fpu\t\t: yes\n\
                 fpu_exception\t: yes\n\
                 cpuid level\t: 27\n\
                 wp\t\t: yes\n\
                 flags\t\t: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ss ht syscall nx pdpe1gb rdtscp lm constant_tsc rep_good nopl xtopology nonstop_tsc cpuid aperfmperf tsc_known_freq pni pclmulqdq monitor ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm abm 3dnowprefetch cpuid_fault ssbd ibrs ibpb stibp ibrs_enhanced fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid mpx avx512f avx512dq rdseed adx smap clflushopt clwb avx512cd avx512bw avx512vl xsaveopt xsavec xgetbv1 xsaves arat umip pku ospke avx512_vnni md_clear arch_capabilities\n\
                 bugs\t\t: spectre_v1 spectre_v2 spec_store_bypass swapgs\n\
                 bogomips\t: 5586.87\n\
                 clflush size\t: 64\n\
                 cache_alignment\t: 64\n\
                 address sizes\t: 46 bits physical, 48 bits virtual\n\
                 power management:\n\n",
                i, cpu_count, i, cpu_count, i, i,
            ));
        }
        // For other architectures (e.g. armv7), emit a minimal block.
        #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
        {
            content.push_str(&format!("processor\t: {}\n\n", i));
        }
    }
    write_file(proc_dir, "cpuinfo", &content)
}

/// `/proc/meminfo` — `MemTotal: mem_mb MB`, `MemFree: mem_mb/4 MB`, etc.
///
/// The format is documented in `man 5 proc`. We synthesise a
/// reasonable subset — the guest's `ActivityManagerService` reads
/// `MemTotal` to decide how aggressive to be about killing background
/// processes.
fn write_proc_meminfo(proc_dir: &str, mem_mb: u64) -> std::io::Result<()> {
    let free_mb = mem_mb / 4;
    let avail_mb = mem_mb / 2;
    let buffers_mb = mem_mb / 16;
    let cached_mb = mem_mb / 4;
    let content = format!(
        "MemTotal:       {:>8} kB\n\
         MemFree:        {:>8} kB\n\
         MemAvailable:   {:>8} kB\n\
         Buffers:        {:>8} kB\n\
         Cached:         {:>8} kB\n\
         SwapCached:            0 kB\n\
         Active:         {:>8} kB\n\
         Inactive:       {:>8} kB\n\
         Active(anon):   {:>8} kB\n\
         Inactive(anon): {:>8} kB\n\
         Active(file):   {:>8} kB\n\
         Inactive(file): {:>8} kB\n\
         Unevictable:        1024 kB\n\
         Mlocked:            1024 kB\n\
         SwapTotal:            0 kB\n\
         SwapFree:             0 kB\n\
         Dirty:                0 kB\n\
         Writeback:            0 kB\n\
         AnonPages:      {:>8} kB\n\
         Mapped:         {:>8} kB\n\
         Shmem:          {:>8} kB\n\
         KReclaimable:   {:>8} kB\n\
         Slab:           {:>8} kB\n\
         SReclaimable:   {:>8} kB\n\
         SUnreclaim:     {:>8} kB\n\
         KernelStack:       16384 kB\n\
         PageTables:        32768 kB\n\
         NFS_Unstable:        0 kB\n\
         Bounce:              0 kB\n\
         WritebackTmp:        0 kB\n\
         CommitLimit:   {:>8} kB\n\
         Committed_AS:  {:>8} kB\n\
         VmallocTotal:  {:>8} kB\n\
         VmallocUsed:   {:>8} kB\n\
         VmallocChunk:  {:>8} kB\n\
         Percpu:           1024 kB\n\
         HardwareCorrupted:   0 kB\n\
         AnonHugePages:        0 kB\n\
         ShmemHugePages:       0 kB\n\
         ShmemPmdMapped:       0 kB\n\
         FileHugePages:        0 kB\n\
         FilePmdMapped:        0 kB\n\
         CmaTotal:       {:>8} kB\n\
         CmaFree:        {:>8} kB\n\
         HugePages_Total:     0\n\
         HugePages_Free:      0\n\
         HugePages_Rsvd:      0\n\
         HugePages_Surp:      0\n\
         Hugepagesize:      2048 kB\n\
         Hugetlb:              0 kB\n\
         DirectMap4k:    {:>8} kB\n\
         DirectMap2M:    {:>8} kB\n\
         DirectMap1G:    {:>8} kB\n",
        mem_mb * 1024,
        free_mb * 1024,
        avail_mb * 1024,
        buffers_mb * 1024,
        cached_mb * 1024,
        avail_mb * 1024,
        cached_mb * 1024,
        avail_mb * 1024 / 2,
        cached_mb * 1024 / 2,
        avail_mb * 1024 / 2,
        cached_mb * 1024 / 2,
        avail_mb * 1024 / 2,
        avail_mb * 1024 / 2,
        avail_mb * 1024 / 2,
        cached_mb * 1024 / 2,
        cached_mb * 1024 / 2,
        cached_mb * 1024 / 2,
        cached_mb * 1024 / 4,  // SUnreclaim
        mem_mb * 1024,         // CommitLimit
        avail_mb * 1024,       // Committed_AS
        536_870_912,           // VmallocTotal (512 GB)
        cached_mb * 1024 / 16, // VmallocUsed
        536_870_912,           // VmallocChunk
        mem_mb * 1024 / 16,    // CmaTotal
        mem_mb * 1024 / 32,    // CmaFree
        mem_mb * 1024 / 8,     // DirectMap4k
        mem_mb * 1024,         // DirectMap2M
        mem_mb * 1024 / 2,     // DirectMap1G
    );
    write_file(proc_dir, "meminfo", &content)
}

/// `/proc/cmdline` — `androidboot.hardware=… androidboot.bootdevice=…`.
///
/// The guest's `init` reads this to learn:
///   - `androidboot.hardware` → which `/system/etc/init/hw/init.<hw>.rc` to load
///   - `androidboot.bootdevice` → where the boot partition is
///   - `androidboot.serialno` → device serial number
///   - `androidboot.boot_slots_suffix` → A/B slot suffix
///   - `androidboot.verifiedbootstate` → "green" for verified boot
///
/// See `system/core/init/README.md` in AOSP for the full list.
fn write_proc_cmdline(proc_dir: &str) -> std::io::Result<()> {
    // Use a Treble-style hardware name. The GSI expects `androidboot.hardware=twoyi`
    // — we patch the guest's `init.rc` to look for `/system/etc/init/hw/init.twoyi.rc`
    // (which we ship as part of the ROM patches).
    let content = concat!(
        "androidboot.hardware=twoyi ",
        "androidboot.bootdevice=virtual ",
        "androidboot.boot_slots_suffix=_a ",
        "androidboot.serialno=twoyi0001 ",
        "androidboot.verifiedbootstate=green ",
        "androidboot.space=false ",
        "androidboot.mode=normal ",
        "androidboot.vbmeta.device=virtual ",
        "androidboot.vbmeta.avb_version=1.2 ",
        "androidboot.baseband=unknown ",
        "androidboot.kerneltype=twoyi ",
        "androidboot.bootreason=kernel-replacement ",
        // CPU ABI info — required by Zygote to decide which zygote to start.
        // Without these, Zygote cannot boot.
        "androidboot.product.cpu.abi=arm64-v8a ",
        "androidboot.product.cpu.abilist=arm64-v8a,armeabi-v7a,armeabi ",
        "androidboot.product.cpu.abilist64=arm64-v8a ",
        "androidboot.product.cpu.abilist32=armeabi-v7a,armeabi ",
        // Additional boot params Android expects
        "androidboot.veritymode=enforcing ",
        "androidboot.fstab_suffix=default ",
        "androidboot.zygote=zygote64_32 ",
        "kpti=off ",
        "ssbd=force-off ",
        "rcu_nocbs=0-7 ",
        "rw",
        "\n",
    );
    write_file(proc_dir, "cmdline", content)
}

/// `/proc/mounts` and `/proc/self/mounts` — list of the guest's mounts.
///
/// Format: `<device> <mountpoint> <fstype> <options> <dump> <pass>`
fn write_proc_mounts(proc_dir: &str) -> std::io::Result<()> {
    let content = concat!(
        "rootfs / rootfs rw,relatime 0 0\n",
        "/dev/root / ext4 rw,relatime 0 0\n",
        "tmpfs /dev tmpfs rw,seclabel,nosuid,relatime,mode=755 0 0\n",
        "proc /proc proc rw,relatime,gid=3009,hidepid=invisible 0 0\n",
        "sysfs /sys sysfs rw,seclabel,relatime 0 0\n",
        "/dev/system /system ext4 ro,seclabel,relatime 0 0\n",
        "/dev/vendor /vendor ext4 ro,seclabel,relatime 0 0\n",
        "/dev/product /product ext4 ro,seclabel,relatime 0 0\n",
        "/dev/system_ext /system_ext ext4 ro,seclabel,relatime 0 0\n",
        "/dev/data /data ext4 rw,seclabel,nosuid,nodev,noatime 0 0\n",
        "/dev/cache /cache ext4 rw,seclabel,nosuid,nodev,noatime 0 0\n",
        "tmpfs /apex tmpfs ro,seclabel,nosuid,nodev,noexec,relatime,mode=755 0 0\n",
        "tmpfs /mnt tmpfs rw,seclabel,nosuid,nodev,noexec,relatime,mode=755,gid=1000 0 0\n",
        "tmpfs /storage tmpfs rw,seclabel,nosuid,nodev,noexec,relatime,mode=755,gid=1000 0 0\n",
    );
    write_file(proc_dir, "mounts", content)?;

    // /proc/self/mounts — same content as /proc/mounts.
    // Use write_file (not fs::File::create) so the file gets the same
    // 0o444 mode + idempotency-chmod-to-writable-on-re-run treatment
    // as every other /proc file. Without this, the second populate_proc
    // call dies with EACCES here: write_file() above chmods the symlink
    // TARGET (which is /proc/self/mounts) to 0o444, then this raw
    // File::create can't open it.
    let self_dir = format!("{}/self", proc_dir);
    fs::create_dir_all(&self_dir)?;
    write_file(&self_dir, "mounts", content)?;

    // /proc/mounts → symlink to /proc/self/mounts (kernel convention).
    let _ = fs::remove_file(format!("{}/mounts", proc_dir));
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let _ = symlink("self/mounts", format!("{}/mounts", proc_dir));
    }
    Ok(())
}

/// `/proc/self/` — symlinks to the per-VM process view.
///
/// For the MVP we just create a `/proc/self` symlink that points to
/// `/proc/<pid>` (using the daemon's own pid as a placeholder). The
/// production version will dynamically resolve `self` per-process.
fn write_proc_self(proc_dir: &str) -> std::io::Result<()> {
    let pid = std::process::id();
    let self_dir = format!("{}/self", proc_dir);
    fs::create_dir_all(&self_dir)?;

    // /proc/self/exe → /system/bin/init (the guest's init binary).
    // /proc/self/cwd → / (init's working directory is /, not the binary path)
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let _ = symlink("/system/bin/init", format!("{}/exe", self_dir));
        let _ = symlink("/", format!("{}/cwd", self_dir));
        // Note: /proc/self/.. is a reserved directory entry — symlink() would
        // fail with EEXIST. The kernel resolves ".." via the actual parent
        // directory, so this is unnecessary. Removed.
    }

    // /proc/self/status — minimal content. Use `write_file` (NOT
    // `fs::File::create` + `write_all`) so the file gets the same
    // 0o444 read-only mode + idempotency-chmod-to-writable-on-re-run
    // treatment as every other `/proc` file. Without this, the file
    // was created at the default 0o644 (writable by owner), which:
    //   (a) is inconsistent with the kernel convention (`/proc` files
    //       are mode 0444 — the kernel synthesises them read-only), and
    //   (b) would let a misbehaving guest process `open()` it for
    //       writing and corrupt the synthesised status, which other
    //       guest processes reading `/proc/self/status` would then see.
    let status_content = format!(
        "Name:\tinit\n\
         Umask:\t0022\n\
         State:\tS (sleeping)\n\
         Tgid:\t{}\n\
         Ngid:\t0\n\
         Pid:\t{}\n\
         PPid:\t1\n\
         TracerPid:\t0\n\
         Uid:\t0\t0\t0\t0\n\
         Gid:\t0\t0\t0\t0\n\
         FDSize:\t256\n\
         Groups:\n\
         VmPeak:\t   10000 kB\n\
         VmSize:\t    8000 kB\n\
         VmLck:\t        0 kB\n\
         VmPin:\t        0 kB\n\
         VmHWM:\t    4000 kB\n\
         VmRSS:\t    4000 kB\n\
         VmData:\t    2000 kB\n\
         VmStk:\t      132 kB\n\
         VmExe:\t     1024 kB\n\
         VmLib:\t    4000 kB\n\
         VmPTE:\t       64 kB\n\
         VmSwap:\t        0 kB\n\
         Threads:\t1\n\
         SigQ:\t0/256\n\
         SigPnd:\t0000000000000000\n\
         ShdPnd:\t0000000000000000\n\
         SigBlk:\t0000000000000000\n\
         SigIgn:\t0000000000000000\n\
         SigCgt:\t0000000180000000\n\
         CapInh:\t0000000000000000\n\
         CapPrm:\t000001ffffffffff\n\
         CapEff:\t000001ffffffffff\n\
         CapBnd:\t000001ffffffffff\n\
         CapAmb:\t0000000000000000\n\
         Seccomp:\t0\n\
         Cpus_allowed:\tff\n\
         Cpus_allowed_list:\t0-7\n",
        pid, pid,
    );
    write_file(&self_dir, "status", &status_content)?;

    Ok(())
}

/// `/proc/sys/kernel/*` and `/proc/sys/vm/*` — kernel-hardening sysctls
/// that Android 11+ checks at boot.
///
/// See `VM_KR64_ANALYSIS.md` §4.3 (key 0xbe): the A 11 variant
/// synthesises `/proc/sys/kernel/kptr_restrict = 1` and
/// `/proc/sys/vm/mmap_rnd_bits = 16` so the guest passes its boot
/// hardening checks.
fn write_proc_sys(proc_dir: &str) -> std::io::Result<()> {
    let kernel_dir = format!("{}/sys/kernel", proc_dir);
    let vm_dir = format!("{}/sys/vm", proc_dir);
    fs::create_dir_all(&kernel_dir)?;
    fs::create_dir_all(&vm_dir)?;

    write_file(&kernel_dir, "kptr_restrict", "1\n")?;
    write_file(&kernel_dir, "dmesg_restrict", "1\n")?;
    write_file(&kernel_dir, "ngroups_max", "65536\n")?;
    write_file(&kernel_dir, "hostname", "twoyi\n")?;
    write_file(&kernel_dir, "domainname", "localdomain\n")?;
    write_file(
        &kernel_dir,
        "osrelease",
        "4.14.190-g45619c7d3dc8-ab7891234\n",
    )?;
    write_file(&kernel_dir, "ostype", "Linux\n")?;

    write_file(&vm_dir, "mmap_rnd_bits", "16\n")?;
    write_file(&vm_dir, "mmap_rnd_compat_bits", "16\n")?;
    write_file(&vm_dir, "overcommit_memory", "1\n")?;
    write_file(&vm_dir, "overcommit_ratio", "50\n")?;
    write_file(&vm_dir, "swappiness", "60\n")?;
    write_file(&vm_dir, "max_map_count", "65536\n")?;

    Ok(())
}

/// Write the `ro.vm.*` system properties to `{rootfs}/system/etc/ro.vm.prop`.
///
/// This mirrors VM's `VMPropSetter` (see `VM_KR64_ANALYSIS.md` §2.9) which
/// injects a set of `ro.vm.*` properties into the guest's property service
/// so guest apps and framework code can detect that they're running inside
/// a VM (and query its capabilities). The guest's `init` loads `.prop`
/// files from `/system/etc/` during early boot via the
/// `load_system_props()` call in `system/core/init/property_service.cpp`,
/// so dropping a file here is the canonical way to add new `ro.` props.
///
/// # Properties written
///
/// | Property                    | Value                                  |
/// |-----------------------------|----------------------------------------|
/// | `ro.vm.id`                  | the per-VM id (currently 0)            |
/// | `ro.vm.name`                | "twoyi" (the runtime identifier)       |
/// | `ro.vm.runtime`             | "twoyi" (vs "qemu"/"gce" for others)   |
/// | `ro.vm.runtime_version`     | crate version (env `CARGO_PKG_VERSION`)|
/// | `ro.vm.hypervisor`          | "twoyi-kr64" (our kernel-replacement)  |
/// | `ro.vm.cpu_count`           | synthesised CPU count                  |
/// | `ro.vm.memory_mb`           | synthesised memory size in MB          |
/// | `ro.vm.arch`                | host arch (aarch64 / x86_64 / ...)     |
/// | `ro.vm.gpu.enabled`         | "1"                                    |
/// | `ro.vm.gpu.renderer`        | "emugl" (AOSP emugl backend)           |
/// | `ro.vm.audio.enabled`       | "1"                                    |
/// | `ro.vm.sensors.enabled`     | "1"                                    |
/// | `ro.vm.battery.enabled`     | "1"                                    |
/// | `ro.vm.rootfs`              | guest-visible rootfs marker ("/")      |
///
/// # Why here (and not in a separate `props` module)
///
/// The proc_emu module is already responsible for "synthesise the
/// environment the guest sees", which is exactly what these props do —
/// they're the system-property analogue of the synthesised `/proc` files.
/// Keeping them together avoids a tiny one-function module.
pub fn write_proc_vm_properties(rootfs: &str, cpu_count: u32, mem_mb: u64) -> std::io::Result<()> {
    let etc_dir = format!("{}/system/etc", rootfs);
    fs::create_dir_all(&etc_dir)?;

    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "arm") {
        "armv7"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else {
        "unknown"
    };

    // env!("CARGO_PKG_VERSION") is the kr64 crate version, baked in at
    // compile time. This is what the guest's `getprop ro.vm.runtime_version`
    // returns — apps can compare against it to feature-detect.
    let runtime_version = env!("CARGO_PKG_VERSION");

    let content = format!(
        concat!(
            "# Auto-generated by twoyi kr64 — do not edit.\n",
            "# ro.vm.* properties describing the virtual machine.\n",
            "ro.vm.id=0\n",
            "ro.vm.name=twoyi\n",
            "ro.vm.runtime=twoyi\n",
            "ro.vm.runtime_version={ver}\n",
            "ro.vm.hypervisor=twoyi-kr64\n",
            "ro.vm.cpu_count={cpu}\n",
            "ro.vm.memory_mb={mem}\n",
            "ro.vm.arch={arch}\n",
            "ro.vm.gpu.enabled=1\n",
            "ro.vm.gpu.renderer=emugl\n",
            "ro.vm.audio.enabled=1\n",
            "ro.vm.sensors.enabled=1\n",
            "ro.vm.battery.enabled=1\n",
            "ro.vm.rootfs=/\n",
            "ro.vm.secure=1\n",
            "ro.vm.debuggable=0\n"
        ),
        ver = runtime_version,
        cpu = cpu_count,
        mem = mem_mb,
        arch = arch,
    );

    let prop_path = format!("{}/ro.vm.prop", etc_dir);
    fs::write(&prop_path, &content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&prop_path, fs::Permissions::from_mode(0o644));
    }
    info!(
        "[KR64][proc_emu] wrote ro.vm.* properties ({} bytes) to {}",
        content.len(),
        prop_path
    );
    Ok(())
}

/// Write boot-critical preset properties to `{rootfs}/system/build.prop`.
///
/// `PropertyLoadBootDefaults()` in `system/core/init/property_service.cpp`
/// loads a FIXED list of .prop files (it does NOT scan `/system/etc/`
/// generically — only `/system/etc/prop.default`, `/system/build.prop`,
/// `/vendor/build.prop`, `/product/build.prop`, etc.). The previously-used
/// `/system/etc/ro.vm.prop` is therefore NOT loaded by init's property
/// service; only `ro.vm.*` consumers that read the file directly (none in
/// AOSP 11) would see those values. To inject properties that init's
/// property service actually loads, we must append to one of the files in
/// `PropertyLoadBootDefaults`'s fixed list. `/system/build.prop` is the
/// safest target: it always exists in a standard Android rootfs and is
/// always loaded unconditionally.
///
/// # Properties injected
///
/// | Property             | Value        | Why                                                          |
/// |----------------------|--------------|--------------------------------------------------------------|
/// | `apexd.status`       | `activated`  | Unblocks `wait_for_prop apexd.status activated` at           |
/// |                      |              | init.rc:763. apexd exits 0 immediately in our container      |
/// |                      |              | ("This device does not support updatable APEX. Exiting")     |
/// |                      |              | and sets `apexd.status=activated` in its OWN per-process     |
/// |                      |              | property table (the loader's in-memory `g_props` is          |
/// |                      |              | per-process, not shared). Init's `wait_for_prop` busy-loops  |
/// |                      |              | forever because the loader's `__system_property_wait_any`    |
/// |                      |              | hook returns immediately, so a missing property = infinite   |
/// |                      |              | spin. Pre-setting it in init's table (via this .prop file)   |
/// |                      |              | lets `__system_property_find("apexd.status")` succeed and    |
/// |                      |              | `wait_for_prop` return immediately.                          |
///
/// # Why not also set `ro.crypto.state`?
///
/// The `on zygote-start && property:ro.crypto.state=unsupported` action
/// (init.rc:832) is what directly calls `start zygote`. However, previous
/// attempts to pre-set `ro.crypto.state` caused an SIGABRT regression
/// (see OVERNIGHT_PROGRESS.md ~04:15 and ~06:40 on 2026-08-11). The
/// SIGABRT is believed to be a downstream effect of `createProcessGroup`
/// failing for critical services started by the zygote-start action, not
/// a direct result of the property itself. Rather than re-trigger that
/// crash, we leave `ro.crypto.state` unset and rely on the loader's
/// existing pre-set of `vold.decrypt=trigger_restart_framework` (which
/// fires `class_start main` → starts zygote via a different path).
///
/// # Why append (not overwrite)?
///
/// `/system/build.prop` already contains `ro.build.*` and other standard
/// properties that the guest's framework expects. Overwriting would break
/// those. Appending is safe: `LoadProperties` in property_service.cpp
/// stores later entries in a `std::map` keyed by property name, and
/// `PropertySet` honours "last write wins" for non-`ro.` properties.
/// `apexd.status` is not `ro.`, so our appended value is the one that
/// sticks.
pub fn write_boot_preset_properties(rootfs: &str) -> std::io::Result<()> {
    let build_prop_path = format!("{}/system/build.prop", rootfs);

    // Read the existing /system/build.prop (if it exists) so we can append
    // rather than overwrite. If it doesn't exist (unlikely for a standard
    // Android rootfs, but possible for minimal test rootfs), create it.
    let existing = fs::read_to_string(&build_prop_path).unwrap_or_default();

    // Only append if the property isn't already present (idempotency: a
    // previous kr64 run may have already appended it). We check for the
    // exact line `apexd.status=activated` to avoid duplicate entries.
    let marker = "apexd.status=activated";
    if existing.contains(marker) {
        info!("[KR64][proc_emu] build.prop already contains apexd.status=activated — skip append");
        return Ok(());
    }

    // Make the file writable (it may be 0444 from a previous run or from
    // the rootfs extraction). Ignore errors — if we can't chmod, the
    // append below will fail with a clearer error.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&build_prop_path, fs::Permissions::from_mode(0o644));
    }

    // Build the append block. The leading newline ensures we don't
    // concatenate with the last line of the existing file. No `format!`
    // needed — there are no interpolation placeholders.
    let append_block = "\n\
         # --- twoyi kr64 boot preset properties ---\n\
         # Appended by kr64's write_boot_preset_properties() to unblock\n\
         # init's wait_for_prop apexd.status activated (init.rc:763).\n\
         # apexd exits 0 immediately in our container and sets this prop\n\
         # only in its own per-process table; init never sees it.\n\
         apexd.status=activated\n";

    // Open in append mode (creates the file if it doesn't exist).
    // `std::io::Write` is already imported at the top of this file.
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&build_prop_path)?;
    f.write_all(append_block.as_bytes())?;

    // Restore 0644 (writable by owner, readable by all — standard for
    // build.prop so init's property service can read it).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&build_prop_path, fs::Permissions::from_mode(0o644));
    }

    info!(
        "[KR64][proc_emu] appended apexd.status=activated to {} ({} bytes appended)",
        build_prop_path,
        append_block.len()
    );
    Ok(())
}

/// Write `{rootfs}/vendor/default.prop` with boot-critical properties.
///
/// These properties are read by init's `PropertyLoadBootDefaults()` (see
/// `app/cpp/init/property_service.cpp:891` — `/vendor/default.prop` is
/// one of the FIXED list of `.prop` files init loads, alongside
/// `/system/build.prop`, `/vendor/build.prop`, `/product/build.prop`,
/// etc.) before `/dev/__properties__` is fully populated, to configure
/// HALs and runtime.
///
/// The key property is `ro.hardware=goldfish` which tells init to load
/// the AOSP Goldfish virtualization HALs (`audio.primary.goldfish.so`,
/// `gralloc.goldfish.so`, etc.) that tolerate missing real hardware —
/// critical for a containerized guest that has no real device drivers.
/// The Goldfish HALs are the AOSP-shipped virtualization HALs originally
/// written for the QEMU Android emulator; they're the standard AOSP
/// mechanism for running Android in a virtualised environment.
///
/// Adopted from Nogitsune's `ensureVendorDefaultProp`
/// (`BootHelper.kt:191-206`) which writes the same 6 keys before init
/// runs. Nogitsune pulls `lcd_density` from the host
/// `DisplayMetrics.densityDpi` and `language`/`country`/`timezone` from
/// the host `Locale` / `TimeZone.getDefault()` — twoyi currently
/// hardcodes sensible defaults (see TODO below).
///
/// # Properties written
///
/// | Property                  | Value (default)            | Why                          |
/// |---------------------------|----------------------------|------------------------------|
/// | `persist.sys.language`    | `en`                       | Java Locale for the guest    |
/// | `persist.sys.country`     | `US`                       | Java Locale for the guest    |
/// | `persist.sys.timezone`   | `America/New_York`         | TimeZone.getDefault()       |
/// | `ro.sf.lcd_density`       | `320` (TODO: cfg.dpi)      | DisplayMetrics.densityDpi    |
/// | `ro.zygote`               | `zygote64`                 | Spawn the 64-bit zygote      |
/// | `ro.hardware`             | `goldfish` (CRITICAL)      | Triggers AOSP virtualization |
/// |                           |                            | HALs (audio.primary.goldfish,|
/// |                           |                            | gralloc.goldfish, etc.)     |
///
/// # Why this matters for both TWRP and Android modes
///
/// TWRP's recovery service (`recovery` binary forked+exec'd by init)
/// probes for HALs via `libminuitwrp.so` — without `ro.hardware=goldfish`,
/// it falls back to the build-time `ro.hardware` (e.g. `ranchu`), whose
/// HALs may not exist in the container's `/vendor/lib64/hw/`. Android
/// guest obviously needs the HAL trigger throughout boot. Hence we run
/// this for BOTH modes (no `boot_recovery` gating in `populate_proc`).
///
/// # Complement to 2-A's VFS work (commit `f720934`)
///
/// 2-A added a VFS module that serves `/dev/__properties__/properties_serial`
/// as a minimal AOSP `__system_property_area__` (128-byte header, 0
/// properties) so that `find_property()` iterates 0 entries and returns
/// NULL naturally — replacing the old binary-patch hack. THAT provides
/// the *property read area*; THIS function provides the *property values*
/// that init loads at boot via `PropertyLoadBootDefaults()` from the
/// on-disk `.prop` files. Together they give the guest a working
/// property service: init loads `ro.hardware=goldfish` from
/// `/vendor/default.prop` into its in-memory property table, and
/// subsequent `__system_property_find("ro.hardware")` calls (which go
/// through the VFS-served area) return that value.
pub fn write_vendor_default_prop(
    rootfs: &str,
    lcd_density: u32,
    language: &str,
    country: &str,
    timezone: &str,
) -> std::io::Result<()> {
    let vendor_dir = format!("{}/vendor", rootfs);
    fs::create_dir_all(&vendor_dir)?;

    let default_prop = format!("{}/default.prop", vendor_dir);

    // Idempotency: a previous run (or rootfs extraction) may have left
    // the file at mode 0o444. `fs::write` opens with O_WRONLY|O_CREAT|
    // O_TRUNC, which fails with EACCES on a read-only file even if you
    // own it. Restore the writable mode first — ignore errors (file may
    // not exist yet on the first run, in which case set_permissions
    // returns ENOENT).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&default_prop, fs::Permissions::from_mode(0o644));
    }

    // Build the file content. Use concat! + format! with named args
    // (matches the style of `write_proc_vm_properties` above) so the
    // property table is easy to scan visually.
    let content = format!(
        concat!(
            "# Generated by twoyi kr64 (proc_emu::write_vendor_default_prop)\n",
            "# Boot-critical properties for containerized Android/TWRP guest.\n",
            "# Adopted from Nogitsune's ensureVendorDefaultProp (BootHelper.kt:191-206).\n",
            "persist.sys.language={lang}\n",
            "persist.sys.country={country}\n",
            "persist.sys.timezone={tz}\n",
            "ro.sf.lcd_density={density}\n",
            "ro.zygote=zygote64\n",
            "ro.hardware=goldfish\n",
            "# goldfish triggers AOSP virtualization HALs (audio.primary.goldfish,\n",
            "# gralloc.goldfish, etc.) that tolerate missing real hardware.\n"
        ),
        lang = language,
        country = country,
        tz = timezone,
        density = lcd_density,
    );

    fs::write(&default_prop, &content)?;

    // Restore 0o644 (writable by owner, readable by all — standard for
    // .prop files so init's property service can read them).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&default_prop, fs::Permissions::from_mode(0o644));
    }

    info!(
        "[KR64][proc_emu] wrote {} ({} bytes) — ro.hardware=goldfish triggers AOSP virtualization HALs",
        default_prop,
        content.len()
    );
    Ok(())
}

/// Internal helper: write `content` to `{dir}/{name}` with mode 0444
/// (read-only — these are kernel-synthesised files).
fn write_file(dir: &str, name: &str, content: &str) -> std::io::Result<()> {
    let path = format!("{}/{}", dir, name);

    // Idempotency: a previous write_file() leaves the file at mode 0o444
    // (read-only). Re-opening with O_WRONLY (what fs::File::create does)
    // returns EACCES on a read-only file even if you own it. Restore the
    // writable mode first — ignore errors (file may not exist yet on the
    // first run, in which case set_permissions returns ENOENT).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o644));
    }

    let mut f = fs::File::create(&path)?;
    f.write_all(content.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o444));
    }
    // Sanity: make sure the file exists.
    debug_assert!(
        Path::new(&path).exists(),
        "proc_emu: failed to create {}",
        path
    );
    Ok(())
}

// ============================================================================
// Tests.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Each test gets a UNIQUE tmpdir (counter + pid) so parallel tests
    /// don't collide on the same path. Without this, `populate_proc`'s
    /// `chmod 0o555` on the proc dir would make the dir read-only for
    /// the next test that tries to write into it. The monotonic counter
    /// alone guarantees uniqueness within a single test process (it's
    /// the same pattern used by `battery.rs` / `sensors.rs` / `binder.rs`).
    fn tmpdir() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut p = std::env::temp_dir();
        p.push(format!("kr64-proc-test-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&p).unwrap();
        p.to_string_lossy().to_string()
    }

    #[test]
    fn populate_creates_all_files() {
        let rootfs = tmpdir();
        populate_proc(&rootfs, 4, 2048).expect("populate_proc");

        for name in &["version", "cpuinfo", "meminfo", "cmdline", "mounts"] {
            let p = format!("{}/proc/{}", rootfs, name);
            assert!(Path::new(&p).exists(), "missing {}", p);
        }
        for name in &["sys/kernel/kptr_restrict", "sys/vm/mmap_rnd_bits"] {
            let p = format!("{}/proc/{}", rootfs, name);
            assert!(Path::new(&p).exists(), "missing {}", p);
        }
        let _ = std::fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn proc_version_has_linux_prefix() {
        let rootfs = tmpdir();
        populate_proc(&rootfs, 1, 1024).unwrap();
        let v = std::fs::read_to_string(format!("{}/proc/version", rootfs)).unwrap();
        assert!(v.starts_with("Linux version 4.14."));
        let _ = std::fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn proc_cmdline_has_androidboot() {
        let rootfs = tmpdir();
        populate_proc(&rootfs, 1, 1024).unwrap();
        let c = std::fs::read_to_string(format!("{}/proc/cmdline", rootfs)).unwrap();
        assert!(c.contains("androidboot.hardware=twoyi"));
        assert!(c.contains("androidboot.bootdevice=virtual"));
        let _ = std::fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn proc_meminfo_has_memtotal() {
        let rootfs = tmpdir();
        populate_proc(&rootfs, 1, 2048).unwrap();
        let m = std::fs::read_to_string(format!("{}/proc/meminfo", rootfs)).unwrap();
        assert!(m.contains("MemTotal:"));
        assert!(m.contains("2097152 kB")); // 2048 MB * 1024 = 2097152 kB
        let _ = std::fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn proc_cpuinfo_has_n_processors() {
        let rootfs = tmpdir();
        populate_proc(&rootfs, 8, 1024).unwrap();
        let c = std::fs::read_to_string(format!("{}/proc/cpuinfo", rootfs)).unwrap();
        let count = c.matches("processor\t:").count();
        assert_eq!(count, 8);
        let _ = std::fs::remove_dir_all(&rootfs);
    }

    /// Regression test: populate_proc() ends by chmod-ing /proc to 0o555
    /// (read-only). A second call must restore 0o755 up-front, otherwise
    /// every write_file() inside it fails with EACCES.
    #[test]
    fn populate_proc_is_idempotent() {
        let rootfs = tmpdir();
        populate_proc(&rootfs, 2, 1024).expect("first populate_proc");
        // Second run on the same rootfs must not fail.
        populate_proc(&rootfs, 4, 2048).expect("second populate_proc");

        // And the content must reflect the second run (cpu_count=4).
        let c = std::fs::read_to_string(format!("{}/proc/cpuinfo", rootfs)).unwrap();
        assert_eq!(c.matches("processor\t:").count(), 4);
        let _ = std::fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn write_proc_vm_properties_writes_prop_file() {
        let rootfs = tmpdir();
        write_proc_vm_properties(&rootfs, 8, 4096).expect("write_proc_vm_properties");
        let prop_path = format!("{}/system/etc/ro.vm.prop", rootfs);
        assert!(Path::new(&prop_path).exists(), "ro.vm.prop should exist");
        let content = std::fs::read_to_string(&prop_path).unwrap();
        assert!(content.contains("ro.vm.id=0"));
        assert!(content.contains("ro.vm.name=twoyi"));
        assert!(content.contains("ro.vm.runtime=twoyi"));
        assert!(content.contains("ro.vm.hypervisor=twoyi-kr64"));
        assert!(content.contains("ro.vm.cpu_count=8"));
        assert!(content.contains("ro.vm.memory_mb=4096"));
        assert!(content.contains("ro.vm.gpu.renderer=emugl"));
        let _ = std::fs::remove_dir_all(&rootfs);
    }

    #[test]
    fn populate_proc_also_writes_ro_vm_props() {
        let rootfs = tmpdir();
        populate_proc(&rootfs, 4, 2048).expect("populate_proc");
        let prop_path = format!("{}/system/etc/ro.vm.prop", rootfs);
        assert!(
            Path::new(&prop_path).exists(),
            "populate_proc should also write ro.vm.prop"
        );
        let _ = std::fs::remove_dir_all(&rootfs);
    }

    /// Regression test: `/proc/self/status` must be created with mode
    /// 0o444 (read-only), matching the kernel convention and every
    /// other synthesised `/proc` file. Before the fix, `write_proc_self`
    /// used `fs::File::create` directly, which left the file at the
    /// default 0o644 (owner-writable) — inconsistent with the rest of
    /// `/proc` and a potential corruption vector if a guest process
    /// opened it for writing.
    #[test]
    fn proc_self_status_is_read_only_mode_0444() {
        let rootfs = tmpdir();
        populate_proc(&rootfs, 1, 1024).expect("populate_proc");

        let status_path = format!("{}/proc/self/status", rootfs);
        assert!(
            Path::new(&status_path).exists(),
            "/proc/self/status should exist"
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(&status_path).unwrap();
            let mode = meta.permissions().mode();
            assert_eq!(
                mode & 0o777,
                0o444,
                "/proc/self/status should be mode 0o444 (read-only), got 0o{:o}",
                mode
            );
        }

        // Also verify the content is non-empty and starts with "Name:".
        let content = std::fs::read_to_string(&status_path).unwrap();
        assert!(
            content.starts_with("Name:\t"),
            "/proc/self/status should start with Name:"
        );
        assert!(content.contains("VmRSS:\t    4000 kB"));

        let _ = std::fs::remove_dir_all(&rootfs);
    }

    /// Regression test: `/proc/self/status` must survive a re-run of
    /// `populate_proc` (idempotency). The `write_file` helper chmods
    /// the file to 0o644 before re-opening it for writing, then back
    /// to 0o444 — the previous `fs::File::create` approach didn't do
    /// this, but since the file was 0o644 it happened to work. This
    /// test guards the `write_file`-based path against future regressions.
    #[test]
    fn proc_self_status_survives_rerun() {
        let rootfs = tmpdir();
        populate_proc(&rootfs, 1, 1024).expect("first populate_proc");
        // The file is now mode 0o444 (read-only). A second run must
        // restore the writable mode, overwrite the content, and set
        // it back to 0o444 — without panicking or returning Err.
        populate_proc(&rootfs, 2, 2048).expect("second populate_proc");

        let status_path = format!("{}/proc/self/status", rootfs);
        let content = std::fs::read_to_string(&status_path).unwrap();
        assert!(content.starts_with("Name:\tinit\n"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&status_path)
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(
                mode & 0o777,
                0o444,
                "/proc/self/status should still be 0o444 after re-run, got 0o{:o}",
                mode
            );
        }

        let _ = std::fs::remove_dir_all(&rootfs);
    }

    /// Regression test: `write_boot_preset_properties` must append
    /// `apexd.status=activated` to `{rootfs}/system/build.prop`. Without
    /// this, init's `wait_for_prop apexd.status activated` (init.rc:763)
    /// busy-loops forever because the loader's
    /// `__system_property_wait_any` hook returns immediately, so a
    /// missing property = infinite spin.
    #[test]
    fn write_boot_preset_properties_appends_apexd_status() {
        let rootfs = tmpdir();

        // Pre-create /system/build.prop with some existing content to
        // verify we APPEND (not overwrite).
        let build_prop = format!("{}/system/build.prop", rootfs);
        std::fs::create_dir_all(format!("{}/system", rootfs)).unwrap();
        std::fs::write(&build_prop, "# existing build.prop\nro.build.id=TEST\n").unwrap();

        write_boot_preset_properties(&rootfs).expect("write_boot_preset_properties");

        let content = std::fs::read_to_string(&build_prop).unwrap();
        // Existing content preserved
        assert!(
            content.contains("ro.build.id=TEST"),
            "existing build.prop content should be preserved: {}",
            content
        );
        // New content appended
        assert!(
            content.contains("apexd.status=activated"),
            "apexd.status=activated should be appended: {}",
            content
        );

        let _ = std::fs::remove_dir_all(&rootfs);
    }

    /// Regression test: `write_boot_preset_properties` is idempotent —
    /// calling it twice on the same rootfs must NOT append the
    /// `apexd.status=activated` line a second time (which would be
    /// harmless but messy).
    #[test]
    fn write_boot_preset_properties_is_idempotent() {
        let rootfs = tmpdir();
        let build_prop = format!("{}/system/build.prop", rootfs);
        std::fs::create_dir_all(format!("{}/system", rootfs)).unwrap();
        std::fs::write(&build_prop, "# existing\n").unwrap();

        write_boot_preset_properties(&rootfs).expect("first call");
        write_boot_preset_properties(&rootfs).expect("second call");

        let content = std::fs::read_to_string(&build_prop).unwrap();
        // Exactly one occurrence of "apexd.status=activated"
        let count = content.matches("apexd.status=activated").count();
        assert_eq!(
            count, 1,
            "apexd.status=activated should appear exactly once (got {}): {}",
            count, content
        );

        let _ = std::fs::remove_dir_all(&rootfs);
    }

    /// Regression test: `populate_proc` must also invoke
    /// `write_boot_preset_properties`, so that the apexd.status preset
    /// is applied whenever the /proc tree is populated (the canonical
    /// kr64 setup path).
    #[test]
    fn populate_proc_also_writes_boot_preset_properties() {
        let rootfs = tmpdir();
        // Pre-create /system/build.prop so the append has a file to
        // append to (populate_proc itself doesn't create /system/).
        std::fs::create_dir_all(format!("{}/system", rootfs)).unwrap();
        std::fs::write(format!("{}/system/build.prop", rootfs), "# existing\n").unwrap();

        populate_proc(&rootfs, 1, 1024).expect("populate_proc");

        let build_prop = format!("{}/system/build.prop", rootfs);
        let content = std::fs::read_to_string(&build_prop).unwrap();
        assert!(
            content.contains("apexd.status=activated"),
            "populate_proc should append apexd.status=activated to build.prop: {}",
            content
        );

        let _ = std::fs::remove_dir_all(&rootfs);
    }

    /// Regression test: `write_vendor_default_prop` must create
    /// `{rootfs}/vendor/default.prop` with the expected boot-critical
    /// keys (locale, timezone, density). Without this, init loads wrong
    /// HALs (or none) for a containerized guest.
    ///
    /// Adopted from Nogitsune's ensureVendorDefaultProp
    /// (BootHelper.kt:191-206) which writes the same 6 keys before init
    /// runs.
    #[test]
    fn test_write_vendor_default_prop_creates_file() {
        let rootfs = tmpdir();
        write_vendor_default_prop(&rootfs, 440, "en", "US", "America/New_York")
            .expect("write_vendor_default_prop");

        let prop_path = format!("{}/vendor/default.prop", rootfs);
        assert!(
            Path::new(&prop_path).exists(),
            "vendor/default.prop should exist"
        );

        let content = std::fs::read_to_string(&prop_path).unwrap();
        assert!(
            content.contains("persist.sys.language=en"),
            "missing persist.sys.language=en: {}",
            content
        );
        assert!(
            content.contains("persist.sys.country=US"),
            "missing persist.sys.country=US: {}",
            content
        );
        assert!(
            content.contains("persist.sys.timezone=America/New_York"),
            "missing persist.sys.timezone=America/New_York: {}",
            content
        );
        assert!(
            content.contains("ro.sf.lcd_density=440"),
            "missing ro.sf.lcd_density=440: {}",
            content
        );

        let _ = std::fs::remove_dir_all(&rootfs);
    }

    /// Regression test: `vendor/default.prop` MUST contain
    /// `ro.hardware=goldfish` — this is the CRITICAL line that tells
    /// init to load AOSP Goldfish virtualization HALs (the AOSP-shipped
    /// audio.primary.goldfish.so, gralloc.goldfish.so, etc.) that
    /// tolerate missing real hardware. Without this, init looks for
    /// HALs matching the build-time ro.hardware (e.g. ranchu) which
    /// may not exist in the container's /vendor/lib64/hw/.
    ///
    /// This is the headline value of the whole function — guards
    /// against accidentally renaming or removing this single line.
    #[test]
    fn test_vendor_default_prop_contains_goldfish() {
        let rootfs = tmpdir();
        write_vendor_default_prop(&rootfs, 320, "en", "US", "America/New_York")
            .expect("write_vendor_default_prop");

        let prop_path = format!("{}/vendor/default.prop", rootfs);
        let content = std::fs::read_to_string(&prop_path).unwrap();
        assert!(
            content.contains("ro.hardware=goldfish"),
            "ro.hardware=goldfish missing from vendor/default.prop: {}",
            content
        );

        let _ = std::fs::remove_dir_all(&rootfs);
    }

    /// Regression test: `vendor/default.prop` MUST contain
    /// `ro.zygote=zygote64` — this tells init to spawn the 64-bit
    /// zygote (not the 32-bit one). Nogitsune's ensureVendorDefaultProp
    /// hardcodes this (BootHelper.kt:201) for both Android and TWRP
    /// modes since both guest paths run on 64-bit Android.
    #[test]
    fn test_vendor_default_prop_contains_zygote64() {
        let rootfs = tmpdir();
        write_vendor_default_prop(&rootfs, 320, "en", "US", "America/New_York")
            .expect("write_vendor_default_prop");

        let prop_path = format!("{}/vendor/default.prop", rootfs);
        let content = std::fs::read_to_string(&prop_path).unwrap();
        assert!(
            content.contains("ro.zygote=zygote64"),
            "ro.zygote=zygote64 missing from vendor/default.prop: {}",
            content
        );

        let _ = std::fs::remove_dir_all(&rootfs);
    }

    /// Integration test: `populate_proc` must also invoke
    /// `write_vendor_default_prop`, so that vendor/default.prop is
    /// written whenever the /proc tree is populated (the canonical
    /// kr64 setup path — called from lib.rs:2206 for BOTH TWRP and
    /// Android modes). Without this wiring, the function added in the
    /// previous commit would be dead code.
    #[test]
    fn populate_proc_also_writes_vendor_default_prop() {
        let rootfs = tmpdir();
        populate_proc(&rootfs, 1, 1024).expect("populate_proc");

        let prop_path = format!("{}/vendor/default.prop", rootfs);
        assert!(
            Path::new(&prop_path).exists(),
            "populate_proc should write vendor/default.prop"
        );

        let content = std::fs::read_to_string(&prop_path).unwrap();
        // The CRITICAL line — see `test_vendor_default_prop_contains_goldfish`.
        assert!(
            content.contains("ro.hardware=goldfish"),
            "populate_proc-written vendor/default.prop should contain ro.hardware=goldfish: {}",
            content
        );
        assert!(content.contains("ro.zygote=zygote64"));
        // Defaults wired inside populate_proc.
        assert!(content.contains("ro.sf.lcd_density=320"));
        assert!(content.contains("persist.sys.language=en"));

        let _ = std::fs::remove_dir_all(&rootfs);
    }
}
