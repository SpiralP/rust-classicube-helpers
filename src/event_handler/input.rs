use crate::make_event_handler;
use classicube_sys::{cc_bool, Key};
use std::os::raw::{c_float, c_int};

make_event_handler!(
    /// Key input character is typed. Arg is a character
    Input,
    Press,
    Int,
    (
        {
            name: key,
            rust_type: Key,
            c_type: c_int,
            to_rust: |key| key as Key,
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
            rust_type: Key,
            c_type: c_int,
            to_rust: |key| key as Key,
        },
        {
            name: was_down,
            rust_type: bool,
            c_type: cc_bool,
            to_rust: |was_down| was_down != 0,
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
            rust_type: Key,
            c_type: c_int,
            to_rust: |key| key as Key,
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
            c_type: *const classicube_sys::String,
            to_rust: |s_ptr: *const classicube_sys::String| {
                unsafe { s_ptr.as_ref().unwrap() }.to_string()
            },
        },
    )
);
