// CC_VAR extern struct _InputEventsList {
// 	struct Event_Int Press;   /* Key input character is typed. Arg is a character */
// 	struct Event_Input Down;  /* Key or button is pressed. Arg is a member of Key enumeration */
// 	struct Event_Int Up;      /* Key or button is released. Arg is a member of Key enumeration */
// 	struct Event_Float Wheel; /* Mouse wheel is moved/scrolled (Arg is wheel delta) */
// 	struct Event_String TextChanged; /* HTML text input changed */
// } InputEvents;

use crate::{create_callback, EventHandler, EventType};
use classicube_sys::{
    cc_bool, Event_RegisterFloat, Event_RegisterInput, Event_RegisterInt, Event_RegisterString,
    Event_UnregisterFloat, Event_UnregisterInput, Event_UnregisterInt, Event_UnregisterString,
    InputEvents,
};
use std::{
    os::raw::{c_float, c_int, c_void},
    pin::Pin,
};

#[derive(Debug)]
pub enum InputEvent {
    /// Key input character is typed. Arg is a character
    Press(c_int),
    /// Key or button is pressed. Arg is a member of Key enumeration
    Down(c_int, cc_bool),
    /// Key or button is released. Arg is a member of Key enumeration
    Up(c_int),
    /// Mouse wheel is moved/scrolled (Arg is wheel delta)
    Wheel(c_float),
    /// HTML text input changed
    TextChanged(String),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum InputEventType {
    /// Key input character is typed. Arg is a character
    Press,
    /// Key or button is pressed. Arg is a member of Key enumeration
    Down,
    /// Key or button is released. Arg is a member of Key enumeration
    Up,
    /// Mouse wheel is moved/scrolled (Arg is wheel delta)
    Wheel,
    /// HTML text input changed
    TextChanged,
}

impl EventType for InputEvent {
    type EventType = InputEventType;

    fn event_type(&self) -> Self::EventType {
        match self {
            InputEvent::Press(..) => InputEventType::Press,
            InputEvent::Down(..) => InputEventType::Down,
            InputEvent::Up(..) => InputEventType::Up,
            InputEvent::Wheel(..) => InputEventType::Wheel,
            InputEvent::TextChanged(..) => InputEventType::TextChanged,
        }
    }
}

pub struct InputEventListener {
    event_handler: Pin<Box<EventHandler<InputEvent>>>,
}

impl InputEventListener {
    /// register event listeners, listeners will unregister on drop
    pub fn register() -> Self {
        let event_handler = Box::pin(EventHandler::new());

        let mut this = Self { event_handler };

        unsafe {
            this.register_listeners();
        }

        this
    }

    pub fn on<F>(&mut self, event_type: InputEventType, callback: F)
    where
        F: FnMut(&InputEvent),
        F: 'static,
    {
        self.event_handler.on(event_type, callback);
    }

    unsafe fn register_listeners(&mut self) {
        let ptr: *mut EventHandler<InputEvent> = self.event_handler.as_mut().get_unchecked_mut();

        Event_RegisterInt(
            &mut InputEvents.Press,
            ptr as *mut c_void,
            Some(on_input_press),
        );
        Event_RegisterInput(
            &mut InputEvents.Down,
            ptr as *mut c_void,
            Some(on_input_down),
        );
        Event_RegisterInt(&mut InputEvents.Up, ptr as *mut c_void, Some(on_input_up));
        Event_RegisterFloat(
            &mut InputEvents.Wheel,
            ptr as *mut c_void,
            Some(on_input_wheel),
        );
        Event_RegisterString(
            &mut InputEvents.TextChanged,
            ptr as *mut c_void,
            Some(on_input_text_changed),
        );
    }

    unsafe fn unregister_listeners(&mut self) {
        let ptr: *mut EventHandler<InputEvent> = self.event_handler.as_mut().get_unchecked_mut();

        Event_UnregisterInt(
            &mut InputEvents.Press,
            ptr as *mut c_void,
            Some(on_input_press),
        );
        Event_UnregisterInput(
            &mut InputEvents.Down,
            ptr as *mut c_void,
            Some(on_input_down),
        );
        Event_UnregisterInt(&mut InputEvents.Up, ptr as *mut c_void, Some(on_input_up));
        Event_UnregisterFloat(
            &mut InputEvents.Wheel,
            ptr as *mut c_void,
            Some(on_input_wheel),
        );
        Event_UnregisterString(
            &mut InputEvents.TextChanged,
            ptr as *mut c_void,
            Some(on_input_text_changed),
        );
    }
}

impl Drop for InputEventListener {
    fn drop(&mut self) {
        unsafe {
            self.unregister_listeners();
        }
    }
}

create_callback!(on_input_press, (key: c_int), InputEvent, InputEvent::Press);

create_callback!(
    on_input_down,
    (key: c_int, was: cc_bool),
    InputEvent,
    InputEvent::Down
);

create_callback!(on_input_up, (key: c_int), InputEvent, InputEvent::Up);

create_callback!(
    on_input_wheel,
    (delta: c_float),
    InputEvent,
    InputEvent::Wheel
);

create_callback!(
    on_input_text_changed,
    (s_ptr: *const classicube_sys::String),
    InputEvent,
    {
        let s = unsafe { s_ptr.as_ref().unwrap() }.to_string();
        InputEvent::TextChanged(s)
    }
);
