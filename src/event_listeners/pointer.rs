// CC_VAR extern struct _PointerEventsList {
// 	struct Event_PointerMove Moved; /* Pointer position changed (Arg is delta from last position) */
// 	struct Event_Int Down;          /* Left mouse or touch is pressed (Arg is index) */
// 	struct Event_Int Up;            /* Left mouse or touch is released (Arg is index) */
// 	struct Event_PointerMove RawMoved; /* Raw pointer position changed (Arg is delta) */
// } PointerEvents;

use crate::{create_callback, EventHandler, EventType};
use classicube_sys::{
    Event_RegisterInt, Event_RegisterMove, Event_UnregisterInt, Event_UnregisterMove, PointerEvents,
};
use std::{
    os::raw::{c_int, c_void},
    pin::Pin,
};

#[derive(Debug)]
pub enum PointerEvent {
    /// Pointer position changed (Arg is delta from last position)
    Moved(c_int, c_int, c_int),
    /// Left mouse or touch is pressed (Arg is index)
    Down(c_int),
    /// Left mouse or touch is released (Arg is index)
    Up(c_int),
    /// Raw pointer position changed (Arg is delta)
    RawMoved(c_int, c_int, c_int),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum PointerEventType {
    /// Pointer position changed (Arg is delta from last position)
    Moved,
    /// Left mouse or touch is pressed (Arg is index)
    Down,
    /// Left mouse or touch is released (Arg is index)
    Up,
    /// Raw pointer position changed (Arg is delta)
    RawMoved,
}

impl EventType for PointerEvent {
    type EventType = PointerEventType;

    fn event_type(&self) -> Self::EventType {
        match self {
            PointerEvent::Moved(..) => PointerEventType::Moved,
            PointerEvent::Down(..) => PointerEventType::Down,
            PointerEvent::Up(..) => PointerEventType::Up,
            PointerEvent::RawMoved(..) => PointerEventType::RawMoved,
        }
    }
}

pub struct PointerEventListener {
    event_handler: Pin<Box<EventHandler<PointerEvent>>>,
}

impl PointerEventListener {
    /// register event listeners, listeners will unregister on drop
    pub fn register() -> Self {
        let event_handler = Box::pin(EventHandler::new());

        let mut this = Self { event_handler };

        unsafe {
            this.register_listeners();
        }

        this
    }

    pub fn on<F>(&mut self, event_type: PointerEventType, callback: F)
    where
        F: FnMut(&PointerEvent),
        F: 'static,
    {
        self.event_handler.on(event_type, callback);
    }

    unsafe fn register_listeners(&mut self) {
        let ptr: *mut EventHandler<PointerEvent> = self.event_handler.as_mut().get_unchecked_mut();

        Event_RegisterMove(
            &mut PointerEvents.Moved,
            ptr as *mut c_void,
            Some(on_pointer_moved),
        );
        Event_RegisterInt(
            &mut PointerEvents.Down,
            ptr as *mut c_void,
            Some(on_pointer_down),
        );
        Event_RegisterInt(
            &mut PointerEvents.Up,
            ptr as *mut c_void,
            Some(on_pointer_up),
        );

        Event_RegisterMove(
            &mut PointerEvents.RawMoved,
            ptr as *mut c_void,
            Some(on_pointer_raw_moved),
        );
    }

    unsafe fn unregister_listeners(&mut self) {
        let ptr: *mut EventHandler<PointerEvent> = self.event_handler.as_mut().get_unchecked_mut();

        Event_UnregisterMove(
            &mut PointerEvents.Moved,
            ptr as *mut c_void,
            Some(on_pointer_moved),
        );
        Event_UnregisterInt(
            &mut PointerEvents.Down,
            ptr as *mut c_void,
            Some(on_pointer_down),
        );
        Event_UnregisterInt(
            &mut PointerEvents.Up,
            ptr as *mut c_void,
            Some(on_pointer_up),
        );

        Event_UnregisterMove(
            &mut PointerEvents.RawMoved,
            ptr as *mut c_void,
            Some(on_pointer_raw_moved),
        );
    }
}

impl Drop for PointerEventListener {
    fn drop(&mut self) {
        unsafe {
            self.unregister_listeners();
        }
    }
}

create_callback!(
    on_pointer_moved,
    (idx: c_int, x_delta: c_int, y_delta: c_int),
    PointerEvent,
    PointerEvent::Moved
);

create_callback!(
    on_pointer_down,
    (idx: c_int),
    PointerEvent,
    PointerEvent::Down
);

create_callback!(on_pointer_up, (idx: c_int), PointerEvent, PointerEvent::Up);

create_callback!(
    on_pointer_raw_moved,
    (idx: c_int, x_delta: c_int, y_delta: c_int),
    PointerEvent,
    PointerEvent::RawMoved
);
