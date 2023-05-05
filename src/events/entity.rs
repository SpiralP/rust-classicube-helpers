use crate::make_event_handler;
use std::os::raw::c_int;

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
            to_rust: |id| u8::try_from(id).unwrap(),
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
            to_rust: |id| u8::try_from(id).unwrap(),
        },
    )
);
