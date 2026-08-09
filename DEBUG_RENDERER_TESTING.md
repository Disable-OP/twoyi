# Debug Renderer Testing Guide

This document describes how to test the new Debug Renderer feature.

## Prerequisites

- Twoyi app installed on device
- Container set up and working
- Access to device storage (/sdcard)

## Test Steps

### 1. Enable Debug Renderer

1. Open Twoyi app
2. Go to **Settings** (three dots menu)
3. Scroll to **Advanced** section
4. Find **Debug Renderer** checkbox
5. ✅ Verify checkbox is **unchecked** by default
6. Check the **Debug Renderer** checkbox
7. ✅ Verify summary text warns about huge logs
8. Note that reboot is required

### 2. Enable New Renderer (if not already enabled)

The debug renderer works with both old and new renderers, but socket monitoring is most useful with the new renderer.

1. In Settings → Advanced
2. Check **Use New Renderer** if not already checked
3. Reboot container

### 3. Launch Container

1. Go back to main screen
2. Tap **Launch Container**
3. Wait for container to boot
4. ✅ Check logcat for debug messages:
   ```bash
   adb logcat | grep -E "CLIENT_EGL|NEW_RENDERER|SOCKET_MONITOR"
   ```
5. ✅ Expected log entries:
   - `[CORE] Debug renderer set to: true`
   - `[NEW_RENDERER] Debug mode set to: true`
   - `[NEW_RENDERER] Starting socket monitoring for debug mode`
   - `[SOCKET_MONITOR] Starting socket monitoring for debug renderer`

### 4. Verify Log Files Created

The debug logs are in the app's private storage, so you'll need to use the bugreport feature or adb to access them.

#### Method 1: Using Send Log Feature (Recommended)
1. Go to Settings → Send Log
2. This creates a bugreport.zip file
3. Share/save the file
4. Extract the zip file
5. ✅ Verify `renderer_debug/` folder exists in the zip
6. ✅ Verify log files inside:
   - `pipe_*_write.log` - QEMU pipe writes
   - `pipe_*_read.log` - QEMU pipe reads (if any)
   - `opengles_commands.log` - GL commands
   - `gralloc_buffers.log` - Buffer operations
   - `gralloc_events.log` - Gralloc events
   - `socket_*.log` - Socket monitoring (multiple files)

#### Method 2: Using ADB (Requires Root or Backup)
```bash
# Via adb if rooted
adb shell su -c "ls -la /data/data/io.twoyi/files/twoyi_renderer_debug/"

# Or via backup (non-root)
adb backup -f twoyi_backup.ab io.twoyi
# Extract backup and find files/twoyi_renderer_debug/
```

✅ Verify directory exists and contains log files

### 5. Verify Log Contents

Since logs are in private storage, use the Send Log feature or adb:

#### Via Send Log Feature
1. Go to Settings → Send Log
2. Save the bugreport.zip
3. Extract and examine `renderer_debug/` folder
4. ✅ Check contents of log files

#### Via ADB (if device is rooted)
```bash
adb shell su -c "cat /data/data/io.twoyi/files/twoyi_renderer_debug/opengles_commands.log" | head -20
```

#### Check QEMU Pipe Logs (in bugreport)
✅ Should contain:
- Timestamps in milliseconds
- "WRITE X bytes" entries
- Hex data: `Hex: [...]`
- ASCII representation: `ASCII: ...`
- Integer values: `i32: [...]`

#### Check OpenGL ES Commands Log (in bugreport)
✅ Should contain:
- Initialize command with width, height, DPI, FPS parameters
- SetWindowSize commands
- Repaint commands
- Timestamps

#### Check Gralloc Logs (in bugreport)
✅ Should contain:
- BUFFER_LOCK entries
- Width, height, stride, format values

#### Check Socket Logs (in bugreport)
In the extracted bugreport.zip, check the `renderer_debug/` folder for socket log files.

✅ Should show multiple socket log files (those that exist and were connectable)

### 6. Interact with Container

1. Perform actions in the container:
   - Touch screen
   - Launch apps
   - Navigate UI
2. Generate another bugreport (Settings → Send Log)
3. ✅ Verify log files have grown in size
4. ✅ Verify new entries are added with updated timestamps

### 7. Disable Debug Renderer

1. Go back to Settings → Advanced
2. Uncheck **Debug Renderer**
3. Reboot container
4. Generate a new bugreport
5. ✅ Verify no `renderer_debug/` folder in new bugreport (or it's empty)
6. ✅ Verify old logs from previous session are not included

### 8. Profile-Specific Testing

1. Create a new profile (Settings → Profile Manager)
2. Switch to new profile
3. ✅ Verify Debug Renderer is OFF by default
4. Enable it for this profile
5. ✅ Verify separate log directory or separate logs per profile

## Expected Results

### When Debug Renderer is OFF (default)
- ✅ No debug log files created
- ✅ No performance impact
- ✅ Normal container operation
- ✅ Bugreport does not include renderer_debug folder

### When Debug Renderer is ON
- ✅ Log directory `<app_files_dir>/files/twoyi_renderer_debug/` created
- ✅ Multiple log files generated
- ✅ Log files contain detailed data
- ✅ Logs accessible via Send Log feature
- ✅ Bugreport includes renderer_debug/ folder with all logs
- ✅ May notice slight performance degradation due to file I/O

## Common Issues

### No renderer_debug Folder in Bugreport
- Check if debug mode was actually enabled in settings
- Check logcat for error messages
- Verify files directory is writable
- Try using new renderer (more comprehensive logging)

### Log Files Not Growing
- Ensure container is actually being used
- Try interacting with container UI
- Generate multiple bugreports to compare timestamps

### Socket Logs Missing
- Socket monitoring depends on socket availability
- Not all sockets exist in all container configurations
- Some sockets may not accept connections
- This is normal - only connectable sockets are logged

### Log Files Too Large
- This is expected behavior with debug mode
- Log files can grow to hundreds of MB quickly
- Disable debug mode when not needed
- Manually delete old logs: `adb shell rm -rf /sdcard/twoyi_renderer_debug/`

## Performance Notes

Debug mode has significant performance overhead:
- File I/O for every pipe operation
- Thread overhead for socket monitoring
- Disk space consumption

**Use only for debugging, not for normal operation.**

## Cleanup

Debug logs are in the app's private storage and will be:
- Automatically cleaned when app data is cleared
- Included in app backups
- Removed when app is uninstalled

To manually clean logs via Send Log:
1. The bugreport.zip contains a copy, but doesn't delete originals
2. Logs accumulate until app data is cleared or debug mode is disabled

For developers with root access:
```bash
adb shell su -c "rm -rf /data/data/io.twoyi/files/twoyi_renderer_debug/"
```
