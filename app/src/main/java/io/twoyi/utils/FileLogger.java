/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.utils;

import android.content.Context;
import android.os.Build;
import android.os.Process;
import android.os.SystemClock;
import android.util.Log;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.io.PrintWriter;
import java.io.RandomAccessFile;
import java.io.StringWriter;
import java.nio.charset.StandardCharsets;
import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.Locale;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;

/**
 * Comprehensive file-backed logger for twoyi.
 *
 * <p>ALL app-side log calls — and a continuous capture of {@code logcat -v threadtime}
 * (which includes the Rust-side {@code CLIENT_EGL} / {@code kr64} tags) — are
 * tee'd to plain-text log files under
 * {@code /sdcard/Android/data/io.twoyi/files/log/}. This directory is the
 * app-private external storage returned by {@link Context#getExternalFilesDir(String)};
 * it requires <em>no</em> runtime permission and is reachable from a file
 * manager (or {@code adb pull}) on unrooted devices.
 *
 * <h2>Files written (mirrors scripts/kvm-e2e-test.sh artifacts)</h2>
 * <ul>
 *   <li><strong>app.log</strong> — every {@link #v}/{@link #d}/{@link #i}/
 *       {@link #w}/{@link #e} call from Java code, timestamped, with tag
 *       and (optionally) a throwable stack trace. Also forwarded to
 *       {@link android.util.Log} so logcat still works.
 *       <p><em>KVM e2e equivalent:</em> the union of {@code logcat.txt}
 *       (Java-side) and the host's stdout.</li>
 *   <li><strong>logcat.log</strong> — continuous {@code logcat -v threadtime}
 *       pump covering every tag in the app's process tree, including the
 *       native {@code CLIENT_EGL} (Rust {@code android_logger}) and any
 *       {@code kr64} / loader stderr that bionic redirects to logcat.
 *       Rotates at {@link #MAX_LOG_BYTES} (5 MiB) and keeps the last
 *       {@link #MAX_ROTATED} rotations ({@code .1}, {@code .2}, …).
 *       <p><em>KVM e2e equivalent:</em> {@code logcat.txt}.</li>
 *   <li><strong>logcat-filtered.log</strong> — filtered view of logcat
 *       containing only the lines that match the boot-milestone regex
 *       ({@code KR64 INFO|KR64 WARN|KR64 ERROR|CORE|NEW_RENDERER|
 *       CLIENT_EGL|SOCKET_MONITOR|BOOT_COMPLETED|TWOYI_RENDERER|emugl|
 *       Render2Activity|RamdiskImporter|BootCompletionServer}). Same
 *       filter the KVM e2e script uses to produce
 *       {@code logcat-filtered.txt}.</li>
 *   <li><strong>kr64.log</strong> — periodic tee of the kr64 binary's
 *       stdout/stderr from {@code <dataDir>/kr64-app-stderr.log} (the
 *       file core.rs redirects kr64's output to). Refreshed every 2 s.
 *       <p><em>KVM e2e equivalent:</em> {@code kr64-stderr.log} +
 *       {@code twoyi-log.txt}.</li>
 *   <li><strong>boot.log</strong> — structured boot timeline: one line per
 *       milestone (renderer init, surface created, BOOT_COMPLETED received,
 *       boot failed, etc.). {@link #boot(String, String)} is the only
 *       writer. <em>KVM e2e equivalent:</em> {@code boot-verdict.txt}.</li>
 *   <li><strong>crash.log</strong> — uncaught exceptions captured by the
 *       global handler installed in {@link #init(Context)}. Each entry
 *       includes the thread name and a full stack trace.
 *       <p><em>KVM e2e equivalent:</em> {@code tombstones/} +
 *       {@code dropbox/}.</li>
 *   <li><strong>kmsg.log</strong> — kernel ring buffer capture
 *       ({@code dmesg}). Best-effort: will be empty/short on unrooted
 *       devices because {@code /dev/kmsg} requires CAP_SYS_ADMIN. We
 *       still attempt it because some device trees grant log read access
 *       to {@code shell}/{@code system} groups.
 *       <p><em>KVM e2e equivalent:</em> {@code dmesg.log}.</li>
 *   <li><strong>proc.log</strong> — periodic snapshot of the app's own
 *       process state: {@code /proc/self/cmdline}, {@code /proc/self/status},
 *       {@code /proc/self/task/} (thread list), {@code /proc/self/fd/}
 *       (open file descriptors), and {@code ps -A} (best-effort — may
 *       only show self on Android 16+). Refreshed every 15 s.
 *       <p><em>KVM e2e equivalent:</em> {@code twrp-init-cmdline.log} +
 *       {@code twrp-init-status.log} + {@code twrp-init-threads.log} +
 *       {@code twrp-init-fds.log} + {@code twrp-ps-ef.log}.</li>
 *   <li><strong>deviceinfo.txt</strong> — written once at init: build
 *       fingerprint, ABI, profile, ROM md5, etc.
 *       <p><em>KVM e2e equivalent:</em> the {@code basic.txt} section of
 *       the bugreport zip.</li>
 * </ul>
 *
 * <h2>Thread safety</h2>
 * Every public method is thread-safe. Writes to {@code app.log} /
 * {@code boot.log} / {@code crash.log} go through a single
 * {@link ScheduledExecutorService} (single-threaded, so writes are
 * naturally serialized; no extra locking needed). The logcat pump runs
 * on its own dedicated thread so a slow disk can't back-pressure logcat
 * into dropping messages.
 *
 * <h2>Failure mode</h2>
 * Every {@code write()} call is wrapped in try/catch — if the external
 * storage is unmounted or read-only, the logger silently falls back to
 * {@link android.util.Log} only. The app's behaviour never depends on
 * the file logger succeeding.
 *
 * @author Disable-OP
 * @date 2026/08/12.
 */
public final class FileLogger {

    private static final String TAG = "FileLogger";

    // -------------------------------------------------------------------------
    // Configuration
    // -------------------------------------------------------------------------

    /** Max size of any single log file before it rotates. 5 MiB. */
    private static final long MAX_LOG_BYTES = 5L * 1024 * 1024;

    /** Number of rotated backups to keep (app.log.1, app.log.2, …). */
    private static final int MAX_ROTATED = 3;

    // -------------------------------------------------------------------------
    // State
    // -------------------------------------------------------------------------

    private static volatile FileLogger INSTANCE;

    private final File mLogDir;
    private final File mAppLog;
    private final File mBootLog;
    private final File mCrashLog;

    private final ScheduledExecutorService mWriter =
            Executors.newSingleThreadScheduledExecutor(r -> {
                Thread t = new Thread(r, "FileLogger-Writer");
                t.setDaemon(true);
                t.setPriority(Thread.NORM_PRIORITY - 1);
                return t;
            });

    /** Monotonic ms since process start — used for the elapsed-time column. */
    private final long mStartMs = SystemClock.elapsedRealtime();

    private final AtomicBoolean mLogcatPumpRunning = new AtomicBoolean(false);
    private final AtomicLong mLogcatDropped = new AtomicLong(0);

    /**
     * Thread-local because SimpleDateFormat is NOT thread-safe: formatLine()
     * runs on whichever thread called FileLogger.i/e/..., and the old shared
     * instance could corrupt timestamps or throw under concurrent logging
     * (the class javadoc promises thread-safety).
     */
    private static final ThreadLocal<SimpleDateFormat> DATE_FORMAT =
            ThreadLocal.withInitial(() ->
                    new SimpleDateFormat("yyyy-MM-dd HH:mm:ss.SSS", Locale.US));

    // -------------------------------------------------------------------------
    // Construction / initialization
    // -------------------------------------------------------------------------

    private FileLogger(File logDir) {
        this.mLogDir = logDir;
        this.mAppLog = new File(logDir, "app.log");
        this.mBootLog = new File(logDir, "boot.log");
        this.mCrashLog = new File(logDir, "crash.log");
    }

    /**
     * Initialize the file logger. MUST be called from
     * {@link io.twoyi.TwoyiApplication#attachBaseContext(Context)} before
     * any other app code runs, so every subsequent log call has somewhere
     * to write.
     *
     * <p>Idempotent: a second call is a no-op (returns the existing
     * instance).
     */
    public static FileLogger init(Context context) {
        if (INSTANCE != null) {
            return INSTANCE;
        }
        synchronized (FileLogger.class) {
            if (INSTANCE != null) {
                return INSTANCE;
            }
            // getExternalFilesDir("log") -> /sdcard/Android/data/io.twoyi/files/log/
            // No permission needed on API 19+. The path is reachable via
            // file managers on unrooted devices.
            File logDir = context.getExternalFilesDir("log");
            if (logDir == null) {
                // External storage not mounted (rare on a phone, common on
                // TV / Wear). Fall back to internal cache so we at least
                // have *some* on-device log — adb pull still works.
                logDir = new File(context.getCacheDir(), "log");
            }
            if (!logDir.exists() && !logDir.mkdirs()) {
                Log.w(TAG, "Could not create log dir: " + logDir);
            }
            FileLogger inst = new FileLogger(logDir);
            INSTANCE = inst;

            inst.writeDeviceInfo(context);
            inst.installCrashHandler();
            inst.startLogcatPump(context);
            inst.startFilteredLogcatPump();
            inst.startKr64LogTee(context);
            inst.startProcSnapshotPump();
            inst.startDmesgPump();

            // First line in app.log — a clear marker that the logger is up.
            inst.i(TAG, "── FileLogger initialized; log dir=" + logDir
                    + " pid=" + Process.myPid()
                    + " uid=" + Process.myUid()
                    + " abi=" + Build.SUPPORTED_ABIS[0]
                    + " sdk=" + Build.VERSION.SDK_INT
                    + " ──");
            inst.boot("filelogger_init", "log_dir=" + logDir);

            return inst;
        }
    }

    /** Returns the singleton, or {@code null} if {@link #init} hasn't run yet. */
    public static FileLogger get() {
        return INSTANCE;
    }

    public static File getLogDir() {
        FileLogger inst = INSTANCE;
        return inst == null ? null : inst.mLogDir;
    }

    // -------------------------------------------------------------------------
    // Public logging API — drop-in replacements for android.util.Log
    // -------------------------------------------------------------------------

    public static void v(String tag, String msg) {
        Log.v(tag, msg);
        tee('V', tag, msg, null);
    }

    public static void d(String tag, String msg) {
        Log.d(tag, msg);
        tee('D', tag, msg, null);
    }

    public static void i(String tag, String msg) {
        Log.i(tag, msg);
        tee('I', tag, msg, null);
    }

    public static void w(String tag, String msg) {
        Log.w(tag, msg);
        tee('W', tag, msg, null);
    }

    public static void w(String tag, String msg, Throwable t) {
        Log.w(tag, msg, t);
        tee('W', tag, msg, t);
    }

    public static void e(String tag, String msg) {
        Log.e(tag, msg);
        tee('E', tag, msg, null);
    }

    public static void e(String tag, String msg, Throwable t) {
        Log.e(tag, msg, t);
        tee('E', tag, msg, t);
    }

    /**
     * Structured boot-milestone log. Writes a single line to
     * {@code boot.log} (and tees to {@code app.log} + logcat).
     *
     * @param event short snake_case identifier (e.g. {@code "surface_created"})
     * @param detail free-form detail (may be {@code null})
     */
    public static void boot(String event, String detail) {
        FileLogger inst = INSTANCE;
        if (inst == null) {
            return;
        }
        String line = inst.formatLine('B', "Boot", event + (detail == null ? "" : " | " + detail));
        // Boot events go to BOTH boot.log and app.log so the timeline is
        // visible in either file.
        inst.enqueueWrite(mAppLogFor(inst), line);
        inst.enqueueWrite(inst.mBootLog, line);
    }

    // -------------------------------------------------------------------------
    // Internals — tee / format / write
    // -------------------------------------------------------------------------

    private static void tee(char level, String tag, String msg) {
        tee(level, tag, msg, null);
    }

    private static void tee(char level, String tag, String msg, Throwable t) {
        FileLogger inst = INSTANCE;
        if (inst == null) {
            return;
        }
        String line = inst.formatLine(level, tag, msg);
        inst.enqueueWrite(inst.mAppLog, line);
        if (t != null) {
            inst.enqueueWrite(inst.mAppLog, stackTraceToString(t));
        }
    }

    private static File mAppLogFor(FileLogger inst) {
        return inst.mAppLog;
    }

    private String formatLine(char level, String tag, String msg) {
        long elapsed = SystemClock.elapsedRealtime() - mStartMs;
        // Thread name (truncated to 15 chars — matches logcat column width).
        String tname = Thread.currentThread().getName();
        if (tname.length() > 15) {
            tname = tname.substring(0, 15);
        }
        // 2026-08-12 14:30:00.123  +12345.678  [main           ] I/Tag: message
        return String.format(Locale.US,
                "%s  +%8.3f  [%-15s] %c/%s: %s",
                DATE_FORMAT.get().format(new Date()),
                elapsed / 1000.0,
                tname,
                level,
                tag,
                msg == null ? "" : msg);
    }

    private static String stackTraceToString(Throwable t) {
        StringWriter sw = new StringWriter();
        t.printStackTrace(new PrintWriter(sw));
        return sw.toString();
    }

    private void enqueueWrite(File target, String line) {
        if (target == null) return;
        // Ensure each line ends with exactly one '\n'.
        String nl = line.endsWith("\n") ? line : line + "\n";
        mWriter.execute(() -> {
            try {
                writeOrRotate(target, nl);
            } catch (Throwable ignored) {
                // Never let a logging failure crash the app.
            }
        });
    }

    /**
     * Append {@code line} to {@code target}, rotating if it would exceed
     * {@link #MAX_LOG_BYTES}. Caller is on the single-threaded
     * {@link #mWriter} executor, so no extra locking.
     */
    private void writeOrRotate(File target, String line) throws IOException {
        if (target.exists() && target.length() + line.length() > MAX_LOG_BYTES) {
            rotate(target);
        }
        try (OutputStreamWriter osw = new OutputStreamWriter(
                new FileOutputStream(target, true), StandardCharsets.UTF_8)) {
            osw.write(line);
        }
    }

    private void rotate(File target) {
        // Delete the oldest, rename .(N-1) -> .N, … .1 -> .2, target -> .1
        File oldest = new File(target.getParentFile(),
                target.getName() + "." + MAX_ROTATED);
        if (oldest.exists() && !oldest.delete()) {
            // Can't delete oldest — give up rotation to avoid losing logs.
            return;
        }
        for (int i = MAX_ROTATED - 1; i >= 1; i--) {
            File src = new File(target.getParentFile(), target.getName() + "." + i);
            if (src.exists()) {
                File dst = new File(target.getParentFile(), target.getName() + "." + (i + 1));
                //noinspection ResultOfMethodCallIgnored
                src.renameTo(dst);
            }
        }
        File first = new File(target.getParentFile(), target.getName() + ".1");
        //noinspection ResultOfMethodCallIgnored
        target.renameTo(first);
    }

    // -------------------------------------------------------------------------
    // Logcat pump — continuous `logcat -v threadtime` -> logcat.log
    // -------------------------------------------------------------------------

    private void startLogcatPump(Context context) {
        if (!mLogcatPumpRunning.compareAndSet(false, true)) {
            return;
        }
        File logcatFile = new File(mLogDir, "logcat.log");
        Thread pump = new Thread(() -> {
            // `logcat -v threadtime` prints: "MM-DD HH:MM:SS.mmm PID TID LEVEL/TAG: message"
            // We use -v threadtime so we get PID+TID (useful for diagnosing
            // crashes in the kr64 child process vs. the Java UI thread).
            ProcessBuilder pb = new ProcessBuilder(
                    "logcat", "-v", "threadtime", "*:V");
            pb.redirectErrorStream(true);
            java.lang.Process proc = null;
            BufferedReader reader = null;
            try {
                proc = pb.start();
                reader = new BufferedReader(
                        new InputStreamReader(proc.getInputStream(), StandardCharsets.UTF_8));
                byte[] newline = "\n".getBytes(StandardCharsets.UTF_8);
                String line;
                while ((line = reader.readLine()) != null) {
                    try {
                        byte[] bytes = (line + "\n").getBytes(StandardCharsets.UTF_8);
                        // Append directly (bypassing the mWriter executor) on
                        // THIS thread — logcat can produce hundreds of lines
                        // per second, and queueing each one would OOM the
                        // executor's task queue under heavy load.
                        synchronized (FileLogger.class) {
                            if (logcatFile.exists()
                                    && logcatFile.length() + bytes.length > MAX_LOG_BYTES) {
                                rotate(logcatFile);
                            }
                            try (FileOutputStream fos = new FileOutputStream(logcatFile, true)) {
                                fos.write(bytes);
                            }
                        }
                    } catch (IOException ignored) {
                        // Single-line write failure — keep going.
                        mLogcatDropped.incrementAndGet();
                    }
                }
            } catch (IOException ioe) {
                // logcat binary not available (rare — even unrooted devices
                // have it, but some heavily-locked-down TV boxes strip it).
                // Log to app.log so the user knows why logcat.log is empty.
                tee('W', TAG, "logcat pump failed: " + ioe.getMessage(), ioe);
            } finally {
                if (reader != null) try { reader.close(); } catch (IOException ignored) {}
                if (proc != null) proc.destroy();
                mLogcatPumpRunning.set(false);
            }
        }, "FileLogger-Logcat");
        pump.setDaemon(true);
        pump.start();
    }

    // -------------------------------------------------------------------------
    // Filtered logcat pump — grep for boot milestones → logcat-filtered.log
    // -------------------------------------------------------------------------

    /**
     * Regex matching the boot-milestone tags the KVM e2e script greps for
     * in its logcat-filtered.txt. Mirrors the grep -E pattern in
     * scripts/kvm-e2e-test.sh line 1447.
     */
    private static final java.util.regex.Pattern FILTER_PATTERN =
            java.util.regex.Pattern.compile(
                    "KR64 INFO|KR64 WARN|KR64 ERROR|CORE|NEW_RENDERER|"
                  + "CLIENT_EGL|SOCKET_MONITOR|BOOT_COMPLETED|"
                  + "TWOYI_RENDERER|emugl|Render2Activity|RamdiskImporter|"
                  + "BootCompletionServer|FileLogger|TwoyiSocketServer|"
                  + "SettingsActivity|ProfileManager|RomManager");

    private void startFilteredLogcatPump() {
        File filteredFile = new File(mLogDir, "logcat-filtered.log");
        Thread t = new Thread(() -> {
            // Same logcat invocation as the main pump, but we filter each
            // line through FILTER_PATTERN and only write matches. This
            // gives the user a compact "boot milestone" view without the
            // noise of the full logcat.
            ProcessBuilder pb = new ProcessBuilder(
                    "logcat", "-v", "threadtime", "*:V");
            pb.redirectErrorStream(true);
            java.lang.Process proc = null;
            BufferedReader reader = null;
            try {
                proc = pb.start();
                reader = new BufferedReader(
                        new InputStreamReader(proc.getInputStream(), StandardCharsets.UTF_8));
                String line;
                while ((line = reader.readLine()) != null) {
                    if (!FILTER_PATTERN.matcher(line).find()) {
                        continue;
                    }
                    try {
                        byte[] bytes = (line + "\n").getBytes(StandardCharsets.UTF_8);
                        synchronized (FileLogger.class) {
                            if (filteredFile.exists()
                                    && filteredFile.length() + bytes.length > MAX_LOG_BYTES) {
                                rotate(filteredFile);
                            }
                            try (FileOutputStream fos = new FileOutputStream(filteredFile, true)) {
                                fos.write(bytes);
                            }
                        }
                    } catch (IOException ignored) {
                        // Keep going.
                    }
                }
            } catch (IOException ignored) {
                // logcat binary not available — the main pump already logged this.
            } finally {
                if (reader != null) try { reader.close(); } catch (IOException ignored) {}
                if (proc != null) proc.destroy();
            }
        }, "FileLogger-LogcatFiltered");
        t.setDaemon(true);
        t.start();
    }

    // -------------------------------------------------------------------------
    // kr64 log tee — periodic copy of <dataDir>/kr64-app-stderr.log → kr64.log
    // -------------------------------------------------------------------------

    /**
     * core.rs redirects the kr64 binary's stdout+stderr to
     * {@code <dataDir>/kr64-app-stderr.log} (see app/rs/src/core.rs line 422).
     * This pump copies that file to the external log dir every 2 s so the
     * user can read it without root. The FILE COPY stays a full copy each
     * time (not an incremental tail) because kr64 truncates the file on
     * each launch, so an incremental offset would be wrong after a
     * relaunch.
     * <p>
     * 6-Z87 FIX 3: the LOGCAT tee leg is now INCREMENTAL (see
     * {@link #logcatTeeIncremental}). Pre-Z87 it re-logged the ENTIRE file
     * via {@code Log.i("KR64", …)} every 2 s — an O(N²) logcat flood (the
     * file only grows, so pass N re-logs N passes' worth of lines, and
     * chatty collapsed them into the illusion of a "2 s relaunch loop").
     * There is NO relaunch loop: one container runs continuously and the
     * 2 s "metronome" was this tee. A truncation (the real relaunch
     * signature) now logs exactly one notice line and resets the offset.
     */
    private void startKr64LogTee(Context context) {
        final File src = new File(context.getApplicationInfo().dataDir, "kr64-app-stderr.log");
        final File dst = new File(mLogDir, "kr64.log");
        // Also tee the fallback linker path log (log.txt) into the same file.
        final File src2 = new File(context.getApplicationInfo().dataDir, "log.txt");
        Thread t = new Thread(() -> {
            // 6-Z87 FIX 3: incremental logcat offsets, PRIVATE to this pump
            // thread (next unread byte; 0 = nothing logged yet). Reset to 0
            // by logcatTeeIncremental when the file shrinks (fresh launch).
            long teeOffset = 0;   // kr64-app-stderr.log (primary)
            long tee2Offset = 0;  // log.txt (fallback linker path)
            while (true) {
                try {
                    // 6-Z184 STREAM COPY: the previous pass materialised the
                    // ENTIRE source file in a StringBuilder before writing
                    // (bounded per LINE, but total memory = file size — a
                    // 145 MB kr64 log still OOM'd at sb.toString().getBytes()).
                    // Stream in 64 KB chunks instead, capping the copied tail
                    // at MAX_TEE_COPY_BYTES so kr64.log stays bounded no
                    // matter how large the guest's stderr grows.
                    synchronized (FileLogger.class) {
                        try (FileOutputStream fos = new FileOutputStream(dst, false)) {
                            copyTailBounded(src, "kr64-app-stderr.log", fos);
                            copyTailBounded(src2, "log.txt", fos);
                        } catch (IOException ioe) {
                            android.util.Log.w(TAG, "kr64 tee copy failed: " + ioe.getMessage());
                        }
                    }
                    // 6-Z87 FIX 3: the logcat tee now logs only the NEW
                    // lines since the last pass (full-file re-logging was
                    // the O(N²) flood that fabricated the "2 s relaunch
                    // loop" myth — see the method javadoc). The kr64.log
                    // FILE COPY above stays full-file on purpose.
                    teeOffset = logcatTeeIncremental(src, teeOffset,
                            "-- kr64 log truncated (fresh container launch) --");
                    tee2Offset = logcatTeeIncremental(src2, tee2Offset,
                            "-- log.txt truncated (fresh container launch) --");
                } catch (OutOfMemoryError | Exception e) {
                    // 6-Z131: NEVER let the tee kill the app. Run
                    // 32786386000 died exactly here — the old unbounded
                    // readLine() loop fed a 145 MB "line" into
                    // String.getBytes → OutOfMemoryError propagated out
                    // of this thread → Android's default uncaught-
                    // exception handler killed the whole app (and the
                    // guest) mid-boot. Log a short note and keep polling.
                    try {
                        Log.w(TAG, "kr64 tee pass failed ("
                                + e.getClass().getSimpleName() + ") — continuing");
                    } catch (Throwable ignored) {
                        // Even the note failed — still keep the pump alive.
                    }
                }
                try {
                    Thread.sleep(2_000L);
                } catch (InterruptedException ie) {
                    Thread.currentThread().interrupt();
                    return;
                }
            }
        }, "FileLogger-Kr64Tee");
        t.setDaemon(true);
        t.start();
    }

    // -------------------------------------------------------------------------
    // 6-Z131: bounded-line reads for the kr64 tee (the run-32786386000 OOM)
    // -------------------------------------------------------------------------

    /**
     * History (6-Z131 -> 6-Z184): guest children inherit kr64's stderr fd
     * and can write huge binary blobs with no newline for megabytes on
     * end. The original BufferedReader.readLine() loop materialised a
     * 145 MB "line" and OOM-killed the whole app (run 32786386000);
     * the 6-Z131 fix bounded per-LINE reads but still built the whole
     * file in a StringBuilder before writing. The 6-Z184 stream copy
     * below allocates a fixed 64 KB buffer regardless of source size.
     */

    /** Max bytes copyTailBounded copies from any one source file (8 MiB). */
    private static final long MAX_TEE_COPY_BYTES = 8L * 1024 * 1024;

    /**
     * 6-Z184: stream-copy {@code src} into {@code fos} in 64 KB chunks,
     * NEVER materialising the file in memory. A file larger than
     * {@link #MAX_TEE_COPY_BYTES} contributes only its LAST cap bytes,
     * preceded by a "[skipped N bytes]" marker, so kr64.log stays a
     * bounded, byte-faithful tail of the guest's stderr no matter how
     * large the source grows (the previous version built the whole file
     * in a StringBuilder first and OOM'd on a 145 MB log).
     */
    private static void copyTailBounded(File src, String label, FileOutputStream fos)
            throws IOException {
        if (src == null || !src.exists() || src.length() == 0) return;
        final long len = src.length();
        try (FileInputStream fis = new FileInputStream(src)) {
            fos.write(("\u2500\u2500 " + label + " (" + len + " bytes) \u2500\u2500\n")
                    .getBytes(StandardCharsets.UTF_8));
            if (len > MAX_TEE_COPY_BYTES) {
                final long toSkip = len - MAX_TEE_COPY_BYTES;
                long skipped = 0;
                while (skipped < toSkip) {
                    long s = fis.skip(toSkip - skipped);
                    if (s <= 0) break;
                    skipped += s;
                }
                fos.write(("...[skipped " + skipped + " of " + len
                        + " bytes \u2014 tail follows...]
")
                        .getBytes(StandardCharsets.UTF_8));
            }
            byte[] buf = new byte[64 * 1024];
            int n;
            while ((n = fis.read(buf)) > 0) {
                fos.write(buf, 0, n);
            }
            fos.write('\n');
        }
    }

    /**
     * 6-Z87 FIX 3: incremental logcat leg of the kr64 tee. Reads only the
     * bytes past {@code offset} (RandomAccessFile seek → read to EOF), logs
     * each NEW complete line via {@code Log.i("KR64", line)}, and returns
     * the new offset. Only complete lines (up to the last '\n' in the new
     * chunk) are logged + consumed, so a mid-write partial line is never
     * split across two logcat entries. If the file shrank below
     * {@code offset} it was truncated — the REAL relaunch signature (kr64
     * truncates the log at container start) — so the offset resets to 0 and
     * exactly one {@code truncatedMsg} notice is logged.
     *
     * @return the updated offset (file position after the last logged line)
     */
    private static long logcatTeeIncremental(File src, long offset, String truncatedMsg) {
        if (src == null || !src.exists()) {
            return offset;
        }
        long len = src.length();
        if (len == 0 || len == offset) {
            return offset; // nothing new
        }
        if (len < offset) {
            Log.i("KR64", truncatedMsg);
            offset = 0;
        }
        String chunk;
        try (RandomAccessFile raf = new RandomAccessFile(src, "r")) {
            raf.seek(offset);
            long remaining = raf.length() - offset;
            if (remaining <= 0) {
                return offset;
            }
            byte[] buf = new byte[(int) Math.min(remaining, 1 << 20)];
            int n = raf.read(buf);
            if (n <= 0) {
                return offset;
            }
            chunk = new String(buf, 0, n, StandardCharsets.UTF_8);
        } catch (IOException ignored) {
            // Briefly locked/rotated by kr64's writer — retry next pass.
            return offset;
        }
        // Consume only COMPLETE lines; a trailing partial line stays
        // un-logged (its offset is not advanced past it) until its '\n'
        // arrives in a later pass.
        int lastNl = chunk.lastIndexOf('\n');
        if (lastNl < 0) {
            return offset;
        }
        for (String line : chunk.substring(0, lastNl).split("\n")) {
            if (!line.isEmpty()) {
                Log.i("KR64", line);
            }
        }
        return offset + lastNl + 1;
    }

    // -------------------------------------------------------------------------
    // Process snapshot pump — /proc/self/* + ps -A → proc.log (every 15 s)
    // -------------------------------------------------------------------------

    private void startProcSnapshotPump() {
        final File dst = new File(mLogDir, "proc.log");
        Thread t = new Thread(() -> {
            while (true) {
                StringBuilder sb = new StringBuilder();
                sb.append("══ proc snapshot @ ").append(DATE_FORMAT.get().format(new Date()))
                  .append(" (pid=").append(Process.myPid()).append(") ══\n\n");

                // /proc/self/cmdline (NUL-separated → spaces)
                sb.append("── /proc/self/cmdline ──\n");
                appendProcFile(sb, "/proc/self/cmdline", true);
                sb.append("\n");

                // /proc/self/status
                sb.append("── /proc/self/status ──\n");
                appendProcFile(sb, "/proc/self/status", false);
                sb.append("\n");

                // /proc/self/task/ (thread list)
                sb.append("── /proc/self/task/ (threads) ──\n");
                File taskDir = new File("/proc/self/task");
                String[] tasks = taskDir.list();
                if (tasks != null) {
                    for (String tid : tasks) {
                        String name = readProcOneLine("/proc/self/task/" + tid + "/comm");
                        sb.append("  ").append(tid).append("  ").append(name).append("\n");
                    }
                }
                sb.append("\n");

                // /proc/self/fd/ (open file descriptors) — ls -la style
                sb.append("── /proc/self/fd/ (open fds) ──\n");
                File fdDir = new File("/proc/self/fd");
                String[] fds = fdDir.list();
                if (fds != null) {
                    for (String fd : fds) {
                        try {
                            String target = java.nio.file.Files.readSymbolicLink(
                                    java.nio.file.Paths.get("/proc/self/fd/" + fd)).toString();
                            sb.append("  fd ").append(fd).append(" → ").append(target).append("\n");
                        } catch (Throwable ignored) {
                            // fd may have closed between list() and readSymbolicLink.
                        }
                    }
                }
                sb.append("\n");

                // ps -A (best-effort — on Android 16+ only shows self)
                sb.append("── ps -A (best-effort; may only show self on Android 16+) ──\n");
                try {
                    java.lang.Process p = new ProcessBuilder("ps", "-A").redirectErrorStream(true).start();
                    try (BufferedReader br = new BufferedReader(
                            new InputStreamReader(p.getInputStream(), StandardCharsets.UTF_8))) {
                        String line;
                        while ((line = br.readLine()) != null) {
                            sb.append(line).append("\n");
                        }
                    }
                    p.waitFor(2, TimeUnit.SECONDS);
                } catch (Throwable tw) {
                    sb.append("  (ps failed: ").append(tw.getMessage()).append(")\n");
                }

                sb.append("\n");
                try {
                    byte[] bytes = sb.toString().getBytes(StandardCharsets.UTF_8);
                    synchronized (FileLogger.class) {
                        if (dst.exists() && dst.length() + bytes.length > MAX_LOG_BYTES) {
                            rotate(dst);
                        }
                        try (FileOutputStream fos = new FileOutputStream(dst, true)) {
                            fos.write(bytes);
                        }
                    }
                } catch (IOException ignored) {
                    // Best-effort.
                }
                try {
                    Thread.sleep(15_000L);
                } catch (InterruptedException ie) {
                    Thread.currentThread().interrupt();
                    return;
                }
            }
        }, "FileLogger-Proc");
        t.setDaemon(true);
        t.start();
    }

    private void appendProcFile(StringBuilder sb, String path, boolean nulToSpace) {
        try (BufferedReader br = new BufferedReader(
                new InputStreamReader(new FileInputStream(path), StandardCharsets.UTF_8))) {
            if (nulToSpace) {
                int ch;
                while ((ch = br.read()) != -1) {
                    sb.append(ch == 0 ? ' ' : (char) ch);
                }
                sb.append("\n");
            } else {
                String line;
                while ((line = br.readLine()) != null) {
                    sb.append(line).append("\n");
                }
            }
        } catch (IOException ioe) {
            sb.append("  (could not read ").append(path).append(": ")
              .append(ioe.getMessage()).append(")\n");
        }
    }

    private String readProcOneLine(String path) {
        try (BufferedReader br = new BufferedReader(
                new InputStreamReader(new FileInputStream(path), StandardCharsets.UTF_8))) {
            return br.readLine();
        } catch (IOException ioe) {
            return "(unreadable)";
        }
    }

    // -------------------------------------------------------------------------
    // dmesg pump — best-effort /dev/kmsg capture
    // -------------------------------------------------------------------------

    private void startDmesgPump() {
        // /dev/kmsg requires CAP_SYS_ADMIN on Android, which a normal app
        // never has. But on some device trees the shell group can read
        // /proc/kmsg, and on emulator-based test setups dmesg is often
        // world-readable. Try once at init, log what we get, then poll
        // every 30 s for new lines. If the first read fails, we silently
        // give up — no point in retrying on a device we know denied us.
        //
        // IMPORTANT: The `dmesg` binary itself calls `syslog(2, ...)` which
        // is syscall 103 on x86_64. This syscall is NOT in Android's seccomp
        // allowlist for untrusted_app — calling it sends SIGSYS (signal 31)
        // which kills the `dmesg` process instantly. The SIGSYS crash
        // appears in logcat and pollutes the BootLogTexture with crash
        // output that looks like a kr64 crash but isn't.
        //
        // To avoid this noise, we skip the dmesg pump entirely. kmsg.log
        // will be empty on all devices, but that's acceptable — the app's
        // own logs (app.log, logcat.log, kr64.log) are far more valuable
        // for debugging than the kernel ring buffer.
        tee('I', TAG, "dmesg pump skipped (seccomp blocks syslog() syscall on untrusted_app; would cause SIGSYS noise in logcat)");
    }

    // -------------------------------------------------------------------------
    // deviceinfo.txt — written once at init
    // -------------------------------------------------------------------------

    private void writeDeviceInfo(Context context) {
        File f = new File(mLogDir, "deviceinfo.txt");
        mWriter.execute(() -> {
            try (PrintWriter pw = new PrintWriter(new OutputStreamWriter(
                    new FileOutputStream(f, false), StandardCharsets.UTF_8))) {
                pw.println("── twoyi device info ──");
                pw.println("Captured: " + DATE_FORMAT.get().format(new Date()));
                pw.println();
                pw.println("BRAND:          " + Build.BRAND);
                pw.println("MANUFACTURER:   " + Build.MANUFACTURER);
                pw.println("MODEL:          " + Build.MODEL);
                pw.println("PRODUCT:        " + Build.PRODUCT);
                pw.println("DEVICE:         " + Build.DEVICE);
                pw.println("BOARD:          " + Build.BOARD);
                pw.println("DISPLAY:        " + Build.DISPLAY);
                pw.println("FINGERPRINT:    " + Build.FINGERPRINT);
                pw.println("SDK_INT:        " + Build.VERSION.SDK_INT);
                pw.println("RELEASE:        " + Build.VERSION.RELEASE);
                pw.println("SECURITY_PATCH: " + Build.VERSION.SECURITY_PATCH);
                pw.println("ABI:            " + Build.SUPPORTED_ABIS[0]);
                pw.println("ABIS:           " + String.join(",", Build.SUPPORTED_ABIS));
                pw.println();
                pw.println("── Process ──");
                pw.println("PID:  " + Process.myPid());
                pw.println("UID:  " + Process.myUid());
                pw.println("TID:  " + Process.myTid());
                pw.println();
                pw.println("── Package ──");
                try {
                    pw.println("Package:  " + context.getPackageName());
                    pw.println("Apk:      " + context.getApplicationInfo().sourceDir);
                    pw.println("DataDir:  " + context.getApplicationInfo().dataDir);
                    pw.println("FilesDir: " + context.getFilesDir());
                    pw.println("ExtFiles: " + context.getExternalFilesDir(null));
                } catch (Throwable ignored) {}
                pw.println();
                pw.println("── Profile ──");
                try {
                    pw.println("ActiveProfile: " + ProfileManager.getActiveProfile(context));
                } catch (Throwable ignored) {}
                pw.println();
                pw.println("── ROM ──");
                try {
                    RomManager.RomInfo info = RomManager.getCurrentRomInfo(context);
                    pw.println("RomVersion: " + info.version);
                    pw.println("RomCode:    " + info.code);
                    pw.println("RomAuthor:  " + info.author);
                    pw.println("RomMd5:     " + info.md5);
                } catch (Throwable ignored) {}
            } catch (IOException ignored) {
                // Best-effort.
            }
        });
    }

    // -------------------------------------------------------------------------
    // Global crash handler
    // -------------------------------------------------------------------------

    private void installCrashHandler() {
        final Thread.UncaughtExceptionHandler prev = Thread.getDefaultUncaughtExceptionHandler();
        Thread.setDefaultUncaughtExceptionHandler((thread, throwable) -> {
            // Write to crash.log immediately (synchronous — we're crashing
            // anyway, so blocking on disk is the least of our problems).
            try {
                String stamp = DATE_FORMAT.get().format(new Date());
                StringWriter sw = new StringWriter();
                PrintWriter pw = new PrintWriter(sw);
                pw.println("── Uncaught exception ──");
                pw.println("Time:       " + stamp);
                pw.println("Thread:     " + thread.getName()
                        + " (id=" + thread.getId() + ")");
                pw.println("Process:    pid=" + Process.myPid()
                        + " uid=" + Process.myUid());
                pw.println("BootElapsedMs: " + (SystemClock.elapsedRealtime() - mStartMs));
                pw.println();
                throwable.printStackTrace(pw);
                pw.println();
                pw.flush();
                try (FileOutputStream fos = new FileOutputStream(mCrashLog, true)) {
                    fos.write(sw.toString().getBytes(StandardCharsets.UTF_8));
                }
            } catch (Throwable ignored) {
                // Nothing we can do.
            }
            // Delegate to the previous handler (usually calls ActivityManager
            // + kills the process).
            if (prev != null) {
                prev.uncaughtException(thread, throwable);
            }
        });
    }
}
