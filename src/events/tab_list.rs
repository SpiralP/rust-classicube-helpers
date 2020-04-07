use crate::{make_event_handler, tab_list::TabListEntry};
use std::os::raw::c_int;

make_event_handler!(
    /// Tab list entry is created
    TabList,
    Added,
    Int,
    (
        {
            name: entry,
            rust_type: TabListEntry,
            c_type: c_int,
            to_rust: |id| TabListEntry::from_id(id as u8),
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
            name: entry,
            rust_type: TabListEntry,
            c_type: c_int,
            to_rust: |id| TabListEntry::from_id(id as u8),
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
            to_rust: |id| id as u8,
        },
    )
);
