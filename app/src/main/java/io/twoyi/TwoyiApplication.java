/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi;

import android.app.Application;
import android.content.Context;
import android.content.res.Resources;

import com.microsoft.appcenter.AppCenter;
import com.microsoft.appcenter.analytics.Analytics;
import com.microsoft.appcenter.crashes.Crashes;

import java.lang.reflect.Field;

import io.twoyi.utils.FileLogger;
import io.twoyi.utils.ProfileManager;
import io.twoyi.utils.RomManager;

/**
 * @author weishu
 * @date 2020/12/24.
 */

public class TwoyiApplication extends Application {

    @Override
    protected void attachBaseContext(Context base) {
        super.attachBaseContext(base);

        // Initialize the file logger FIRST, before any other app code —
        // so every subsequent Log.x call (and every Rust-side android_logger
        // call, captured via the logcat pump) has somewhere to write.
        // Writes to /sdcard/Android/data/io.twoyi/files/log/{app.log,
        // logcat.log, boot.log, crash.log, kmsg.log, deviceinfo.txt}.
        FileLogger.init(base);

        ProfileManager.initializeProfiles(base);
        RomManager.ensureBootFiles(base);

        TwoyiSocketServer.getInstance(base).start();
    }

    @Override
    public void onCreate() {
        super.onCreate();

        AppCenter.start(this, BuildConfig.APPCENTER_API_KEY,
                Analytics.class, Crashes.class);
        if (BuildConfig.DEBUG) {
            AppCenter.setEnabled(false);
        }
    }

    static volatile int statusBarHeight = -1;

    public static int getStatusBarHeight(Context context) {
        if (statusBarHeight != -1) {
            return statusBarHeight;
        }

        int resId = context.getResources().getIdentifier("status_bar_height", "dimen", "android");
        if (resId > 0) {
            statusBarHeight = context.getResources().getDimensionPixelSize(resId);
        }

        if (statusBarHeight < 0) {
            // Fixed: original code unconditionally ran `statusBarHeight = result`
            // in a finally block, where `result` defaulted to 0. If reflection
            // threw (which it does on every Android release where
            // com.android.internal.R$dimen no longer has the field, or where
            // Class.newInstance() is blocked), the finally overwrote
            // statusBarHeight to 0 — which then defeated the 25dp fallback
            // below (`if (statusBarHeight < 0)` was false). The net effect was
            // that on devices where the resource lookup failed, the status bar
            // height was reported as 0px, breaking layout insets.
            try {
                Class<?> clazz = Class.forName("com.android.internal.R$dimen");
                // Fixed: Class.newInstance() is deprecated since Java 9 and
                // throws if the constructor is inaccessible; the modern
                // equivalent propagates the underlying InvocationTargetException.
                Object obj = clazz.getDeclaredConstructor().newInstance();
                Field field = clazz.getField("status_bar_height");
                int resourceId = Integer.parseInt(field.get(obj).toString());
                statusBarHeight = context.getResources().getDimensionPixelSize(resourceId);
            } catch (Throwable ignored) {
            }
        }

        //Use 25dp if no status bar height found
        if (statusBarHeight < 0) {
            statusBarHeight = dip2px(context, 25);
        }
        return statusBarHeight;
    }

    private static int dip2px(Context context, float dpValue) {
        float scale = context.getResources().getDisplayMetrics().density;
        int px = (int) (dpValue * scale + 0.5f);
        return px;
    }

    public static float px2dp(float pxValue) {
        return (pxValue / Resources.getSystem().getDisplayMetrics().density);
    }
}
