//! Expose host-side logging to the game's Lua VM as `sle.log` / `sle.print`.
//!
//! Port of the C++ mod's `sleAddLuaBindings` (`crates/that-sky-lua/src/bindings.c`):
//! the `sle` namespace is installed into the game's own VM, and `sle.log`
//! becomes a real C closure that forwards to [`crate::log_info!`] (logcat).
//!
//! Registration wraps the captured `lua_State*` with mlua via
//! [`Lua::get_or_init_from_ptr`]. This is only sound because our mlua links
//! the exact Lua 5.2.0 the game embeds (`flake.nix` builds it from the
//! pristine lua-5.2.0 tarball), so the `lua_State` layout matches. The old
//! approach used mlua's vendored LuaJIT and froze the game on an ABI
//! mismatch; the LuaJIT-FFI fallback is gone because the game's Lua is not
//! LuaJIT.

use crate::{log_error, log_info};

use super::update_sync;

/// Install `_G.sle.log` / `_G.sle.print` into the game's Lua VM.
///
/// The wrapped state is **not owned** by mlua (`owned = false`), so the
/// handle's `Drop` never calls `lua_close` — the game keeps its VM. The
/// registered functions stay valid for the state's lifetime: mlua caches
/// its instance in the VM registry and resolves it from there whenever the
/// game later calls `sle.log` / `sle.print`.
///
/// # Safety
///
/// Must be called on the game's main thread with the VM idle (the
/// [`update_sync`] capture frame satisfies this). Returns `true` on
/// success.
pub unsafe fn register() -> bool {
    let state = update_sync::lua_state();
    if state.is_null() {
        log_error!("bindings: Lua state not captured yet");
        return false;
    }

    // SAFETY: the game keeps the VM alive for the whole process, and the
    // caller runs on the game thread with no other code touching the VM.
    let lua = unsafe { mlua::Lua::get_or_init_from_ptr(state.cast::<mlua::lua_State>()) };

    let result = (|| -> mlua::Result<()> {
        let globals = lua.globals();

        // `sle` namespace — create the table if missing, like `bindings.c`.
        if globals.get::<mlua::Value>("sle")?.is_nil() {
            globals.set("sle", lua.create_table()?)?;
        }
        let sle: mlua::Table = globals.get("sle")?;

        // `sle.log(msg)` → host-side logcat via `log_info!`.
        let log_fn = lua.create_function(|_, msg: String| {
            log_info!("[sle] {}", msg);
            Ok(())
        })?;
        sle.set("log", log_fn)?;

        // `sle.print(...)` — stringify each arg, join with a tab, log.
        let print_fn = lua.create_function(|lua, args: mlua::MultiValue| {
            let mut parts: Vec<String> = Vec::new();
            for value in args {
                let part = match value {
                    mlua::Value::Nil => "nil".to_string(),
                    mlua::Value::Boolean(b) => b.to_string(),
                    mlua::Value::Integer(i) => i.to_string(),
                    mlua::Value::Number(n) => n.to_string(),
                    mlua::Value::String(s) => s.to_string_lossy(),
                    other => lua
                        .coerce_string(other)?
                        .map(|s| s.to_string_lossy())
                        .unwrap_or_else(|| "<non-string>".to_string()),
                };
                parts.push(part);
            }
            log_info!("[sle] {}", parts.join("\t"));
            Ok(())
        })?;
        sle.set("print", print_fn)?;

        Ok(())
    })();

    match result {
        Ok(()) => {
            log_info!("bindings: sle.log / sle.print installed");
            true
        }
        Err(e) => {
            log_error!("bindings: install failed — {}", e);
            false
        }
    }
}
