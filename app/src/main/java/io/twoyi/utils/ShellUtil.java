/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi.utils;

import com.topjohnwu.superuser.Shell;

/**
 * @author weishu
 * @date 2022/1/4.
 */

public final class ShellUtil {

    private ShellUtil() {
    }

    /**
     * 6-Z268: cached non-root shell. {@link #newSh()} used to spawn a
     * BRAND-NEW {@code sh} process (fork/exec + libsu stream-pump
     * threads + prompt handshake, ~20–80 ms) for EVERY call — and the
     * boot path calls it 2–3× per launch (orphan kill, data/local/tmp
     * mkdir, dalvik wipe). libsu shells are stateless for job purposes
     * (each {@code newJob()} runs its commands independently), so one
     * shared instance serves all callers; jobs serialize through it,
     * which is fine at these call rates. A failed create falls back to
     * the per-call builder so behaviour never regresses.
     */
    private static volatile Shell sCachedSh;

    public static Shell newSh() {
        Shell cached = sCachedSh;
        if (cached != null) {
            return cached;
        }
        synchronized (ShellUtil.class) {
            if (sCachedSh == null) {
                try {
                    sCachedSh = Shell.Builder.create()
                            .setFlags(Shell.FLAG_NON_ROOT_SHELL)
                            .build("sh");
                } catch (Throwable ignored) {
                    // Shell creation failed (hostile environment) — fall
                    // through and let the caller's job fail as before.
                    return Shell.Builder.create()
                            .setFlags(Shell.FLAG_NON_ROOT_SHELL)
                            .build("sh");
                }
            }
            return sCachedSh;
        }
    }
}
