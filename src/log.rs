//! Logging framework
//!
//! Outputs to Android logcat via `__android_log_print`.
use libc::{c_char, c_int};

pub const TAG: &str = "ThatSkyHook";
pub const ANDROID_LOG_INFO: libc::c_int = 4;
pub const ANDROID_LOG_WARN: libc::c_int = 5;
pub const ANDROID_LOG_ERROR: libc::c_int = 6;

#[link(name = "log")]
unsafe extern "C" {
    fn __android_log_print(prio: c_int, tag: *const c_char, fmt: *const c_char, ...) -> c_int;
}

pub fn log(prio: libc::c_int, msg: &str) {
    let tag_c = std::ffi::CString::new(TAG).unwrap_or_default();
    let msg_c = std::ffi::CString::new(msg).unwrap_or_default();
    unsafe {
        __android_log_print(prio, tag_c.as_ptr(), msg_c.as_ptr());
    }
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => { $crate::log::log($crate::log::ANDROID_LOG_INFO, &format!($($arg)*)) };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => { $crate::log::log($crate::log::ANDROID_LOG_WARN, &format!($($arg)*)) };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => { $crate::log::log($crate::log::ANDROID_LOG_ERROR, &format!($($arg)*)) };
}
