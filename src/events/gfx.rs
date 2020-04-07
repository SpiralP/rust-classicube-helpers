use crate::make_event_handler;

make_event_handler!(
    /// View/fog distance is changed */
    Gfx,
    ViewDistanceChanged,
    Void,
    ()
);

make_event_handler!(
    /// Insufficient VRAM detected, need to free some GPU resources */
    Gfx,
    LowVRAMDetected,
    Void,
    ()
);

make_event_handler!(
    /// Projection matrix has changed */
    Gfx,
    ProjectionChanged,
    Void,
    ()
);

make_event_handler!(
    /// Context is destroyed after having been previously created */
    Gfx,
    ContextLost,
    Void,
    ()
);

make_event_handler!(
    /// Context is recreated after having been previously lost */
    Gfx,
    ContextRecreated,
    Void,
    ()
);
