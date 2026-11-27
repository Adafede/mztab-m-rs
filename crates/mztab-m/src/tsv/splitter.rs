//! Bracket- and quote-aware `|` list splitter shared by reader and writer.

/// Calls `visit(offset, item)` for each `|`-separated item, stopping early
/// and returning `false` if `visit` returns `false`. With `brackets`, a `|`
/// inside `[…]` or inside double quotes does not split (parameter lists).
pub(crate) fn visit_list<'a>(
    text: &'a str,
    brackets: bool,
    mut visit: impl FnMut(usize, &'a str) -> bool,
) -> bool {
    let mut start = 0;
    let mut splitter = Splitter::new(brackets);
    for (at, byte) in text.bytes().enumerate() {
        if splitter.step(byte) {
            if !visit(start, text.get(start..at).unwrap_or_default()) {
                return false;
            }
            start = at + 1;
        }
    }
    visit(start, text.get(start..).unwrap_or_default())
}

/// The state of a list split: which `|` separate items. The reader splits
/// with it and the writer checks its output with it, so the two agree by
/// construction.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Splitter {
    /// Parameter lists: `|` inside `[…]` or `"…"` does not split.
    brackets: bool,
    depth: usize,
    quoted: bool,
}

impl Splitter {
    pub(crate) const fn new(brackets: bool) -> Self {
        Self {
            brackets,
            depth: 0,
            quoted: false,
        }
    }

    /// Consumes one byte; `true` if it separates two items.
    pub(crate) const fn step(&mut self, byte: u8) -> bool {
        match byte {
            b'"' if self.brackets => self.quoted = !self.quoted,
            b'[' if self.brackets && !self.quoted => self.depth += 1,
            b']' if self.brackets && !self.quoted => self.depth = self.depth.saturating_sub(1),
            b'|' => return self.depth == 0 && !self.quoted,
            _ => {}
        }
        false
    }

    /// Whether a `|` here would separate items.
    pub(crate) const fn at_rest(&self) -> bool {
        self.depth == 0 && !self.quoted
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::visit_list;
    use alloc::vec::Vec;

    #[test]
    fn list_splitting() {
        let split = |s, brackets| -> Vec<_> {
            let mut items = Vec::new();
            assert!(visit_list(s, brackets, |at, item| {
                items.push((at, item));
                true
            }));
            items
        };
        let plain = split("a|b||c", false);
        assert_eq!(plain, [(0, "a"), (2, "b"), (4, ""), (5, "c")], "plain");
        let params = split(r#"[a,b,"x|y",]|[,,"]|",v|w]"#, true);
        assert_eq!(
            params,
            [(0, r#"[a,b,"x|y",]"#), (13, r#"[,,"]|",v|w]"#)],
            "bracket and quote aware"
        );
        assert_eq!(
            split("]|[", true),
            [(0, "]"), (2, "[")],
            "unbalanced close does not underflow"
        );
        assert_eq!(split("", false), [(0, "")], "empty");
        assert_eq!(split("a|", false), [(0, "a"), (2, "")], "trailing pipe");
        assert_eq!(
            split("a\"|b", false),
            [(0, "a\""), (3, "b")],
            "quotes only matter in parameter lists"
        );

        let mut seen = Vec::new();
        let completed = visit_list("a|b|c", false, |_, item| {
            seen.push(item);
            item != "b"
        });
        assert!(!completed, "early stop returns false");
        assert_eq!(seen, ["a", "b"], "stopped after second item");
    }
}
