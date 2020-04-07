use crate::make_event_handler;

make_event_handler!(Net, Connected, Void, ());

make_event_handler!(Net, Disconnected, Void, ());
