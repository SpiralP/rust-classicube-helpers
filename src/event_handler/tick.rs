use crate::callback_handler::CallbackHandler;
use classicube_sys::*;
use detour::static_detour;
use std::{cell::Cell, pin::Pin};

// hack so that our tick detour can call our custom callbacks
// TODO we're only allowing 1 instance of TickEventListener...
thread_local!(
    static TICK_CALLBACK: Cell<Option<*mut CallbackHandler<TickEvent>>> = Cell::new(None);
);

static_detour! {
  pub static TICK_DETOUR: unsafe extern "C" fn(*mut ScheduledTask);
}

#[derive(Debug)]
pub struct TickEvent {
    task: *mut ScheduledTask,
}

pub struct TickEventHandler {
    registered: bool,
    callback_handler: Pin<Box<CallbackHandler<TickEvent>>>,
}

impl TickEventHandler {
    pub fn new() -> Self {
        Self {
            registered: false,
            callback_handler: Box::pin(CallbackHandler::new()),
        }
    }

    pub fn on<F>(&mut self, callback: F)
    where
        F: FnMut(&TickEvent),
        F: 'static,
    {
        self.callback_handler.on(callback);

        unsafe {
            self.register_listener();
        }
    }

    unsafe fn register_listener(&mut self) {
        if !self.registered {
            let ptr: *mut CallbackHandler<TickEvent> =
                self.callback_handler.as_mut().get_unchecked_mut();

            debug_assert!(Server.IsSinglePlayer == 0);
            debug_assert!(Server.Tick.is_some());

            if Server.IsSinglePlayer == 0 {
                if let Some(tick_original) = Server.Tick {
                    TICK_DETOUR
                        .initialize(tick_original, Self::callback)
                        .unwrap();
                    TICK_DETOUR.enable().unwrap();
                }
            }

            TICK_CALLBACK.with(|cell| {
                cell.set(Some(ptr));
            });

            self.registered = true;
        }
    }

    unsafe fn unregister_listener(&mut self) {
        if self.registered {
            {
                // ignore result
                let _ = TICK_DETOUR.disable();
            }

            TICK_CALLBACK.with(|cell| {
                cell.take();
            });
        }
    }

    fn callback(task: *mut ScheduledTask) {
        unsafe {
            // call original Server.Tick
            TICK_DETOUR.call(task);
        }

        TICK_CALLBACK.with(|maybe_ptr| {
            if let Some(ptr) = maybe_ptr.get() {
                let callback_handler = unsafe { &mut *ptr };
                callback_handler.handle_event(TickEvent { task });
            }
        });
    }
}

impl Drop for TickEventHandler {
    fn drop(&mut self) {
        unsafe {
            self.unregister_listener();
        }
    }
}

impl Default for TickEventHandler {
    fn default() -> Self {
        Self::new()
    }
}
