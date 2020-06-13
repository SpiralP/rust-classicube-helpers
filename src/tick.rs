use crate::{callback_handler::CallbackHandler, CellGetSet};
use classicube_sys::*;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

thread_local!(
    static CALLBACK_REGISTERED: Cell<bool> = Cell::new(false);
);

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
        if !CALLBACK_REGISTERED.get() {
            CALLBACK_REGISTERED.set(true);

            unsafe {
                // 60 fps
                ScheduledTask_Add(GAME_NET_TICKS, Some(Self::hook));
            }
        }
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
    }

    extern "C" fn hook(task: *mut ScheduledTask) {
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
