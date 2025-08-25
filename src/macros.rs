use std::cmp::Ordering;

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
                callback_handler: Box<$crate::callback_handler::CallbackHandler<[<$event_name Event>]>>,
            }

            impl [<$event_name EventHandler>] {
                #[must_use]
                pub fn new() -> Self {
                    Self {
                        registered: false,
                        callback_handler: Box::new($crate::callback_handler::CallbackHandler::new()),
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

                // have to use c_type here because there's no way to use a converted OwnedString
                // and keep it alive until after Raise() returns
                pub fn raise(
                    $($name: $c_type, )*
                ) {
                    unsafe {
                        ::classicube_sys::[<Event_Raise $func_type>](
                            &mut *&raw mut ::classicube_sys::[<$event_type Events>].$event_name,
                            $($name, )*
                        );
                    }
                }

                #[must_use]
                pub fn index(&self) -> Option<usize> {
                    let event = unsafe { &*&raw const ::classicube_sys::[<$event_type Events>].$event_name };
                    let ptr: *const $crate::callback_handler::CallbackHandler<[<$event_name Event>]> =
                        self.callback_handler.as_ref();
                    let ptr: *const ::std::os::raw::c_void = ptr.cast();

                    #[allow(clippy::cast_sign_loss)]
                    for i in 0..event.Count as usize {
                        let handler = event.Handlers[i];
                        let obj: *const ::std::os::raw::c_void = event.Objs[i].cast();
                        #[allow(unpredictable_function_pointer_comparisons)]
                        if handler
                            .map(|handler| handler == Self::callback)
                            .unwrap_or(false)
                            && obj == ptr
                        {
                            return Some(i);
                        }
                    }

                    None
                }

                /// Reorder by index
                #[must_use]
                pub fn reorder(&self, new_index: usize) -> Option<()> {
                    let old_index = self.index()?;

                    let event = unsafe { &mut *&raw mut ::classicube_sys::[<$event_type Events>].$event_name };
                    #[allow(clippy::cast_sign_loss)]
                    let count = event.Count as usize;

                    let handlers = &mut event.Handlers[..count];
                    let objs = &mut event.Objs[..count];
                    $crate::macros::reorder(handlers, old_index, new_index);
                    $crate::macros::reorder(objs, old_index, new_index);

                    Some(())
                }

                unsafe fn register_listener(&mut self) {
                    if !self.registered {
                        let handlers = unsafe { &mut *&raw mut ::classicube_sys::[<$event_type Events>].$event_name };
                        let ptr: *mut $crate::callback_handler::CallbackHandler<[<$event_name Event>]> =
                            self.callback_handler.as_mut();

                        unsafe {
                            ::classicube_sys::[<Event_Register $func_type>](
                                handlers,
                                ptr.cast(),
                                Some(Self::callback),
                            );
                        }

                        self.registered = true;
                    }
                }

                unsafe fn unregister_listener(&mut self) {
                    if self.registered {
                        let ptr: *mut $crate::callback_handler::CallbackHandler<[<$event_name Event>]> =
                            self.callback_handler.as_mut();

                        unsafe {
                            ::classicube_sys::[<Event_Unregister $func_type>](
                                &mut *&raw mut ::classicube_sys::[<$event_type Events>].$event_name,
                                ptr.cast(),
                                Some(Self::callback),
                            );
                        }

                        self.registered = false;
                    }
                }

                extern "C" fn callback(
                    obj: *mut ::std::os::raw::c_void,
                    $($name: $c_type, )*
                ) {
                    let event_handler = obj.cast::<$crate::callback_handler::CallbackHandler<[<$event_name Event>]>>();
                    let event_handler = unsafe { &mut *event_handler };

                    #[allow(clippy::redundant_closure_call)]
                    let event = [<$event_name Event>] {
                        $($name: ($to_rust)($name), )*
                    };

                    $crate::tracing::debug!("{} {:?}", stringify!([<$event_type>]), event);

                    event_handler.handle_event(&event);
                }
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

pub fn reorder<T>(slice: &mut [T], old_index: usize, new_index: usize) {
    match new_index.cmp(&old_index) {
        Ordering::Less => {
            slice[new_index..=old_index].rotate_right(1);
        }
        Ordering::Greater => {
            slice[old_index..=new_index].rotate_left(1);
        }
        Ordering::Equal => {}
    }
}

#[test]
fn test_reorder() {
    //           0  1  2  3  4  5  6  7  8  9
    let mut a = [2, 3, 4, 5, 6, 7, 8, 0, 0, 0];
    let count = 7;

    // move 7 to where 4 is, shift 4,5,6 to the right
    reorder(&mut a[..count], 5, 2);
    //               0  1  2  3  4  5  6  7  8  9
    assert_eq!(&a, &[2, 3, 7, 4, 5, 6, 8, 0, 0, 0]);

    // move 7 to where 6 is, move 4,5,6 to the left
    reorder(&mut a[..count], 2, 5);
    //               0  1  2  3  4  5  6  7  8  9
    assert_eq!(&a, &[2, 3, 4, 5, 6, 7, 8, 0, 0, 0]);

    // move 2 to where 3 is, and 3 to where 2 is
    reorder(&mut a[..count], 0, 1);
    //               0  1  2  3  4  5  6  7  8  9
    assert_eq!(&a, &[3, 2, 4, 5, 6, 7, 8, 0, 0, 0]);

    // do nothing
    reorder(&mut a[..count], 0, 0);
    //               0  1  2  3  4  5  6  7  8  9
    assert_eq!(&a, &[3, 2, 4, 5, 6, 7, 8, 0, 0, 0]);
}

#[macro_export]
macro_rules! time {
    ($title:tt, $block:block) => {{
        let before = ::std::time::Instant::now();
        let res = $block;
        let after = ::std::time::Instant::now();
        let diff = after - before;
        debug!("{} ({:?})", $title, diff);
        res
    }};

    ($title:expr, $high_millis:tt, $block:block) => {{
        let before = ::std::time::Instant::now();
        let res = $block;
        let after = ::std::time::Instant::now();
        let diff = after - before;
        if diff > ::std::time::Duration::from_millis($high_millis) {
            $crate::tracing::warn!("{} ({:?})", $title, diff);
        } else {
            $crate::tracing::debug!("{} ({:?})", $title, diff);
        }
        res
    }};
}

#[macro_export]
macro_rules! time_silent {
    ($title:expr, $high_millis:tt, $block:block) => {{
        let before = ::std::time::Instant::now();
        let res = $block;
        let after = ::std::time::Instant::now();
        let diff = after - before;
        if diff > ::std::time::Duration::from_millis($high_millis) {
            $crate::tracing::warn!("{} ({:?})", $title, diff);
        }
        res
    }};
}

#[macro_export]
macro_rules! test_noop_fn {
    ($name:tt) => {
        #[cfg(test)]
        #[unsafe(no_mangle)]
        pub extern "C" fn $name() {}
    };
}

#[macro_export]
macro_rules! test_noop_static {
    ($name:tt) => {
        #[cfg(test)]
        #[unsafe(no_mangle)]
        pub static mut $name: () = ();
    };
}
