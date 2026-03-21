// Copyright Disclaimer: AI-Generated Content
// This file was created by GitHub Copilot, an AI coding assistant.
// AI-generated content is not subject to copyright protection and is provided
// without any warranty, express or implied, including warranties of merchantability,
// fitness for a particular purpose, or non-infringement.
// Use at your own risk.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Open-source Dynamic Library Loader
//! 
//! This library provides a simple dynamic library loader that can load and execute
//! shared libraries. It's designed as a replacement for the legacy proprietary
//! libloader.so, which is used to bootstrap the Android container environment.
//!
//! The loader supports:
//! - Dynamic library loading via dlopen
//! - Symbol resolution via dlsym
//! - Direct execution as a PIE executable
//! - Environment variable-based configuration

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

/// RTLD flags for dlopen
const RTLD_LAZY: c_int = 0x00001;
const RTLD_NOW: c_int = 0x00002;
const RTLD_LOCAL: c_int = 0x00000;
const RTLD_GLOBAL: c_int = 0x00100;

// External C functions from libdl (dynamic linker)
extern "C" {
    fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlerror() -> *mut c_char;
}

/// Load a shared library and return its handle
/// 
/// # Safety
/// This function uses FFI to call dlopen
pub unsafe fn load_library(path: &str, flags: c_int) -> Result<*mut c_void, String> {
    let path_cstr = match CString::new(path) {
        Ok(s) => s,
        Err(_) => return Err("Invalid path string".to_string()),
    };
    
    let handle = dlopen(path_cstr.as_ptr(), flags);
    
    if handle.is_null() {
        let err_ptr = dlerror();
        let err_msg = if !err_ptr.is_null() {
            CStr::from_ptr(err_ptr).to_string_lossy().into_owned()
        } else {
            "Unknown error".to_string()
        };
        return Err(format!("Failed to load library {}: {}", path, err_msg));
    }
    
    Ok(handle)
}

/// Find a symbol in a loaded library
/// 
/// # Safety
/// This function uses FFI to call dlsym
pub unsafe fn find_symbol(handle: *mut c_void, symbol: &str) -> Result<*mut c_void, String> {
    let symbol_cstr = match CString::new(symbol) {
        Ok(s) => s,
        Err(_) => return Err("Invalid symbol string".to_string()),
    };
    
    // Clear any previous error
    dlerror();
    
    let sym_ptr = dlsym(handle, symbol_cstr.as_ptr());
    
    // Check if dlsym failed
    let err_ptr = dlerror();
    if !err_ptr.is_null() {
        let err_msg = CStr::from_ptr(err_ptr).to_string_lossy().into_owned();
        return Err(format!("Failed to find symbol {}: {}", symbol, err_msg));
    }
    
    Ok(sym_ptr)
}

/// Close a loaded library
/// 
/// # Safety
/// This function uses FFI to call dlclose
pub unsafe fn close_library(handle: *mut c_void) -> Result<(), String> {
    if dlclose(handle) != 0 {
        let err_ptr = dlerror();
        let err_msg = if !err_ptr.is_null() {
            CStr::from_ptr(err_ptr).to_string_lossy().into_owned()
        } else {
            "Unknown error".to_string()
        };
        return Err(format!("Failed to close library: {}", err_msg));
    }
    
    Ok(())
}

/// Main entry point when executed as a standalone executable
/// 
/// This allows the loader to be executed directly (as loader64) to load
/// and bootstrap libraries.
#[no_mangle]
pub extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    use std::env;
    use std::io::{self, Write};
    
    // Parse command line arguments
    let mut args: Vec<String> = Vec::new();
    
    if argc > 0 && !argv.is_null() {
        unsafe {
            for i in 0..argc as isize {
                let arg_ptr = *argv.offset(i);
                if !arg_ptr.is_null() {
                    if let Ok(arg_cstr) = CStr::from_ptr(arg_ptr).to_str() {
                        args.push(arg_cstr.to_string());
                    }
                }
            }
        }
    }
    
    // Print basic info
    let _ = writeln!(io::stderr(), "[LOADER] Open-source dynamic library loader v1.0.0");
    let _ = writeln!(io::stderr(), "[LOADER] Arguments: {:?}", args);
    
    // Check for library to load from arguments or environment
    let library_path = if args.len() > 1 {
        // First argument (after program name) is the library to load
        args[1].clone()
    } else if let Ok(env_lib) = env::var("LD_PRELOAD") {
        env_lib
    } else {
        let _ = writeln!(io::stderr(), "[LOADER] Usage: {} <library.so> [args...]", 
                        args.get(0).unwrap_or(&"loader".to_string()));
        let _ = writeln!(io::stderr(), "[LOADER] Or set LD_PRELOAD environment variable");
        return 1;
    };
    
    let _ = writeln!(io::stderr(), "[LOADER] Loading library: {}", library_path);
    
    // Load the library
    unsafe {
        let handle = match load_library(&library_path, RTLD_NOW | RTLD_GLOBAL) {
            Ok(h) => {
                let _ = writeln!(io::stderr(), "[LOADER] Library loaded successfully");
                h
            },
            Err(e) => {
                let _ = writeln!(io::stderr(), "[LOADER] Error: {}", e);
                return 1;
            }
        };
        
        // Try to find and execute a main function in the loaded library
        if let Ok(main_fn) = find_symbol(handle, "main") {
            let _ = writeln!(io::stderr(), "[LOADER] Found main() in library, executing...");
            
            // Cast to function pointer and call
            type MainFn = extern "C" fn(c_int, *const *const c_char) -> c_int;
            let main_func: MainFn = std::mem::transmute(main_fn);
            
            // Pass remaining arguments to the library's main
            let lib_argc = (args.len() - 1) as c_int;
            
            // Create CStrings that will live for the duration of the call
            let lib_args: Vec<CString> = args.iter()
                .skip(1)
                .map(|s| CString::new(s.as_str()).unwrap())
                .collect();
            
            // Create pointers from the CStrings
            let lib_argv: Vec<*const c_char> = lib_args.iter()
                .map(|s| s.as_ptr())
                .collect();
            
            let result = main_func(lib_argc, lib_argv.as_ptr());
            let _ = writeln!(io::stderr(), "[LOADER] Library main() returned: {}", result);
            
            // Clean up
            if let Err(e) = close_library(handle) {
                let _ = writeln!(io::stderr(), "[LOADER] Warning: {}", e);
            }
            
            return result;
        } else {
            let _ = writeln!(io::stderr(), "[LOADER] No main() function found in library");
            let _ = writeln!(io::stderr(), "[LOADER] Library loaded and symbols available");
            
            // Keep the library loaded - don't close it
            // This allows the process to use the library's symbols
            return 0;
        }
    }
}

/// Initialize function that can be called from JNI or other contexts
#[no_mangle]
pub extern "C" fn loader_init() {
    // Initialization if needed
}

/// Load a library by path (C API)
#[no_mangle]
pub extern "C" fn loader_load(path: *const c_char) -> *mut c_void {
    if path.is_null() {
        return ptr::null_mut();
    }
    
    unsafe {
        let path_str = match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        };
        
        match load_library(path_str, RTLD_NOW | RTLD_GLOBAL) {
            Ok(handle) => handle,
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Find a symbol in a library (C API)
#[no_mangle]
pub extern "C" fn loader_symbol(handle: *mut c_void, symbol: *const c_char) -> *mut c_void {
    if handle.is_null() || symbol.is_null() {
        return ptr::null_mut();
    }
    
    unsafe {
        let symbol_str = match CStr::from_ptr(symbol).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        };
        
        match find_symbol(handle, symbol_str) {
            Ok(ptr) => ptr,
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Close a library (C API)
#[no_mangle]
pub extern "C" fn loader_close(handle: *mut c_void) -> c_int {
    if handle.is_null() {
        return -1;
    }
    
    unsafe {
        match close_library(handle) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
}
