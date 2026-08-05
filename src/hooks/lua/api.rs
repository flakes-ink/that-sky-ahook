//! Public Lua engine API surface.
//!
//! [`lua_exec`] runs a Lua script inline (caller's responsibility to be
//! on a thread where the Lua VM is usable), [`queue_script`] defers
//! execution to the next game frame, and [`is_lua_ready`] reports whether
//! the VM state has been captured. [`log`] is a host helper intended for
//! registration as a Lua-callable global.

use crate::{log_error, log_info};
use std::ffi::CString;

use super::{debugdostring, update_sync};

/// Execute a Lua script string inside the game's Lua VM.
///
/// Returns `true` on success (when `lua_debugdostring` returns 0).
pub unsafe fn lua_exec(script: &str) -> bool {
    let state = update_sync::lua_state();
    if state.is_null() {
        log_error!("lua_exec: Lua state not yet captured");
        return false;
    }

    let fn_ptr = match debugdostring::get() {
        Some(f) => f,
        None => {
            log_error!("lua_exec: lua_debugdostring not found");
            return false;
        }
    };

    let c_script = match CString::new(script) {
        Ok(s) => s,
        Err(e) => {
            log_error!("lua_exec: script contains null byte — {}", e);
            return false;
        }
    };

    let ret = unsafe { fn_ptr(state, c_script.as_ptr()) };
    if ret != 0 {
        log_error!("lua_exec: lua_debugdostring returned error {}", ret);
        return false;
    }

    log_info!("lua_exec: script executed successfully");
    true
}

/// Push a Lua script into the queue. It will execute on the next game
/// frame.
pub fn queue_script(script: &str) {
    update_sync::push_script(script);
}

/// Returns `true` once the game's Lua state has been captured.
pub fn is_lua_ready() -> bool {
    update_sync::is_ready()
}
