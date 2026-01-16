/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

package io.twoyi;

import android.view.MotionEvent;
import android.view.Surface;

/**
 * @author weishu
 * @date 2021/10/20.
 */
public class Renderer {

    static {
        System.loadLibrary("twoyi");
    }

    public static native void init(Surface surface, String loader, int width, int height, float xdpi, float ydpi, int fps);

    public static native void resetWindow(Surface surface, int top, int left, int width, int height, int fbWidth, int fbHeight);

    public static native void removeWindow(Surface surface);

    public static native void handleTouch(MotionEvent event);

    public static native void sendKeycode(int keycode);
    
    /**
     * Set the renderer type to use
     * @param useNewRenderer true to use new open-source renderer, false for old renderer
     */
    public static native void setRendererType(int useNewRenderer);
}
