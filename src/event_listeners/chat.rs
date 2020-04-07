use crate::{create_callback, EventHandler, EventType};
use classicube_sys::{ChatEvents, Event_RegisterChat, Event_UnregisterChat, MsgType};
use std::{
  os::raw::{c_int, c_void},
  pin::Pin,
};

#[derive(Debug)]
pub enum ChatEvent {
  Received(String, MsgType),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ChatEventType {
  Received,
}

impl EventType for ChatEvent {
  type EventType = ChatEventType;

  fn event_type(&self) -> Self::EventType {
    match self {
      ChatEvent::Received(..) => ChatEventType::Received,
    }
  }
}

pub struct ChatEventListener {
  event_handler: Pin<Box<EventHandler<ChatEvent>>>,
}

impl ChatEventListener {
  /// register event listeners, listeners will unregister on drop
  pub fn register() -> Self {
    let event_handler = Box::pin(EventHandler::new());

    let mut this = Self { event_handler };

    unsafe {
      this.register_listeners();
    }

    this
  }

  pub fn on<F>(&mut self, event_type: ChatEventType, callback: F)
  where
    F: FnMut(&ChatEvent),
    F: 'static,
  {
    self.event_handler.on(event_type, callback);
  }

  unsafe fn register_listeners(&mut self) {
    let ptr: *mut EventHandler<ChatEvent> = self.event_handler.as_mut().get_unchecked_mut();

    Event_RegisterChat(
      &mut ChatEvents.ChatReceived,
      ptr as *mut c_void,
      Some(on_chat_received),
    );
  }

  unsafe fn unregister_listeners(&mut self) {
    let ptr: *mut EventHandler<ChatEvent> = self.event_handler.as_mut().get_unchecked_mut();

    Event_UnregisterChat(
      &mut ChatEvents.ChatReceived,
      ptr as *mut c_void,
      Some(on_chat_received),
    );
  }
}

impl Drop for ChatEventListener {
  fn drop(&mut self) {
    unsafe {
      self.unregister_listeners();
    }
  }
}

create_callback!(
  on_chat_received,
  (full_msg: *const classicube_sys::String, msg_type: c_int),
  ChatEvent,
  {
    let full_msg = if full_msg.is_null() {
      return;
    } else {
      unsafe { *full_msg }
    };

    let full_msg = full_msg.to_string();

    let msg_type: MsgType = msg_type as MsgType;

    ChatEvent::Received(full_msg, msg_type)
  }
);
