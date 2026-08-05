//! Expose host-side logging to the game's Lua VM via LuaJIT FFI.
//!
//! Unlike registering a C cfunction through `mlua::ffi` (which requires
//! mlua's vendored LuaJIT to share its `lua_State` layout with the
//! game's embedded Lua VM — an ABI match we cannot guarantee and that
//! froze the game on this device), this approach **never touches the
//! `lua_State*`**.
//!
//! Instead we declare a plain C entry point, [`sle_log`], export it
//! from this `.so`, and queue a small Lua source chunk that uses the
//! game's own LuaJIT FFI (`ffi.C.sle_log`) to call it. The game's
//! LuaJIT resolves the symbol against the global symbol table; if the
//! game's Lua is not LuaJIT (or our `.so` was loaded with
//! `RTLD_LOCAL`), the `pcall` simply fails and `sle.log` stays unbound
//! — the game keeps running, and the failure is visible in logcat as
//! a Lua error rather than as a crash/freeze.

use crate::log_info;
use std::ffi::{CStr, c_char};

use super::update_sync;

/// `sle_log(msg)` — host entry point callable from LuaJIT FFI.
///
/// Logs the Lua-supplied string to logcat under the host tag via
/// [`log_info!`]. Exported with `#[unsafe(no_mangle)]` so `ffi.C` can
/// resolve it by name.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sle_log(msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    unsafe {
        let s = CStr::from_ptr(msg).to_string_lossy();
        log_info!("[sle] {}", s);
    }
}

/// Queue the Lua source that installs `_G.sle.log` / `_G.sle.print`
/// using LuaJIT FFI. The game runs the chunk through its own
/// `lua_debugdostring` on the same frame.
///
/// Wrapped in `pcall(require, "ffi")`: if the game's Lua is not
/// LuaJIT, `ffi` is unavailable and the bindings silently stay unset
/// (no freeze). Check logcat for the Lua error in that case.
pub fn register() {
    let chunk = concat!(
        // Probe for LuaJIT FFI without aborting the chunk on failure.
        "local ok, ffi = pcall(require, \"ffi\") ",
        "if not ok or not ffi then return end ",
        // Declare our host entry point.
        "ffi.cdef[[ void sle_log(const char *msg); ]] ",
        // Install the `sle` namespace.
        "_G.sle = _G.sle or {} ",
        "_G.sle.log = _G.sle.log or function(msg) ",
        "  ffi.C.sle_log(tostring(msg)) ",
        "end ",
        // `sle.print` joins args with a tab and forwards to `sle.log`.
        "_G.sle.print = _G.sle.print or function(...) ",
        "  _G.sle.log(table.concat({...}, \"\\t\")) ",
        "end",
    );
    update_sync::push_script(chunk);
}
