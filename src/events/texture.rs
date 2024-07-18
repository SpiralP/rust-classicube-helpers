#![allow(clippy::not_unsafe_ptr_arg_deref)]

use classicube_sys::{cc_string, Stream};

use crate::make_event_handler;

make_event_handler!(
    /// Terrain atlas (terrain.png) is changed
    Texture,
    AtlasChanged,
    Void,
    ()
);

make_event_handler!(
    /// Texture pack is changed
    Texture,
    PackChanged,
    Void,
    ()
);

make_event_handler!(
    /// File in a texture pack is changed (terrain.png, rain.png)
    Texture,
    FileChanged,
    Entry,
    (
        {
            name: stream,
            rust_type: *mut Stream,
            c_type: *mut Stream,
            to_rust: |a| a,
        },
        {
            name: name,
            rust_type: String,
            c_type: *const cc_string,
            to_rust: |name: *const cc_string| {
                unsafe { name.as_ref().unwrap() }.to_string()
            },
        },
    )
);
