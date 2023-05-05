#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::make_event_handler;
use classicube_sys::{cc_string, MsgType};
use std::os::raw::c_int;

make_event_handler!(
    /// User changes whether system chat font used, and when the bitmapped font texture changes
    Chat,
    FontChanged,
    Void,
    ()
);

make_event_handler!(
    /// Raised when message is being added to chat
    Chat,
    ChatReceived,
    Chat,
    (
        {
            name: message,
            rust_type: String,
            c_type: *const cc_string,
            to_rust: |message: *const cc_string| {
                unsafe { message.as_ref().unwrap() }.to_string()
            },
        },
        {
            name: message_type,
            rust_type: MsgType,
            c_type: c_int,
            to_rust: |message_type| MsgType::try_from(message_type).unwrap(),
        },
    )
);

make_event_handler!(
    /// Raised when user sends a message
    Chat,
    ChatSending,
    Chat,
    (
        {
            name: message,
            rust_type: String,
            c_type: *const cc_string,
            to_rust: |message: *const cc_string| {
                unsafe { message.as_ref().unwrap() }.to_string()
            },
        },
        {
            name: message_type,
            rust_type: MsgType,
            c_type: c_int,
            to_rust: |message_type| MsgType::try_from(message_type).unwrap(),
        },
    )
);

make_event_handler!(
    /// Raised when a colour code changes
    Chat,
    ColCodeChanged,
    Int,
    (
        {
            name: code,
            rust_type: c_int,
            c_type: c_int,
            to_rust: |code| code,
        },
    )
);
