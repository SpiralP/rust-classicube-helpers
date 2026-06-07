//! Chat-message specialization of the generic [`ProtocolHook`].
//!
//! Wraps a [`ProtocolHook`] on `OPCODE_MESSAGE`, parsing its raw payload (a
//! 1-byte `MsgType` tag followed by a `STRING_SIZE`-byte `cc_string` buffer)
//! and handing the decoded text to a `FnMut(&str) -> bool` callback for
//! `MSG_TYPE_NORMAL` messages. Non-normal messages are always forwarded without
//! calling the callback.
//!
//! All the wipe-safety, reload, and `dlclose` reasoning lives on
//! [`ProtocolHook`]; this is a thin text adapter on top of it.

#[cfg(test)]
mod tests;

use classicube_sys::{
    MsgType, MsgType_MSG_TYPE_NORMAL, OPCODE__OPCODE_MESSAGE, STRING_SIZE, UNSAFE_GetString,
};

use crate::protocol_hook::ProtocolHook;

/// `OPCODE_MESSAGE` as the `u8` opcode index [`ProtocolHook`] expects.
/// classicube-sys types the constant as a `c_uint`, but every opcode is a fixed
/// engine value indexing the 256-entry handler table, so it always fits `u8`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "OPCODE_MESSAGE is a fixed engine opcode well within u8 range"
)]
const OPCODE_MESSAGE: u8 = OPCODE__OPCODE_MESSAGE as u8;

/// Size of the `Handlers[OPCODE_MESSAGE]` payload: a 1-byte `MsgType` tag
/// followed by a `STRING_SIZE`-byte `cc_string` text buffer. `UNSAFE_GetString`
/// requires the trailing slice to be at least `STRING_SIZE` bytes, so this stays
/// tied to the engine constant rather than a literal 65.
const MESSAGE_BUF_LEN: usize = 1 + STRING_SIZE as usize;

/// Returns `true` if `type_byte` indicates a normal chat message
/// (`MSG_TYPE_NORMAL`). Pure function -- does not access engine state.
#[must_use]
fn is_normal_message(type_byte: u8) -> bool {
    MsgType::from(type_byte) == MsgType_MSG_TYPE_NORMAL
}

/// Decode the chat text from a raw `OPCODE_MESSAGE` payload, returning
/// `Some(text)` for a long-enough `MSG_TYPE_NORMAL` packet and `None`
/// otherwise (non-normal type, or a buffer shorter than `MESSAGE_BUF_LEN`).
/// Factored out so the `&[u8] -> &str` adapter can be unit-tested without the
/// `Protocol`/`Server` globals the FFI hook install touches.
#[must_use]
fn decode_normal_text(bytes: &[u8]) -> Option<String> {
    if bytes.len() >= MESSAGE_BUF_LEN && is_normal_message(bytes[0]) {
        // SAFETY: bytes[1..] is the cc_string buffer (>= STRING_SIZE bytes),
        // which UNSAFE_GetString borrows for the call.
        Some(unsafe { UNSAFE_GetString(&bytes[1..]) }.to_string())
    } else {
        None
    }
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
/// Dropping uninstalls (if on top) and clears the callback -- see
/// [`ProtocolHook`] for the full lifetime and unload semantics.
pub struct ProtocolMessageHook(ProtocolHook);

impl ProtocolMessageHook {
    /// Install the chat-message hook and return a handle.
    ///
    /// Returns `None` in singleplayer (no `Protocol` layer; nothing to hook).
    ///
    /// # Panics
    ///
    /// Panics if a hook for `OPCODE_MESSAGE` is already installed in this plugin
    /// binary. Drop the existing handle before installing again.
    #[must_use]
    pub fn install<F>(mut callback: F) -> Option<Self>
    where
        F: FnMut(&str) -> bool + 'static,
    {
        ProtocolHook::install(OPCODE_MESSAGE, move |bytes: &[u8]| {
            decode_normal_text(bytes).is_some_and(|text| callback(&text))
        })
        .map(Self)
    }

    /// Re-hook after `ClassiCube` wipes `Protocol.Handlers` on disconnect.
    ///
    /// Call from your plugin component's `Reset` callback. See
    /// [`ProtocolHook::reinstall`].
    pub fn reinstall(&self) {
        self.0.reinstall();
    }
}
