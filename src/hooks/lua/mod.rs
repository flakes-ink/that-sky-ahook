//! Hooks 3 & 4 — Lua engine integration.
//!
//! 1. Signature-scans [`debugdostring`] (`lua_debugdostring`) so we can
//!    execute arbitrary Lua inside the game VM.
//! 2. Hooks [`update_sync`] (`Game::UpdateSync`) to capture the
//!    `lua_State*` on the first frame and drain the script queue every
//!    frame thereafter. On the capture frame, [`bindings`] queues a
//!    small Lua source run by the game's own `lua_debugdostring`; it
//!    uses the game's LuaJIT FFI to call our exported `sle_log`, so
//!    `_G.sle.log` / `_G.sle.print` reach host [`crate::log_info!`]
//!    without the Rust side ever touching `lua_State*`.
//!
//! Public helpers are re-exported flat from this module; see [`api`].

mod api;
mod bindings;
mod debugdostring;
mod update_sync;

// Re-export public helpers so callers see a flat `hooks::lua::*` namespace.
pub use api::{is_lua_ready, lua_exec, queue_script};

use crate::log_error;

/// Install the Lua-related hooks. Must be called after
/// [`debugdostring::find`] succeeds.
pub(super) unsafe fn install() -> bool {
    let ok = unsafe { debugdostring::find() && update_sync::install() };
    if !ok {
        log_error!("lua: install failed — skipping Lua hooks");
    }
    ok
}
