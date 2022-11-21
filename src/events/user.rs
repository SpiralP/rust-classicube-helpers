use classicube_sys::{BlockID, IVec3};

use crate::make_event_handler;

make_event_handler!(
    /// User changes a block
    User,
    BlockChanged,
    Block,
    (
        {
            name: coords,
            rust_type: IVec3,
            c_type: IVec3,
            to_rust: |a| a,
        },
        {
            name: old_block,
            rust_type: BlockID,
            c_type: BlockID,
            to_rust: |a| a,
        },
        {
            name: block,
            rust_type: BlockID,
            c_type: BlockID,
            to_rust: |a| a,
        },
    )
);

make_event_handler!(
    /// Hack permissions of the player changes
    User,
    HackPermsChanged,
    Void,
    ()
);

make_event_handler!(
    /// Held block in hotbar changes
    User,
    HeldBlockChanged,
    Void,
    ()
);

make_event_handler!(
    /// Hack states changed (e.g. stops flying)
    User,
    HacksStateChanged,
    Void,
    ()
);
