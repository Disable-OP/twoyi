# libvm.so Deep Analysis — Complete JNI Surface & Rendering Pipeline

> **Date:** 2026-08-07
> **Binary:** libvm.so (9.7MB, AArch64, Android NDK r23b)
> **Method:** Static analysis with Capstone disassembler + XOR brute force

---

## 1. Complete JNI Surface (65 methods, 12 classes)

All class/method names were XOR-obfuscated per-string. Recovered via relocation analysis + XOR brute force:

| Class | Methods | Key Functions |
|-------|---------|---------------|
| `VMInstance` | 1 | `startOS(IILString;)I` — spawns guest VM |
| `BinderService` | 2 | `setupBinder(IIIILString;[B)I` — binder hook |
| `InputService` | 5 | `nativeOnTouchEvent(JIIJFF)Z` |
| `AudioService` | 4 | `nativeStartService(J)I` |
| `DisplayService` | 7 | `nativeAddSurface(JILSurface;IIF)Z`, `nativeGetFPS(J)F` |
| `HALManager` | 10 | GPS/camera/sensor/phone/WiFi injection |
| `NetlinkManager` | 8 | WiFi scan/connect/disconnect + network config |
| `VpnManager` | 6 | TUN fd management |
| `VpnTunService` | 6 | TUN loop init/start/stop |
| `NativeHelper` | 6 | `clearZombieProcess`, `chmodRecursively` |
| `OsExt` | 8 | xattr operations (set/get/list) |
| `KLog` | 2 | Kernel log |

## 2. Rendering Pipeline

### emugl ColorBuffer Model:
```
Java Surface → ANativeWindow_fromSurface → emugl createOpenGLSubwindow
  → Guest GLES draws into ColorBuffer (EGL pbuffer/FBO)
  → setWindowSurfaceColorBuffer binds ColorBuffer to display
  → repaintOpenGLDisplay blits to ANativeWindow
  → setPostCallback for frame observation
```

### GLSL Shaders (TextureDraw rotation blit):
- Vertex: rotation matrix (cs/sn uniforms), position/inCoord attributes
- Fragment: texture2D sample, GLES2-style (gl_FragColor)

### GL Dispatch:
- Statically-linked `init_gles1_dispatch` / `init_gles2_dispatch`
- No runtime dlsym of GL functions
- GLES1 + GLES2 supported, no Vulkan

## 3. QEMU Pipe Protocol

- **Magic**: `GEGLA` (emugl address_space device handshake)
- **Transport**: Socket-based (NOT /dev/qemu_pipe char device)
  - 369 `socket()` calls, 246 `bind()`, 819 `setsockopt()`
  - 8 listen/accept/connect sites = 8 pipe service endpoints
- **Channels**: `STARTgps` (qemud multiplexer), opengles, sensors, camera, telephony, wifi
- **Command pipe**: `pipe2()` for in-process signaling ("No data on command pipe!")

## 4. Networking (libslirp + TUN + VPN)

### libslirp (statically linked):
- Full TCP/UDP/ICMP/ICMPv6 user-mode NAT stack
- Entry points: `slirp_new`, `slirp_input`, `slirp_output`, `ip_input`, `ip_output`, `tcp_input`, `tcp_output`, `udp_input`, `icmp_input`, etc.

### TUN/VPN:
- `VpnTunService.nativeInitTunLoop(int, int[], int[])` — allocates TUN fd pair
- Android `VpnService.Builder` TUN fd plumbed into slirp
- Per-VM TUN: `nativeAddVMTun(JII)I` / `nativeDelVMTun(JII)I`

### NetlinkManager:
- `NETLINK_ROUTE` socket for host route/carrier monitoring
- WiFi scan results / connect / disconnect events replayed to guest

## 5. HAL Virtualization

### GPS:
- `nativeGPSNmeaChanged(JLString;)V` — NMEA sentence injection
- Uses `sendto()` + `creat()` for qemud:gps channel
- Protocol: `STARTgps\n` handshake then NMEA sentences

### Sensors:
- `nativeSensorChanged(JIJFFF)V` — (handle, type, timestamp, x, y, z)
- 3-axis format matches goldfish sensor device

### Camera:
- `nativeCameraPicture(JLString;[B)V` — JPEG/preview frame injection
- `nativeCameraPreview(JLString;[B)V` — live preview frames
- String arg = mime type (e.g. "image/jpeg")

### Phone/Telephony:
- `nativePhoneUnsolicited(JLString;)V` — RIL URC injection
- Call state, SMS, signal strength

### WiFi:
- `nativeWIFIChanged(JLString;)V` — state changes
- `nativeOnWifiScanResultsChanged(JLList;)V` — scan results
- `nativeOnWifiConnected(J)V` / `nativeOnWifiDisconnected(J)V`

### Battery:
- Via NetlinkManager uevent for power_supply
- `qon_` prefix pattern suggests `qon_battery_*` family

## 6. Boot Flow

```
VMInstance.startOS(vmid, width, height, rootPath)
  → internal 0x4c66ac
  → strlen×12 (command line assembly)
  → allocate VMID, configure framebuffer
  → set guest rootfs path
  → spawn VM worker thread
  → DisplayService.nativeStartService brings up renderer
```

## 7. Anti-Tamper

- 109 `.datadiv_decode*` OLLVM string deobfuscation stubs in `.init_array`
- OLLVM opaque predicates on every JNI registration: `(n+1)*n & 1` parity check
- All strings XOR-encrypted with per-string keys (different key for each string)

---

*This analysis was performed entirely statically. No dynamic instrumentation was used.*
