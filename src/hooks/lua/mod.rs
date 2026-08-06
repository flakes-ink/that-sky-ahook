//! Hooks 3 & 4 — Lua engine integration.
//!
//! 1. Signature-scans [`debugdostring`] (`lua_debugdostring`) so we can
//!    execute arbitrary Lua inside the game VM.
//! 2. Hooks [`update_sync`] (`Game::UpdateSync`) to capture the
//!    `lua_State*` on the first frame and drain the script queue every
//!    frame thereafter. On the capture frame, [`bindings`] installs the
//!    `sle` namespace (`sle.log` / `sle.print`) into the game VM on the
//!    game thread; the functions forward to host [`crate::log_info!`]
//!    through the mlua wrapper (exact Lua 5.2.0 ABI, see `flake.nix`).
//!    [`local_engine`] is an independent mlua VM used for host-side
//!    script evaluation.
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
    log_error!("lua: install failed — skipping Lua hooks");
    ok
}
