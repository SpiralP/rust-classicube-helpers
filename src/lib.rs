#![feature(coerce_unsized)]
#![feature(unsize)]
#![allow(clippy::redundant_closure_call)]

mod callback_handler;
pub mod entities;
pub mod events;
pub mod shared;
pub mod tab_list;
pub mod tick;
pub mod with_inner;

pub use detour;

#[doc(hidden)]
#[macro_export]
macro_rules! make_event_handler {
    (
        $(#[$attr:meta])*
        $event_type:ident,
        $event_name:ident,
        $func_type:ident,
        (
            $( {
                name: $name:ident,
                rust_type: $rust_type:ty,
                c_type: $c_type:ty,
                to_rust: $to_rust:expr,
            }, )*
        )
    ) => {
        paste::item! {
            #[derive(Debug)]
            pub struct [<$event_name Event>] {
                $(pub $name: $rust_type, )*
            }

            $(#[$attr])*
            pub struct [<$event_name EventHandler>] {
                registered: bool,
                callback_handler: ::std::pin::Pin<Box<$crate::callback_handler::CallbackHandler<[<$event_name Event>]>>>,
            }

            impl [<$event_name EventHandler>] {
                pub fn new() -> Self {
                    Self {
                        registered: false,
                        callback_handler: Box::pin($crate::callback_handler::CallbackHandler::new()),
                    }
                }

                pub fn on<F>(&mut self, callback: F)
                where
                    F: FnMut(&[<$event_name Event>]),
                    F: 'static,
                {
                    self.callback_handler.on(callback);

                    unsafe {
                        self.register_listener();
                    }
                }

                unsafe fn register_listener(&mut self) {
                    if !self.registered {
                        let ptr: *mut $crate::callback_handler::CallbackHandler<[<$event_name Event>]> =
                            self.callback_handler.as_mut().get_unchecked_mut();

                        ::classicube_sys::[<Event_Register $func_type>](
                            &mut ::classicube_sys::[<$event_type Events>].$event_name,
                            ptr as *mut ::std::os::raw::c_void,
                            Some(Self::callback),
                        );

                        self.registered = true;
                    }
                }

                unsafe fn unregister_listener(&mut self) {
                    if self.registered {
                        let ptr: *mut $crate::callback_handler::CallbackHandler<[<$event_name Event>]> =
                            self.callback_handler.as_mut().get_unchecked_mut();

                        ::classicube_sys::[<Event_Unregister $func_type>](
                            &mut ::classicube_sys::[<$event_type Events>].$event_name,
                            ptr as *mut ::std::os::raw::c_void,
                            Some(Self::callback),
                        );
                    }
                }

                extern "C" fn callback(
                    obj: *mut ::std::os::raw::c_void,
                    $($name: $c_type, )*
                ) {
                    let event_handler = obj as *mut $crate::callback_handler::CallbackHandler<[<$event_name Event>]>;
                    let event_handler = unsafe { &mut *event_handler };

                    event_handler.handle_event([<$event_name Event>] {
                        $($name: ($to_rust)($name), )*
                    });
                }

                // fn send_internal(
                //     &mut self,
                //     $($arg_name: $arg_type, )*
                // ) {
                //     unsafe {
                //         [<Event_Raise $event_type>](
                //             &mut [<$event_type Events>].$event_name,
                //             $($arg_name, )*
                //         );
                //     }
                // }

                // pub fn send(&mut self, event: [<$event_name Event>]) {
                //     let message = OwnedString::new(event.message);
                //     let message_type = event.message_type;
                // }
            }

            impl Drop for [<$event_name EventHandler>] {
                fn drop(&mut self) {
                    unsafe {
                        self.unregister_listener();
                    }
                }
            }

            impl Default for [<$event_name EventHandler>] {
                fn default() -> Self {
                    Self::new()
                }
            }
        }
    };
}
