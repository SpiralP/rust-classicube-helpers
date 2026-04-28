#![allow(clippy::not_unsafe_ptr_arg_deref)]

use classicube_sys::PadAxisUpdate;

use crate::make_event_handler;

make_event_handler!(
    /// Raw analog controller movement
    Controller,
    AxisUpdate,
    PadAxis,
    (
        {
            name: upd,
            rust_type: *mut PadAxisUpdate,
            c_type: *mut PadAxisUpdate,
            to_rust: |a| a,
        },
    )
);
