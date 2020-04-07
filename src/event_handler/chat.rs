use crate::make_event_handler;
use classicube_sys::MsgType;
use std::os::raw::c_int;

make_event_handler!(
    Chat,
    ChatReceived,
    Chat,
    (
        {
            name: message,
            rust_type: String,
            c_type: *const classicube_sys::String,
            to_rust: |message: *const classicube_sys::String| {
                unsafe { message.as_ref().unwrap() }.to_string()
            },
        },
        {
            name: message_type,
            rust_type: MsgType,
            c_type: c_int,
            to_rust: |message_type| message_type as MsgType,
        },
    )
);
