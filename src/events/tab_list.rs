use std::os::raw::c_int;

use crate::make_event_handler;

make_event_handler!(
    /// Tab list entry is created
    TabList,
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
    /// Tab list entry is modified
    TabList,
    Changed,
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
    /// Tab list entry is removed
    TabList,
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
