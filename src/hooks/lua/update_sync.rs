//! Hook `Game::UpdateSync`.
//!
//! On the first frame we capture the game's `lua_State*` and install the
//! `sle` bindings on the game thread ([`super::bindings`]); every frame
//! thereafter we drain the script queue and feed each pending script to
//! `lua_debugdostring`.

use crate::{log_error, log_info};
use color_hook::hook::hook;
use color_hook::memory::sigscan::{Signature, sig_scan_module_phdr};
use std::collections::VecDeque;
use std::ffi::{CString, c_void};
use std::str::FromStr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicPtr, Ordering};

use super::debugdostring;

const GAME_UPDATE_SYNC_PATTERN: &str = "FD 7B BD A9 FC 0B 00 F9 F4 4F 02 A9 FD 03 00 91 E9 43 27 D1 3F E9 7B 92 E8 63 26 91 A9 63 00 91 F3 03 00 AA A8 0F 00 F9 E9 CF 04 F9";
const GAME_UPDATE_SYNC_MODULE: &str = "libBootloader.so";

static mut GAME_UPDATE_SYNC_BACKUP: *const c_void = std::ptr::null();
static GAME_LUA_STATE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static SCRIPT_QUEUE: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

type GameUpdateSyncFn = unsafe extern "C" fn(a1: *mut *mut u64);

#[unsafe(no_mangle)]
#[inline(never)]
extern "C" fn hook_game_update_sync(a1: *mut *mut u64) {
    // --- first-call: capture the Lua state --------------------------------
    {
        if GAME_LUA_STATE.load(Ordering::Relaxed).is_null() {
            let lua_state = unsafe { *(a1.add(4) as *const *mut c_void) };
            if !lua_state.is_null() {
                GAME_LUA_STATE.store(lua_state, Ordering::Release);
                log_info!("game_update_sync: captured Lua state at {:p}", lua_state);
                // First frame only: install the `sle` bindings into the
                // game VM. Runs on the game thread — see `bindings::register`.
                unsafe { super::bindings::register() };
            }
        }
    }

    // --- drain and execute the script queue -------------------------------
    {
        let state = GAME_LUA_STATE.load(Ordering::Acquire);
        if !state.is_null() {
            if let Some(fn_ptr) = debugdostring::get() {
                let pending: VecDeque<String> = {
                    let mut queue = SCRIPT_QUEUE.lock().unwrap();
                    std::mem::take(&mut *queue)
                };

                for script in pending {
                    let preview: String = script
                        .chars()
                        .take(80)
                        .chain(if script.len() > 80 { Some('…') } else { None })
                        .collect();
                    log_info!("running queued script: {}", preview);

                    let c_script = match CString::new(script.as_str()) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    let ret = unsafe { fn_ptr(state, c_script.as_ptr()) };
                    if ret != 0 {
                        log_error!("queued script failed: lua_debugdostring => {}", ret);
                    }
                }
            }
        }
    }

    // --- tail-call the original function ---------------------------------
    let original: GameUpdateSyncFn = unsafe { std::mem::transmute(GAME_UPDATE_SYNC_BACKUP) };
    unsafe { original(a1) };
}

pub(super) unsafe fn install() -> bool {
    let sig = match Signature::from_str(GAME_UPDATE_SYNC_PATTERN) {
        Ok(s) => s,
        Err(e) => {
            log_error!("game_update_sync: bad pattern — {}", e);
            return false;
        }
    };

    let target = match sig_scan_module_phdr(&sig, GAME_UPDATE_SYNC_MODULE) {
        Some(addr) => {
            log_info!("game_update_sync: found at 0x{:X}", addr);
            addr
        }
        None => {
            log_error!(
                "game_update_sync: pattern not found in {}",
                GAME_UPDATE_SYNC_MODULE
            );
            return false;
        }
    };

    let backup = std::ptr::addr_of_mut!(GAME_UPDATE_SYNC_BACKUP);
    unsafe {
        match hook(
            target as *const c_void,
            hook_game_update_sync as *const c_void,
            &mut *backup,
        ) {
            Ok(()) => {
                log_info!("game_update_sync: hook installed");
                true
            }
            Err(e) => {
                log_error!("game_update_sync: hook failed — {}", e);
                false
            }
        }
    }
}

/// Push a Lua script onto the queue. It will execute on the next game
/// frame.
pub(super) fn push_script(script: &str) {
    let mut queue = SCRIPT_QUEUE.lock().unwrap();
    queue.push_back(script.to_string());
    log_info!("script queued ({} pending)", queue.len());
}

/// Read-only view of the captured Lua VM pointer (`null` until captured).
pub(super) fn lua_state() -> *mut c_void {
    GAME_LUA_STATE.load(Ordering::Acquire)
}

/// Returns `true` once the game's Lua state has been captured.
pub(super) fn is_ready() -> bool {
    !GAME_LUA_STATE.load(Ordering::Relaxed).is_null()
}
