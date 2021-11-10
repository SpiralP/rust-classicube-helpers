use std::cell::Cell;
use tracing::info;

thread_local!(
    static SIMULATING: Cell<bool> = Cell::new(false);
);

pub fn print<S: Into<String>>(s: S) {
    let s: String = s.into();
    info!("{}", s);

    #[cfg(not(test))]
    {
        use crate::traits::CellGetSet;

        let s = if s.len() > 255 {
            let mut s = s;
            s.truncate(255);
            s
        } else {
            s
        };

        SIMULATING.set(true);

        let owned_string = classicube_sys::OwnedString::new(s);

        unsafe {
            classicube_sys::Chat_Add(owned_string.as_cc_string());
        }

        SIMULATING.set(false);
    }
}

pub fn send<S: Into<String>>(s: S) {
    let s = s.into();
    info!("{}", s);

    #[cfg(not(test))]
    {
        let owned_string = classicube_sys::OwnedString::new(s);

        unsafe {
            classicube_sys::Chat_Send(owned_string.as_cc_string(), 0);
        }
    }
}
