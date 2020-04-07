use crate::make_event_handler;

make_event_handler!(
    /// Block permissions (can place/delete) for a block changes
    Block,
    PermissionsChanged,
    Void,
    ()
);

make_event_handler!(
    /// Block definition is changed or removed
    Block,
    BlockDefChanged,
    Void,
    ()
);
