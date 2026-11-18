//! Streaming in arbitrary chunks gives the same document as a whole read.
//! The first byte picks the chunk size.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mztab_m::{Event, IntoOwned, Parsed, StreamReader};

fuzz_target!(|data: &[u8]| {
    let Some((&size, input)) = data.split_first() else {
        return;
    };
    let size = usize::from(size).max(1);
    let mut events = Vec::new();
    let mut reader = StreamReader::new();
    let mut keep = |event: Event<'_>| events.push(event.into_owned());
    for chunk in input.chunks(size) {
        reader.push(chunk, &mut keep);
    }
    reader.finish(&mut keep);
    let streamed: Parsed<'static> = events.into_iter().collect();
    assert!(streamed == mztab_m::parse_bytes(input).into_owned(), "chunks of {size} differ");
});
