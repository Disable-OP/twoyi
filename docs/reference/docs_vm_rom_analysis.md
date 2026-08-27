# Virtual Master — ROM Image Analysis (Task VM-ROM-1)

> **APK under test:** `com.clone.android.dual.space` (Virtual Master)
> **APK location:** `/tmp/vm.apk` on codespace `twoyi-dev-3-jr47xg6xvx7ghq6p`
> **APK total size:** 173,408,685 bytes (~165 MiB) — 1,732 entries
> **Analysis date:** 2026-08-05

---

## TL;DR — The user's hypothesis is INCORRECT

**There is NO GSI / ROM image bundled inside the Virtual Master APK.**

What IS bundled in the APK under `assets/plugins/` is four **AES-128-ECB-encrypted ZIP archives** of *add-ons* that get layered on top of the ROM at install time:

| Asset file (in APK) | Size (encrypted) | Decrypted contents | Purpose |
|---|---|---|---|
| `assets/plugins/play.zip`     | 101,868,720 B (~98 MiB) | GApps: `GoogleServicesFramework.apk` (4.0 MB), `Phonesky.apk` (Play Store, 53.7 MB), `PrebuiltGmsCore.apk` (Play Services, 99.2 MB) | Google Apps bundle |
| `assets/plugins/magisk.zip`   | 19,271,488 B  (~18 MiB) | `sbin/magisk32`, `sbin/magisk64`, `sbin/stub.apk`, busybox, boot_patch.sh, chromeos/futility … | Magisk root framework |
| `assets/plugins/xposed.zip`   | 4,550,224 B  (~4.3 MiB) | `system/app/XposedInstaller/XposedInstaller.apk`, `system/framework/XposedBridge.jar`, `system/lib64/libxposed_art.so` | Xposed framework |
| `assets/plugins/superuser.zip`| 1,451,168 B  (~1.4 MiB) | `system/app/Superuser/Superuser.apk`, `system/xbin/daemonsu` | KingRoot-style superuser |

The actual **system image** (the GSI/ROM) is **downloaded at runtime** from `https://api.virtualmaster.app/…` and is *not* part of the APK. Six ROM versions are offered by the server:

| Internal URI | Display name | `os_version` | `support_a32` | `support_a64` | `minimum_sdk_int` | Asset size (bytes / MiB) |
|---|---|---:|:---:|:---:|:---:|---:|
| `pad://rom_4_2_2`     | Android 4.2.2     | 4  | true  | false | 21 |  69,156,234  (~66 MiB) |
| `pad://rom_5_1_1`     | Android 5.1.1     | 5  | true  | true  | 21 | 231,894,043  (~221 MiB) |
| `pad://rom_7_1_2_32`  | Android 7.1.2 32  | 7  | true  | false | 24 | 246,138,575  (~235 MiB) |
| `pad://rom_7_1_2`     | Android 7.1.2     | 7  | true  | true  | 24 | 312,103,368  (~298 MiB) |
| `pad://rom_9_0_0`     | Android 9.0.0     | 9  | false | true  | 28 | 295,470,236  (~282 MiB) |
| `pad://rom_11_0_0`    | Android 11.0.0    | 11 | false | true  | 30 | 367,928,823  (~351 MiB) |

(Decoded from `r3/C3947WWWWWWWW.java` — the in-app ROM catalog. Sizes are the compressed download sizes; the uncompressed system image will be larger.)

---

## 1. How the APK was investigated

1. `unzip -l /tmp/vm.apk` — full enumeration. Largest files in the APK, sorted by size:

   ```
   101,868,720   assets/plugins/play.zip          ← GApps (NOT a ROM)
    19,271,488   assets/plugins/magisk.zip        ← Magisk
    11,031,888   classes.dex
     7,743,072   lib/arm64-v8a/libvm.so           ← VM runtime (libOpenglRender equivalent)
     5,105,764   lib/armeabi-v7a/libvm.so
     4,550,224   assets/plugins/xposed.zip        ← Xposed
     2,431,244   classes3.dex
     2,145,156   resources.arsc
     2,031,728   lib/arm64-v8a/libkr64.11.so      ← Kernel-replacement for Android 11
     1,866,244   lib/arm64-v8a/libkr32.11.so
     1,866,244   lib/armeabi-v7a/libkr32.11.so
     1,716,644   lib/armeabi-v7a/libkr32.so       ← Kernel-replacement (default, A 7.1.2)
     1,716,644   lib/arm64-v8a/libkr32.so
     1,505,200   lib/arm64-v8a/libkr64.so
     1,451,168   assets/plugins/superuser.zip     ← Superuser
       864,616   lib/arm64-v8a/libcrashlytics-common.so
       …
   ```

   Total of all `assets/plugins/*.zip` = ~127 MiB. No `.img`, `.iso`, `.rom`, `.squashfs`, `.ext4`, `system.img`, or `vendor.img` exists anywhere in the APK.

2. Magic-byte inspection of the four `assets/plugins/*.zip` files:

   ```
   play.zip     :  a2 f3 60 55 95 84 68 6f 10 ff 73 d9 d7 30 28 e5   ← not PK, not 7z, not gzip
   magisk.zip   :  83 4d a4 50 66 88 ae cd 36 39 34 d2 be bd c5 b6   ← same: encrypted
   xposed.zip   :  13 e4 71 b4 d4 37 6a 10 56 a0 3a 64 92 d7 38 12   ← same: encrypted
   superuser.zip:  13 e4 71 b4 d4 37 6a 10 56 a0 3a 64 92 d7 38 12   ← IDENTICAL first 16 bytes to xposed.zip!
   ```

   The identical first 16 bytes between `xposed.zip` and `superuser.zip` is the giveaway that **both files share the same plaintext (a ZIP local-file header) encrypted with the same AES-128-ECB key** (ECB encrypts identical 16-byte plaintext blocks to identical ciphertext).

3. `7z` and `unzip` both refused to open the plugin files (`Can not open the file as [zip] archive` / `End-of-central-directory signature not found`), confirming they're not real ZIPs on disk.

4. Reverse-engineered the Java loader to find the AES key (see §2 below).

---

## 2. How the plugins are encrypted — full derivation

The plugin loader is `com.android.vmcore.installer.ImageInstallerV1` (decompiled with jadx). Its `m5202WWWWWWWW()` factory method chooses between three output streams depending on the URI's `e=` query parameter:

```java
if (uri.getQueryParameter("e").equals("n")) {                // e=n  → no encryption
    return new BufferedOutputStream(new FileOutputStream(str));
} else if (uri.getQueryParameter("e").equals("x")) {          // e=x  → XOR with key
    return new BufferedOutputStream(new XOROutputStream(new FileOutputStream(str), KEY));
} else {                                                       // else → AES/ECB with key
    Cipher cipher = Cipher.getInstance("AES");
    cipher.init(Cipher.DECRYPT_MODE, new SecretKeySpec(KEY.getBytes(), "AES"));
    return new BufferedOutputStream(new CipherOutputStream(new FileOutputStream(str), cipher));
}
```

The `KEY` and the algorithm strings are obfuscated with **StringFog** (Vigenère-style XOR with a per-string byte-array key). The StringFog algorithm (`x5.WWWWWWWW.m17835WWWWWWWW`) is:

```java
for (int i = 0, j = 0; i < bArr.length; i++, j++) {
    if (j >= bArr2.length) j = 0;
    bArr[i] ^= bArr2[j];
}
return new String(bArr, UTF_8);
```

After decoding every `StringFog` call in `ImageInstallerV1.java`, we obtain:

| Field | Decoded value |
|---|---|
| Cipher algorithm              | `AES`            (= `AES/ECB/PKCS5Padding` in Java default) |
| SecretKeySpec algorithm       | `AES` |
| AES key (16 bytes, UTF-8)     | **`%z89aviCM0KkbEs9`** (hex: `25 7a 38 39 61 76 69 43 4d 30 4b 6b 62 45 73 39`) |
| XOR key (16 bytes, UTF-8)     | **`%z89aviCM0KkbEs9`** (same key reused) |
| Query-param name              | `e` |
| Query-param value (no enc)    | `n` |
| Query-param value (xor)       | `x` |

**Verification** (openssl on the codespace):

```bash
$ head -c 64 /tmp/vm-plugins/assets/plugins/play.zip \
  | openssl enc -d -aes-128-ecb -K 257a3839617669434d304b6b62457339 -nopad \
  | od -A x -t x1z -v

000000  50 4b 03 04 14 00 00 00 00 00 2a b1 38 56 00 00  |PK........*.8V..|
000010  00 00 00 00 00 00 00 00 00 00 18 00 00 00 47 6f  |..............Go|
000020  6f 67 6c 65 53 65 72 76 69 63 65 73 46 72 61 6d  |ogleServicesFram|
000030  65 77 6f 72 6b 2f 50 4b 03 04 14 00 00 00 08 00  |ework/PK........|
```

`PK\x03\x04` is the ZIP local-file-header signature. The first entry is `GoogleServicesFramework/`. Decrypting the full 101 MB file with the same key produced a valid ZIP containing exactly three APKs (see table in §0).

Full-file decryption command (took 0.23 s on the codespace):

```bash
openssl enc -d -aes-128-ecb -K 257a3839617669434d304b6b62457339 -nopad \
  -in play.zip -out play.zip.decrypted
```

(101,868,720 bytes / 16 = 6,366,795 — exactly divisible by the AES block size, so no padding is needed.)

The other three plugins decrypt identically with the same key, all yielding real ZIP files.

---

## 3. Where the actual ROM image lives

The ROM image is **not** in the APK. The app fetches a JSON `RomConfig` from the server and stores it in `SharedPreferences` under the key **`rom_config`** (decoded from StringFog). The `RomConfig` JSON has these fields (decoded from `com.android.vmcore.RomConfig`):

| JSON key             | Java field              | Type        | Meaning |
|---|---|---|---|
| `display_name`       | `f8846WWWWWWWW`         | String      | e.g. `"Android 7.1.2"` |
| `rom_version`        | `f8845WWWWoWWWWo`       | String      | Build version string |
| `os_version`         | `f8853WWWoWWWo`         | int         | Major Android version (4, 5, 7, 9, 11) |
| `minimum_sdk_int`    | `f8847WWWWWWWW`         | int         | Min host SDK |
| `minimum_app_ver`    | `f8848WWWWWWWW`         | boolean (?) | Min app version gate |
| `support_a32`        | `f8855WWoWWo`           | boolean     | 32-bit ABI supported |
| `support_a64`        | `f8849WWWWWWWW`         | int/bool    | 64-bit ABI supported |
| **`rom_uri`**        | `f8854WWWoWWWo`         | String[]    | **List of ROM-image download URLs (mirrors)** |
| `overlay_uri`        | `f8851WWWWWWWW`         | String[]    | Overlay package URLs |
| `magisk_uri`         | `f8852WWWWWWWW`         | String      | Magisk package URL |
| `su_uri`             | `f8857WWWW`             | String      | Superuser package URL |
| `xposed_uri`         | `f8858WoWo`             | String      | Xposed package URL |
| `play_uri`           | `f8856WWoWWo`           | String      | GApps package URL |

For the four add-on plugins, `VMConfig.m5050WWWWoWWWWo()` migrates legacy `asset:///rom/rom_7_1_2/<plugin>.zip` URIs to the current `asset:///plugins/<plugin>.zip` URIs:

```java
if ("asset:///rom/rom_7_1_2/magisk.zip".equals(romConfig.f8852WWWWWWWW)) {
    romConfig.f8852WWWWWWWW = "asset:///plugins/magisk.zip";  // current APK path
}
// (same for superuser_uri, xposed_uri, play_uri)
```

That migration is the **only** place `asset:///plugins/...` shows up — i.e. **only the four add-on plugins are bundled**; `rom_uri` has no in-APK fallback and must come from the server.

### 3.1 The `pad://rom_X_Y_Z` URI scheme

The bundled ROM catalog (`r3/C3947WWWWWWWW.java`) registers six `RomModel` entries whose `Asset.uri` is a synthetic `pad://rom_X_Y_Z` URI. The app resolves these (presumably via `https://api.virtualmaster.app/account/v1/...`) to actual HTTPS download URLs at install time — the `pad://` URIs are stable logical identifiers that survive mirror changes.

### 3.2 The `!/rom.zip` APK-internal path

In `l3/C3394WWWWWWWW.java` (the RomRepository) the code constructs the URI:

```java
sb.append("file:///");                       // ← decoded StringFog
sb.append(file.getAbsolutePath());           // user-imported file path
sb.append("!/rom.zip");                      // ← decoded StringFog
```

This is the standard `apk!/<internal-path>` syntax for accessing a file inside another archive. **It is used only when the user manually imports a local ROM file** — the imported file is treated as a wrapper containing `rom.zip` at its root. So end users can side-load a ROM by shipping a `rom.zip` inside any `.zip`/`.apk` they like.

---

## 4. Evidence that the bundled ROM is a **Treble-style multi-partition image** (not a single system.img)

The dex files contain many Treble-specific path constants (decoded from StringFog across the entire jadx output) that the app manipulates after extracting `rom.zip`:

### 4.1 System-partition paths
```
/system/build.prop
/system/etc/prop.default
/system/etc/init/hw/init.rc
/system/bin/app_process32, app_process32_xposed
/system/bin/app_process64, app_process64_xposed
/system/bin/sh
/system/bin/su
/system/bin/device_config set_sync_disabled_for_tests persistent; …
/system/lib/libhostlibui.so, libhostlibui_10.so
/system/lib/libui.so, libui10.so, libui51.so
/system/lib64/libhostlibui.so, libhostlibui_10.so
/system/lib64/libui.so, libui10.so, libui51.so
/system/priv-app/GoogleServicesFramework/
/system/priv-app/Phonesky/Phonesky.apk
/system/priv-app/PrebuiltGmsCore/PrebuiltGmsCore.apk
/system/app/Superuser/Superuser.apk
/system/app/XposedInstaller/XposedInstaller.apk
/system/framework/XposedBridge.jar
/system/xposed.prop
/system/xbin/su, daemonsu
```

### 4.2 Treble-only paths (these only exist on Treble / Android 8.0+ GSIs)
```
/system/product/build.prop             ← product partition (Treble)
/system/system_ext/build.prop          ← system_ext partition (Android 10+)
/vendor/build.prop                     ← vendor partition (Treble)
/vendor/etc/vintf/manifest/vibrator-default.xml      ← VINTF HAL manifest (Treble)
/vendor/etc/init/vibrator-default.rc                  ← vendor init .rc
/vendor/bin/hw/android.hardware.vibrator-service.example   ← vendor HAL binary
```

The presence of `/system/product/`, `/system/system_ext/`, and especially **`/vendor/etc/vintf/manifest/*.xml`** (the Treble HAL manifest, introduced in Android 8.0) strongly suggests the newer ROMs (9.0, 11.0) are real **Treble GSIs**. The Android 7.1.2 ROM may be a custom AOSP build with these paths backported, since 7.1 predates Treble.

### 4.3 The "BugNFixTask" startup tasks

`com.android.vmcore.startup.Bug1FixTask` … `Bug8FixTask` exist — these are workarounds for known issues across the various ROM versions. Together with `BuildTmpfsTask`, `BuildVMPropTask`, `BuildExecPathTask`, `ApplyOverlaysTask`, `GooglePlayTask`, `MagiskTask`, `SuperuserTask`, `XposedTask`, they form a post-install pipeline that mutates the extracted `rom.zip` tree on disk before boot.

### 4.4 Per-Android-version kernel / libui selection

The `.so` library naming tells the same story:

| Library                       | Used for |
|---|---|
| `libkr32.so`, `libkr64.so`             | Default kernel-replacement (Android 7.1.2) |
| `libkr32.11.so`, `libkr64.11.so`       | Android 11 kernel-replacement |
| `libui.so`     | Android 7.1.2 system `libui.so` (replaced by host-aware shim) |
| `libui10.so`   | Android 10 variant |
| `libui51.so`   | Android 5.1 variant |
| `libhostlibui.so`, `libhostlibui_10.so` | Host-side UI shim matching each Android version |
| `libvm.so`     | emugl-style GL renderer (`/dev/qemu_pipe` transport, see VIRTUAL_MASTER_FULL_ANALYSIS.md) |

---

## 5. Conclusion / answers to the task's specific questions

> **Q1: Exact filename and size of the ROM image in the APK**

There is **no ROM image in the APK**. The four large `assets/plugins/*.zip` files are *not* ROMs — they are AES-128-ECB-encrypted ZIP archives of GApps/Magisk/Xposed/Superuser add-ons that get layered onto the ROM after it is downloaded. Their (encrypted) sizes are:

| File | Encrypted size | Decrypted ZIP size (sum of entries) |
|---|---:|---:|
| `assets/plugins/play.zip`      | 101,868,720 B | 164,373,476 B |
| `assets/plugins/magisk.zip`    |  19,271,488 B |  23,023,351 B |
| `assets/plugins/xposed.zip`    |   4,550,224 B |   9,568,484 B |
| `assets/plugins/superuser.zip` |   1,451,168 B |   2,807,117 B |

> **Q2: Whether it's a GSI or a custom ROM**

The ROM itself (which is downloaded from `https://api.virtualmaster.app/...`) is structured like a **Treble-style multi-partition GSI** for the Android 9 and 11 variants (VINTF manifest, `/system/product/`, `/system/system_ext/`, `/vendor/`). The Android 4.2.2 / 5.1.1 / 7.1.2 variants predate Treble and are more likely custom AOSP builds. We cannot definitively say without downloading the ROM from the server, but the in-APK evidence (path constants, HAL manifest references) points to GSI for the newer versions.

> **Q3: The filesystem type (ext4, squashfs, etc.)**

Unknown — the ROM is downloaded as a `rom.zip` (a ZIP archive, not a raw filesystem image). The ZIP presumably contains compressed partition images (likely `system.img`, `vendor.img`, etc.), but we'd need to download and unzip it to inspect each partition's filesystem type. None of the APK's assets themselves are ext4/squashfs images (the only filesystem magic bytes found were coincidental 2-byte matches of `0x53ef`).

> **Q4: The build.prop contents**

Could not be retrieved — `build.prop` lives inside the downloaded ROM, not in the APK. The dex does reference these paths for runtime manipulation:
- `/system/build.prop`
- `/system/etc/prop.default`
- `/system/product/build.prop`
- `/system/system_ext/build.prop`
- `/vendor/build.prop`

> **Q5: Treble-specific files found**

Yes — paths decoded from StringFog inside the dex:
- `/vendor/etc/vintf/manifest/vibrator-default.xml` — Treble HAL manifest
- `/vendor/etc/init/vibrator-default.rc` — vendor init script
- `/vendor/bin/hw/android.hardware.vibrator-service.example` — vendor HAL binary
- `/system/product/build.prop` — Treble product partition
- `/system/system_ext/build.prop` — Android 10+ system_ext partition
- `/vendor/build.prop` — Treble vendor partition

> **Q6: How the ROM is structured (single system.img or multiple partitions)**

Structured as a **ZIP archive (`rom.zip`) containing multiple partition images**, based on the path constants above. The app's startup pipeline (`PrepareFsTask` → `InstallFsTask` → `BuildTmpfsTask` → `ApplyOverlaysTask` → `GooglePlayTask` → `MagiskTask` → `SuperuserTask` → `XposedTask` → `Bug1FixTask`…`Bug8FixTask` → `ChmodFsTask` → `LoadVMPropTask`) extracts `rom.zip` to a tmpfs-based filesystem at `vMConfig.f8868WWWWWWWW` (the VM data dir) and then layers the four add-on ZIPs on top.

---

## 6. How to actually obtain the ROM image (next steps)

Since the ROM is not in the APK, future work to obtain a real ROM image should:

1. **Capture network traffic** from a running Virtual Master install to record the `https://api.virtualmaster.app/.../rom/...` download URL, OR
2. **Set up a MITM proxy** (e.g. mitmproxy) and install Virtual Master's CA cert into the device, then trigger a ROM download from the in-app "ROM" fragment (`com.android.vmapp.ui.vm.rom.RomFragment`) — the resulting request URL will be the real `rom_uri`.
3. The `rom.zip` URL is gated behind the `account/v1` auth flow (`h2/C2687WWWWWWWW.java` uses `VerifyAssertionRequest` / `GetTokenResponse`), so an account token is required even for the bundled `pad://rom_X_Y_Z` URIs.

Once `rom.zip` is downloaded, run:
```bash
unzip rom.zip                          # extract partition images
file system.img vendor.img product.img system_ext.img
dumpe2fs system.img 2>/dev/null | head # if ext4
unsquashfs -s system.img               # if squashfs
```

---

## 7. Reproducibility — how to verify these findings

All commands run on codespace `twoyi-dev-3-jr47xg6xvx7ghq6p`:

```bash
# 1. List APK contents sorted by size
unzip -l /tmp/vm.apk | sort -rn -k1 | head -30

# 2. Extract the four encrypted plugin zips
mkdir -p /tmp/vm-plugins && cd /tmp/vm-plugins
unzip -o /tmp/vm.apk 'assets/plugins/*'

# 3. Decrypt each plugin with AES-128-ECB key "%z89aviCM0KkbEs9" (hex: 257a3839617669434d304b6b62457339)
for f in play magisk xposed superuser; do
  openssl enc -d -aes-128-ecb -K 257a3839617669434d304b6b62457339 -nopad \
    -in assets/plugins/$f.zip -out $f.zip.decrypted
  unzip -l $f.zip.decrypted | head -20
done

# 4. Decompile the APK to find StringFog-encrypted ROM URLs
wget https://github.com/skylot/jadx/releases/download/v1.4.7/jadx-1.4.7.zip
unzip -q jadx-1.4.7.zip -d /tmp/jadx
/tmp/jadx/bin/jadx -d /tmp/jadx-out --no-res /tmp/vm.apk

# 5. Decode every StringFog string (Vigenère XOR) and grep for ROM/URL hints:
python3 - <<'PY'
import re, os
TC = {'LF_NORMAL':0x30,'LF_LINK':0x31,'LF_SYMLINK':0x32,'LF_CHR':0x33,'LF_BLK':0x34,'LF_DIR':0x35,'LF_FIFO':0x36,'LF_CONTIG':0x37,'LF_GNUTYPE_LONGLINK':0x4B,'LF_GNUTYPE_LONGNAME':0x4C,'LF_MULTIVOLUME':0x4D,'LF_GNUTYPE_SPARSE':0x53,'LF_PAX_EXTENDED_HEADER_LC':0x78,'LF_PAX_GLOBAL_EXTENDED_HEADER':0x67,'LF_PAX_EXTENDED_HEADER_UC':0x58}
CP = {'CP_Class':7,'CP_Fieldref':9,'CP_Methodref':10,'CP_InterfaceMethodref':11,'CP_NameAndType':12,'CP_String':8}
BC = {'Byte.MAX_VALUE':127,'Byte.MIN_VALUE':-128}
def r(t):
    t=t.strip()
    if t in TC: return TC[t]
    if t in CP: return CP[t]
    if t in BC: return BC[t]
    if t.startswith('ConstantPoolEntry.'): return CP[t.split('.',1)[1]]
    if t.startswith('TarConstants.'): return TC[t.split('.',1)[1]]
    if t.startswith('Byte.'): return BC[t]
    return int(t)
def dec(b,k):
    o=bytearray(len(b))
    for i in range(len(b)): o[i]=(b[i]&0xff)^(k[i%len(k)]&0xff)
    try: return o.decode('utf-8')
    except: return None
P=re.compile(r'(?:WWWWWWWW|StringFog)\.m(?:17835|5049)WWWWWWWW\(new byte\[\]\{([^}]+)\},\s*new byte\[\]\{([^}]+)\}\)')
for root,_,fs in os.walk('/tmp/jadx-out'):
    for f in fs:
        if not f.endswith('.java'): continue
        t=open(os.path.join(root,f),encoding='utf-8',errors='ignore').read()
        for m in P.finditer(t):
            try:
                d=dec([r(x) for x in m.group(1).split(',') if x.strip()],
                      [r(x) for x in m.group(2).split(',') if x.strip()])
                if d and any(k in d.lower() for k in ['rom','img','http','vintf','/system','/vendor','build.prop']):
                    print(d)
            except: pass
PY
```

---

## 8. File artifacts produced on the codespace

| Path on codespace | Description |
|---|---|
| `/tmp/vm-plugins/assets/plugins/play.zip.decrypted`      | Decrypted GApps ZIP (101 MB) |
| `/tmp/vm-plugins/assets/plugins/magisk.zip.decrypted`    | Decrypted Magisk ZIP (18 MB) |
| `/tmp/vm-plugins/assets/plugins/xposed.zip.decrypted`    | Decrypted Xposed ZIP (4.3 MB) |
| `/tmp/vm-plugins/assets/plugins/superuser.zip.decrypted` | Decrypted Superuser ZIP (1.4 MB) |
| `/tmp/jadx-out/` | Full jadx decompilation of `com.clone.android.dual.space` |
| `/tmp/decode_sf.py`, `/tmp/decode_all.py`, `/tmp/decode_lines.py`, `/tmp/find_str.py` | StringFog decoder scripts |
