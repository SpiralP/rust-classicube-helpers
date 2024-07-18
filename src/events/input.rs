#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::os::raw::{c_float, c_int};

use classicube_sys::{cc_bool, cc_string, InputButtons};

use crate::make_event_handler;

make_event_handler!(
    /// Key input character is typed. Arg is a unicode character
    Input,
    Press,
    Int,
    (
        {
            name: key,
            rust_type: char,
            c_type: c_int,
            to_rust: |key| char::from_u32(u32::try_from(key).expect("u32::try_from(key)")).expect("char::from_u32(key)"),
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
            to_rust: |key| InputButtons::try_from(key).expect("InputButtons::try_from(key)"),
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
            to_rust: |key| InputButtons::try_from(key).expect("InputButtons::try_from(key)"),
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
    /// Text in the on-screen input keyboard changed (for Mobile)
    Input,
    TextChanged,
    String,
    (
        {
            name: s_ptr,
            rust_type: String,
            c_type: *const cc_string,
            to_rust: |s_ptr: *const cc_string| {
                unsafe { s_ptr.as_ref().expect("s_ptr.as_ref()") }.to_string()
            },
        },
    )
);
