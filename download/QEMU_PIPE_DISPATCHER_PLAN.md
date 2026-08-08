# `qemu_pipe` GL Command Dispatcher — Implementation Plan

> **Task ID:** 2-qemu-pipe-plan
> **Author:** Plan sub-agent (sonnet)
> **Date:** 2026-08-05
> **Status:** PLANNING — no code changes; this document only.
> **Scope:** Detailed implementation plan for replacing the kr64 daemon's
> `qemu_pipe` single-byte echo stub with a real GL command dispatcher
> that routes the guest `SurfaceFlinger`'s GL commands to twoyi's
> AOSP-built `libOpenglRender.so`.

This is the project's #1 functional blocker per
`HONEST_STATUS_CORRECTED.md` ("the renderer can't write to the pipe")
and `FINAL_STATUS.md` ("Create twoyi's OWN `/dev/qemu_pipe` via the
kr64 daemon"). The plan covers the current stub, the AOSP emugl wire
protocol, the host renderer's expectations, the end-to-end data flow,
the threading model, specific functions to implement, and a phased
roadmap.

---

## 0. TL;DR

The kr64 daemon already binds `/dev/qemu_pipe` as a Unix-domain socket
(`devices.rs::create_qemu_pipe`), but the accept thread in
`lib.rs::spawn_accept_thread` is a stub that writes a single 0 byte and
closes — the wrong direction (guest should write first) and the wrong
content (no emugl protocol). Worse, twoyi's `libOpenglRender.so` is
currently built from **only** `twoyi_api.cpp` (a custom stub that just
does `eglClear + eglSwapBuffers`); the full AOSP emugl pipeline —
`RenderServer`, `RenderThread`, `FrameBuffer`, `GLESv1Decoder`,
`GLESv2Decoder`, `renderControl_decoder` — is **not compiled** (see
`app/cpp/emugl/CMakeLists.txt`).

The fix is therefore two-phase:

1. **Phase 0 (renderer side):** Re-enable the full AOSP emugl pipeline
   in `CMakeLists.txt` and rework `twoyi_api.cpp::startOpenGLRenderer`
   so it (a) initializes `FrameBuffer`, (b) starts a `RenderServer`
   listening on the Unix sockets `$TWOYI_ROOTFS/opengles{,2,3}`
   (already wired up by the patched `UnixStream.cpp`), and (c) keeps
   the ANativeWindow → `FrameBuffer::setupSubWindow` binding that
   surfaces the framebuffer to the Java `SurfaceView`.
2. **Phase 1 (kr64 daemon side):** Replace the
   `spawn_accept_thread(device_set.qemu_pipe, "qemu_pipe")` call with
   a new `spawn_qemu_pipe_proxy()` that accepts guest connections,
   reads the channel-name handshake (`pipe:opengles`), opens an
   outbound connection to the matching `/opengles*` Unix socket, and
   pumps bytes bidirectionally between the two.

Once both phases land, the data path is:

```
Guest SurfaceFlinger
  → connect(/dev/qemu_pipe) [Unix socket]
  → write("pipe:opengles")
  → kr64 daemon's qemu_pipe proxy thread
      → connect({rootfs}/opengles) [Unix socket]
      → bidirectional byte pump
  → libOpenglRender.so::RenderServer
  → RenderThread (reads u32 clientFlags + command stream)
  → GLESv1/GLESv2/renderControl decoders
  → host EGL (libOpenglRender's FrameBuffer)
  → ANativeWindow (the Java SurfaceView)
```

This is the smallest set of changes that turns the project's
"init executes, pipe connects" status (per `FINAL_STATUS.md`) into
"the container renders."

---

## 1. Current state of the `qemu_pipe` stub

### 1.1 Device creation — `app/rs/kr64/src/devices.rs:198-206`

```rust
pub fn create_qemu_pipe(rootfs: &str) -> std::io::Result<DeviceSocket> {
    let path = format!("{}/dev/qemu_pipe", rootfs);
    let listener = bind_unix_socket(&path)?;
    Ok(DeviceSocket {
        listener: Some(listener),
        path,
        should_unlink: true,
    })
}
```

`bind_unix_socket()` (lines 155-185) creates the parent directory
(`/dev`), removes any stale socket file, calls `UnixListener::bind`,
and chmods the resulting socket file to `0666` so the guest (which may
run as a different uid inside the chroot) can `connect()`.

**What's correct:**
- The Unix socket file appears at `{rootfs}/dev/qemu_pipe`, which is
  exactly the path the guest's `libEGL` opens.
- The `0666` mode lets the chrooted guest connect.
- `take_listener()` semantics let the worker thread own the FD.

**What's wrong:** nothing on the creation side — the bug is purely in
the accept/dispatch loop (next section).

### 1.2 Accept thread — `app/rs/kr64/src/lib.rs:983-1029`

```rust
fn spawn_accept_thread(mut dev: devices::DeviceSocket, name: &'static str) {
    let listener = match dev.take_listener() { /* ... */ };
    let fd = listener.as_raw_fd();
    let _ = unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) };

    std::thread::Builder::new()
        .name(format!("kr64-accept-{}", name))
        .spawn(move || {
            info!("[KR64][{}] accept thread started (fd={})", name, fd);
            loop {
                match listener.accept() {
                    Ok((mut stream, _addr)) => {
                        info!("[KR64][{}] client connected", name);
                        // Echo a single byte so the guest sees SOME response.
                        use std::io::Write;
                        let _ = stream.write_all(&[0u8]);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(e) => {
                        warning!("[KR64][{}] accept error: {}", name, e);
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        })
        .expect("spawn kr64 accept thread");
}
```

This is wired into `run()` at `lib.rs:935`:

```rust
spawn_accept_thread(device_set.qemu_pipe, "qemu_pipe");
```

**What's wrong:**
1. **Direction mismatch.** The stub writes a 0 byte to the guest
   immediately after `accept()`. The AOSP `qemu_pipe` protocol requires
   the GUEST to write first — the channel name `pipe:opengles` (see
   §2.1). By writing first, the stub corrupts the guest's expected
   read/write ordering, which is exactly why the guest then sees
   `EINVAL` on its subsequent `write()` (see `HONEST_STATUS_CORRECTED.md`
   point 7).
2. **No protocol decoding.** Even if the byte was not written, the stub
   doesn't read the channel name, doesn't route by channel, and doesn't
   keep the connection alive long enough for the guest to send a single
   GL command.
3. **No proxying.** The stub drops the `stream` immediately, so the
   guest's connection is closed before it can exchange any data with
   `libOpenglRender.so`.
4. **Same stub for every device.** The function is shared by
   `qemu_pipe`, `touch`, `key`, `event`, `gb`, `gb2`, `dm-user` (see
   `lib.rs:935-940` + `lib.rs:602`). Replacing it for `qemu_pipe`
   specifically requires either parameterising the dispatcher or
   factoring out a per-device `spawn_*` helper (see §6.2).

### 1.3 Why the guest can't render with the stub

Per `HONEST_STATUS_CORRECTED.md` step 7:

> ❌ **Pipe write fails**: `Failed to write to pipe: Invalid argument (os error 22)`

The guest's libEGL/libGLESv2 (built against AOSP's `qemu_pipe.h`)
opens `/dev/qemu_pipe`, then writes `"pipe:opengles"` as the channel
open message. Twoyi's stub either:

- Wrote a spurious 0 byte first, desynchronising the protocol so the
  guest's next `write()` lands at the wrong offset and the kernel
  rejects it with `EINVAL` (because the byte count doesn't match a
  pending channel-open write the kernel's `qemu_pipe` driver would
  expect), or
- Closed the connection before the guest even sent the channel name,
  so the guest's `write()` returns `EPIPE`/`EINVAL`.

Either way: the GL context creation log line (`[NEW_RENDERER] GL
context created successfully`) is misleading — it fires before the
pipe-write attempt; the write itself fails, the renderer never
initialises, `core.rs::init_renderer` never spawns `./init`, and the
guest stays on the loading screen forever.

---

## 2. The AOSP emugl wire protocol over `qemu_pipe`

This section reconstructs the wire protocol from the AOSP source
(`sdk/emulator/qemu_pipe/`, `device/generic/goldfish-opengl/`,
`external/qemu/android/hw-pipe.*`) cross-referenced with the
in-tree emugl source (`app/cpp/emugl/`).

### 2.1 Channel-open handshake — guest writes first

The guest-side `qemu_pipe_open()` in
`sdk/emulator/qemu_pipe/qemu_pipe.cpp` does:

```c
int qemu_pipe_open(const char* pChannelName) {
    int fd = open("/dev/qemu_pipe", O_RDWR);
    if (fd < 0) return -1;

    // Build "pipe:<channel>" and write it as the channel-open message.
    char buf[256];
    int len = snprintf(buf, sizeof(buf), "pipe:%s", pChannelName);
    if (write(fd, buf, len) != len) { close(fd); return -1; }
    return fd;
}
```

For the OpenGL ES channel, `pChannelName = "opengles"`. So the very
first thing the guest writes to the socket is the **13-byte ASCII
string `"pipe:opengles"`** (no length prefix, no NUL terminator).

The host must:
1. Read up to e.g. 256 bytes (or read until newline/EOF — AOSP's
   implementation reads until the buffer is full or `EAGAIN`, but a
   simpler "read until we see a non-`pipe:`-prefix byte or get
   `EAGAIN`" works in practice).
2. Strip the `"pipe:"` prefix.
3. Look up the channel name and route accordingly.

Twoyi only needs to handle the `"opengles"` channel (with optional
`"opengles2"` / `"opengles3"` aliases that AOSP uses to multiplex
multiple renderer instances — see §2.4).

### 2.2 After channel open — the emugl protocol

Once the channel-open handshake succeeds, the same FD is a
bidirectional byte stream that carries the AOSP **emugl** protocol.
The first message is from the client (guest) → server (renderer):

```
+--------------------+
| u32 clientFlags    |   // 4 bytes, little-endian on aarch64/x86_64
+--------------------+
```

`clientFlags` is a bitmask; bit 0 (`IOSTREAM_CLIENT_EXIT_SERVER = 1`)
asks the renderer to shut down. For a normal session, `clientFlags =
0`. See `app/cpp/emugl/include/libOpenglRender/IOStream.h:100` and
`RenderServer.cpp:74-86`.

After `clientFlags`, the client streams **command packets**. Each
packet has an 8-byte header followed by opcode-specific payload:

```
+--------------------+--------------------+------------------+
| u32 opcode         | u32 packetLen      |  payload bytes   |
+--------------------+--------------------+------------------+
       4 bytes              4 bytes         (packetLen - 8)
```

`packetLen` includes the 8-byte header, so a packet with no payload
has `packetLen = 8`. The decoder loop in
`app/cpp/emugl/generated/gl2_dec/gl2_dec.cpp:15-39` shows this
exactly:

```c
size_t gl2_decoder_context_t::decode(void *buf, size_t len, IOStream *stream) {
    size_t pos = 0;
    if (len < 8) return pos;
    unsigned char *ptr = (unsigned char *)buf;
    while ((len - pos >= 8) && !unknownOpcode) {
        int opcode = *(int *)ptr;                  // bytes 0..3
        unsigned int packetLen = *(int *)(ptr + 4); // bytes 4..7
        if (len - pos < packetLen) return pos;     // wait for more bytes
        switch(opcode) {
            case OP_glActiveTexture: /* ... */     // payload starts at ptr+8
            case OP_glAttachShader:  /* ... */
            // ...
        }
        pos += packetLen;
        ptr += packetLen;
    }
}
```

### 2.3 Opcodes — three namespaces

The emugl protocol has three decoders running side-by-side per
`RenderThread` (see `RenderThread.cpp:46-138`):

| Decoder           | Opcode range  | Header file                                          | Purpose |
|-------------------|---------------|------------------------------------------------------|---------|
| `renderControl`   | 10000-10024   | `generated/renderControl_dec/renderControl_opcodes.h` | Renderer control (create context / surface / colorbuffer, make current, post) |
| GLESv1 (`gl`)     | 1-2047        | `generated/gl_dec/gl_opcodes.h`                      | OpenGL ES 1.0/1.1 commands |
| GLESv2 (`gl2`)    | 2048-...      | `generated/gl2_dec/gl2_opcodes.h`                    | OpenGL ES 2.0 commands |

Each iteration of the decode loop tries all three decoders in
sequence (`RenderThread.cpp:112-135`) — whichever decoder matches
the opcode consumes `packetLen` bytes and the loop continues.

### 2.4 Why `opengles`, `opengles2`, `opengles3`?

Looking at the patched `app/cpp/emugl/OpenglCodecCommon/UnixStream.cpp`
(lines 39-62) and the patched copy at
`app/cpp/emugl/shared/OpenglCodecCommon/UnixStream.cpp`:

```c
static int make_unix_path(char *path, size_t pathlen, int port_number) {
    const char *rootfs = getenv("TWOYI_ROOTFS");
    if (rootfs == NULL || rootfs[0] == 0) {
        rootfs = "/data/data/io.twoyi/rootfs";
    }
    const char *suffix;
    int idx = port_number % 3;
    if (idx == 0)      suffix = "opengles";
    else if (idx == 1) suffix = "opengles2";
    else               suffix = "opengles3";
    snprintf(path, pathlen, "%s/%s", rootfs, suffix);
    return 0;
}
```

So `port % 3` selects one of three Unix socket paths under the
rootfs. The "port" here is the integer passed to
`initOpenGLRenderer(width, height, portNum, ...)` — historically the
TCP port number (default `CODEC_SERVER_PORT = 22468` per
`codec_defs.h`), but in Unix-socket mode it's used as a "renderer
instance index." For twoyi's single-VM case, **port = 0** suffices;
the renderer listens on `$TWOYI_ROOTFS/opengles`.

### 2.5 The guest's view

Guest-side `libEGL` (built from `device/generic/goldfish-opengl/`)
discovers `qemu_pipe` at runtime:

```c
int fd = open("/dev/qemu_pipe", O_RDWR);
write(fd, "pipe:opengles", 13);
// fd is now a bidirectional emugl pipe
unsigned int clientFlags = 0;
write(fd, &clientFlags, 4);
// ... then stream GL command packets ...
```

(Simplified — the real client uses `IOStream`'s buffered `alloc` /
`commitBuffer` pattern from `IOStream.h:44-69`, but the wire bytes
are the same.)

The guest's `SurfaceFlinger` does NOT directly talk to `qemu_pipe`.
Instead:
1. `SurfaceFlinger` loads the goldfish EGL/GLES driver from
   `/vendor/lib64/egl/libEGL_goldfish.so` (or the system's
   `libGLESv2.so` if goldfish is not present).
2. The goldfish driver opens `qemu_pipe` and writes `pipe:opengles`.
3. The driver encodes each EGL/GLES call into a command packet and
   writes it to the pipe.
4. The driver blocks on `read()` for any return value (e.g.
   `glGetIntegerv` needs the result back).

For twoyi to work, the host side must:
- Accept the connection (kr64 daemon).
- Forward it to libOpenglRender's RenderServer.
- The RenderServer reads `clientFlags`, spawns a `RenderThread`.
- The RenderThread reads packets, decodes, executes on host EGL,
  writes any return values back.

---

## 3. What does `libOpenglRender.so` expect?

### 3.1 The current (broken) state — `twoyi_api.cpp`

`app/cpp/emugl/twoyi_api.cpp` (483 lines, the SOLE compiled source
per `CMakeLists.txt`) implements `startOpenGLRenderer()` as a
standalone EGL clear-loop:

```c
int startOpenGLRenderer(void* win, int width, int height,
                        int xdpi, int ydpi, int fps) {
    // eglGetDisplay + eglInitialize + eglChooseConfig + eglCreateContext
    // + eglCreateWindowSurface(win)
    // + pthread_create(render_thread_main)
    //   render_thread_main: glClear + eglSwapBuffers at fps
}
```

It does **NOT**:
- Call `FrameBuffer::initialize()`
- Call `RenderServer::create(portNum)`
- Spawn any `RenderThread`
- Decode any emugl packets
- Open any Unix socket on `/opengles*`

So even if the kr64 daemon's `qemu_pipe` proxy was perfect, there
would be nothing on the host side to receive the proxied connection.
The current `libOpenglRender.so` is essentially a "display a black
screen" stub.

### 3.2 What the original AOSP `initOpenGLRenderer` does

From `app/cpp/emugl/libOpenglRender/render_api.cpp:79-192` (kept in
the tree for reference but NOT compiled):

```c
int initOpenGLRenderer(int width, int height, int portNum,
                       OnPostFn onPost, void* onPostContext) {
    if (s_renderProc || s_renderThread) return false;
    s_renderPort = portNum;

    bool inited = FrameBuffer::initialize(width, height, onPost, onPostContext);
    if (!inited) return false;

    s_renderThread = RenderServer::create(portNum);
    if (!s_renderThread) return false;
    s_renderThread->start();
    return true;
}
```

`RenderServer::create(portNum)` (`RenderServer.cpp:37-61`) allocates
a `UnixStream` and calls `listen(port)`, which (via the patched
`make_unix_path`) binds to `$TWOYI_ROOTFS/opengles`.

`RenderServer::Main()` (`RenderServer.cpp:63-153`) loops:
1. `accept()` on the listening socket.
2. Read `u32 clientFlags` (4 bytes).
3. If `clientFlags & IOSTREAM_CLIENT_EXIT_SERVER`, break.
4. Create a `RenderThread` for the stream, call `rt->start()`.
5. Periodically reap finished threads.

`RenderThread::Main()` (`RenderThread.cpp:46-162`) loops:
1. `readBuf.getData()` — block on the stream until bytes arrive.
2. Try all three decoders (`m_glDec.decode`, `m_gl2Dec.decode`,
   `m_rcDec.decode`) in a tight loop until no more progress.
3. Each decoder consumes some bytes and dispatches the opcode to the
   bound `s_egl.*` / `s_gl.*` / `s_gl2.*` EGL/GL function pointer.

`FrameBuffer::initialize()` (`FrameBuffer.cpp:104-...`) does the
host EGL setup (display, config, pbuffer context, GL dispatch) and
exposes the singleton via `FrameBuffer::getFB()`. The RenderControl
decoder (`RenderControl.cpp`) calls `FrameBuffer::createRenderContext`
/ `createWindowSurface` / `createColorBuffer` / `bindContext` /
`post` to manage host-side GL objects that mirror the guest's.

### 3.3 So: does `startOpenGLRenderer` open the pipe?

**No.** The original AOSP `initOpenGLRenderer` opens its OWN
listening socket (the `RenderServer`) on `/opengles*`. It expects the
EMULATOR (or, in twoyi's case, the kr64 daemon) to **connect TO that
socket** after the guest opens `/dev/qemu_pipe`.

This is the key architectural insight:

```
                AOSP emulator architecture:
                ----------------------------
guest -> /dev/qemu_pipe (goldfish char dev) -> emulator process
                                                  -> connects to 127.0.0.1:22468
                                                  -> libOpenglRender's RenderServer
                                                  -> RenderThread -> GL decoders

                twoyi architecture (proposed):
                ------------------------------
guest -> /dev/qemu_pipe (Unix socket) -> kr64 daemon
                                          -> proxy thread opens /opengles
                                          -> libOpenglRender's RenderServer
                                          -> RenderThread -> GL decoders
```

The kr64 daemon plays the role of "the thing that translates
goldfish-pipe → TCP/Unix-socket", which in the AOSP emulator is done
by `hw-pipe.c` inside the QEMU process.

### 3.4 The renderer is the SERVER, kr64 is the proxy (CLIENT to the renderer)

- `libOpenglRender.so::RenderServer` is the **server** — it `listen()`s
  on `/opengles*` and `accept()`s.
- The kr64 daemon's qemu_pipe proxy thread is a **client to the
  renderer** — for each guest connection it `connect()`s to
  `/opengles*` and shuttles bytes both ways.
- The guest is a **client to the kr64 daemon** — it `connect()`s to
  `/dev/qemu_pipe`.

Two clients, two servers, one proxy in the middle. The proxy's job is
purely byte-shuffling; it does not interpret the emugl protocol at
all.

---

## 4. End-to-end data flow

```
+-----------------+        +-----------------+        +-----------------+
| Guest (init/SF) |        | kr64 daemon     |        | libOpenglRender |
|                 |        | (in host app    |        | (in host app    |
|                 |        |  process)       |        |  process)       |
|                 |        |                 |        |                 |
| /dev/qemu_pipe  |        | /dev/qemu_pipe  |        | /opengles       |
|   (socket)      |        |   (listener)    |        |   (listener)    |
|     ▲           |        |     ▲           |        |     ▲           |
|     | connect() |        |     | accept()  |        |     | accept()  |
|     | write(    |        |     |           |        |     |           |
|     |  "pipe:   |        |     ▼           |        |     ▼           |
|     |   opengles")       |  read channel   |        |  read           |
|     |           |        |  name           |        |  clientFlags    |
|     |           |        |     |           |        |     |           |
|     |           |        |  connect(       |        |     |           |
|     |           |        |   /opengles)----|--------|-----+           |
|     |           |        |     |           |        |                 |
|     | write(    |        |  spawn proxy    |        |  RenderThread   |
|     |  cmd_pkt) |        |  thread (pump   |        |   .decode()     |
|     |           |        |  bytes both     |        |   ↓             |
|     |           |        |  directions)    |        |  GLESv1/v2/RC   |
|     |           |        |     |           |        |  decoders       |
|     |           |        |     |           |        |   ↓             |
|     |           |        |     |           |        |  FrameBuffer    |
|     |           |        |     |           |        |   ↓             |
|     |           |        |     |           |        |  host EGL/GL    |
|     |           |        |     |           |        |   ↓             |
|     |           |        |     |           |        |  ANativeWindow  |
|     |           |        |     |           |        |  (SurfaceView)  |
+-----------------+        +-----------------+        +-----------------+
       (guest rootfs)         (host app data dir)        (host app UI)
```

**Bytes-on-the-wire summary:**

| Direction | First bytes | Meaning |
|-----------|-------------|---------|
| guest → kr64 | `pipe:opengles` (13 bytes ASCII) | channel open |
| kr64 → renderer | (kr64 opens a fresh connection to `/opengles`) | no bytes yet |
| guest → kr64 → renderer | `00 00 00 00` (4 bytes) | `clientFlags = 0` |
| guest → kr64 → renderer | `XX XX XX XX` `LL LL LL LL` `[payload]` | first emugl command packet (8-byte header + payload) |
| ... more packets ... | | |
| renderer → kr64 → guest | (return values for blocking calls like `glGetIntegerv`) | stream of bytes back |

The proxy thread does NOT touch the bytes — it just `recv` from one
FD and `send` to the other, and vice versa. (One important detail:
because emugl uses `read()` rather than `readFully()` on the host
side — see `SocketStream::read()` in `SocketStream.cpp:134-152 — the
proxy is free to forward partial reads, which simplifies the pump.)

---

## 5. Threading model

### 5.1 Per-connection proxy threads

The kr64 daemon already spawns one accept thread per device socket
(`lib.rs:935-940`). The proposed change for `qemu_pipe` is:

```
kr64 main thread
  ├─ spawn "kr64-accept-qemu_pipe" thread  ← takes ownership of listener
  │     └─ loop {
  │          accept() → (guest_stream, _)
  │          read channel name from guest_stream  (e.g. "pipe:opengles")
  │          open /opengles (UnixStream::connect)
  │          spawn "kr64-pipe-proxy-<id>" thread
  │            └─ spawn two sub-threads (or use poll(2)):
  │                 ├─ forward guest → renderer
  │                 └─ forward renderer → guest
  │       }
  ├─ spawn "kr64-accept-touch" thread  ← unchanged (still uses stub for now)
  ├─ spawn "kr64-accept-key" thread    ← unchanged
  ├─ spawn "kr64-accept-event" thread  ← unchanged
  ├─ spawn "kr64-accept-gb" thread     ← unchanged
  ├─ spawn "kr64-accept-gb2" thread    ← unchanged
  ├─ spawn "kr64-accept-dm-user" thread ← unchanged
  ├─ binder proxy thread pool (already implemented)
  ├─ audio pump thread (already implemented)
  ├─ sensor pump thread (already implemented)
  ├─ battery refresh thread (already implemented)
  └─ waitpid(guest_pid)  ← blocks until guest exits
```

### 5.2 Pump implementation: `poll(2)` vs two threads

Each guest connection results in one "proxy session" that needs to
forward bytes in both directions. Two implementation strategies:

**Strategy A — two threads per session:**
- One thread `recv(guest) → send(renderer)`.
- One thread `recv(renderer) → send(guest)`.
- When either side closes, the other thread is cancelled (via a
  shutdown() on its FD or a shared atomic flag).

**Strategy B — single thread + `poll(2)`:**
- One thread `poll()`s both FDs and forwards whichever has data.
- Cleaner teardown (one thread to join).
- Slightly more complex code.

**Recommendation: Strategy A** for the MVP. The reference pattern is
already in `binder.rs` (which uses a thread pool) and `audio.rs`
(which spawns a pump thread per direction). Two threads per session
is fine — a typical session has 1-3 GL contexts and the overhead is
negligible.

### 5.3 How this interacts with the existing renderer thread in `core.rs`

`core.rs::init_renderer` (lines 92-327) already spawns a thread that
calls `startOpenGLRenderer()`. With Phase 0 in place, that call will
start the RenderServer inside the host app process. The kr64 daemon
(the child of the same app process, forked at `lib.rs:790`) inherits
the same data directory and the same `$TWOYI_ROOTFS` env var, so its
proxy thread can `connect()` to the renderer's `/opengles` socket
that lives in the same address space (different process? — see §5.4).

**Wait — is kr64 a separate process or a thread?** Looking at
`lib.rs:790`, kr64 does `libc::fork()` — the child becomes the guest
init, the PARENT becomes the daemon. The daemon IS a separate
process from the host app. So the renderer (started by `core.rs` in
the host app process) and the kr64 daemon (the parent after fork)
are **two separate processes**, but they share the same filesystem
namespace (the fork hasn't yet `pivot_root`-ed), so they can both
see `$TWOYI_ROOTFS/opengles` as a Unix socket path. ✓

So the data flow involves an extra process hop:

```
guest init (PID = fork child)
   ↘
   /dev/qemu_pipe (Unix socket file in shared rootfs)
   ↘
kr64 daemon (PID = fork parent)
   ↘ (Unix socket connect)
   /opengles (Unix socket file in shared rootfs)
   ↘
host app process (Renderer.java's renderer thread)
   ↘
   libOpenglRender's RenderServer + RenderThread
   ↘
   host EGL + ANativeWindow (SurfaceView)
```

### 5.4 Lifecycle and ownership

- The kr64 daemon owns the `/dev/qemu_pipe` listener.
- The host app process owns the `/opengles` listener (started by
  `core.rs::init_renderer` calling `startOpenGLRenderer`).
- Each guest `connect()` to `/dev/qemu_pipe` triggers a kr64-side
  proxy thread that opens a corresponding `connect()` to
  `/opengles`.
- When the guest closes its end (or `init` exits), the proxy thread
  sees `recv() == 0` (EOF) and shuts down the renderer-side socket.
- The renderer's `RenderThread` then exits its decode loop, marks
  itself finished, and is reaped by `RenderServer::Main()`'s periodic
  cleanup pass (`RenderServer.cpp:115-128`).

### 5.5 Ordering: who starts first?

The guest will try to `open("/dev/qemu_pipe")` very early in boot
(as soon as `SurfaceFlinger` starts, which is in the `init`'s
`surfaceflinger` service — typically within the first 2 seconds of
guest boot). The renderer must be listening on `/opengles` BEFORE
that happens, or the kr64 proxy's `connect()` will fail with
`ECONNREFUSED` and the guest will see `EPIPE` on its first write.

`core.rs::init_renderer` (line 154) already spawns the renderer
thread BEFORE spawning `./init` (line 303), so the ordering is
correct **provided** `startOpenGLRenderer` returns promptly. The
current stub returns immediately (after spawning the render thread);
the Phase 0 version must do the same — `RenderServer::start()`
should not block waiting for the first connection.

---

## 6. Specific functions to implement

### 6.1 New Rust module: `app/rs/kr64/src/qemu_pipe.rs`

A new ~250-line module that owns the GL pipe proxy. Mirrors the
structure of `binder.rs` and `audio.rs`. Skeleton:

```rust
//! `qemu_pipe` GL command proxy.
//!
//! Accepts the guest's connection to /dev/qemu_pipe, reads the
//! "pipe:<channel>" channel-open handshake, and forwards the
//! resulting bidirectional stream to libOpenglRender's RenderServer
//! listening on /opengles (or /opengles2 / /opengles3).

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use crate::{info, warning, error};

/// Magic prefix the guest writes immediately after connect.
const PIPE_PREFIX: &str = "pipe:";

/// Default channel name for OpenGL ES.
const CHANNEL_OPENGLES: &str = "opengles";

/// A handle returned by `spawn_qemu_pipe_proxy` that keeps the
/// listener alive. Dropping it shuts the proxy down.
pub struct QemuPipeProxyHandle {
    shutdown: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    path: String,
}

impl QemuPipeProxyHandle {
    pub fn path(&self) -> &str { &self.path }

    /// Shutdown the proxy. Idempotent.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
        // Connecting to the listener wakes up accept() so the thread
        // can observe the shutdown flag and exit.
        let _ = UnixStream::connect(&self.path);
    }
}

impl Drop for QemuPipeProxyHandle {
    fn drop(&mut self) {
        self.shutdown();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Spawn the qemu_pipe proxy. Takes ownership of the listener.
///
/// `rootfs` is the guest rootfs path (so we can compute /opengles
/// from the same rootfs, matching UnixStream.cpp's TWOYI_ROOTFS
/// resolution).
pub fn spawn_qemu_pipe_proxy(
    listener: UnixListener,
    path: String,
    rootfs: String,
) -> std::io::Result<QemuPipeProxyHandle> {
    // Non-blocking so the accept loop can poll the shutdown flag.
    let fd = std::os::unix::io::AsRawFd::as_raw_fd(&listener);
    let _ = unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) };

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    let path_for_thread = path.clone();

    let thread = std::thread::Builder::new()
        .name("kr64-accept-qemu_pipe".into())
        .spawn(move || {
            info!("[KR64][qemu_pipe] proxy thread started (listener={})", path_for_thread);
            let mut next_session_id: u64 = 0;
            loop {
                if shutdown_clone.load(Ordering::Acquire) {
                    info!("[KR64][qemu_pipe] shutdown flag set, exiting accept loop");
                    break;
                }
                match listener.accept() {
                    Ok((guest_stream, _addr)) => {
                        let sid = next_session_id;
                        next_session_id += 1;
                        info!("[KR64][qemu_pipe] guest connected (session={})", sid);
                        let rootfs_clone = rootfs.clone();
                        std::thread::Builder::new()
                            .name(format!("kr64-pipe-handshake-{}", sid))
                            .spawn(move || {
                                if let Err(e) = handle_session(guest_stream, &rootfs_clone, sid) {
                                    warning!("[KR64][qemu_pipe] session {} ended: {}", sid, e);
                                }
                            })
                            .ok();
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                    Err(e) => {
                        warning!("[KR64][qemu_pipe] accept error: {}", e);
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                }
            }
            info!("[KR64][qemu_pipe] proxy thread exiting");
        })?;

    Ok(QemuPipeProxyHandle {
        shutdown,
        thread: Some(thread),
        path,
    })
}

/// Per-connection handler. Reads the channel name, opens the
/// matching renderer socket, and pumps bytes both directions until
/// either side closes.
fn handle_session(
    mut guest: UnixStream,
    rootfs: &str,
    sid: u64,
) -> std::io::Result<()> {
    // Step 1: read the "pipe:<channel>" handshake.
    let channel = read_channel_name(&guest)?;
    info!("[KR64][qemu_pipe] session {} channel = {}", sid, channel);

    if channel != "opengles" && channel != "opengles2" && channel != "opengles3" {
        // Unknown channel — close. (Future: route "audio", "camera", etc.)
        warning!("[KR64][qemu_pipe] session {} unknown channel '{}', closing", sid, channel);
        return Ok(());
    }

    // Step 2: open the matching renderer socket under the same rootfs.
    let renderer_path = format!("{}/{}", rootfs, channel);
    let renderer = UnixStream::connect(&renderer_path)
        .map_err(|e| {
            error!("[KR64][qemu_pipe] session {} connect to {} failed: {}",
                   sid, renderer_path, e);
            e
        })?;

    // Step 3: spawn two pump threads.
    let g2r_done = Arc::new(AtomicBool::new(false));
    let r2g_done = Arc::new(AtomicBool::new(false));

    let guest_for_write = guest.try_clone()?;
    let renderer_for_read = renderer.try_clone()?;
    let r2g_done_clone = r2g_done.clone();
    let g2r_thread = std::thread::Builder::new()
        .name(format!("kr64-pipe-g2r-{}", sid))
        .spawn(move || pump(&mut guest, &mut renderer.try_clone().unwrap(), &g2r_done, &r2g_done_clone))?;

    let g2r_done_clone = g2r_done.clone();
    let r2g_thread = std::thread::Builder::new()
        .name(format!("kr64-pipe-r2g-{}", sid))
        .spawn(move || pump(&mut renderer_for_read, &mut guest_for_write, &r2g_done, &g2r_done_clone))?;

    let _ = g2r_thread.join();
    let _ = r2g_thread.join();
    Ok(())
}

/// Read the "pipe:<channel>" handshake. Returns the channel name.
fn read_channel_name(stream: &mut UnixStream) -> std::io::Result<String> {
    let mut buf = [0u8; 256];
    let mut total = 0;
    while total < buf.len() {
        let n = stream.read(&mut buf[total..])?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "guest closed before sending channel name",
            ));
        }
        total += n;
        // We have at least one byte; check if we have a complete
        // "pipe:<name>" prefix. AOSP writes the channel name in a
        // single write() so the first recv typically has all of it.
        if let Some(name) = parse_channel_name(&buf[..total]) {
            return Ok(name.to_string());
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "channel name too long",
    ))
}

/// If `buf` starts with "pipe:" and contains a printable name, return it.
fn parse_channel_name(buf: &[u8]) -> Option<&str> {
    if !buf.starts_with(PIPE_PREFIX.as_bytes()) { return None; }
    let name_bytes = &buf[PIPE_PREFIX.len()..];
    let end = name_bytes.iter().position(|&b| b == 0 || b < 0x20 || b > 0x7e)
        .unwrap_or(name_bytes.len());
    if end == 0 { return None; }
    std::str::from_utf8(&name_bytes[..end]).ok()
}

/// Bidirectional byte pump. Reads from `from`, writes to `to`.
/// Sets `my_done` when its direction closes; checks `other_done`
/// and exits early if the other direction has closed.
fn pump(
    from: &mut UnixStream,
    to: &mut UnixStream,
    my_done: &Arc<AtomicBool>,
    other_done: &Arc<AtomicBool>,
) {
    let mut buf = [0u8; 16 * 1024];
    loop {
        if other_done.load(Ordering::Acquire) { break; }
        let n = match from.read(&mut buf) {
            Ok(0) => break,                       // EOF
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        if let Err(_) = to.write_all(&buf[..n]) { break; }
    }
    my_done.store(true, Ordering::Release);
    // Signal the other side to wake up (recv returns ECONNRESET).
    let _ = to.shutdown(std::net::Shutdown::Both);
}
```

### 6.2 Changes to `lib.rs`

**Remove** the `spawn_accept_thread(device_set.qemu_pipe, "qemu_pipe")`
call at `lib.rs:935`.

**Replace** with:

```rust
// qemu_pipe → real GL proxy (new module)
let _qemu_pipe_proxy = {
    let dev = device_set.qemu_pipe;   // consume the DeviceSocket
    let listener = dev.take_listener().expect("qemu_pipe listener");
    match qemu_pipe::spawn_qemu_pipe_proxy(
        listener, dev.path.clone(), cfg.rootfs.clone(),
    ) {
        Ok(h) => {
            info!("[KR64] qemu_pipe proxy listening at {} (rootfs={})",
                  h.path(), cfg.rootfs);
            Some(h)
        }
        Err(e) => {
            error!("[KR64] failed to start qemu_pipe proxy: {}", e);
            None
        }
    }
};
```

Hold `_qemu_pipe_proxy` until the end of `run()` so the proxy shuts
down cleanly when the guest exits.

Add `pub mod qemu_pipe;` to the module list at `lib.rs:67-75`.

### 6.3 Phase 0 — Re-enable the AOSP emugl pipeline in CMakeLists.txt

`app/cpp/emugl/CMakeLists.txt` currently has:

```cmake
set(EMUGL_SOURCES
    twoyi_api.cpp
)
```

Replace with the full source list (Phase 0):

```cmake
set(EMUGL_SOURCES
    # entry points (FFI to Rust)
    twoyi_api.cpp
    # renderer core
    libOpenglRender/render_api.cpp
    libOpenglRender/RenderServer.cpp
    libOpenglRender/RenderThread.cpp
    libOpenglRender/FrameBuffer.cpp
    libOpenglRender/RenderControl.cpp
    libOpenglRender/ColorBuffer.cpp
    libOpenglRender/WindowSurface.cpp
    libOpenglRender/RenderContext.cpp
    libOpenglRender/FBConfig.cpp
    libOpenglRender/ReadBuffer.cpp
    libOpenglRender/ThreadInfo.cpp
    libOpenglRender/EGLDispatch.cpp
    libOpenglRender/GLDispatch.cpp
    libOpenglRender/GL2Dispatch.cpp
    # codec common
    shared/OpenglCodecCommon/SocketStream.cpp
    shared/OpenglCodecCommon/UnixStream.cpp
    shared/OpenglCodecCommon/TcpStream.cpp
    shared/OpenglCodecCommon/glUtils.cpp
    shared/OpenglCodecCommon/GLClientState.cpp
    shared/OpenglCodecCommon/GLSharedGroup.cpp
    shared/OpenglCodecCommon/TimeUtils.cpp
    # OS utils
    shared/OpenglOsUtils/osThreadUnix.cpp
    shared/OpenglOsUtils/osProcessUnix.cpp
    shared/OpenglOsUtils/osDynLibrary.cpp
    # emugen-generated decoders
    generated/gl_dec/gl_dec.cpp
    generated/gl_dec/gl_server_context.cpp
    generated/gl2_dec/gl2_dec.cpp
    generated/gl2_dec/gl2_server_context.cpp
    generated/renderControl_dec/renderControl_dec.cpp
    generated/renderControl_dec/renderControl_server_context.cpp
    # compat shims (cutils/log.h etc.)
    compat/compat.cpp
)
```

Also remove the existing comment that says "the full AOSP emugl
renderer is not compiled."

**Risks:** the in-tree `compat/` shims (`cutils/sockets.h`,
`utils/threads.h`, etc.) are minimal — they may need extending to
cover everything the full pipeline references. Build will likely
fail a few times until the shims are complete.

### 6.4 Phase 0 — Rework `twoyi_api.cpp::startOpenGLRenderer`

The new `startOpenGLRenderer` should:

1. Call `initLibrary()` (loads EGL/GL dispatch tables — already
   exists in `render_api.cpp`).
2. Call `setStreamMode(STREAM_MODE_UNIX)` so `RenderServer::create`
   uses `UnixStream` (not `TcpStream`).
3. Call `initOpenGLRenderer(width, height, portNum=0, onPost=null,
   onPostContext=null)` — starts `FrameBuffer` + `RenderServer`
   listening on `$TWOYI_ROOTFS/opengles`.
4. Call `createOpenGLSubwindow(window, 0, 0, width, height, 0)` to
   bind the ANativeWindow.
5. Keep the existing `setNativeWindow` / `resetSubWindow` /
   `removeSubWindow` entry points but route them to
   `createOpenGLSubwindow` / `FrameBuffer::setupSubWindow` /
   `FrameBuffer::removeSubWindow`.

Concretely, `twoyi_api.cpp` shrinks to ~80 lines that delegate to
the original AOSP functions. The existing custom EGL clear-loop
code is **deleted** — it's no longer needed once the real pipeline
is wired up.

### 6.5 New FFI bindings in `renderer_bindings.rs`

No new FFI symbols are needed — `startOpenGLRenderer`,
`setNativeWindow`, `resetSubWindow`, `removeSubWindow`,
`destroyOpenGLSubwindow`, `repaintOpenGLDisplay` are already
declared. The C++ side's `startOpenGLRenderer` signature does not
change (it just internally calls `initOpenGLRenderer` now).

**Optional addition** for explicit lifecycle control:

```rust
extern "C" {
    // ... existing ...
    pub fn stopOpenGLRenderer() -> ::std::os::raw::c_int;
    pub fn setStreamMode(mode: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
}
```

Used during `init_renderer` to select Unix socket mode before
starting the renderer.

### 6.6 `core.rs::init_renderer` changes

Currently (line 159-167):

```rust
let result = unsafe {
    renderer_bindings::startOpenGLRenderer(
        window, virtual_width, virtual_height,
        xdpi, ydpi, fps,
    )
};
```

No change required — `startOpenGLRenderer` keeps the same signature.
The new C++ implementation will internally:
1. `setStreamMode(STREAM_MODE_UNIX)` (one-time, idempotent).
2. `initLibrary()` (one-time, idempotent).
3. `initOpenGLRenderer(virtual_width, virtual_height, 0, NULL, NULL)`.
4. `createOpenGLSubwindow(window, 0, 0, virtual_width, virtual_height, 0)`.

If `RENDERER_STARTED` was already true (the second-call branch at
`core.rs:113-132`), just call `createOpenGLSubwindow` again with the
new window.

---

## 7. Testing strategy

### 7.1 Unit tests for the channel-name parser

Pure Rust, no Android deps:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_opengles() {
        assert_eq!(parse_channel_name(b"pipe:opengles"), Some("opengles"));
    }
    #[test]
    fn parse_with_trailing_garbage() {
        // The guest may write a single packet that includes both the
        // channel name and the first 4 bytes of clientFlags. The
        // parser must stop at the first non-printable byte.
        assert_eq!(parse_channel_name(b"pipe:opengles\x00\x00\x00\x00"),
                   Some("opengles"));
    }
    #[test]
    fn parse_rejects_no_prefix() {
        assert_eq!(parse_channel_name(b"opengles"), None);
    }
    #[test]
    fn parse_rejects_empty() {
        assert_eq!(parse_channel_name(b""), None);
    }
}
```

### 7.2 Mock-renderer integration test

Spin up a fake "renderer" listener on a tmpdir path that just
echoes back any bytes it receives. Connect to the kr64 proxy with
a mock guest that:
1. Writes `pipe:opengles`.
2. Writes a 4-byte `clientFlags = 0`.
3. Writes a hardcoded emugl packet (e.g. `OP_rcGetRendererVersion`
   = `0x00 0x27 0x00 0x00` (10000) + packetLen = `0x08 0x00 0x00 0x00`).
4. Reads back the response.

Verify the proxy:
- Connects to the mock renderer.
- Forwards all bytes intact (no corruption, no reordering).
- Closes cleanly when the guest closes.

```rust
#[test]
fn proxy_forwards_bytes_bidirectionally() {
    let rootfs = tmpdir();
    let proxy_listener = UnixListener::bind(format!("{}/dev/qemu_pipe", rootfs)).unwrap();

    // Mock renderer: just echo.
    let renderer_listener = UnixListener::bind(format!("{}/opengles", rootfs)).unwrap();
    let renderer_thread = thread::spawn(move || {
        let (mut s, _) = renderer_listener.accept().unwrap();
        let mut buf = [0u8; 1024];
        loop {
            let n = s.read(&mut buf).unwrap();
            if n == 0 { break; }
            s.write_all(&buf[..n]).unwrap();
        }
    });

    let _proxy = spawn_qemu_pipe_proxy(proxy_listener, ...).unwrap();

    let mut guest = UnixStream::connect(format!("{}/dev/qemu_pipe", rootfs)).unwrap();
    guest.write_all(b"pipe:opengles").unwrap();
    guest.write_all(&0u32.to_le_bytes()).unwrap();
    guest.write_all(&10000u32.to_le_bytes()).unwrap();   // opcode
    guest.write_all(&8u32.to_le_bytes()).unwrap();       // packetLen
    let mut echo = [0u8; 8];
    guest.read_exact(&mut echo).unwrap();
    assert_eq!(&echo[..4], &10000u32.to_le_bytes());

    drop(guest);
    let _ = renderer_thread.join();
}
```

### 7.3 End-to-end smoke test on device

Requires a real Android device + the twoyi APK. The test is:

1. Build the APK with the Phase 0 + Phase 1 changes.
2. Launch twoyi, tap "Launch Container".
3. `adb logcat | grep -E "TWOYI_RENDERER|KR64"`.
4. Expected log sequence:
   - `[CORE] Starting AOSP libOpenglRender.so` (existing)
   - `[KR64][devices] bound unix socket: .../dev/qemu_pipe`
   - `[KR64] qemu_pipe proxy listening at .../dev/qemu_pipe`
   - `[TWOYI_RENDERER] startOpenGLRenderer: ...` (Phase 0)
   - `[TWOYI_RENDERER] RenderServer listening on .../opengles`
   - `[KR64][qemu_pipe] guest connected (session=0)` (Phase 1)
   - `[KR64][qemu_pipe] session 0 channel = opengles`
   - `[TWOYI_RENDERER] RenderThread started`
5. The SurfaceView should now show the guest's boot animation /
   launcher instead of twoyi's BootLogTexture loading screen.

This is the ultimate acceptance test — and the project's #1 blocker
is resolved when it passes.

---

## 8. Estimated complexity and risks

### 8.1 What's the hardest part?

**Phase 0 (C++ side) is the hard part, not the Rust proxy.** The
Rust proxy is ~250 lines of straightforward byte-shuffling. The
hard work is re-enabling the full AOSP emugl pipeline:

1. **CMakeLists.txt expansion.** The current build only compiles
   `twoyi_api.cpp`. Adding 20+ source files will surface missing
   includes, missing compat shims, and emugen-generated code that
   may not compile clean against the NDK.
2. **`compat/` shims.** The emugl source uses `cutils/sockets.h`
   (`socket_local_server`, `socket_local_client`), `utils/threads.h`
   (`android::Mutex`, `android::Condition`), and `log.h`. The
   in-tree `compat/` directory has minimal versions of these —
   they'll likely need extending. Specifically:
   - `socket_local_server(path, namespace, type)` — currently the
     patched `UnixStream::listen` calls this. Verify it's
     implemented.
   - `android::Mutex` / `android::Condition` — used by `FrameBuffer`.
3. **EGL/GLES dispatch.** `init_egl_dispatch()` / `init_gl_dispatch()`
   use `dlopen("libEGL.so", ...)` and `dlsym` for every function.
   On Android this should "just work" against the system EGL, but
   the dispatch tables are large and any missing symbol = runtime
   failure.
4. **The `OnPostFn` callback.** AOSP's `initOpenGLRenderer` takes a
   frame-callback that's invoked after each `eglSwapBuffers`. For
   twoyi we pass `NULL` (we don't need it — the SurfaceView displays
   the ANativeWindow directly).

### 8.2 Unknowns

1. **Does the guest's goldfish EGL driver exist in the rootfs?**
   The GSI rootfs from `system-images;android-30;google_apis;x86_64`
   should include `/vendor/lib64/egl/libEGL_goldfish.so` — but
   verification is needed. If the guest uses a different EGL
   driver (e.g. SwiftShader), the wire protocol may differ.
2. **Does the guest actually write `"pipe:opengles"` or just
   `"opengles"`?** Different AOSP versions have slightly different
   `qemu_pipe_open` implementations. The proxy must accept both.
3. **Channel negotiation for audio/camera/etc.** AOSP's `qemu_pipe`
   supports multiple channels (`"audio"`, `"camera"`, `"gsm"`,
   etc.). Twoyi's MVP only handles `"opengles"`; future work
   extends the proxy to route other channels to their respective
   HAL stubs.
4. **Does the EGL context need to be current on the RenderServer
   thread or the RenderThread?** AOSP's design: `FrameBuffer::initialize`
   makes the pbuffer context current on the calling thread;
   `RenderThread`s `eglMakeCurrent` per-context when dispatching
   `rcMakeCurrent`. Twoyi's existing `twoyi_api.cpp` makes the
   context current on its own render thread — that thread must be
   retired before Phase 0 lands, or there will be an EGL context
   conflict.

### 8.3 MVP vs. full implementation scope

**Phase 0 MVP (smallest viable change to get rendering working):**
- Re-enable the full AOSP emugl pipeline in `CMakeLists.txt`.
- Rework `twoyi_api.cpp` to delegate to `initOpenGLRenderer` /
  `createOpenGLSubwindow`.
- Verify the build succeeds and the renderer starts.
- Estimated effort: 2-3 days.

**Phase 1 MVP (smallest viable proxy):**
- Implement `qemu_pipe.rs` with the channel parser + proxy pump.
- Wire it into `lib.rs::run()` in place of `spawn_accept_thread`.
- Handle only the `opengles` channel; close other channels with a
  warning.
- Estimated effort: 1-2 days.

**Combined MVP:** ~4-5 days for an experienced engineer. The
result should be a guest that boots and renders its launcher.

**Full implementation (post-MVP):**
- Route other `qemu_pipe` channels (`audio`, `camera`, etc.) to
  their respective HAL modules.
- Replace the per-session two-thread pump with `epoll` for better
  scalability.
- Add a `qemu_pipe` integration test in `app/cpp/emugl/tests/` that
  builds a mock guest and exercises the full pipeline.
- Implement `stopOpenGLRenderer` lifecycle for clean shutdown.
- Estimated additional effort: 1-2 weeks.

---

## 9. Phased implementation roadmap

```
Phase 0 — Re-enable AOSP emugl pipeline  (PR #1, ~3 days)
├─ 0.1 Expand CMakeLists.txt to compile the full source list.
├─ 0.2 Extend compat/ shims as needed (cutils/sockets.h, utils/threads.h).
├─ 0.3 Rework twoyi_api.cpp to delegate to initOpenGLRenderer.
├─ 0.4 Verify the build succeeds for arm64-v8a AND x86_64.
├─ 0.5 Verify libOpenglRender.so exposes the same 6 FFI entry points.
└─ 0.6 Update build.sh if any new compile flags are needed.

Phase 1 — qemu_pipe proxy in kr64        (PR #2, ~2 days, depends on Phase 0)
├─ 1.1 Create app/rs/kr64/src/qemu_pipe.rs (the module sketched in §6.1).
├─ 1.2 Add `pub mod qemu_pipe;` to lib.rs.
├─ 1.3 Replace spawn_accept_thread(qemu_pipe) with spawn_qemu_pipe_proxy.
├─ 1.4 Hold the proxy handle in run() until the guest exits.
├─ 1.5 Add unit tests for parse_channel_name and the mock-renderer flow.
└─ 1.6 Verify cargo test passes (165+ tests) and cargo clippy is clean.

Phase 2 — End-to-end on-device validation (PR #3, ~2 days, depends on Phase 1)
├─ 2.1 Build the APK with the new libOpenglRender.so + kr64.
├─ 2.2 Launch the container; capture logcat.
├─ 2.3 Verify the log sequence described in §7.3.
├─ 2.4 If the SurfaceView shows the guest boot animation, capture a screenshot.
└─ 2.5 If rendering fails, debug via FRAMEBUFFER_DEBUG env var (AOSP
       emugl supports per-frame dumps via RENDERER_DUMP_DIR).

Phase 3 — Hardening + extensions         (post-MVP, ~1-2 weeks)
├─ 3.1 Route non-opengles channels (audio, camera) to their HAL modules.
├─ 3.2 Replace two-thread pump with epoll-based single thread.
├─ 3.3 Implement stopOpenGLRenderer for clean shutdown.
├─ 3.4 Add a CI smoke test that builds the APK + runs the existing
│      kr64 test suite + a new "qemu_pipe protocol" integration test.
└─ 3.5 Performance tuning (buffer sizes, TCP_NODELAY equivalent for
       Unix sockets — SO_SNDBUF tuning).
```

---

## 10. References

### 10.1 Source files read for this plan

**kr64 daemon (Rust):**
- `app/rs/kr64/src/devices.rs` — `create_qemu_pipe()` at line 198,
  `DeviceSocket` struct at line 74, `bind_unix_socket()` at line 155.
- `app/rs/kr64/src/lib.rs` — `spawn_accept_thread()` at line 983
  (the stub to replace), the daemon `run()` at line 530, device
  creation + accept-thread wiring at lines 935-940.
- `app/rs/kr64/Cargo.toml` — `libc = "0.2.112"` only.

**Renderer (Rust FFI to C++):**
- `app/rs/src/renderer_bindings.rs` — FFI declarations to the 6
  entry points in libOpenglRender.so.
- `app/rs/src/core.rs` — `init_renderer()` at line 92 (spawns
  renderer thread + spawns guest init).
- `app/rs/src/lib.rs` — JNI entry points (`renderer_init` etc.).
- `app/rs/src/openglrender.h` — C header (matches renderer_bindings.rs).

**AOSP emugl source (C++):**
- `app/cpp/emugl/CMakeLists.txt` — currently only `twoyi_api.cpp`.
- `app/cpp/emugl/twoyi_api.cpp` — the EGL clear-loop stub (483 lines).
- `app/cpp/emugl/libOpenglRender/render_api.cpp` — the original
  `initOpenGLRenderer` (kept but not compiled).
- `app/cpp/emugl/libOpenglRender/RenderServer.cpp` — server accept loop.
- `app/cpp/emugl/libOpenglRender/RenderThread.cpp` — per-connection
  GL decode loop.
- `app/cpp/emugl/libOpenglRender/FrameBuffer.h` / `.cpp` — host EGL
  singleton + ColorBuffer / WindowSurface / RenderContext maps.
- `app/cpp/emugl/libOpenglRender/ColorBuffer.h` / `.cpp`.
- `app/cpp/emugl/libOpenglRender/RenderControl.cpp` — the
  renderControl decoder's host-side handlers (rcCreateContext etc.).
- `app/cpp/emugl/libOpenglRender/ReadBuffer.h` / `.cpp` —
  stream-buffered read helper used by RenderThread.
- `app/cpp/emugl/include/libOpenglRender/IOStream.h` — base class
  for SocketStream; defines `IOSTREAM_CLIENT_EXIT_SERVER`.
- `app/cpp/emugl/OpenglCodecCommon/SocketStream.h` / `.cpp` — base
  socket stream (readFully / writeFully / commitBuffer).
- `app/cpp/emugl/OpenglCodecCommon/UnixStream.cpp` — the patched
  `make_unix_path` that builds `$TWOYI_ROOTFS/opengles{,2,3}`.
- `app/cpp/emugl/OpenglCodecCommon/TcpStream.cpp` — the TCP variant
  (used when `gRendererStreamMode == STREAM_MODE_TCP`).
- `app/cpp/emugl/OpenglCodecCommon/codec_defs.h` — `CODEC_SERVER_PORT = 22468`.
- `app/cpp/emugl/include/libOpenglRender/render_api.h` — public C
  API for libOpenglRender (DECL macros, STREAM_MODE_* constants).
- `app/cpp/emugl/libOpenglRender/EGLDispatch.h` / `ThreadInfo.h` /
  `GL2Decoder.h` — supporting types.
- `app/cpp/emugl/generated/gl2_dec/gl2_dec.cpp` — the emugen-generated
  decoder that shows the exact 8-byte packet header layout.
- `app/cpp/emugl/generated/gl2_dec/gl2_opcodes.h` — opcode constants
  (2048+ for GLESv2).
- `app/cpp/emugl/generated/renderControl_dec/renderControl_opcodes.h`
  — opcode constants (10000+ for renderControl).
- `app/cpp/build.sh` — the NDK build script.

**Analysis docs:**
- `KR64_SKELETON.md` — the kr64 skeleton design doc.
- `BINDER_SKELETON.md` — binder virtualisation (proxy pattern
  reference for the qemu_pipe proxy).
- `HAL_VIRTUALIZATION_ANALYSIS.md` — Display HAL section §1.1
  confirms `qemu_pipe` is the GL transport.
- `HONEST_STATUS_CORRECTED.md` — the blocker description.
- `FINAL_STATUS.md` — the "single most important next step" this
  plan addresses.
- `download/VM_KR64_ANALYSIS.md` — §6 confirms VM creates
  `/dev/qemu_pipe` via mknodat(S_IFSOCK) and routes GL commands
  through it.

### 10.2 External references (background knowledge)

- AOSP `sdk/emulator/qemu_pipe/qemu_pipe.cpp` — guest-side
  `qemu_pipe_open()` writes `"pipe:<channel>"`.
- AOSP `external/qemu/android/hw-pipe.c` — host-side pipe service
  multiplexer (the conceptual model the kr64 proxy replicates).
- AOSP `device/generic/goldfish-opengl/system/OpenglSystemCommon/` —
  guest-side `QEMU_PIPE_CHANNEL_NAME = "opengles"` constant.

---

## 11. Summary of architectural decisions

| Decision | Rationale |
|---|---|
| The kr64 daemon plays proxy (not the renderer). | Mirrors the AOSP emulator's split between `hw-pipe.c` (multiplexer in QEMU process) and `libOpenglRender` (renderer in QEMU process). Keeps the renderer's listening-socket ownership in the host app process, where the EGL context lives. |
| The renderer listens on Unix sockets under `$TWOYI_ROOTFS/opengles{,2,3}`. | Already wired up by the patched `UnixStream::make_unix_path`. No code change needed beyond re-enabling the full pipeline. |
| The proxy uses two threads per session (one per direction). | Matches the existing pattern in `binder.rs` / `audio.rs`. Simpler than `epoll`-based single-thread; MVP-appropriate. |
| Phase 0 (C++ rebuild) and Phase 1 (Rust proxy) are separate PRs. | Each is independently testable: Phase 0 can be verified by checking that `libOpenglRender.so` opens `/opengles` and accepts a mock connection. Phase 1 can be verified by checking that the proxy connects to a mock renderer. Splitting reduces review surface. |
| Only the `opengles` channel is handled in the MVP. | The guest's first GL connect is the critical path. Other channels (`audio`, `camera`, etc.) can be added incrementally; closing them with a warning is fine for boot. |
| `twoyi_api.cpp` is reworked (not deleted) to delegate to the AOSP originals. | Preserves the 6-symbol FFI ABI that `renderer_bindings.rs` links against. Avoids touching the Rust side. |
| The proxy holds the listener for the lifetime of `run()`. | Matches the binder/audio/sensor pattern. Ensures clean shutdown when the guest exits. |

---

*End of plan. Implementation should proceed in the order Phase 0 →
Phase 1 → Phase 2 (validation) → Phase 3 (hardening).*
