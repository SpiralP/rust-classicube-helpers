use super::*;

// ------------- is_normal_message ------------------

#[test]
fn is_normal_message_zero_is_true() {
    assert!(is_normal_message(0));
}

#[test]
fn is_normal_message_nonzero_is_false() {
    for byte in 1u8..=5 {
        assert!(!is_normal_message(byte), "expected false for byte {byte}");
    }
}

// ------------- decode_normal_text adapter (pure, Linux-CI safe) ------------
//
// UNSAFE_GetString / cc_string::to_string are pure-Rust table lookups (no
// extern "C" symbol), so the &[u8] -> &str adapter links and runs on Linux
// without the Windows import library. Build a fake payload that mirrors the
// engine layout: 1 MsgType byte + STRING_SIZE-byte cc_string buffer.

/// Build a `MESSAGE_BUF_LEN`-byte payload: `type_byte` then `text` (UTF-8 bytes)
/// padded with NULs to fill the `STRING_SIZE`-byte string buffer.
fn make_payload(type_byte: u8, text: &str) -> Vec<u8> {
    let mut buf = vec![0u8; MESSAGE_BUF_LEN];
    buf[0] = type_byte;
    let text = text.as_bytes();
    buf[1..=text.len()].copy_from_slice(text);
    buf
}

#[test]
fn decode_normal_returns_trimmed_text() {
    // MSG_TYPE_NORMAL is 0; UNSAFE_GetString trims trailing NUL/space padding.
    let buf = make_payload(0, "hello");
    assert_eq!(decode_normal_text(&buf).as_deref(), Some("hello"));
}

#[test]
fn decode_non_normal_returns_none() {
    let buf = make_payload(1, "hello");
    assert_eq!(decode_normal_text(&buf), None);
}

#[test]
fn decode_short_buffer_returns_none() {
    // A buffer shorter than 1 + STRING_SIZE has no full cc_string to read.
    let buf = vec![0u8; MESSAGE_BUF_LEN - 1];
    assert_eq!(decode_normal_text(&buf), None);
}
