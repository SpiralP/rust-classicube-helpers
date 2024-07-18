use std::cell::Cell;

use classicube_sys::{Chat_Send, OwnedString};
use tracing::info;

thread_local!(
    static SIMULATING: Cell<bool> = const { Cell::new(false) };
);

pub fn print<S: Into<String>>(s: S) {
    let s: String = s.into();
    info!("{}", s);

    let s = if s.len() > 255 {
        let mut s = s;
        s.truncate(255);
        s
    } else {
        s
    };

    SIMULATING.set(true);

    let owned_string = OwnedString::new(s);
    unsafe {
        classicube_sys::Chat_Add(owned_string.as_cc_string());
    }

    SIMULATING.set(false);
}

pub fn send<S: Into<String>>(s: S) {
    let s = s.into();
    info!("{}", s);

    let owned_string = OwnedString::new(s);
    unsafe {
        Chat_Send(owned_string.as_cc_string(), 0);
    }
}
