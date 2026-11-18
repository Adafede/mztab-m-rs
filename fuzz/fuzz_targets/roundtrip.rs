//! Writing a read document reproduces it (invalid UTF-8 decoded lossily),
//! and, for UTF-8 input, the written text reads back to the same model.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let parsed = mztab_m::parse_bytes(data);
    let written = mztab_m::to_string(&parsed.mztab).expect("a read model is writable");
    assert_eq!(written, String::from_utf8_lossy(data), "write(read(d)) != d");
    if core::str::from_utf8(data).is_ok() {
        assert!(mztab_m::parse(&written).mztab == parsed.mztab, "read(write(m)) != m");
    }
});
