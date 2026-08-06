// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use jni::objects::{JFieldID, JValue};
use jni::signature::{JavaType, Primitive};
use jni::sys::{jclass, jfieldID, jfloat, jint, jobject, JNI_ERR, jstring};
use jni::JNIEnv;
use jni::{JavaVM, NativeMethod};
use log::{debug, error, info, Level};
use ndk_sys;
use std::ffi::c_void;
use std::sync::OnceLock;

use android_logger::Config;

mod input;
mod renderer_bindings;
mod core;

// Reference the interp symbol from C to force it to be linked
extern "C" {
    #[link_name = "interp"]
    static INTERP: [u8; 0];
}

// Force the interp symbol to be included by referencing it
#[used]
static INTERP_REF: &'static [u8; 0] = unsafe { &INTERP };

/// ## Examples
/// ```
/// let method:NativeMethod = jni_method!(native_method, "(Ljava/lang/String;)V");
/// ```
macro_rules! jni_method {
    ( $name: tt, $method:tt, $signature:expr ) => {{
        jni::NativeMethod {
            name: jni::strings::JNIString::from(stringify!($name)),
            sig: jni::strings::JNIString::from($signature),
            fn_ptr: $method as *mut c_void,
        }
    }};
}

#[no_mangle]
pub extern "C" fn renderer_init(
    env: JNIEnv,
    _clz: jclass,
    surface: jobject,
    loader: jstring,
    width: jint,
    height: jint,
    xdpi: jfloat,
    ydpi: jfloat,
    fps: jint,
) {
    debug!("renderer_init");
    let window = unsafe { ndk_sys::ANativeWindow_fromSurface(env.get_native_interface(), surface) };

    let window = match std::ptr::NonNull::new(window) {
        Some(x) => x,
        None => {
            error!("ANativeWindow_fromSurface was null!");
            return;
        }
    };

    let window = unsafe { ndk::native_window::NativeWindow::from_ptr(window) };

    let surface_width = window.width();
    let surface_height = window.height();

    // Use the virtual display dimensions passed from Java
    let virtual_width = width;
    let virtual_height = height;

    let loader_path: String = match env.get_string(loader.into()) {
        Ok(s) => s.into(),
        Err(e) => {
            error!("Failed to get loader path string: {}", e);
            return;
        }
    };
    let window_ptr = window.ptr().as_ptr() as *mut c_void;

    // Acquire an extra reference to the ANativeWindow so it stays alive
    // after this function returns and the NativeWindow wrapper is dropped.
    // The renderer thread will use this pointer. Without this, the window
    // is freed when the NativeWindow drops, causing a use-after-free.
    unsafe { ndk_sys::ANativeWindow_acquire(window_ptr as *mut ndk_sys::ANativeWindow); }

    core::init_renderer(
        window_ptr,
        loader_path,
        surface_width,
        surface_height,
        virtual_width,
        virtual_height,
        xdpi as i32,
        ydpi as i32,
        fps as i32,
    );
}

/// Set the app's data directory. This MUST be called before init()
/// so that all paths (rootfs, log, input sockets, opengles pipes)
/// resolve correctly — especially in work profiles where the data
/// dir is /data/user/<uid>/io.twoyi instead of /data/data/io.twoyi.
#[no_mangle]
pub extern "C" fn set_data_dir(
    env: JNIEnv,
    _clz: jclass,
    data_dir: jstring,
) {
    let dir: String = match env.get_string(data_dir.into()) {
        Ok(s) => s.into(),
        Err(e) => {
            error!("Failed to get data_dir string: {}", e);
            return;
        }
    };
    debug!("set_data_dir: {}", dir);
    core::set_data_dir(dir);
}

#[no_mangle]
pub extern "C" fn renderer_reset_window(
    env: JNIEnv,
    _clz: jclass,
    surface: jobject,
    _top: jint,
    _left: jint,
    _width: jint,
    _height: jint,
    _fb_width: jint,
    _fb_height: jint,
) {
    debug!("reset_window: surface={}x{}, framebuffer={}x{}", _width, _height, _fb_width, _fb_height);
    unsafe {
        let window = ndk_sys::ANativeWindow_fromSurface(env.get_native_interface(), surface);
        if window.is_null() {
            error!("ANativeWindow_fromSurface returned null in renderer_reset_window");
            return;
        }
        core::reset_window(window as *mut c_void, _top, _left, _width, _height, _fb_width, _fb_height);
        // Release the reference acquired by ANativeWindow_fromSurface
        ndk_sys::ANativeWindow_release(window);
    }
}

#[no_mangle]
pub extern "C" fn renderer_remove_window(env: JNIEnv, _clz: jclass, surface: jobject) {
    debug!("renderer_remove_window");

    unsafe {
        let window = ndk_sys::ANativeWindow_fromSurface(env.get_native_interface(), surface);
        if window.is_null() {
            error!("ANativeWindow_fromSurface returned null in renderer_remove_window");
            return;
        }
        core::remove_window(window as *mut c_void);
        // Release the reference acquired by ANativeWindow_fromSurface
        ndk_sys::ANativeWindow_release(window);
    }
}

/// Cached JNI field ID for `android.view.MotionEvent.mNativePtr`.
///
/// `handle_touch` is invoked on every touch event (60–120 Hz). Looking up the
/// field by name (`env.get_field(event, "mNativePtr", "J")`) on every call
/// performs a string-based `GetFieldID` resolution (plus a `GetObjectClass`)
/// each time, which is wasteful. JNI field IDs are JVM-global handles that
/// remain valid for the lifetime of the JVM (the defining class cannot be
/// unloaded while any instance is reachable), so the ID can safely be looked
/// up once and reused for the rest of the process.
///
/// `jfieldID` is a raw pointer and therefore not `Send`/`Sync` by default,
/// which would prevent storing it in a `static`. The newtype below asserts the
/// safety of sharing it across threads: the field ID is immutable after the
/// one-time lookup and never dereferenced on the Rust side.
#[derive(Clone, Copy)]
struct MotionEventNativePtrField(jfieldID);
unsafe impl Send for MotionEventNativePtrField {}
unsafe impl Sync for MotionEventNativePtrField {}

static MOTION_EVENT_NATIVE_PTR_FIELD: OnceLock<MotionEventNativePtrField> = OnceLock::new();

#[no_mangle]
pub extern "C" fn handle_touch(env: JNIEnv, _clz: jclass, event: jobject) {
    // Resolve the field ID once and cache it; subsequent calls skip the
    // string-based `GetFieldID` lookup entirely.
    let field_id = match MOTION_EVENT_NATIVE_PTR_FIELD.get() {
        Some(&id) => id,
        None => {
            let class = match env.get_object_class(event) {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to get MotionEvent class: {}", e);
                    return;
                }
            };
            let id = match env.get_field_id(class, "mNativePtr", "J") {
                Ok(id) => id,
                Err(e) => {
                    error!("Failed to get mNativePtr field ID: {}", e);
                    // A failed GetFieldID raises a pending Java exception
                    // (NoSuchFieldError). Clear it so later calls can retry.
                    if env.exception_check().unwrap_or(false) {
                        let _ = env.exception_describe();
                        let _ = env.exception_clear();
                    }
                    return;
                }
            };
            let raw = MotionEventNativePtrField(id.into_inner());
            // Note: the `class` local reference above is freed automatically
            // when this native method returns to Java (JNI local refs are
            // per-call), so no manual cleanup is needed.
            let _ = MOTION_EVENT_NATIVE_PTR_FIELD.set(raw);
            // .get() again to hand back the stored copy (the `set` call above
            // may have lost the race with another thread and stored a
            // different — but equivalent — ID).
            match MOTION_EVENT_NATIVE_PTR_FIELD.get() {
                Some(&id) => id,
                None => return,
            }
        }
    };

    // Use the cached field ID. `JFieldID::from(jfieldID)` is free (it just
    // reattaches a lifetime); no JVM work happens here.
    let ptr = match env.get_field_unchecked(
        event,
        JFieldID::from(field_id.0),
        JavaType::Primitive(Primitive::Long),
    ) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to get mNativePtr field: {}", e);
            return;
        }
    };

    if let JValue::Long(p) = ptr {
        if p == 0 {
            error!("MotionEvent mNativePtr is null");
            return;
        }
        let ev = unsafe {
            let nonptr = match std::ptr::NonNull::new(p as *mut ndk_sys::AInputEvent) {
                Some(n) => n,
                None => {
                    error!("Failed to create NonNull from MotionEvent ptr");
                    return;
                }
            };
            ndk::event::MotionEvent::from_ptr(nonptr)
        };
        input::handle_touch(ev)
    } else {
        error!("mNativePtr field was not a Long");
    }
}

#[no_mangle]
pub extern "C" fn send_key_code(_env: JNIEnv, _clz: jclass, keycode: jint) {
    debug!("send key code!");
    input::send_key_code(keycode);
}

unsafe fn register_natives(jvm: &JavaVM, class_name: &str, methods: &[NativeMethod]) -> jint {
    // Try to get env - if this fails, we can't continue
    let env: JNIEnv = match jvm.get_env() {
        Ok(e) => e,
        Err(e) => {
            // Can't log here since logger might not be initialized
            eprintln!("Failed to get JNI environment: {:?}", e);
            return JNI_ERR;
        }
    };

    // Fixed: unwrap() across FFI boundary can panic — UB during library load
    let jni_version = match env.get_version() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[KR64] Failed to get JNI version: {:?}", e);
            return JNI_ERR;
        }
    };
    let version: jint = jni_version.into();

    debug!("JNI Version : {:#?} ", jni_version);
    debug!("Registering {} methods for class: {}", methods.len(), class_name);

    let clazz = match env.find_class(class_name) {
        Ok(clazz) => {
            debug!("Found class: {}", class_name);
            clazz
        },
        Err(e) => {
            error!("java class not found : {:?}", e);
            // Check for pending exception and clear it
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_describe();
                let _ = env.exception_clear();
            }
            return JNI_ERR;
        }
    };
    debug!("clazz: {:#?}", clazz);

    let result = env.register_native_methods(clazz, &methods);

    if result.is_ok() {
        info!("register_natives : succeed - registered {} methods", methods.len());
        version
    } else {
        error!("register_natives : failed {:?}", result);
        // Check for pending exception and clear it
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_describe();
            let _ = env.exception_clear();
        }
        JNI_ERR
    }
}

#[no_mangle]
#[allow(non_snake_case)]
unsafe fn JNI_OnLoad(jvm: JavaVM, _reserved: *mut c_void) -> jint {
    // Initialize logger - if this fails, continue anyway
    let _ = android_logger::init_once(
        Config::default()
            .with_min_level(Level::Info)
            .with_tag("CLIENT_EGL"),
    );

    debug!("JNI_OnLoad started");

    let class_name: &str = "io/twoyi/Renderer";
    // renderer (the AOSP emugl libOpenglRender.so built from source).
    let jni_methods = [
        jni_method!(init, renderer_init, "(Landroid/view/Surface;Ljava/lang/String;IIFFI)V"),
        jni_method!(
            resetWindow,
            renderer_reset_window,
            "(Landroid/view/Surface;IIIIII)V"
        ),
        jni_method!(
            removeWindow,
            renderer_remove_window,
            "(Landroid/view/Surface;)V"
        ),
        jni_method!(handleTouch, handle_touch, "(Landroid/view/MotionEvent;)V"),
        jni_method!(sendKeycode, send_key_code, "(I)V"),
        jni_method!(setDataDir, set_data_dir, "(Ljava/lang/String;)V"),
    ];

    let result = register_natives(&jvm, class_name, jni_methods.as_ref());
    debug!("JNI_OnLoad completed with result: {}", result);
    result
}

// Exported C functions that can be called from shell or other tools
// These provide the same functionality as the JNI interface but without Android dependencies

/// Start the input system - can be called from shell via dlopen/dlsym
#[no_mangle]
pub extern "C" fn twoyi_start_input_system(width: i32, height: i32) {
    input::start_input_system(width, height);
}

/// Display version and help information
#[no_mangle]
pub extern "C" fn twoyi_print_help() {
    use std::io::{self, Write};
    let _ = writeln!(io::stdout(), "Twoyi Native Library - v0.1.0");
    let _ = writeln!(io::stdout(), "\nExported Functions:");
    let _ = writeln!(io::stdout(), "  twoyi_start_input_system(width, height) - Start input system");
    let _ = writeln!(io::stdout(), "  twoyi_print_help() - Show this help");
    let _ = writeln!(io::stdout(), "  twoyi_send_keycode(keycode) - Send a keycode event");
    let _ = writeln!(io::stdout(), "\nUsage from shell:");
    let _ = writeln!(io::stdout(), "  This library can be loaded via System.loadLibrary(\"twoyi\") in Android apps");
    let _ = writeln!(io::stdout(), "  Or called from shell using the twoyi wrapper script");
}

/// Send a keycode - exposed for shell access
#[no_mangle]
pub extern "C" fn twoyi_send_keycode(keycode: i32) {
    input::send_key_code(keycode);
}

// Main function for standalone execution when invoked directly or via linker64
#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const libc::c_char) -> i32 {
    use std::io::{self, Write};
    use std::ffi::CStr;

    let _ = writeln!(io::stdout(), "Twoyi Renderer - Standalone Mode");

    // Parse arguments from argc/argv
    let mut args: Vec<String> = Vec::new();

    if argc > 0 && !argv.is_null() {
        unsafe {
            for i in 0..argc as isize {
                let arg_ptr = *argv.offset(i);
                if !arg_ptr.is_null() {
                    // arg_ptr is *const i8, CStr::from_ptr expects *const i8
                    if let Ok(arg_cstr) = CStr::from_ptr(arg_ptr).to_str() {
                        args.push(arg_cstr.to_string());
                    }
                }
            }
        }
    }

    let _ = writeln!(io::stdout(), "argc: {}", argc);
    if !args.is_empty() {
        let _ = writeln!(io::stdout(), "Arguments:");
        for (i, arg) in args.iter().enumerate() {
            let _ = writeln!(io::stdout(), "  [{}]: {}", i, arg);
        }
    }

    let _ = writeln!(io::stdout(), "\nUsage: ./libtwoyi.so [OPTIONS]");
    let _ = writeln!(io::stdout(), "Options:");
    let _ = writeln!(io::stdout(), "  --help                Show this help message");
    let _ = writeln!(io::stdout(), "  --width <width>       Set virtual display width (default: 720)");
    let _ = writeln!(io::stdout(), "  --height <height>     Set virtual display height (default: 1280)");
    let _ = writeln!(io::stdout(), "  --loader <path>       Set loader path");
    let _ = writeln!(io::stdout(), "  --start-input         Start input system only");
    let _ = writeln!(io::stdout(), "\nNote: This library is primarily designed to be loaded by the Twoyi app.");
    let _ = writeln!(io::stdout(), "For full functionality, use it as a JNI library via System.loadLibrary(\"twoyi\")");

    // Parse arguments
    let mut width = 720;
    let mut height = 1280;
    let mut start_input = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                twoyi_print_help();
                return 0;
            }
            "--width" => {
                i += 1;
                if i < args.len() {
                    if let Ok(w) = args[i].parse::<i32>() {
                        width = w;
                    }
                }
            }
            "--height" => {
                i += 1;
                if i < args.len() {
                    if let Ok(h) = args[i].parse::<i32>() {
                        height = h;
                    }
                }
            }
            "--start-input" => {
                start_input = true;
            }
            _ => {}
        }
        i += 1;
    }

    if start_input {
        let _ = writeln!(io::stdout(), "\nStarting input system with dimensions: {}x{}", width, height);
        twoyi_start_input_system(width, height);
        let _ = writeln!(io::stdout(), "Input system started. Press Ctrl+C to exit.");

        // Keep the program running
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    0
}
