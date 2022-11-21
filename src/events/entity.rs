use std::os::raw::c_int;

use crate::make_event_handler;

make_event_handler!(
    /// Entity is spawned in the current world
    Entity,
    Added,
    Int,
    (
        {
            name: id,
            rust_type: u8,
            c_type: c_int,
            to_rust: |id| id as u8,
        },
    )
);

make_event_handler!(
    /// Entity is despawned from the current world
    Entity,
    Removed,
    Int,
    (
        {
            name: id,
            rust_type: u8,
            c_type: c_int,
            to_rust: |id| id as u8,
        },
    )
);
