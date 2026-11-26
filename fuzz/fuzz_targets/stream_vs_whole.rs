//! Streaming in arbitrary variable-length (including empty) chunks gives the
//! same document as a whole read.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mztab_m::{Event, IntoOwned, Parsed, StreamReader};

fuzz_target!(|data: &[u8]| {
    let Some((&count, rest)) = data.split_first() else {
        return;
    };
    let n_cuts = usize::from(count & 0x0f).min(rest.len());
    let (sizes, input) = rest.split_at(n_cuts);
    let mut events = Vec::new();
    let mut reader = StreamReader::new();
    let mut keep = |event: Event<'_>| events.push(event.into_owned());
    let mut tail = input;
    for &size in sizes {
        let take = usize::from(size).min(tail.len());
        let (chunk, next) = tail.split_at(take);
        reader.push(chunk, &mut keep);
        tail = next;
    }
    reader.push(tail, &mut keep);
    reader.finish(&mut keep);
    let streamed: Parsed<'static> = events.into_iter().collect();
    assert!(
        streamed == mztab_m::parse_bytes(input).into_owned(),
        "variable chunks differ"
    );
});
