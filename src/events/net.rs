use crate::make_event_handler;

make_event_handler!(
    /// Connection to a server was established.
    Net,
    Connected,
    Void,
    ()
);

make_event_handler!(
    /// Connection to the server was lost.
    Net,
    Disconnected,
    Void,
    ()
);
