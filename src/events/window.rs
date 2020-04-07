use crate::make_event_handler;

make_event_handler!(
    /// Window contents invalidated, should be redrawn
    Window,
    Redraw,
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
    /// WindowState of the window changed
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
