#![feature(coerce_unsized)]
#![feature(unsize)]

mod entities;
mod event_handler;
mod event_listeners;
mod shared;
mod tab_list;

pub use crate::{entities::*, event_handler::*, event_listeners::*, shared::*, tab_list::*};

#[macro_export]
macro_rules! create_callback(


    // create_callback!(
    //     on_disconnected,
    //     (),
    //     TabListEvent,
    //     TabListEvent::Disconnected
    // );
    (
        $callback:ident,
        (),
        $event_type:ty,
        $event:path
    ) => {
        create_callback!(
            $callback,
            (),
            $event_type,
            { $event }
        );
    };


    // create_callback!(
    //     on_input_press,
    //     (key: c_int),
    //     InputEvent,
    //     InputEvent::Press
    // );
    (
        $callback:ident,
        ( $($arg:ident: $arg_type:ty),* ),
        $event_type:ty,
        $event:path
    ) => {
        create_callback!(
            $callback,
            ( $($arg: $arg_type),* ),
            $event_type,
            $event(
                $($arg),*
            )
        );
    };


    // create_callback!(
    //     on_input_text_changed,
    //     (s_ptr: *const classicube_sys::String),
    //     InputEvent,
    //     {
    //         let s = unsafe { s_ptr.as_ref().unwrap() }.to_string();
    //         InputEvent::TextChanged(s)
    //     }
    // );
    (
        $callback:ident,
        ( $($arg:ident: $arg_type:ty),* ),
        $event_type:ty,
        $event:expr
    ) => {
        extern "C" fn $callback(
            obj: *mut c_void,
            $($arg: $arg_type),*
        ) {
            let event_handler = obj as *mut EventHandler<$event_type>;
            let event_handler = unsafe { &mut *event_handler };

            event_handler.handle_event($event);
        }
    };
);
