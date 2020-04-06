use crate::{EventHandler, EventType};
use classicube_sys::*;
use detour::static_detour;
use std::{cell::Cell, pin::Pin};

// hack so that our tick detour can call our custom callbacks
// TODO we're only allowing 1 instance of TickEventListener...
thread_local!(
  static TICK_CALLBACK: Cell<Option<*mut EventHandler<TickEvent>>> = Cell::new(None);
);

static_detour! {
  pub static TICK_DETOUR: unsafe extern "C" fn(*mut ScheduledTask);
}

#[derive(Debug)]
pub enum TickEvent {
  Tick(*mut ScheduledTask),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum TickEventType {
  Tick,
}

impl EventType for TickEvent {
  type EventType = TickEventType;

  fn event_type(&self) -> Self::EventType {
    match self {
      TickEvent::Tick(..) => TickEventType::Tick,
    }
  }
}

pub struct TickEventListener {
  event_handler: Pin<Box<EventHandler<TickEvent>>>,
}

impl TickEventListener {
  pub fn register() -> Self {
    let event_handler = Box::pin(EventHandler::new());

    let mut this = Self { event_handler };

    unsafe {
      this.register_detour();
    }

    this
  }

  pub fn on<F>(&mut self, event_type: TickEventType, callback: F)
  where
    F: FnMut(&TickEvent),
    F: 'static,
  {
    self.event_handler.on(event_type, callback);
  }

  unsafe fn register_detour(&mut self) {
    if Server.IsSinglePlayer == 0 {
      if let Some(tick_original) = Server.Tick {
        TICK_DETOUR.initialize(tick_original, tick_detour).unwrap();
        TICK_DETOUR.enable().unwrap();
      } else {
        unimplemented!("Server.Tick null");
      }
    } else {
      unimplemented!("IsSinglePlayer");
    }

    let ptr: *mut EventHandler<TickEvent> = self.event_handler.as_mut().get_unchecked_mut();

    TICK_CALLBACK.with(|cell| {
      cell.set(Some(ptr));
    });
  }

  unsafe fn unregister_detour(&mut self) {
    {
      // ignore result
      let _ = TICK_DETOUR.disable();
    }

    TICK_CALLBACK.with(|cell| {
      cell.take();
    });
  }
}

impl Drop for TickEventListener {
  fn drop(&mut self) {
    unsafe {
      self.unregister_detour();
    }
  }
}

fn tick_detour(task: *mut ScheduledTask) {
  unsafe {
    // call original Server.Tick
    TICK_DETOUR.call(task);
  }

  TICK_CALLBACK.with(|maybe_ptr| {
    if let Some(ptr) = maybe_ptr.get() {
      let event_handler = unsafe { &mut *ptr };
      event_handler.handle_event(TickEvent::Tick(task));
    }
  });
}
