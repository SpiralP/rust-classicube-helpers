use std::os::raw::{c_float, c_int};

use crate::make_event_handler;

make_event_handler!(
    /// Player begins loading a new world
    Controller,
    AxisUpdate,
    PadAxis,
    (
        {
            name: port,
            rust_type: c_int,
            c_type: c_int,
            to_rust: |a| a,
        },
        {
            name: axis,
            rust_type: c_int,
            c_type: c_int,
            to_rust: |a| a,
        },
        {
            name: x,
            rust_type: c_float,
            c_type: c_float,
            to_rust: |a| a,
        },
        {
            name: y,
            rust_type: c_float,
            c_type: c_float,
            to_rust: |a| a,
        },
    )
);
