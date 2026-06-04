//! Wipe-safe `Protocol.Handlers[OPCODE_MESSAGE]` hook.
//!
//! `ClassiCube`'s `Protocol` component wipes `Protocol.Handlers` on every
//! `Game_Reset` (disconnect). [`ProtocolMessageHook`] re-installs the hook
//! after the wipe and avoids the infinite-recursion trap when a foreign
//! plugin has stacked on top.
//!
//! Each plugin binary that links classicube-helpers gets its own copy of
//! the internal `trampoline` function (rlib static linking), so the hook
//! address is unique per plugin `.so` even though all share this source.
//!
//! # Main-thread requirement
//!
//! All public methods must be called on `ClassiCube`'s main game-loop thread.
//! The internal thread-local storage (`OLD`, `CALLBACK`) is keyed to that
//! thread; calling from a worker thread would silently create a separate,
//! useless slot.

use std::{
    cell::{Cell, RefCell},
    ptr, slice,
};

use classicube_sys::{
    MsgType, MsgType_MSG_TYPE_NORMAL, Net_Handler, OPCODE__OPCODE_MESSAGE, Protocol, Server,
    UNSAFE_GetString,
};

#[cfg(test)]
mod tests;

type Callback = RefCell<Option<Box<dyn FnMut(&str) -> bool>>>;

thread_local! {
    static OLD: Cell<Net_Handler> = const { Cell::new(None) };
    static CALLBACK: Callback = RefCell::default();
}

const IDX: usize = OPCODE__OPCODE_MESSAGE as usize;

/// Returns `true` if `type_byte` indicates a normal chat message
/// (`MSG_TYPE_NORMAL`). Pure function -- does not access engine state.
#[must_use]
fn is_normal_message(type_byte: u8) -> bool {
    MsgType::from(type_byte) == MsgType_MSG_TYPE_NORMAL
}

// This function's address in each plugin .so is unique (rlib static linking),
// making it safe to use as a hook identity via `ptr::fn_addr_eq`.
extern "C" fn trampoline(data: *mut u8) {
    // SAFETY: ClassiCube's Handlers callback contract: `data` points to at
    // least 65 bytes (1 MsgType byte + 64-byte cc_string buffer).
    let bytes = unsafe { slice::from_raw_parts(data, 65) };
    let should_suppress = if is_normal_message(bytes[0]) {
        // SAFETY: `bytes[1..]` is exactly 64 bytes; UNSAFE_GetString requires
        // at least STRING_SIZE (64) bytes and borrows them for the call.
        let text = unsafe { UNSAFE_GetString(&bytes[1..]) }.to_string();
        // try_borrow_mut: if the callback somehow re-enters the trampoline
        // (e.g. via chat_print -> ChatEvents -> Protocol roundtrip), don't
        // panic; just forward the inner message without calling the callback.
        CALLBACK.with(|cell| {
            cell.try_borrow_mut()
                .is_ok_and(|mut opt| opt.as_mut().is_some_and(|cb| cb(&text)))
        })
    } else {
        false
    };
    if !should_suppress {
        OLD.with(|c| {
            if let Some(f) = c.get() {
                // SAFETY: `f` is the previously-installed Net_Handler, which
                // ClassiCube guarantees is valid while the Protocol layer is live.
                unsafe { f(data) }
            }
        });
    }
}

#[must_use]
fn ours() -> unsafe extern "C" fn(*mut u8) {
    trampoline
}

#[must_use]
fn is_our_handler(h: Net_Handler) -> bool {
    h.is_some_and(|f| ptr::fn_addr_eq(f, ours()))
}

#[must_use]
fn handlers_eq(a: Net_Handler, b: Net_Handler) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => ptr::fn_addr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

/// Returns `true` when running in singleplayer, where `ClassiCube`'s `Protocol`
/// component is inert (its `OnInit`/`OnReset` early-return) -- there is no
/// handler table to hook or re-hook.
#[must_use]
fn is_single_player() -> bool {
    // SAFETY: Server is valid by the time any plugin component is loaded.
    unsafe { (&raw const Server.IsSinglePlayer).read() != 0 }
}

fn install_inner() {
    // SAFETY: reading from the handler table; Protocol is valid once
    // ClassiCube's component chain has initialised (before any Net handler
    // can be invoked).
    let current: Net_Handler = unsafe {
        (&raw const Protocol.Handlers)
            .cast::<Net_Handler>()
            .add(IDX)
            .read()
    };
    if is_our_handler(current) {
        return; // already on top
    }
    let old = OLD.with(Cell::get);
    // Distinguish ClassiCube wipe from foreign-plugin-on-top:
    //   current == old  -> wipe restored the default handler we originally
    //                      chained onto -> re-install.
    //   current != old  -> a foreign plugin stacked above us; we are still
    //                      reachable via their chain; re-pushing would loop
    //                      -> leave it alone.
    if old.is_some() && !handlers_eq(current, old) {
        return;
    }
    // SAFETY: writing the same table under the same lifetime constraints.
    unsafe {
        (&raw mut Protocol.Handlers)
            .cast::<Net_Handler>()
            .add(IDX)
            .write(Some(ours()));
    }
    OLD.with(|c| c.set(current));
}

fn uninstall_inner() {
    let current: Net_Handler = unsafe {
        (&raw const Protocol.Handlers)
            .cast::<Net_Handler>()
            .add(IDX)
            .read()
    };
    if is_our_handler(current) {
        let prior = OLD.with(Cell::take);
        unsafe {
            (&raw mut Protocol.Handlers)
                .cast::<Net_Handler>()
                .add(IDX)
                .write(prior);
        }
    }
    // Else: a foreign plugin is on top. Leave the slot and OLD alone. The
    // trampoline is still reachable via their chain; with CALLBACK cleared by
    // Drop it will just forward to OLD without calling plugin-specific code.
}

/// RAII handle for a `Protocol.Handlers[OPCODE_MESSAGE]` hook.
///
/// The callback receives the message text for `MSG_TYPE_NORMAL` messages and
/// returns `true` to suppress (hide) the message or `false` to pass it on.
/// Non-normal messages are always forwarded without calling the callback.
///
/// Call [`reinstall`](Self::reinstall) from your plugin component's `Reset`
/// callback after each reconnect; `ClassiCube`'s `Protocol` component wipes the
/// handler table on every disconnect.
///
/// Dropping uninstalls (if on top) and clears the callback. Any foreign plugin
/// that stacked above will keep calling through the trampoline, but it becomes
/// a transparent forwarder with no plugin-specific processing.
pub struct ProtocolMessageHook;

impl ProtocolMessageHook {
    /// Install the hook and return a handle.
    ///
    /// Returns `None` in singleplayer (no `Protocol` layer; nothing to hook).
    ///
    /// # Panics
    ///
    /// Panics if a hook is already installed. There is only one
    /// `Protocol.Handlers[OPCODE_MESSAGE]` slot per plugin binary; installing
    /// twice would silently overwrite the callback and corrupt the first
    /// handle's `Drop`. Call `drop` (or let the handle go out of scope) before
    /// installing again.
    #[must_use]
    pub fn install<F>(callback: F) -> Option<Self>
    where
        F: FnMut(&str) -> bool + 'static,
    {
        if is_single_player() {
            return None;
        }
        assert!(
            CALLBACK.with_borrow(Option::is_none),
            "ProtocolMessageHook already installed; drop the existing handle before calling \
             install again",
        );
        CALLBACK.with_borrow_mut(|opt| *opt = Some(Box::new(callback)));
        install_inner();
        Some(Self)
    }

    /// Re-hook after `ClassiCube` wipes `Protocol.Handlers` on disconnect.
    ///
    /// Call from your plugin component's `Reset` callback. Idempotent: safe
    /// to call if the hook was never wiped (already on top -> no-op). No-op in
    /// singleplayer, where there is no `Protocol` layer to hook -- mirrors
    /// `ClassiCube`'s own `Protocol` `OnReset`, which early-returns there.
    pub fn reinstall(&self) {
        if is_single_player() {
            return;
        }
        install_inner();
    }
}

impl Drop for ProtocolMessageHook {
    fn drop(&mut self) {
        uninstall_inner();
        // Clear CALLBACK unconditionally: even when uninstall_inner no-ops
        // (foreign plugin on top), we must clear the callback so a later
        // `install` call does not spuriously hit the double-install assert.
        CALLBACK.with_borrow_mut(|opt| *opt = None);
    }
}
