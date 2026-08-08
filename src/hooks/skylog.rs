//! Sky Log
//! Hook 1 — certificate verification (always approve).

use crate::{log_error, log_info, ui};
use color_hook::hook::{HookFargs8, hook_wrap8};
use color_hook::memory::sigscan::{Signature, sig_scan_module_phdr_b_all};
use std::ops::Sub;
use std::str::FromStr;

const LOG_PATTERN: &str = "?? ?? ?? ?? E8 ?? ?? B9 E9 ?? ?? F9 3F 69 28 38 08 ?? ?? 52 E8 ?? ?? B9 28 ?? ?? 52 FF 7F ?? A9 E8 ?? ?? 39";
const LOG_MODULE: &str = "libBootloader.so";

#[unsafe(no_mangle)]
#[inline(never)]
extern "C" fn before(f: *mut HookFargs8, _: *mut std::ffi::c_void) {
    unsafe {
        let this = f.read().arg0 as *mut u8;
        let start = this.add(109);

        let pptr = *(this.add(56) as *mut usize) as usize;

        let len = pptr.sub(start as usize);

        let buf = std::slice::from_raw_parts(start, len);

        let msg = String::from_utf8_lossy(buf);

        ui::log!("[sky] {}", msg);
    }
}

extern "C" fn after(_f: *mut HookFargs8, _: *mut std::ffi::c_void) {}

pub(super) unsafe fn install() -> bool {
    let sig = match Signature::from_str(LOG_PATTERN) {
        Ok(s) => s,
        Err(e) => {
            log_error!("SkyLog: bad pattern — {}", e);
            return false;
        }
    };

    let targets = sig_scan_module_phdr_b_all(&sig, LOG_MODULE);

    let target = if targets.len() != 0 {
        log_info!("SkyLog: found at 0x{:X}", targets[0]);
        targets[0]
    } else {
        log_error!("SkyLog: pattern not found in {}", LOG_MODULE);
        return false;
    };

    unsafe {
        match hook_wrap8(
            target as *const libc::c_void,
            before,
            after,
            std::ptr::null_mut() as *mut libc::c_void,
        ) {
            Ok(()) => {
                log_info!("SkyLog: hook installed");
                true
            }
            Err(e) => {
                log_error!("SkyLog: hook failed — {}", e);
                false
            }
        }
    }
}
