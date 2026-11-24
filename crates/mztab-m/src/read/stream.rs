//! Push-based streaming TSV reader ([`StreamReader`]).

use alloc::vec::Vec;

use super::event::Event;
use super::typer::{Typer, strip_ending};
use crate::model::LineEnding;

/// A push-based reader: feed it chunks of bytes, get events for every
/// complete line.
///
/// It owns only a buffer for the line that straddles two chunks, so memory
/// stays proportional to the longest line. Events borrow from the chunk (or
/// that buffer) and live only for the callback; call `into_owned` on an
/// event to keep it.
///
/// Chunk boundaries never change the events: feeding a document in any
/// split produces the same events as [`parse_bytes`](super::parse_bytes) sees.
///
/// # Examples
///
/// ```
/// use mztab_m::{Event, StreamReader};
///
/// let mut reader = StreamReader::new();
/// let mut rows = 0;
/// let mut count = |event: Event<'_>| rows += usize::from(matches!(event, Event::Smf(..)));
/// reader.push(b"MTD\tmzTab-version\t2.1.0-M\nSFH\tSMF_ID\nSMF\t1\nSM", &mut count);
/// reader.push(b"F\t2", &mut count);
/// reader.finish(&mut count);
/// assert_eq!(rows, 2);
/// ```
#[derive(Debug, Default)]
pub struct StreamReader {
    typer: Typer,
    pending: Vec<u8>,
}

impl StreamReader {
    /// A reader at the start of a document.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads `chunk`, calling `on` for the events of every line it completes.
    pub fn push(&mut self, chunk: &[u8], on: &mut dyn FnMut(Event<'_>)) {
        for piece in chunk.split_inclusive(|&b| b == b'\n') {
            if !piece.ends_with(b"\n") {
                // The chunk ends inside a line.
                self.pending.extend_from_slice(piece);
            } else if self.pending.is_empty() {
                let (text, ending) = strip_ending(piece, true);
                self.typer.line(text, ending, on);
            } else {
                self.pending.extend_from_slice(piece);
                let mut line = core::mem::take(&mut self.pending);
                let (text, ending) = strip_ending(&line, true);
                self.typer.line(text, ending, on);
                line.clear();
                self.pending = line;
            }
        }
    }

    /// Ends the document, reading a last line that has no line break.
    pub fn finish(mut self, on: &mut dyn FnMut(Event<'_>)) {
        if !self.pending.is_empty() {
            let line = core::mem::take(&mut self.pending);
            self.typer.line(&line, LineEnding::None, on);
        }
    }
}
