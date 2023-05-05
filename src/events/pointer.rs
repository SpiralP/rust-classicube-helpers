use crate::make_event_handler;
use std::os::raw::{c_float, c_int};

make_event_handler!(
    /// Pointer position changed (Arg is delta from last position)
    Pointer,
    Moved,
    Int,
    (
        {
            name: idx,
            rust_type: c_int,
            c_type: c_int,
            to_rust: |a| a,
        },
    )
);

make_event_handler!(
    /// Left mouse or touch is pressed (Arg is index)
    Pointer,
    Down,
    Int,
    (
        {
            name: idx,
            rust_type: c_int,
            c_type: c_int,
            to_rust: |a| a,
        },
    )
);

make_event_handler!(
    /// Left mouse or touch is released (Arg is index)
    Pointer,
    Up,
    Int,
    (
        {
            name: idx,
            rust_type: c_int,
            c_type: c_int,
            to_rust: |a| a,
        },
    )
);

make_event_handler!(
    /// Raw pointer position changed (Arg is delta)
    Pointer,
    RawMoved,
    RawMove,
    (
        {
            name: x_delta,
            rust_type: c_float,
            c_type: c_float,
            to_rust: |a| a,
        },
        {
            name: y_delta,
            rust_type: c_float,
            c_type: c_float,
            to_rust: |a| a,
        },
    )
);
