use crate::callback_handler::CallbackHandler;
use classicube_sys::*;
use detour::static_detour;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

static_detour! {
  pub static TICK_DETOUR: unsafe extern "C" fn(*mut ScheduledTask);
}

thread_local!(
    static TICK_CALLBACK_HANDLERS: RefCell<Vec<Weak<RefCell<CallbackHandler<TickEvent>>>>> =
        RefCell::new(Vec::new());
);

#[derive(Debug)]
pub struct TickEvent {
    task: *mut ScheduledTask,
}

pub struct TickEventHandler {
    callback_handler: Rc<RefCell<CallbackHandler<TickEvent>>>,
}

impl TickEventHandler {
    pub fn new() -> Self {
        Self {
            callback_handler: Rc::new(RefCell::new(CallbackHandler::new())),
        }
    }

    pub fn on<F>(&mut self, callback: F)
    where
        F: FnMut(&TickEvent),
        F: 'static,
    {
        self.callback_handler.borrow_mut().on(callback);

        unsafe {
            self.register_listener();
        }
    }

    fn check_register_detour() {
        TICK_CALLBACK_HANDLERS.with(|callback_handlers| {
            if callback_handlers.borrow().is_empty() {
                // if we were empty, (re-)detour

                unsafe {
                    let tick_original = Server.Tick.unwrap();

                    TICK_DETOUR.initialize(tick_original, Self::detour).unwrap();
                    TICK_DETOUR.enable().unwrap();
                }
            }
        });
    }

    fn check_unregister_detour() {
        TICK_CALLBACK_HANDLERS.with(|callback_handlers| {
            if callback_handlers.borrow().is_empty() {
                // if we are now empty, remove detour

                unsafe {
                    // ignore result
                    let _ = TICK_DETOUR.disable();
                }
            }
        });
    }

    unsafe fn register_listener(&mut self) {
        Self::check_register_detour();

        let weak = Rc::downgrade(&self.callback_handler);

        TICK_CALLBACK_HANDLERS.with(|callback_handlers| {
            for callback_handler in &*callback_handlers.borrow() {
                if callback_handler.ptr_eq(&weak) {
                    // we already have a handler registered
                    return;
                }
            }

            callback_handlers.borrow_mut().push(weak);
        });
    }

    unsafe fn unregister_listener(&mut self) {
        TICK_CALLBACK_HANDLERS.with(|callback_handlers| {
            let mut callback_handlers = callback_handlers.borrow_mut();

            let my_weak = Rc::downgrade(&self.callback_handler);

            let mut i = 0;
            while i != callback_handlers.len() {
                // if it's our weak, remove it
                if callback_handlers[i].ptr_eq(&my_weak) {
                    callback_handlers.remove(i);
                } else {
                    i += 1;
                }
            }
        });

        Self::check_unregister_detour();
    }

    fn detour(task: *mut ScheduledTask) {
        unsafe {
            // call original Server.Tick
            TICK_DETOUR.call(task);
        }

        TICK_CALLBACK_HANDLERS.with(|callback_handlers| {
            let callback_handlers = callback_handlers.borrow_mut();
            for weak_callback_handler in &*callback_handlers {
                if let Some(callback_handler) = weak_callback_handler.upgrade() {
                    callback_handler
                        .borrow_mut()
                        .handle_event(TickEvent { task });
                }
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
