#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::os::raw::{c_float, c_int};

use classicube_sys::{cc_bool, cc_string, InputButtons};

use crate::make_event_handler;

make_event_handler!(
    /// Key input character is typed. Arg is a character
    Input,
    Press,
    Int,
    (
        {
            name: key,
            rust_type: char,
            c_type: c_int,
            to_rust: |key| u8::try_from(key).unwrap() as char,
        },
    )
);

make_event_handler!(
    /// Key or button is pressed. Arg is a member of Key enumeration
    Input,
    Down,
    Input,
    (
        {
            name: key,
            rust_type: InputButtons,
            c_type: c_int,
            to_rust: |key| InputButtons::try_from(key).unwrap(),
        },
        {
            name: repeating,
            rust_type: bool,
            c_type: cc_bool,
            to_rust: |repeating| repeating != 0,
        },
    )
);

make_event_handler!(
    /// Key or button is released. Arg is a member of Key enumeration
    Input,
    Up,
    Int,
    (
        {
            name: key,
            rust_type: InputButtons,
            c_type: c_int,
            to_rust: |key| InputButtons::try_from(key).unwrap(),
        },
    )
);

make_event_handler!(
    /// Mouse wheel is moved/scrolled (Arg is wheel delta)
    Input,
    Wheel,
    Float,
    (
        {
            name: delta,
            rust_type: c_float,
            c_type: c_float,
            to_rust: |delta| delta,
        },
    )
);

make_event_handler!(
    /// HTML text input changed
    Input,
    TextChanged,
    String,
    (
        {
            name: s_ptr,
            rust_type: String,
            c_type: *const cc_string,
            to_rust: |s_ptr: *const cc_string| {
                unsafe { s_ptr.as_ref().unwrap() }.to_string()
            },
        },
    )
);
