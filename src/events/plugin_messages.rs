use crate::make_event_handler;

make_event_handler!(
    //
    PluginMessage,
    Received,
    PluginMessage,
    (
        {
            name: channel,
            rust_type: u8,
            c_type: u8,
            to_rust: |a| a,
        },
        {
            name: data,
            rust_type: Vec<u8>,
            c_type: *mut u8,
            to_rust: |a| unsafe { std::slice::from_raw_parts(a, 64) }.to_vec(),
        },
    )
);
