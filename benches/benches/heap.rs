//! Peak heap and allocation counts (dhat), and the streaming memory bound
//! on a 1 GB synthetic document (PLAN §7).
//! `cargo bench -p mztab-m-bench --bench heap`.
#![expect(clippy::print_stdout, reason = "the bench prints its report table")]

use mztab_m::cv::NoTerms;
use mztab_m::validate::{self, Config};
use mztab_m::{IntoOwned as _, StreamReader};
use mztab_m_bench::Spec;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

const EXAMPLES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../third_party/mzTab-M/examples"
);

/// Peak heap bytes and total allocations while `work` runs, net of what
/// existed before.
fn measure(work: impl FnOnce()) -> (usize, u64) {
    let profiler = dhat::Profiler::builder().testing().build();
    let before = dhat::HeapStats::get();
    work();
    let after = dhat::HeapStats::get();
    drop(profiler);
    (
        after.max_bytes.saturating_sub(before.curr_bytes),
        after.total_blocks - before.total_blocks,
    )
}

fn row(input: &str, size: usize, stage: &str, (peak, allocations): (usize, u64)) {
    #[expect(clippy::cast_precision_loss, reason = "a ratio for display")]
    let ratio = peak as f64 / size.max(1) as f64;
    println!("| {input} | {stage} | {peak} | {ratio:.2} | {allocations} |");
}

fn main() {
    println!("| Input | Stage | Peak heap (bytes) | Peak / input | Allocations |");
    println!("|---|---|---|---|---|");
    for (name, path) in [
        ("medium", "2.0/LDA_v2.11.1_MTBLS396.mzTab"),
        ("large", "2.1/xcms+MsIO_0.0.11_MTBLS4381_onlySMF.mztab"),
    ] {
        let text = std::fs::read_to_string(format!("{EXAMPLES}/{path}")).unwrap_or_default();
        let size = text.len();
        row(name, size, "parse", measure(|| drop(mztab_m::parse(&text))));
        row(
            name,
            size,
            "parse_owned",
            measure(|| drop(mztab_m::parse(&text).into_owned())),
        );
        let parsed = mztab_m::parse(&text);
        row(
            name,
            size,
            "write",
            measure(|| drop(mztab_m::to_string(&parsed.mztab))),
        );
        let config = Config::default();
        row(
            name,
            size,
            "structure",
            measure(|| drop(validate::structure(&parsed.mztab, &config))),
        );
        row(
            name,
            size,
            "semantic",
            measure(|| drop(validate::semantic(&parsed.mztab, &config, &NoTerms))),
        );
    }
    // About 1 GB, generated and read in 64 KiB chunks: never in memory.
    let spec = Spec {
        assays: 1000,
        rows: 48_000,
        seed: 1,
    };
    let mut bytes = 0;
    let streamed = measure(|| {
        let mut reader = StreamReader::new();
        let mut events = 0_u64;
        for chunk in spec.chunks(65_536) {
            bytes += chunk.len();
            reader.push(chunk.as_bytes(), &mut |_| events += 1);
        }
        reader.finish(&mut |_| events += 1);
        assert!(events > 0, "events");
    });
    row("synthetic (streamed)", bytes, "stream_64k", streamed);
    assert!(
        streamed.0 < 64 * 1024 * 1024,
        "streaming memory is bounded by the longest line, not the file"
    );
}
