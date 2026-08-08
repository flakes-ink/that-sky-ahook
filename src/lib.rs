//! libBootloader.so certificate-verification hook injector.
//!
//! When loaded into the target process, the `.init_array` constructor
//! spawns a worker thread that waits for `libBootloader.so` to load,
//! pattern-scans its readable segments for the certificate verification
//! function, and hooks it to always return `true`.
//!
//! Build:
//!   cargo build --target aarch64-linux-android --release
//! Output: `target/aarch64-linux-android/release/libinject.so`

pub mod hooks;
pub mod log;
pub mod udp;
pub mod ui;

use std::ffi::c_void;

use ui::android::JavaVM;

// --- Worker thread ---

extern "C" fn worker_thread(_arg: *mut c_void) -> *mut c_void {
    log_info!("worker thread started");
    unsafe { libc::sleep(3) };

    // Runs on a pthread (not a Rust thread): a panic must not unwind across
    // the C boundary, so catch it here.
    let _ = std::panic::catch_unwind(|| {
        let installed = hooks::install_all();
        log_info!("{} hook(s) installed", installed);
        if installed == 0 {
            log_error!("failed to install any hooks");
        }

        // Start the UDP listener so external tools can inject Lua remotely.
        udp::start_udp_listener();
    });

    std::ptr::null_mut()
}

// --- Entry points ---

#[unsafe(no_mangle)]
extern "C" fn init_hook() {
    log_info!("init_hook called");

    let mut tid: libc::pthread_t = unsafe { std::mem::zeroed() };
    let ret = unsafe {
        libc::pthread_create(
            &mut tid,
            std::ptr::null(),
            worker_thread,
            std::ptr::null_mut(),
        )
    };
    if ret == 0 {
        unsafe { libc::pthread_detach(tid) };
        log_info!("worker thread created");
    } else {
        log_error!("pthread_create failed: {}", ret);
    }
}

core::arch::global_asm! {
    ".section .init_array\n\
     .align 3\n\
     .xword {init_hook}",
    init_hook = sym init_hook,
}

// --- JNI entry point ---

/// `JNI_OnLoad`: store the JavaVM, set up the overlay, return quickly.
/// The surface wait + rendering run on a dedicated thread.
#[unsafe(no_mangle)]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _reserved: *mut std::ffi::c_void) -> i32 {
    log_info!("[rust] JNI_OnLoad");

    // Do not unwind across the JNI boundary (the process would crash).
    let _ = std::panic::catch_unwind(|| {
        let _ = ui::android::init_jvm(vm.clone());

        let _ = vm
            .with_top_local_frame(|env| match ui::android::init_overlay(env) {
                Ok(()) => {
                    log_info!("[rust] overlay initialized");
                    Ok(())
                }
                Err(e) => {
                    log_error!("[rust] init_overlay FAILED: {:?}", e);
                    Err(e)
                }
            })
            .ok();
    });
    jni::sys::JNI_VERSION_1_6
}
