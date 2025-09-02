use std::os::raw::{c_float, c_int};

use classicube_sys::{cc_bool, cc_uint8};

use crate::make_event_handler;

make_event_handler!(
    /// Player begins loading a new world
    World,
    NewMap,
    Void,
    ()
);

make_event_handler!(
    /// Portion of world is decompressed/generated (Arg is progress from 0-1)
    World,
    Loading,
    Float,
    (
        {
            name: progress,
            rust_type: c_float,
            c_type: c_float,
            to_rust: |a| a,
        },
    )
);

make_event_handler!(
    /// New world has finished loading, player can now interact with it
    World,
    MapLoaded,
    Void,
    ()
);

make_event_handler!(
    /// World environment variable changed by player/CPE/WoM config
    World,
    EnvVarChanged,
    Int,
    (
        {
            name: var,
            rust_type: c_int,
            c_type: c_int,
            to_rust: |a| a,
        },
    )
);

make_event_handler!(
    /// Lighting mode changed.
    World,
    LightingModeChanged,
    LightingMode,
    (
        {
            name: old_mode,
            rust_type: cc_uint8,
            c_type: cc_uint8,
            to_rust: |a| a,
        },
        {
            name: from_server,
            rust_type: cc_bool,
            c_type: cc_bool,
            to_rust: |a| a,
        },
    )
);
