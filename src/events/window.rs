use crate::make_event_handler;

make_event_handler!(
    /// Window contents invalidated and will need to be redrawn
    Window,
    RedrawNeeded,
    Void,
    ()
);

make_event_handler!(
    /// Window is resized
    Window,
    Resized,
    Void,
    ()
);

make_event_handler!(
    /// Window is about to close (should free resources/save state/etc here)
    Window,
    Closing,
    Void,
    ()
);

make_event_handler!(
    /// Focus of the window changed
    Window,
    FocusChanged,
    Void,
    ()
);

make_event_handler!(
    /// State of the window changed (e.g. minimised, fullscreen)
    Window,
    StateChanged,
    Void,
    ()
);

make_event_handler!(
    /// Window has been created, Window_Handle is valid now.
    Window,
    Created,
    Void,
    ()
);

make_event_handler!(
    /// Inactive/background state of the window changed
    Window,
    InactiveChanged,
    Void,
    ()
);

make_event_handler!(
    /// Window contents should be redrawn (as they are about to be displayed)
    Window,
    Redrawing,
    Void,
    ()
);
