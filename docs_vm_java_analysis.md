# Virtual Master APK — Java Code Analysis (Task VM-JAVA-1)

> **APK under test:** `com.clone.android.dual.space` (Virtual Master) v3.2.53
> **APK location:** `/tmp/vm.apk` on codespace `twoyi-dev-3-jr47xg6xvx7ghq6p`
> **Jadx output:** `/tmp/jadx-out/sources/` (decompiled by previous agent)
> **Analysis date:** 2026-08-05
> **Decompiler:** jadx 1.4.7

---

## TL;DR

Virtual Master is a fully-Java-orchestrated Android-in-Android container that boots a downloaded AOSP/Treble ROM inside a per-VM data directory. The whole stack is driven from `com.android.vmapp.VMApp` (Application) → `com.android.vmcore.VMManager` → `com.android.vmcore.VMInstance` (the per-VM state machine), with a single native library `libvm.so` doing all the heavy lifting.

**Key architectural facts (correcting/refining the previous disassembly reports):**

1. **No `NativeActivity`** — Virtual Master uses its own `VMDisplayActivity extends BaseActivity`. A `SurfaceView` (NOT `TextureView`) is created programmatically and the `android.view.Surface` object is passed to the native renderer through a custom JNI API `DisplayService.nativeAddSurface(ptr, surfaceId, surface, w, h, rotation)`. This is a **per-VM renderer pointer pattern**, NOT the AOSP emugl `initOpenGLRenderer()`/`createOpenGLSubwindow()` global-singleton pattern used by twoyi.

2. **Single native library:** `System.loadLibrary("vm")` loads `libvm.so`. ALL JNI bindings — display, input, audio, HAL, binder virtualization, OS boot, process management — live in this one `.so`.

3. **Boot sequence is a Java state machine.** `VMInstance.f8940WWoWWo` walks 11 states (`-5..7`); each state transition fires an `EventBus` `VMStatusEvent` that the UI subscribes to. The actual OS launch is a single JNI call: `VMInstance.startOS(int vmId, int dpi, String libPath)`.

4. **ROM is downloaded and decrypted on the fly** with `AES/ECB/PKCS5Padding` (key `%z89aviCM0KkbEs9`) or XOR (same key) — chosen by the URI's `e=` query parameter. The downloaded stream is piped through `CipherOutputStream` so the cleartext never touches disk; it's then extracted (ZIP or 7z) directly into the VM's `/fs` directory.

5. **Two IPC channels to the guest:**
   - **Unix domain socket** at `<vmDataDir>/dev/event` (Java side: `LocalServerSocket`). The guest connects and exchanges UTF-8 strings of the form `eventName`+backtick+`payload`. 25+ event types (`BOOT_COMPLETED`, `SHUTDOWN`, `START_INSTALL_APP`, `CLIPBOARD_DATA`, `SEND_KEY_EVENT`, `EXECUTE_COMMAND`, …).
   - **Binder virtualization** via native `setupBinder(vmId, ...)`. This installs a per-VM `/vm%d/dev/binder` and proxies the host's `android.app.IActivityManager` IBinder through a Java `Proxy` so the guest's `servicemanager` thinks it's talking to a real OS.

---

## 1. Application class & native library load

**File:** `com/android/vmapp/VMApp.java`

### `attachBaseContext(Context)` — hidden-API bypass

On Android 9+ (SDK ≥ 28), Virtual Master loads a tiny embedded DEX (base64-encoded, ~3 KB) that contains `me.weishu.reflection.BootstrapClass.exemptAll()` — the well-known [FreeReflection](https://github.com/tiann/FreeReflection) trick. The dex is written to `codeCacheDir/<timestamp>.dex`, loaded with `DexFile.loadClass()`, then `exemptAll()` is reflectively invoked. This bypasses the hidden-API restrictions on `dalvik.system.*` and `android.os.Process.*` calls that VMInstance/DisplayService/BinderService need.

### `onCreate()` — Java-side initialization order

1. Initialize `SharedPreferences` (device-protected storage on API 24+)
2. Set app locale, Firebase RemoteConfig defaults
3. Initialize Google Ads SDK (`m7.f15885WWWW = new m7(this, ...)`)
4. Initialize billing (`C1603WWWWWWWW`) and register a `ConnectivityManager.NetworkCallback`
5. Register an `IntentFilter` for `PACKAGE_ADDED/REMOVED/REPLACED` with `dataScheme=package`
6. Initialize Crashlytics (`C2467WWWWWWWW.m13936WWWWoWWWWo().m13950WWWW(this)`)
7. **Load the native library** (synchronized on `VMManager.class`):
   ```java
   System.loadLibrary("vm");                   // decoded from StringFog
   VMManager.f8948WWWWWWWW = new VMManager(this);
   ```
8. For each existing `VMInstance` (read from data dir), create a `VMExtension` (per-VM lifecycle helper) and stash it in `SparseArray<VMExtension>`
9. On Android 12+ (SDK ≥ 31), additionally set up `o2.WoWo` (a privacy/notification module)
10. Register `ComponentCallbacks` and activity-lifecycle callbacks for memory management

The base64 string for `me.weishu.reflection.BootstrapClass` is on line 134 of `VMApp.java` (`byte[] decode = Base64.decode(...)`) — it is one of the longest lines in the file (~3,961 chars). To extract: copy the base64 string between `Base64.decode("` and `")` on that line.

---

## 2. The launch flow — from launcher icon to first rendered frame

### 2.1 The four `VMStartActivityN` classes

The `AndroidManifest.xml` declares four almost-identical activities:

```xml
<activity android:name="com.android.vmapp.vm.VMStartActivity0"
          android:taskAffinity=".vm0" android:launchMode="singleTask"/>
<activity android:name="com.android.vmapp.vm.VMStartActivity1"
          android:taskAffinity=".vm1" android:launchMode="singleTask"/>
<activity android:name="com.android.vmapp.vm.VMStartActivity2"
          android:taskAffinity=".vm2" android:launchMode="singleTask"/>
<activity android:name="com.android.vmapp.vm.VMStartActivity3"
          android:taskAffinity=".vm3" android:launchMode="singleTask"/>
```

The launcher dispatches on `vmId % 4` so that each VM gets its own task affinity (avoids Android's single-task collisions). The static helper is `VMStartActivity0.m5016WWWW(Context, int vmId, boolean forceBoot, boolean restart)` — it picks the right activity class, stuffs the extras `vm_id`, `force_boot`, `restart` into the Intent, and starts it.

### 2.2 The state machine

`VMInstance` has a single int field `f8940WWoWWo` that holds the VM state. State values (decoded from `VMStartActivity0.onVMStatusEvent()` string-resource lookups and `VMInstance.m5100WoWo()`):

| State | Name (UI string)              | Meaning                                                  |
|------:|-------------------------------|----------------------------------------------------------|
| -5    | (stopping)                    | Shutting down — `VMStatusEvent(-5, …)` posted            |
| -4    | `vm_status_start_failed`      | Boot task returned false (kernel path / prop error)      |
| -3    | `vm_status_start_svc_failed`  | HAL/Display/Input/Audio/Netlink service start failed     |
| -2    | `vm_status_install_failed`    | Setup task (PrepareFs/InstallFs/…) returned false        |
| -1    | `vm_status_env_failed`        | Pre-flight check failed (CPU/SDK/path)                   |
|  0    | `STOPPED`                     | Cold — never started, or fully shut down                 |
|  1    | `vm_status_checking_env`      | Pre-flight checks (CPU arch, data-dir path, SDK version) |
|  2    | `vm_status_installing_N`      | Running setup pipeline (PrepareFs → LoadVMProp)          |
|  3    | `vm_status_starting_svc`      | Starting HAL/Input/Audio/Display/Netlink services        |
|  4    | `vm_status_starting`          | Running startup pipeline (ApplyOverlays → BuildExecPath) |
|  5    | `vm_status_os_booting`        | `startOS()` JNI returned; waiting for `BOOT_COMPLETED`   |
|  6    | `vm_status_os_ready1`         | Guest signalled `BOOT_COMPLETED` via event socket        |
|  7    | `vm_status_os_ready2`         | Guest signalled `SHUTDOWN` (clean exit)                  |

### 2.3 Boot sequence (in `VMInstance.m5100WoWo(boolean restart)`)

The method runs on a dedicated `HandlerThread` (named `"VMInstance"`, see `m5098WoWo()`). Step by step:

```
state 0 (STOPPED) → state 1 (STARTING)
  ├─ CPU check: CPUUtils.m5240/m5242 (must be armv7-a/armv8-a)
  ├─ Data-dir check: dataDir must start with "/data/user/" or "/data/data/"
  ├─ SDK check: Build.VERSION.SDK_INT >= RomConfig.minimum_sdk_int
  ├─ App-version check: app version >= RomConfig.minimum_app_ver
  ├─ clearZombieProcess + killProcess loop (m5056) — clean up any stale guest pids
  │
  └─ If ROM changed or first boot → state 2 (INSTALLING):
       │
       │   Sequential setup pipeline (each task returns boolean):
       │   ├─ PrepareFsTask   — chmod the existing fs dir to DEFAULT_LINK_PERM
       │   ├─ InstallFsTask   — download & extract the ROM (see §4)
       │   ├─ FixFsTask       — fix fs paths/symlinks
       │   ├─ CleanFsTask     — remove stale cache/tmp files
       │   ├─ ChmodFsTask     — NativeHelper.chmodRecursively(fsDir, 0xA1FF)
       │   ├─ CleanCacheTask  — clear caches
       │   ├─ FixCPUArchTask  — rewrite /system/bin/app_process{32,64}_xposed shims
       │   └─ LoadVMPropTask  — parse /system/build.prop into VMConfig.f8870 (HashMap)
       │
       └─ Set f8919WWWW=true (installed flag), persist RomConfig to prefs
       │
       └─ BinderService setup (state ~3 implicitly):
       │   ├─ BinderService.m5206WWWWoWWWWo(vMApp, vmId)
       │   │     ├─ Get IBinder for android.app.IActivityManager (via reflection)
       │   │     ├─ Wrap it with a Java Proxy that intercepts getCallingPid()
       │   │     ├─ Call native setupBinder(vmId, …, "com.android.vmcore.service.IBinderService", parcelledIntent)
       │   │     └─ bindService(BinderService.class) — wait up to 5s for onServiceConnected
       │   │
       │   ├─ new InputService(this) → nativeStartService  (touch input)
       │   ├─ new AudioService(this) → start              (microphone + audio)
       │   ├─ new HALManager(this)   → startHALMgr        (camera/sensor/location/wifi/phone/battery)
       │   ├─ new DisplayService(this) → nativeStartService(w, h)
       │   ├─ new NetlinkManager(this) → start             (virtual network tun0)
       │   └─ VMEventManager.m5115WWWWWWWW()               (start the LocalServerSocket thread)
       │
       └─ state 4 (BOOTING): run startup pipeline (sequential, each task returns boolean):
            ├─ ApplyOverlaysTask  — copy /system/product overlay files
            ├─ Bug1FixTask … Bug8FixTask  — ROM-version-specific patches
            ├─ CleanLogTask       — clear /proc/self/fd/2 logs
            ├─ SuperuserTask      — extract superuser.zip → /system/...
            ├─ XposedTask         — extract xposed.zip → /system/...
            ├─ GooglePlayTask     — extract play.zip (GApps) → /system/...
            ├─ MagiskTask         — extract magisk.zip → /system/sbin/..., patch init.rc
            ├─ BuildTmpfsTask     — mount tmpfs on /tmp, /dev, etc. (via native)
            ├─ BuildVMPropTask    — write /system/build.prop (PIE/build fingerprint)
            └─ BuildExecPathTask  — set PATH and exec dirs

       └─ state 5 (RUNNING):
            int pid = startOS(vmId, dpi, kernelPath);   // JNI → libvm.so
            //  - vmId:       vMConfig.f8866WWWWWWWW (0..N)
            //  - dpi:        vMConfig.f8895WWWoWWWo.f8847WWWWWWWW (RomConfig.minimum_sdk_int? actually DPI)
            //  - kernelPath: vMConfig.f8903WWoWWo  = dataDir + "/lib64"
            //                (where libvm.so + libkr64.so live)
            f8938WWWoWWWo = pid;  // save guest pid
            if (pid < 0) → state -4, errorCode = -pid + 117000

       └─ Native libvm.so now spawns the guest init in a child process; the
          guest's servicemanager → /vm%d/dev/binder (per-VM virtual binder);
          guest's SurfaceFlinger → /dev/qemu_pipe (GL transport); guest's
          EventHub → /dev/input/touch; guest's AudioFlinger → /dev/audio.

       └─ Guest eventually calls back through the /dev/event socket with:
            "com.android.vmcore.action.BOOT_COMPLETED`<payload>"
          → VMInstance.mo5013WWWWWWWW(...) sets state = 6.
```

### 2.4 Transition to the display activity

`VMStartActivity0.onVMStatusEvent(VMStatusEvent)` is an `@InterfaceC2472WWWWWWWW(threadMode = MAIN)` EventBus subscriber. When `state >= 5` (or `state >= 7` if user opted for "restart") AND the user-permission flow has completed, it calls `m5021WWoWWo()` which:

```java
Intent intent = new Intent(this, VMDisplayActivity.class);
intent.addFlags(32768);        // FLAG_ACTIVITY_REORDER_TO_FRONT
intent.addFlags(67108864);     // FLAG_ACTIVITY_NO_ANIMATION
startActivity(intent);
finish();
```

The display activity is created, and `onCreate` does:

```java
if (vMInstance.f8940WWoWWo < 5) finish();   // safety check
FrameLayout frameLayout = new FrameLayout(this);
VMSurfaceView vMSurfaceView = new VMSurfaceView(this);
vMSurfaceView.setVM(vMInstance);             // <-- bind to the VM
frameLayout.addView(vMSurfaceView, MATCH_PARENT);
setContentView(frameLayout);
```

### 2.5 The first frame

The `VMSurfaceView` extends `FrameLayout` and **creates a real `SurfaceView` as a child** (NOT TextureView):

```java
// VMSurfaceView.m5233WWWWoWWWWo(context)
SurfaceView sv = new SurfaceView(context);
sv.getHolder().addCallback(this);   // SurfaceHolder.Callback
sv.setOnTouchListener(this);
addView(sv, new LayoutParams(MATCH_PARENT, MATCH_PARENT));
```

When the framework calls back `surfaceCreated(holder)` (or `surfaceChanged`):

```java
// VMSurfaceView.m5235WWWoWWWo(SurfaceHolder)
Surface surface = holder.getSurface();
if (surface != null && surface.isValid()) {
    int w = surfaceView.getWidth();
    int h = surfaceView.getHeight();
    int rotation = f9291WWWW;  // 0/90/180/270, computed in onMeasure()

    if (vMInstance.f8945WoWo == null) {
        vMInstance.f8945WoWo = new DisplayService(vMInstance);
    }
    // Step 1: detach any previous surface for this id
    vMInstance.f8945WoWo.m5126WWWWoWWWWo(hashCode());
    // Step 2: attach the new Surface to the native renderer
    vMInstance.f8945WoWo.m5127WWWWWWWW(hashCode(), surface, w, h, rotation);
}
```

`DisplayService.m5127WWWWWWWW()` is just a thin JNI wrapper:

```java
// DisplayService.java (full content, de-obfuscated)
public class DisplayService {
    private long ptr;  // native DisplayService handle, returned by nativeSetup

    public DisplayService(VMInstance vm) {
        VMConfig cfg = vm.f8937WWWoWWWo;
        this.ptr = nativeSetup(cfg.f8866WWWWWWWW,           // vmId
                               cfg.f8895WWWoWWWo.f8847WWWWWWWW);  // dpi
    }

    private native long  nativeSetup(int vmId, int dpi);
    private native int   nativeStartService(long ptr, int width, int height);
    private native int   nativeStopService(long ptr);
    private native boolean nativeAddSurface(long ptr, int surfaceId,
                                            Surface surface,
                                            int width, int height, float rotation);
    private native boolean nativeRemoveSurface(long ptr, int surfaceId);
    private native float  nativeGetFPS(long ptr);
    private native void   nativeDispose(long ptr);
}
```

So the **Java→native rendering call chain** is:

```
Android Framework creates Surface
  → SurfaceHolder.Callback.surfaceCreated(holder)
  → VMSurfaceView.m5235WWWoWWWo(holder)
  → DisplayService.m5127WWWWWWWW(hashCode, surface, w, h, rotation)
  → DisplayService.nativeAddSurface(ptr, hashCode, surface, w, h, rotation)
  → libvm.so::nativeAddSurface  (JNI)
     (inside libvm.so, this is where the Surface gets bound to the
      per-VM emugl renderer and the QEMU pipe starts producing frames
      onto it — see VIRTUAL_MASTER_FULL_ANALYSIS.md §1)
```

The `hashCode()` of the `VMSurfaceView` is used as a unique `surfaceId` so multiple surfaces can coexist (e.g. preview + fullscreen) — Virtual Master actually uses this in `VMBigPreviewCardView` and `VMSmallPreviewCardView` to render a small live preview of the VM in the launcher before the user opens it.

---

## 3. Rendering pipeline — how the Surface reaches the native renderer

### 3.1 Why Virtual Master does NOT use `android.app.NativeActivity`

There is **no NativeActivity** in the AndroidManifest.xml. The previous disassembly report (VIRTUAL_MASTER_FULL_ANALYSIS.md) said "NativeActivity → libvm.so" but the Java side contradicts that: the activity is `com.android.vmapp.vm.VMDisplayActivity`, a regular `AppCompatActivity` subclass. The renderer entry-point `nativeAddSurface(...)` is invoked manually from `SurfaceHolder.Callback.surfaceCreated()` — Virtual Master doesn't rely on the framework's `NativeActivity` Surface-binding protocol.

### 3.2 Why a `SurfaceView` and not a `TextureView`

`VMSurfaceView` (the public container) is a `FrameLayout` that wraps a `SurfaceView`. This gives the best performance because:

- A `SurfaceView`'s Surface is backed by a separate hardware-composited layer (Flinger `SurfaceControl`), bypassing the View hierarchy's GPU compositing.
- `SurfaceView.getHolder().setFixedSize(w, h)` lets the renderer fix the buffer size at the guest's actual framebuffer resolution (e.g. 720×1280) even when the View is scaled to fit the host's screen.
- The `Surface` object is directly passed to the native EGL context via `nativeAddSurface(...)`.

### 3.3 Aspect-ratio handling (VMSurfaceView.onMeasure)

`onMeasure()` reads the VM's `VMResConfig` (resolution from `vMConfig.f8900WWWoWWWo`), computes the right scale factor + 90/180/270° rotation, and sets `SurfaceView.LayoutParams` margins to letterbox/pillarbox the content. The selected rotation is then sent to the native renderer as the `rotation` parameter of `nativeAddSurface()`.

### 3.4 Touch input

`VMSurfaceView.onTouch()` is a multi-touch handler. For each `MotionEvent`:

1. Get the pointer ID, x, y, event time
2. If rotation != 0, transform (x, y) by the inverse rotation
3. Divide by the scale factor (so coordinates are in guest's framebuffer space)
4. Forward to `VMInstance.f8941WWoWWo.m5131WWWWWWWW(action, pointerId, eventTimeNanos, x, y)` — which is `InputService.nativeOnTouchEvent(ptr, action, pointerId, eventTime, x, y)`.

So the input pipeline is:

```
Touch on screen
  → VMSurfaceView.onTouch(view, event)
  → m5234WWWWWWWW(action, pointerIndex, event)
  → InputService.m5131WWWWWWWW(action, pid, t, x, y)
  → InputService.nativeOnTouchEvent(ptr, action, pid, t, x, y)
  → libvm.so writes the touch event to /dev/input/touch
  → guest's EventHub reads it
```

### 3.5 Key press handling

`VMDisplayActivity.onKeyDown/onKeyUp` intercepts KEYCODE_BACK (4), VOLUME_UP (24), VOLUME_DOWN (25), and VOLUME_MUTE (164) and forwards them to the guest via `vMInstance.m5086WWoWWo(keyCode, action)` (which is `RunnableC1621WWWWWWWW` posted on the VM handler thread — eventually `nativeOnKeyEvent` or similar).

---

## 4. ROM management — download, extract, configure

### 4.1 RomConfig schema (`com.android.vmcore.RomConfig`)

The RomConfig JSON (fetched from server, stored in SharedPreferences under key `rom_config`) has these fields (decoded from the StringFog calls in `RomConfig.m5047WWWWWWWW()`):

| JSON key           | Java field            | Type     | Meaning                                                  |
|--------------------|-----------------------|----------|----------------------------------------------------------|
| `id`               | `f8846WWWWWWWW`       | String   | Server-side ROM ID                                       |
| `display_name`     | `f8845WWWWoWWWWo`     | String   | e.g. `"Android 7.1.2"`                                   |
| `rom_version`      | `f8853WWWoWWWo`       | int      | Major Android version (4/5/7/9/11)                       |
| `minimum_sdk_int`  | `f8847WWWWWWWW`       | int      | Min host SDK to boot this ROM                            |
| `support_a64`      | `f8848WWWWWWWW`       | boolean  | 64-bit ABI supported                                     |
| `support_a32`      | `f8855WWoWWo`         | boolean  | 32-bit ABI supported                                     |
| `minimum_app_ver`  | `f8849WWWWWWWW`       | int      | Min host app version                                     |
| `min_app_version`  | `f8850WWWWWWWW`       | String   | Min app version (string form)                            |
| `rom_uri`          | `f8854WWWoWWWo`       | String[] | List of mirror URLs (HTTPS, asset://, file://)           |
| `overlay_uri`      | `f8851WWWWWWWW`       | String[] | Overlay package URLs                                     |
| `magisk_uri`       | `f8852WWWWWWWW`       | String   | Magisk URL (defaults to `asset:///plugins/magisk.zip`)   |
| `su_uri`           | `f8857WWWW`           | String   | Superuser URL                                            |
| `xposed_uri`       | `f8858WoWo`           | String   | Xposed URL                                               |
| `play_uri`         | `f8856WWoWWo`         | String   | GApps URL                                                |

The same RomConfig object is also persisted as JSON in `SharedPreferences` for offline boot.

### 4.2 Where the ROM lives on disk

For each VM with id `N`, the layout is (decoded from VMInstance constructor):

```
/data/data/com.clone.android.dual.space/
├── lib64/                                ← app native libs (extracted by PackageManager)
│   ├── libvm.so                          ← the JNI library
│   ├── libkr32.so / libkr64.so           ← kernel-replacement (A 7.1.2 default)
│   ├── libkr32.11.so / libkr64.11.so     ← kernel-replacement (A 11)
│   └── libOpenglRender.so?               ← (maybe — see disassembly report)
├── vm/
│   └── vmN/                              ← per-VM data dir (f8867WWWWWWWW)
│       ├── fs/                           ← extracted ROM filesystem (f8868WWWWWWWW)
│       │   ├── system/                   ← system partition
│       │   │   ├── build.prop
│       │   │   ├── bin/app_process32, app_process64, sh, su
│       │   │   ├── lib/libui.so, libhostlibui.so, …
│       │   │   ├── priv-app/GoogleServicesFramework/, Phonesky/, PrebuiltGmsCore/
│       │   │   └── framework/XposedBridge.jar
│       │   ├── vendor/                   ← vendor partition (Treble only)
│       │   │   ├── build.prop
│       │   │   └── etc/vintf/manifest/vibrator-default.xml
│       │   └── ...
│       ├── dev/                          ← virtual device nodes (created by libvm.so)
│       │   ├── event                     ← the LocalServerSocket for IPC (see §5)
│       │   ├── binder                    ← per-VM virtual binder
│       │   ├── qemu_pipe                 ← GL command transport
│       │   ├── input/touch               ← virtual touch
│       │   └── audio                     ← virtual audio
│       └── ...
├── cache/                                ← temporary download cache
│   └── rom.zip_N                         ← downloaded ROM archive (deleted after extract)
└── shared_prefs/
    └── vm_config_N.xml                   ← per-VM SharedPreferences (RomConfig etc.)
```

Note: the Java code calls `dataDir.replace("/data/user/0/", "/data/data/")` — this is because `getApplicationInfo().dataDir` returns the multi-user-aware path (`/data/user/N/<pkg>`) but the chroot-style paths inside the guest expect the canonical `/data/data/<pkg>` form.

### 4.3 Download + decrypt + extract pipeline

The installer is `com.android.vmcore.installer.ImageInstallerV1`. The flow:

```
InstallFsTask.mo5036WWWoWWWo(vMApp, vMInstance)
  │
  │  For each uri in romConfig.rom_uri:
  │    ImageInstallerV1.m5205WWWoWWWo(vMConfig, uris, destDir, installOptions)
  │      └─ parallel executor (cachedThreadPool, parallelism=-4):
  │           for each uri: m5204WWWWWWWW(vMConfig, uri, destDir, installOptions)
  │
  │  m5204WWWWWWWW(vMConfig, uri, destDir, installOptions):
  │    cachePath = cacheDir / (uri.lastPathSegment + "_" + vmId)
  │    FileDeleteUtils.delete(cachePath)
  │
  │    out = m5202WWWWWWWW(uri, cachePath, installOptions):
  │      │  queryParam = uri.getQueryParameter("e")
  │      │  if "n":  → BufferedOutputStream(FileOutputStream(cachePath))    // no encryption
  │      │  if "x":  → BufferedOutputStream(XOROutputStream(FOS, KEY))     // XOR with KEY
  │      │  else  :  → BufferedOutputStream(CipherOutputStream(FOS, AES-ECB-PKCS5, KEY))
  │      │
  │      │  KEY = "%z89aviCM0KkbEs9"  (decoded from StringFog)
  │      │  AES algorithm = "AES" (Java default → AES/ECB/PKCS5Padding)
  │      │  XOR uses the same 16-byte key (Vigenère-style, byte-wise)
  │
  │    if uri starts with "asset:///":
  │       AssetsUtils.copy(uri, out)             // read from APK assets
  │    elif uri contains "!/":
  │       FileCopyUtils.copy(file_uri, internal_path, out)   // user-imported ROM
  │    else:
  │       FileCopyUtils.copy(uri, out)           // HTTP download
  │
  │    extract: m5201WWWWoWWWWo(uri, cachePath, destDir, …):
  │      │  queryParam = uri.getQueryParameter("m")
  │      │  if "7z" AND Process.is64Bit():
  │      │     AndUn7z.extract(cachePath, destDir, …)        // 7zip extraction
  │      │  else:
  │      │     ZipHelper.extract(cachePath, destDir, …, /* aesInline = false */)
  │
  │    FileDeleteUtils.delete(cachePath)         // clean up downloaded archive
  │
  └─ vMInstance.m5077WWWoWWWo(overlays)          // remember overlay URIs for ApplyOverlaysTask
```

### 4.4 The four bundled plugins

The four `assets/plugins/*.zip` files are AES-128-ECB-encrypted with key `%z89aviCM0KkbEs9` (verified in `VIRTUAL_MASTER_FULL_ANALYSIS.md`). They are layered on top of the ROM filesystem by four startup tasks:

| Plugin ZIP       | Startup Task        | Destination in `/fs/`                                          |
|------------------|---------------------|----------------------------------------------------------------|
| `play.zip`       | `GooglePlayTask`    | `/system/priv-app/{GoogleServicesFramework,Phonesky,PrebuiltGmsCore}/` |
| `magisk.zip`     | `MagiskTask`        | `/system/sbin/magisk32, magisk64, busybox` + patches `init.rc`         |
| `xposed.zip`     | `XposedTask`        | `/system/app/XposedInstaller/`, `/system/framework/XposedBridge.jar`   |
| `superuser.zip`  | `SuperuserTask`     | `/system/app/Superuser/`, `/system/xbin/daemonsu`                      |

The `Bug1FixTask`…`Bug8FixTask` are ROM-version-specific patches (e.g. fixing `app_process32`/`app_process64` symlinks, fixing `libui.so` ABI selection for A 5.1 vs A 7.1 vs A 10, etc.).

### 4.5 ROM catalog and `pad://` URI scheme

The bundled ROM catalog (in `r3/C3947WWWWWWWW.java`, decoded by the previous agent) registers six `RomModel` entries with synthetic URIs `pad://rom_4_2_2`, `pad://rom_5_1_1`, `pad://rom_7_1_2_32`, `pad://rom_7_1_2`, `pad://rom_9_0_0`, `pad://rom_11_0_0`. The app resolves these to real HTTPS URLs via `https://api.virtualmaster.app/account/v1/...` (gated behind an auth flow that calls `VerifyAssertionRequest` / `GetTokenResponse` — see `h2/C2687WWWWWWWW.java`).

User-imported ROMs use the `!/rom.zip` APK-internal-archive syntax: e.g. `file:///sdcard/MyRom.apk!/rom.zip` — Virtual Master treats the imported file as a wrapper archive containing `rom.zip` at its root.

---

## 5. IPC with the guest

There are **three** IPC channels between host Java code and guest Android:

### 5.1 Channel A — Unix domain socket (`/dev/event`)

**Java side:** `com.android.vmcore.bridge.VMEventManager`

In `m5115WWWWWWWW()` (called from `VMInstance.m5100WoWo` after the HAL services start), a background `Thread` is spawned that:

1. Creates a `LocalServerSocket` bound to a filesystem path:
   ```
   <vMConfig.f8867WWWWWWWW>/dev/event
   = /data/data/com.clone.android.dual.space/vm/vmN/dev/event
   ```
   (the literal suffix `/dev/event` is decoded from StringFog)
2. Calls `accept()` in a loop until `f8990WWWoWWWo` (stop flag) is set or VM state goes to `-5`
3. Wraps the accepted `LocalSocket`'s I/O streams in `DataInputStream` / `DataOutputStream`
4. In a tight loop, calls `readUTF()` to read a single string per event, then splits it on the backtick separator `` ` `` (0x60):
   ```
   eventName`payload
   ```
   (e.g. `com.android.vmcore.action.BOOT_COMPLETED`)
5. Dispatches each event to all registered `IVMEventCallback` instances — primary callback is `VMInstance.mo5013WWWWWWWW(eventName, payload)`.

The event names (decoded from `com.android.vmcore.bridge.VMEvents` — there are ~25 of them):

| Event name                                       | Direction       | Purpose                                           |
|--------------------------------------------------|-----------------|---------------------------------------------------|
| `com.android.vmcore.action.VM_SERVER_READY`      | guest → host    | Guest's IPC server is up                          |
| `com.android.vmcore.action.ANDROID_OS_READY`     | guest → host    | OS is fully booted (post-zygote)                  |
| `com.android.vmcore.action.BOOT_COMPLETED`       | guest → host    | sys.boot_completed=1 — sets VM state to 6         |
| `com.android.vmcore.action.SHUTDOWN`             | guest → host    | Guest is shutting down — sets VM state to 7       |
| `com.android.vmcore.action.SYNC_PROP_EVENT`      | bidirectional   | Set a System property in the guest                |
| `com.android.vmcore.action.START_INSTALL_APP`    | host → guest    | Host asks guest to install an APK                 |
| `com.android.vmcore.action.INSTALL_APP_RESULT`   | guest → host    | Guest reports install result                      |
| `com.android.vmcore.action.START_UNINSTALL_APP`  | host → guest    | Host asks guest to uninstall an APK               |
| `com.android.vmcore.action.UNINSTALL_APP_RESULT` | guest → host    | Guest reports uninstall result                    |
| `com.android.vmcore.action.TASK_STACK_CHANGED`   | guest → host    | Guest's task stack changed (for recent tasks UI)  |
| `com.android.vmcore.action.CLIPBOARD_DATA`       | bidirectional   | Clipboard sync between host and guest             |
| `com.android.vmcore.action.OPEN_OUTER_PAGE`      | guest → host    | Guest requests opening a URL in host browser      |
| `com.android.vmcore.action.TOGGLE_RECENTS`       | guest → host    | Guest requests recents panel                      |
| `com.android.vmcore.action.SEND_KEY_EVENT`       | bidirectional   | Forward a KeyEvent                                |
| `com.android.vmcore.action.SET_LOCALE_AND_TIME`  | host → guest    | Sync host locale/timezone to guest                |
| `com.android.vmcore.action.SET_GLOBAL_SETTING`   | host → guest    | Set a `Settings.Global` value                     |
| `com.android.vmcore.action.SHOW_NAVIGATION_EVENT`| guest → host    | Guest asks to show nav bar                        |
| `com.android.vmcore.action.HIDE_NAVIGATION_EVENT`| guest → host    | Guest asks to hide nav bar                        |
| `com.android.vmcore.action.SET_NAVIGATION_BAR_RTL`| host → guest   | Set nav bar RTL                                    |
| `com.android.vmcore.action.KILL_APP`             | host → guest    | Kill a guest app by package name                  |
| `com.android.vmcore.action.EXPAND_NOTIFICATIONS_PANEL`  | guest → host | Guest wants notification shade             |
| `com.android.vmcore.action.COLLAPSE_NOTIFICATIONS_PANEL`| guest → host | Guest wants notification shade closed      |
| `com.android.vmcore.action.EXECUTE_COMMAND`      | host → guest    | Execute a shell command in the guest              |

The send side is `VMEventManager.m5116WWWoWWWo(eventName, payload)` which does `writeUTF(eventName + "`" + payload); flush()`.

### 5.2 Channel B — Binder virtualization (`/vm%d/dev/binder`)

**Java side:** `com.android.vmcore.service.BinderService`

This is the most subtle channel. The guest Android's `servicemanager` needs to register with `android.os.ServiceManager` to find system services (`activity`, `package`, `window`, etc.). Virtual Master can't let the guest talk to the host's real `/dev/binder` (it would corrupt the host), so it creates a **per-VM virtual binder** at `/vm%d/dev/binder`.

The setup is in `BinderService.m5206WWWWoWWWWo(vMApp, vmId)`:

1. **Reflect into ActivityManager** (`m5207WWWWWWWW(vMApp)`):
   - On API 26+: get the `IActivityTaskManager` interface, call `getID()` on it, then `mActivityTaskManager.getConfiguration()` etc. (this is the hidden-API path that the `me.weishu.reflection` bypass enables)
   - On older APIs: get the legacy `IActivityManager` similarly
   - Get a handle to the system `IBinder` for `android.app.IActivityManager`
2. **Wrap the IBinder with a Java `Proxy`** that intercepts `transact()` calls. The proxy checks the calling thread ID; when the wrapped `transact()` is called from the same thread that called `peekService()` (in step 4), the proxy captures the integer return code into `iArr[0]`.
3. **Install the proxy back** via reflection on `ActivityManager.IActivityTaskManager` (replacing the system's `IActivityTaskManager` field with the proxied one).
4. **Call `peekService(vMApp, new Intent())`** — this triggers the proxy. The captured integer is the system's binder version.
5. **Call native `setupBinder(vmId, binderVersion, 1, 2, "com.android.vmcore.service.IBinderService", parcelledIntent)`** — this is the JNI call into libvm.so that:
   - Creates `/vm%d/dev/binder` (the per-VM virtual binder device, matches disassembly)
   - Sets up a binder-redirect mapping so the guest's `servicemanager` calls for `activity`/`package`/`window`/etc. are proxied back to the host's `BinderService.f9244WWWWoWWWWo` (the `IBinderService.Stub` instance) — which the host then fulfills (or passes through to the real system service with the host's identity)
6. **`bindService(Intent(BinderService.class), serviceConnection, BIND_AUTO_CREATE)`** — waits up to 5 seconds for `onServiceConnected`, returns `-502` on timeout.

The `IBinderService` AIDL (in `com.android.vmcore.service.IBinderService` interface) is just a stub:

```java
public interface IBinderService extends IInterface {
    abstract class Stub extends Binder implements IBinderService { … }
}
```

So the guest makes a binder transaction → libvm.so's virtual binder intercepts it → routes it to the host's `BinderService` (the Java service) → the host fulfills it. This is how the guest's `Context.startActivity()` ends up calling host `VMDisplayActivity.onDialNumberEvent()` for example.

### 5.3 Channel C — `/dev/qemu_pipe` (GL transport)

**Java side:** none (entirely in native).

The guest's SurfaceFlinger speaks the emugl `qemu_pipe` protocol to ship GL commands to the host's `libvm.so`, which has its own EGL context created on the `Surface` we passed in via `nativeAddSurface(...)`. This is the same mechanism as twoyi (see `VIRTUAL_MASTER_FULL_ANALYSIS.md`). There is no Java code involved.

### 5.4 Other HAL services (each is a `LocalSocket`-or-virtual-dev bridge)

`com.android.vmcore.hal.HALManager` is created per-VM in `VMInstance.m5100WoWo()`:

```java
this.f8933WWWWWWWW = new HALManager(vMApp, vMInstance);
this.f8933WWWWWWWW.startHALMgr();
```

Inside, `HALManager.nativeSetup(vmId)` returns a `long` pointer to a native HAL dispatcher. The dispatcher then calls back into Java via private methods (named `CameraConnect`, `CameraDisconnect`, `CameraStart`, `CameraStop`, `CameraFocus`, `CameraFrame`, `CameraFlash`, `CameraList`, `EnableSensors`, `DisableSensors`, `CheckSensorsSupport`, `ExecPhoneCommand`, `SetLocation`, `GetWifiScanResults`, `SetWifiEnabled`, …). These JNI callbacks let the guest access host hardware (camera, sensors, GPS, WiFi scan, phone state, battery) as if it were native.

The full HAL composition:

| HAL Service           | Java Class                                | What it does                                                          |
|-----------------------|-------------------------------------------|-----------------------------------------------------------------------|
| Display               | `DisplayService`                          | Surface registration (see §3)                                         |
| Input (touch)         | `InputService`                            | Touch event forwarding                                                |
| Audio                 | `AudioService`                            | Audio capture/playback (via `AudioRecord` / `AudioTrack`)             |
| Camera                | `CameraService` (in HALManager)           | Camera1 API proxy (open/start/stop/focus/zoom)                        |
| Sensor                | `SensorService` (in HALManager)           | 12 sensor types (accel/gyro/mag/…) via `SensorManager`                |
| Location              | `LocationService`                         | GPS pass-through + fake-location support                              |
| WiFi scan             | `WiFiService`                             | `WifiManager.getScanResults()` proxy                                  |
| Phone                 | `PhoneService`                            | SIM state, signal strength, SMS, dial (via `TelephonyManager`)        |
| Battery               | `BatteryService`                          | Battery level/charging state (via `ACTION_BATTERY_CHANGED` receiver)  |
| Network               | `NetlinkManager`                          | Virtual network interface (`tun0`) with MAC/IP/gateway/DNS             |
| HW control            | `HWControlService`                        | Hardware buttons (power/volume)                                       |

---

## 6. Application startup → first rendered frame (full timeline)

Here is the complete chronology, with the file:line where each step happens:

```
T=0ms   User taps "Virtual Master" launcher icon
        → ActivityManager starts VMApp (Application)

T=2ms   VMApp.attachBaseContext(base)         [VMApp.java:104]
        ├─ On Android 9+: load me.weishu.reflection.BootstrapClass
        │  from base64-encoded dex, call exemptAll() to bypass hidden-API
        └─ f8424WWWWWWWWWW = this

T=10ms  VMApp.onCreate()                      [VMApp.java:161]
        ├─ Init SharedPreferences (device-protected on API 24+)
        ├─ Set app locale, Firebase RemoteConfig defaults
        ├─ Init Google Ads, billing, package-install IntentFilter
        ├─ Init EventBus (C2467WWWWWWWW.m13950WWWW)
        ├─ synchronized(VMManager.class):
        │    System.loadLibrary("vm")           ← loads libvm.so
        │    VMManager.f8948WWWWWWWW = new VMManager(this)
        │      └─ walks dataDir/vm/vm* to discover existing VMs
        ├─ For each VMInstance: new VMExtension(this, vMInstance)
        └─ On API 31+: init privacy/notification module

T=200ms MainActivity shows VM list (RecyclerView via VMFragment)

T=?     User taps a VM card → VMFragment starts VMStartActivityN
        via VMStartActivity0.m5016WWWW(context, vmId, forceBoot=false, restart=false)

T=?+5   VMStartActivity0.onCreate             [VMStartActivity0.java:136]
        ├─ setContentView(R.layout.activity_vm_start)
        ├─ Find status TextView, Lottie loading animation
        ├─ m5020WWWWWWWW(false):
        │    if vMInstance.f8940WWoWWo < 5:
        │      vMInstance.m5100WoWo(false)    ← START BOOT
        └─ vMInstance.f8939WWWoWWWo.m13950WWWW(this)   ← register EventBus

T=?+10  VMInstance.m5100WoWo(false)            [VMInstance.java:218]
        posted on the per-VM HandlerThread "VMInstance"
        ├─ state = 0 → 1   (post VMStatusEvent(1, 0))
        │   VMStartActivity0.onVMStatusEvent → "Checking environment..."
        ├─ CPU check (CPUUtils.m5240/m5242)
        ├─ Data-dir check (must start with /data/user/ or /data/data/)
        ├─ SDK check (Build.VERSION.SDK_INT >= RomConfig.minimum_sdk_int)
        ├─ App-version check
        ├─ clearZombieProcess loop (NativeHelper.clearZombieProcess + getProcessList)
        │
        ├─ If not installed (f8919WWWW == false) OR romConfig changed:
        │   state = 2   (post VMStatusEvent(2, 0))
        │   VMStartActivity0.onVMStatusEvent → "Installing..." / "Upgrading..." / "Repairing..."
        │   Run setup pipeline sequentially:
        │     PrepareFsTask → InstallFsTask → FixFsTask → CleanFsTask →
        │     ChmodFsTask → CleanCacheTask → FixCPUArchTask → LoadVMPropTask
        │   Each task may post a sub-status string (e.g. "Installing plugin: play.zip")
        │   (Takes seconds-to-minutes depending on ROM size; ~300 MB download for A 7.1.2)
        │
        ├─ state = 3   (implicit) — start all services:
        │   VMStatusEvent(3, 0) → "Starting services..."
        │   ├─ BinderService.m5206WWWWoWWWWo(vMApp, vmId):
        │   │     ├─ reflect into ActivityTaskManager, install IBinder proxy
        │   │     ├─ native setupBinder(vmId, binderVer, 1, 2,
        │   │     │                 "com.android.vmcore.service.IBinderService",
        │   │     │                 parcelledIntent)
        │   │     │   → creates /vmN/dev/binder in libvm.so
        │   │     └─ bindService(BinderService.class) — wait up to 5s
        │   ├─ new InputService → nativeStartService  (creates /dev/input/touch)
        │   ├─ new AudioService → start                (creates /dev/audio)
        │   ├─ new HALManager → startHALMgr:
        │   │     ├─ nativeSetup(vmId) → long ptr
        │   │     ├─ Spawn HandlerThreads for Location/WiFi/Sensor/Battery
        │   │     ├─ registerReceiver(BatteryService, ACTION_BATTERY_CHANGED)
        │   │     └─ nativeStartHALMgr(ptr)  (creates all /dev/* HAL device nodes)
        │   ├─ new DisplayService → nativeStartService(w, h)
        │   │     (initializes the per-VM emugl renderer — opens /dev/qemu_pipe)
        │   ├─ new NetlinkManager → start (creates /dev/netlink_client/*)
        │   │     setNetworkConfig(VMNetworkConfig{ifname="tun0", mac, ip, gw, dns})
        │   └─ VMEventManager.m5115WWWWWWWW():
        │         new Thread() { LocalServerSocket("<vmDataDir>/dev/event"); accept loop }
        │
        ├─ m5084WWoWWo():  state = 4   (post VMStatusEvent(4, 0))
        │   VMStartActivity0.onVMStatusEvent → "Starting..." / "Fixing FS..." / "Applying overlay: ..."
        │   Run startup pipeline sequentially:
        │     ApplyOverlaysTask → Bug1FixTask → … → Bug8FixTask → CleanLogTask →
        │     SuperuserTask → XposedTask → GooglePlayTask → MagiskTask →
        │     BuildTmpfsTask → BuildVMPropTask → BuildExecPathTask
        │
        ├─ int pid = startOS(vmId, dpi, kernelPath)   ← JNI into libvm.so
        │     kernelPath = dataDir + "/lib64"
        │     (libvm.so forks a child process, chroots into <vmDataDir>/fs,
        │      LD_PRELOADs libkr64.so as a "kernel replacement", and exec's
        │      /system/bin/init from the guest ROM)
        │   f8938WWWoWWWo = pid
        │   if (pid < 0) → state = -4, errorCode = -pid + 117000; return
        │
        └─ state = 5   (post VMStatusEvent(5, 0))
            VMStartActivity0.onVMStatusEvent → "OS booting..."

T=?+30s  Guest init starts:
         ├─ /system/bin/init reads /system/etc/init/hw/init.rc
         ├─ mount tmpfs on /tmp, /dev (already done by libvm.so)
         ├─ start servicemanager → registers with /vm0/dev/binder
         ├─ start surfaceflinger → opens /dev/qemu_pipe, sends GL commands
         ├─ start audioserver, cameraserver → /dev/audio, /dev/camera*
         ├─ start zygote → fork system_server
         ├─ system_server boots PackageManagerService, ActivityManagerService, etc.
         │   (these register with /vm0/dev/binder and proxy to host BinderService)
         └─ sys.boot_completed=1 → guest sends event via /dev/event socket:
              "com.android.vmcore.action.BOOT_COMPLETED`"
              VMInstance.mo5013WWWWWWWW(...) → state = 6
              VMStatusEvent(6, 0)
              VMStartActivity0.onVMStatusEvent → "OS ready"
              (state >= 5 → m5021WWoWWo() launches VMDisplayActivity)

T=?+30s  VMDisplayActivity.onCreate            [VMDisplayActivity.java:148]
         ├─ if vMInstance.f8940WWoWWo < 5: finish()  (safety)
         ├─ setSystemUiVisibility(5894)  // immersive fullscreen
         ├─ FrameLayout frameLayout = new FrameLayout(this)
         ├─ VMSurfaceView vMSurfaceView = new VMSurfaceView(this)
         ├─ vMSurfaceView.setVM(vMInstance)   ← bind
         ├─ frameLayout.addView(vMSurfaceView, MATCH_PARENT)
         ├─ setContentView(frameLayout)
         └─ m5011WWoWWo(isPortrait)  ← set orientation, request layout

T=?+30s+ VMSurfaceView.onMeasure → compute aspect-ratio + rotation
         VMSurfaceView.surfaceCreated (Android framework callback)
         ├─ m5235WWWoWWWo(holder):
         │   Surface s = holder.getSurface()
         │   vMInstance.f8945WoWo.m5126WWWWoWWWWo(hashCode())   // remove old
         │   vMInstance.f8945WoWo.m5127WWWWWWWW(hashCode(), s, w, h, rot)
         │     → DisplayService.nativeAddSurface(ptr, hashCode, s, w, h, rot)
         │       → libvm.so binds s to the per-VM emugl ColorBuffer
         │
         └─ FIRST FRAME: SurfaceFlinger's GL commands (received via
              /dev/qemu_pipe) are executed against the host EGL context
              that libvm.so created on Surface s, and the result is
              presented by SurfaceFlinger's HWComposer pipeline.
              The user sees the guest's boot animation / launcher.
```

---

## 7. Differences from twoyi

This section is the most important for the twoyi project. Here are the architectural choices Virtual Master made that twoyi did (or didn't):

| Aspect                          | Twoyi (current)                          | Virtual Master                                                  |
|---------------------------------|------------------------------------------|------------------------------------------------------------------|
| **Native library**              | `libvm.so` + `libOpenglRender.so` + `libloader.so` (3 libs) | Single `libvm.so` (all-in-one)                                  |
| **Application class**           | `io.twoyi.TwoyiApplication`              | `com.android.vmapp.VMApp` (with reflection bypass + Firebase)   |
| **Display activity**            | `io.twoyi.Render2Activity`               | `com.android.vmapp.vm.VMDisplayActivity`                        |
| **Boot progress UI**            | `BootLogTexture` (TextureView overlay)   | `VMStartActivity0..3` (Lottie animation + status text)           |
| **Surface type**                | `TextureView` (originally) / SurfaceView (new) | `SurfaceView` (wrapped in `VMSurfaceView extends FrameLayout`) |
| **Surface→native API**          | AOSP emugl `initOpenGLRenderer()` + `createOpenGLSubwindow()` (global singleton) | **Custom per-VM `DisplayService.nativeAddSurface(ptr, surfaceId, surface, w, h, rot)`** |
| **Multi-VM support**            | No (single VM)                           | **Yes** — up to 4 concurrent VMs (VMStartActivity0..3, taskAffinity `.vm0..3`) |
| **State machine**               | Implicit (boot log lines)                | **Explicit**: 11 states (-5..7) with EventBus events            |
| **Touch input**                 | Unix socket `/dev/input/touch`           | Same path; Java side: `InputService.nativeOnTouchEvent`         |
| **Binder virtualization**       | ❌ None (uses host binder)                | ✅ Per-VM `/vm%d/dev/binder` via `setupBinder()` JNI            |
| **Audio virtualization**        | ❌ None                                   | ✅ `AudioService` (Java) + `/dev/audio` (native)                |
| **Network virtualization**      | ❌ None                                   | ✅ `NetlinkManager` + `tun0` with per-VM MAC/IP/gateway         |
| **Camera/Sensor/GPS HAL**       | ❌ None                                   | ✅ Full HAL proxy (Camera1 API, 12 sensor types, GPS, WiFi scan) |
| **Phone/SMS HAL**               | ❌ None                                   | ✅ `PhoneService` (TelephonyManager proxy)                       |
| **Battery HAL**                 | ❌ None                                   | ✅ `BatteryService` (ACTION_BATTERY_CHANGED receiver)            |
| **ROM source**                  | Bundled `rom.zip` (~100 MB)              | **Downloaded from server** (6 versions, 66–351 MB each)         |
| **ROM encryption**              | None                                     | **AES-128-ECB or XOR** (key `%z89aviCM0KkbEs9`), chosen per-URI by `e=` query param |
| **ROM extraction**              | ZIP                                      | ZIP **or 7z** (chosen by `m=` query param + 64-bit check)       |
| **Add-ons (GApps/Magisk/…)**    | None                                     | 4 AES-encrypted plugin ZIPs bundled in `assets/plugins/`        |
| **IPC channel 1**               | `TwoyiMessenger` (Java-side Handler)     | `VMEventManager` (LocalServerSocket at `/dev/event`)            |
| **IPC channel 2**               | `TwoyiSocketServer` (TCP)                | Binder virtualization (per-VM `/vm%d/dev/binder`)               |
| **Hidden-API bypass**           | None (uses only public APIs)             | `me.weishu.reflection.BootstrapClass.exemptAll()` (embedded dex) |
| **String obfuscation**          | None                                     | StringFog (Vigenère-XOR, per-string key)                        |
| **Native string obfuscation**   | None                                     | Obfuscator-LLVM (per-block XOR, 74 `datadiv_decode` functions)  |
| **Touch coords to guest**       | Direct                                   | Rotated/scaled in Java (`VMSurfaceView.m5234WWWWWWWW`)         |

### 7.1 Key takeaways for twoyi

1. **Per-VM renderer pointer.** Virtual Master's `DisplayService.nativeSetup(vmId, dpi)` returns a `long` that's then passed to every subsequent `nativeAddSurface`/`nativeRemoveSurface`/`nativeStartService` call. This is a much cleaner API than twoyi's global-singleton emugl. **Recommendation: refactor twoyi's `libOpenglRender` to take a per-instance handle.**

2. **Surface-passing is explicit, not via NativeActivity.** Virtual Master shows you don't need `NativeActivity` — just implement `SurfaceHolder.Callback` and call a custom `nativeAddSurface(surface, w, h, rotation)` from `surfaceCreated`. **Recommendation: adopt this pattern in twoyi — it's simpler and supports multiple surfaces.**

3. **The boot state machine matters.** Twoyi currently uses an implicit boot log (line-by-line text). Virtual Master's 11-state explicit machine with EventBus events gives the UI much better feedback ("Installing…", "Fixing FS…", "Starting services…", "OS booting…", "OS ready"). **Recommendation: add a state enum to `TwoyiStatusManager` and emit status events.**

4. **Binder virtualization is the hardest piece.** Virtual Master has it, twoyi doesn't. The Java side is small (`BinderService` ~400 lines) but the native `setupBinder()` JNI is significant — it creates a per-VM `/vm%d/dev/binder` and routes the guest's `servicemanager` calls back to the host. Without this, the guest's `servicemanager` would either fail to start or talk to the host's servicemanager (which is what twoyi does, with limitations — the guest's `getSystemService(ACTIVITY_SERVICE)` returns the *host's* ActivityManager, which is why twoyi's guest can launch host apps but not its own).

5. **ROM download + on-the-fly AES decryption is elegant.** The `CipherOutputStream` wraps `FileOutputStream`, so the cleartext ROM never touches disk during download. **Recommendation: consider this for twoyi if we ever distribute ROMs server-side.**

6. **Multi-VM support is straightforward.** Per-VM data dir (`vm/vmN/fs`), per-VM SharedPreferences (`vm_config_N.xml`), per-VM task affinity, per-VM renderer pointer. **Recommendation: adopt this directory layout in twoyi for future multi-VM support.**

7. **The ROM catalog + `pad://` URI scheme.** Virtual Master separates the *logical* ROM ID (`pad://rom_7_1_2`) from the *physical* download URL (resolved via API call). This lets them swap mirrors without changing the app. **Recommendation: not urgent for twoyi (we bundle the ROM), but useful if we ever ship a "ROM store".**

---

## 8. Key class names and their roles

### Application / launching

| Class                                              | Role                                                                |
|----------------------------------------------------|---------------------------------------------------------------------|
| `com.android.vmapp.VMApp`                          | Application — loads libvm.so, creates VMManager, registers callbacks |
| `com.android.vmapp.ui.MainActivity`                | Launcher activity — shows VM list                                   |
| `com.android.vmapp.ui.vm.main.VMFragment`          | The VM list fragment (RecyclerView of cards)                        |
| `com.android.vmapp.ui.vm.main.VMBigCardView`       | Large VM card (with live preview SurfaceView)                      |
| `com.android.vmapp.ui.vm.main.VMSmallCardView`     | Small VM card                                                       |
| `com.android.vmapp.ui.vm.main.VMBigPreviewCardView`| Card with embedded `VMSurfaceView` for live preview                |
| `com.android.vmapp.vm.VMStartActivity0..3`         | Boot progress activity (one per VM task affinity)                   |
| `com.android.vmapp.vm.VMDisplayActivity`           | Fullscreen display activity (hosts the VMSurfaceView)               |
| `com.android.vmapp.vm.VMStopActivity`              | Stop dialog (transparent)                                           |
| `com.android.vmapp.vm.VMReportActivity`            | Crash report activity                                               |
| `com.android.vmapp.vm.VMCoreService`               | Foreground service (keeps the app alive while VM runs)              |
| `com.android.vmapp.vm.VMExtension`                 | Per-VM lifecycle helper (clipboard, receivers, etc.)                |

### Core virtualization (`com.android.vmcore`)

| Class                                  | Role                                                              |
|----------------------------------------|-------------------------------------------------------------------|
| `VMManager`                            | Singleton — manages the list of VMInstances, creates/loads VMs    |
| `VMInstance`                           | The per-VM state machine. Holds VMConfig, HALManager, DisplayService, etc. |
| `VMConfig`                             | Per-VM configuration (paths, IDs, props) — persisted in SharedPreferences |
| `RomConfig`                            | ROM descriptor (server-fetched) — display_name, rom_uri[], etc.   |
| `VMResConfig`                          | Display resolution config (width, height, dpi, color depth)       |
| `VMNetworkConfig`                      | Network config (ifname, mac, ip, gateway, dns)                    |
| `NativeHelper`                         | Static JNI helpers (chmodRecursively, getProcessList, clearZombieProcess) |
| `StringFog`                            | Vigenère-XOR string deobfuscator (`m17835WWWWWWWW(byte[], byte[])`) |
| `VMProcessInfo`                        | A guest process info (pid, name)                                  |

### Setup pipeline (`com.android.vmcore.setup`)

| Class                | Role                                                                |
|----------------------|---------------------------------------------------------------------|
| `IVMSetupTask`       | Interface for setup tasks (run at install time, state 2)            |
| `PrepareFsTask`      | chmod existing fs dir; mkdir the VM data dir                        |
| `InstallFsTask`      | Download + decrypt + extract ROM (delegates to ImageInstallerV1)    |
| `FixFsTask`          | Fix fs paths/symlinks (ROM-version-specific patches)                |
| `CleanFsTask`        | Remove stale cache/tmp files                                        |
| `ChmodFsTask`        | `NativeHelper.chmodRecursively(fsDir, DEFAULT_LINK_PERM)`           |
| `CleanCacheTask`     | Clear caches                                                        |
| `FixCPUArchTask`     | Rewrite `/system/bin/app_process{32,64}_xposed` shims              |
| `LoadVMPropTask`     | Parse `/system/build.prop` into `VMConfig.f8870` HashMap            |

### Startup pipeline (`com.android.vmcore.startup`)

| Class                | Role                                                                |
|----------------------|---------------------------------------------------------------------|
| `IVMStartupTask`     | Interface for startup tasks (run at boot time, state 4)             |
| `ApplyOverlaysTask`  | Copy `/system/product` overlay files                                |
| `Bug1FixTask..Bug8FixTask` | ROM-version-specific bug fixes                               |
| `CleanLogTask`       | Clear log files                                                     |
| `SuperuserTask`      | Extract `superuser.zip` → `/system/app/Superuser/`, `/system/xbin/daemonsu` |
| `XposedTask`         | Extract `xposed.zip` → `/system/app/XposedInstaller/`, `/system/framework/XposedBridge.jar` |
| `GooglePlayTask`     | Extract `play.zip` (GApps) → `/system/priv-app/{GoogleServicesFramework,Phonesky,PrebuiltGmsCore}/` |
| `MagiskTask`         | Extract `magisk.zip` → `/system/sbin/...`, patch `init.rc` with magisk service entries |
| `BuildTmpfsTask`     | Mount tmpfs on /tmp, /dev, etc. (via native)                        |
| `BuildVMPropTask`    | Write `/system/build.prop` (PIE/build fingerprint/Build.ID)         |
| `BuildExecPathTask`  | Set PATH and exec dirs                                              |

### Installer (`com.android.vmcore.installer`)

| Class                | Role                                                                |
|----------------------|---------------------------------------------------------------------|
| `ImageInstaller`     | Interface for image installers                                      |
| `ImageInstallerV1`   | The V1 installer: parallel download, AES/XOR decrypt, ZIP/7z extract |
| `XORInputStream`     | Vigenère-XOR input stream (for `e=x` mode)                          |
| `XOROutputStream`    | Vigenère-XOR output stream                                          |

### HAL services (`com.android.vmcore.hal`)

| Class              | Role                                                              |
|--------------------|-------------------------------------------------------------------|
| `HALManager`       | Top-level HAL manager — owns all the per-domain services          |
| `DisplayService`   | Surface registration (nativeAddSurface/nativeRemoveSurface)      |
| `InputService`     | Touch event forwarding (nativeOnTouchEvent)                       |
| `AudioService`     | Audio capture/playback                                            |
| `CameraService`    | Camera1 API proxy                                                 |
| `SensorService`    | 12 sensor types proxy                                             |
| `LocationService`  | GPS pass-through + fake-location                                  |
| `WiFiService`      | WifiManager.getScanResults proxy                                  |
| `PhoneService`     | TelephonyManager proxy (SIM, signal, SMS, dial)                   |
| `BatteryService`   | ACTION_BATTERY_CHANGED receiver                                   |
| `HWControlService` | Hardware buttons (power/volume)                                   |
| `NetlinkManager`   | Virtual network interface (tun0)                                  |
| `phone.*`          | Phone HAL helpers (CallPdu, GsmAlphabet, CarrierManager, …)       |

### IPC bridge (`com.android.vmcore.bridge`)

| Class              | Role                                                              |
|--------------------|-------------------------------------------------------------------|
| `LocalServerSocket`| Wraps `android.net.LocalServerSocket` with reflection (to call hidden `listen(backlog)` method) |
| `VMEventManager`   | The event thread — accepts connections, reads UTF events, dispatches to callbacks |
| `VMEvents`         | Constants for all 25+ event names (`com.android.vmcore.action.*`) |
| `IVMEventCallback` | Interface for event callbacks                                     |

### Binder service (`com.android.vmcore.service`)

| Class              | Role                                                              |
|--------------------|-------------------------------------------------------------------|
| `BinderService`    | The host-side binder service. Sets up per-VM virtual binder via `setupBinder()` JNI. Implements `IBinderService.Stub`. |
| `IBinderService`   | AIDL interface (empty stub — used only as the binder interface descriptor `"com.android.vmcore.service.IBinderService"`) |

### UI (`com.android.vmcore.ui`)

| Class              | Role                                                              |
|--------------------|-------------------------------------------------------------------|
| `VMSurfaceView`    | The `FrameLayout` wrapping a `SurfaceView`. Implements `SurfaceHolder.Callback` and `OnTouchListener`. The bridge between Android framework's Surface lifecycle and the native renderer. |

### App management (`com.android.vmcore.app`)

| Class              | Role                                                              |
|--------------------|-------------------------------------------------------------------|
| `VMAppManager`     | Install/uninstall/list apps inside the guest (uses the event socket to send `START_INSTALL_APP` etc.) |
| `InstallAppResult` | Result of an install-app operation                                |

### Events (`com.android.vmcore.event`)

| Class              | Role                                                              |
|--------------------|-------------------------------------------------------------------|
| `VMStatusEvent`    | state + errorCode + subStatus string                              |
| `VMConfigEvent`    | VM config changed                                                 |
| `VMCreationEvent`  | A new VM was created                                              |
| `VMDeletionEvent`  | A VM was deleted                                                  |
| `VMStatusEvent`    | State transition                                                  |
| `ShutdownEvent`    | VM shutdown requested                                             |
| `PermissionEvent`  | Guest requests a permission (with `IPermissionResultCallback`)    |
| `DialNumberEvent`  | Guest wants to dial a phone number                                |
| `SendSmsEvent`     | Guest wants to send an SMS                                        |
| `AppAddEvent`/`AppDelEvent` | App installed/uninstalled in guest                       |
| `ResetEvent`       | Reset VM to factory state                                         |

---

## 9. Reproducibility

### How to reproduce this analysis

```bash
# On codespace twoyi-dev-3-jr47xg6xvx7ghq6p:
# 1. Download jadx (already done by previous agent at /tmp/jadx/)
# 2. Decompile (already done — output at /tmp/jadx-out/sources/)
/tmp/jadx/bin/jadx -d /tmp/jadx-out --no-res /tmp/vm.apk

# 3. Also extract resources (manifest):
/tmp/jadx/bin/jadx -d /tmp/jadx-resources --no-src /tmp/vm.apk
# AndroidManifest is at /tmp/jadx-resources/resources/AndroidManifest.xml

# 4. Read the key files in this order:
#    /tmp/jadx-out/sources/com/android/vmapp/VMApp.java
#    /tmp/jadx-out/sources/com/android/vmcore/VMManager.java
#    /tmp/jadx-out/sources/com/android/vmcore/VMInstance.java      ← 1238 lines, the core
#    /tmp/jadx-out/sources/com/android/vmcore/VMConfig.java
#    /tmp/jadx-out/sources/com/android/vmapp/vm/VMDisplayActivity.java
#    /tmp/jadx-out/sources/com/android/vmapp/vm/VMStartActivity0.java
#    /tmp/jadx-out/sources/com/android/vmcore/ui/VMSurfaceView.java
#    /tmp/jadx-out/sources/com/android/vmcore/hal/DisplayService.java
#    /tmp/jadx-out/sources/com/android/vmcore/hal/InputService.java
#    /tmp/jadx-out/sources/com/android/vmcore/hal/HALManager.java
#    /tmp/jadx-out/sources/com/android/vmcore/bridge/VMEventManager.java
#    /tmp/jadx-out/sources/com/android/vmcore/bridge/VMEvents.java
#    /tmp/jadx-out/sources/com/android/vmcore/service/BinderService.java
#    /tmp/jadx-out/sources/com/android/vmcore/installer/ImageInstallerV1.java
#    /tmp/jadx-out/sources/com/android/vmcore/setup/*.java        ← all 8 tasks
#    /tmp/jadx-out/sources/com/android/vmcore/startup/*.java       ← all 17 tasks

# 5. Decode StringFog strings (Vigenère-XOR with per-string key):
#    (decoder script at /home/z/my-project/vm-java-src/decode_sf.py)
python3 decode_sf.py /tmp/jadx-out/sources | grep -i 'boot\|surface\|socket\|binder\|/dev/\|/system/\|/vm'
```

### Key StringFog-decoded strings (verified during this analysis)

| Source location                         | Decoded value                                       |
|-----------------------------------------|-----------------------------------------------------|
| `VMApp.java:283`                        | `"vm"` (the `System.loadLibrary("vm")` argument)   |
| `VMEventManager.java:134` (socket name) | `"/dev/event"` (suffix appended to vmDataDir)       |
| `VMEventManager.java:82` (event separator) | `` "`" `` (backtick, 0x60)                       |
| `BinderService.java:86` (interface token) | `"android.app.IActivityManager"`                  |
| `BinderService.java:77` (component)     | `"com.android.vmcore.service.BinderService"`        |
| `BinderService.java:91` (intent action) | `"com.android.vmcore.service.IBinderService"`       |
| `VMInstance.java:166` (find path)       | `"/data/user/0/"` (replaced with `/data/data/`)    |
| `VMInstance.java:167` (VM dir suffix)   | `"vm/vm"` (+ vmId → `vm/vm0`, `vm/vm1`, …)         |
| `VMInstance.java:168` (fs dir suffix)   | `"/fs"`                                              |
| `VMInstance.java:169` (kernel path)     | `"/lib64"` (app's lib64 dir, contains libvm.so + libkr*.so) |
| `VMManager.java:269`                    | `"rom_config"` (SharedPreferences key for RomConfig) |
| `VMConfig.java:217,220,223,226`         | `"asset:///rom/rom_7_1_2/{magisk,superuser,xposed,play}.zip"` (legacy paths, migrated to `asset:///plugins/`) |
| `VMEvents.java:28-53`                   | All 25+ `com.android.vmcore.action.*` event names  |
| `ImageInstallerV1.java:79`              | AES key `"%z89aviCM0KkbEs9"` (also used for XOR)    |
| `ImageInstallerV1.java:81`              | Cipher algorithm `"AES"` (= AES/ECB/PKCS5Padding)  |
| `ImageInstallerV1.java:82`              | SecretKeySpec algorithm `"AES"`                     |
| `RomConfig.java:78,79,80,81,82,83,84,85,86,87,88,89,90,91` | JSON keys: `id`, `display_name`, `rom_version`, `minimum_sdk_int`, `support_a64`, `support_a32`, `minimum_app_ver`, `min_app_version`, `rom_uri`, `overlay_uri`, `magisk_uri`, `su_uri`, `xposed_uri`, `play_uri` |

---

## 10. Conclusion

Virtual Master's Java side is a well-architected, modular Android-in-Android container with:

- A clear state machine driving the boot sequence (11 states, EventBus-dispatched)
- A two-stage install/boot pipeline (8 setup tasks → 10 startup tasks)
- Per-VM data isolation (`vm/vmN/fs`, `vm_config_N.xml`, per-VM binder device)
- A clean rendering API that takes a Surface directly via `nativeAddSurface` (no NativeActivity needed)
- A two-channel IPC model (Unix socket for events, virtual binder for system services)
- A comprehensive HAL proxy (display, input, audio, camera, sensor, location, wifi, phone, battery, network)
- Server-side ROM distribution with on-the-fly AES/XOR decryption
- An explicit JNI contract that makes the native libvm.so easily replaceable

The most actionable insights for twoyi are:

1. **Adopt the `nativeAddSurface(ptr, surfaceId, surface, w, h, rot)` API** instead of the global-singleton emugl API. This unblocks multi-VM and multi-surface (preview + fullscreen).
2. **Add an explicit state machine** with EventBus events to TwoyiStatusManager — gives the UI proper boot feedback.
3. **The hardest piece to copy is binder virtualization** — without it, the guest's `servicemanager` either fails or talks to the host's. This is a significant native-side project.
4. **The ROM download + AES decrypt pipeline** is a clean pattern if twoyi ever moves to server-side ROM distribution.

— End of analysis —
