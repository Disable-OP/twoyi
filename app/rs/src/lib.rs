// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use jni::objects::JValue;
use jni::sys::{jclass, jfloat, jint, jobject, JNI_ERR, jstring};
use jni::JNIEnv;
use jni::{JavaVM, NativeMethod};
use log::{debug, error, Level};
use ndk_sys;
use std::ffi::c_void;

use android_logger::Config;

mod input;
mod renderer_bindings;
mod core;

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
pub fn renderer_init(
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

    let loader_path: String = env.get_string(loader.into()).unwrap().into();
    let window_ptr = window.ptr().as_ptr() as *mut c_void;

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

#[no_mangle]
pub fn renderer_reset_window(
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
        core::reset_window(window as *mut c_void, _top, _left, _width, _height, _fb_width, _fb_height);
    }
}

#[no_mangle]
pub fn renderer_remove_window(env: JNIEnv, _clz: jclass, surface: jobject) {
    debug!("renderer_remove_window");

    unsafe {
        let window = ndk_sys::ANativeWindow_fromSurface(env.get_native_interface(), surface);
        core::remove_window(window as *mut c_void);
    }
}

#[no_mangle]
pub fn handle_touch(env: JNIEnv, _clz: jclass, event: jobject) {
    // TODO: cache the field id.
    let ptr = env.get_field(event, "mNativePtr", "J").unwrap();

    if let JValue::Long(p) = ptr {
        let ev = unsafe {
            let nonptr =
            std::ptr::NonNull::new(std::mem::transmute::<i64, *mut ndk_sys::AInputEvent>(p))
                .unwrap();
            ndk::event::MotionEvent::from_ptr(nonptr)
        };
        input::handle_touch(ev)
    }
}

pub fn send_key_code(_env: JNIEnv, _clz: jclass, keycode: jint) {
    debug!("send key code!");
    input::send_key_code(keycode);
}

unsafe fn register_natives(jvm: &JavaVM, class_name: &str, methods: &[NativeMethod]) -> jint {
    let env: JNIEnv = jvm.get_env().unwrap();
    let jni_version = env.get_version().unwrap();
    let version: jint = jni_version.into();

    debug!("JNI Version : {:#?} ", jni_version);

    let clazz = match env.find_class(class_name) {
        Ok(clazz) => clazz,
        Err(e) => {
            error!("java class not found : {:?}", e);
            return JNI_ERR;
        }
    };
    debug!("clazz: {:#?}", clazz);

    let result = env.register_native_methods(clazz, &methods);

    if result.is_ok() {
        debug!("register_natives : succeed");
        version
    } else {
        error!("register_natives : failed ");
        JNI_ERR
    }
}

#[no_mangle]
#[allow(non_snake_case)]
unsafe fn JNI_OnLoad(jvm: JavaVM, _reserved: *mut c_void) -> jint {
    android_logger::init_once(
        Config::default()
            .with_min_level(Level::Info)
            .with_tag("CLIENT_EGL"),
    );

    debug!("JNI_OnLoad");

    let class_name: &str = "io/twoyi/Renderer";
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
    ];

    register_natives(&jvm, class_name, jni_methods.as_ref())
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

// Main function for standalone execution when invoked via linker64
// Note: When called via linker64, argc/argv may not be properly initialized
// We use std::env::args() as a fallback to get arguments from the environment
#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const i8) -> i32 {
    use std::io::{self, Write};
    use std::env;
    
    let _ = writeln!(io::stdout(), "Twoyi Renderer - Standalone Mode");
    
    // When invoked via linker64, argc might be garbage/uninitialized
    // Use std::env::args() instead which reads from the environment properly
    let args: Vec<String> = env::args().collect();
    
    let _ = writeln!(io::stdout(), "Arguments received: {}", args.len());
    if !args.is_empty() {
        let _ = writeln!(io::stdout(), "Arguments:");
        for (i, arg) in args.iter().enumerate() {
            let _ = writeln!(io::stdout(), "  [{}]: {}", i, arg);
        }
    }
    
    let _ = writeln!(io::stdout(), "\nUsage: twoyi [OPTIONS] or linker64 libtwoyi.so [OPTIONS]");
    let _ = writeln!(io::stdout(), "Options:");
    let _ = writeln!(io::stdout(), "  --help                Show this help message");
    let _ = writeln!(io::stdout(), "  --width <width>       Set virtual display width (default: 720)");
    let _ = writeln!(io::stdout(), "  --height <height>     Set virtual display height (default: 1280)");
    let _ = writeln!(io::stdout(), "  --loader <path>       Set loader path");
    let _ = writeln!(io::stdout(), "  --start-input         Start input system only");
    let _ = writeln!(io::stdout(), "\nNote: This library is primarily designed to be loaded by the Twoyi app.");
    let _ = writeln!(io::stdout(), "For full functionality, use it as a JNI library via System.loadLibrary(\"twoyi\")");
    
    // Parse arguments using env::args() which works correctly with linker64
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
