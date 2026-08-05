//! Resolve `lua_debugdostring` via signature scanning.
//!
//! `lua_debugdostring` is the entry point the game uses to execute Lua
//! source from native code. We sigscan it once at install time and cache
//! the function pointer for later script execution.

use crate::{log_error, log_info};
use color_hook::memory::sigscan::{Signature, sig_scan_module_phdr};
use std::ffi::c_void;
use std::str::FromStr;

const LUA_DEBUGDOSTRING_PATTERN: &str = "FD 7B BD A9 FC 0B 00 F9 F4 4F 02 A9 FD 03 00 91 FF 43 24 D1 A8 23 00 D1 A9 63 00 91 F3 03 01 AA F4 03 00 AA A8 0F 00 F9 A9 83 1F F8 ?? ?? ?? ?? C0 00 00 35";
const LUA_DEBUGDOSTRING_MODULE: &str = "libBootloader.so";

type LuaDebugDoStringFn =
    unsafe extern "C" fn(state: *mut c_void, script: *const libc::c_char) -> u64;

static mut LUA_DEBUGDOSTRING_FN: Option<LuaDebugDoStringFn> = None;

/// Resolve `lua_debugdostring` and stash its address. Returns `true` on
/// success.
pub(super) unsafe fn find() -> bool {
    let sig = match Signature::from_str(LUA_DEBUGDOSTRING_PATTERN) {
        Ok(s) => s,
        Err(e) => {
            log_error!("lua_debugdostring: bad pattern — {}", e);
            return false;
        }
    };

    let target = match sig_scan_module_phdr(&sig, LUA_DEBUGDOSTRING_MODULE) {
        Some(addr) => {
            log_info!("lua_debugdostring: found at 0x{:X}", addr);
            addr
        }
        None => {
            log_error!(
                "lua_debugdostring: pattern not found in {}",
                LUA_DEBUGDOSTRING_MODULE
            );
            return false;
        }
    };

    unsafe {
        LUA_DEBUGDOSTRING_FN = Some(std::mem::transmute(target));
    }
    true
}

/// Return the resolved function pointer, if any.
pub(super) fn get() -> Option<LuaDebugDoStringFn> {
    unsafe { LUA_DEBUGDOSTRING_FN }
}
