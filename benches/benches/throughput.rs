//! Throughput of every stage on corpus files and a synthetic 100 MB file
//! (PLAN §7). `cargo bench -p mztab-m-bench --bench throughput`.

use core::hint::black_box;
use core::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput};
use mztab_m::cv::NoTerms;
use mztab_m::validate::{self, Config};
use mztab_m::{IntoOwned as _, StreamReader, json};
use mztab_m_bench::Spec;

const EXAMPLES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../third_party/mzTab-M/examples"
);

/// The inputs: name and text.
fn inputs() -> Vec<(&'static str, String)> {
    let read = |path: &str| {
        let full_path = format!("{EXAMPLES}/{path}");
        #[expect(
            clippy::expect_used,
            reason = "benchmark setup: missing corpus file means submodule is uninitialized"
        )]
        std::fs::read_to_string(&full_path).expect("readable corpus file (init git submodules)")
    };
    let synthetic = Spec {
        assays: 1000,
        rows: 4800,
        seed: 1,
    };
    vec![
        ("small", read("2.1/example_study_variable_group.mztab")),
        ("medium", read("2.0/LDA_v2.11.1_MTBLS396.mzTab")),
        (
            "large",
            read("2.1/xcms+MsIO_0.0.11_MTBLS4381_onlySMF.mztab"),
        ),
        ("synthetic", synthetic.document()),
    ]
}

fn stages(c: &mut Criterion) {
    for (name, text) in inputs() {
        let mut group = c.benchmark_group(name);
        group.throughput(Throughput::Bytes(text.len() as u64));
        if text.len() > 1_000_000 {
            group
                .sample_size(10)
                .measurement_time(Duration::from_secs(5))
                .warm_up_time(Duration::from_secs(1));
        }
        let bytes = text.as_bytes();
        group.bench_function("lines_std", |b| {
            b.iter(|| {
                black_box(bytes)
                    .split(|&c| c == b'\n')
                    .map(|l| l.split(|&c| c == b'\t').count())
                    .sum::<usize>()
            });
        });
        group.bench_function("lines_memchr", |b| {
            b.iter(|| memchr::memchr2_iter(b'\n', b'\t', black_box(bytes)).count());
        });
        group.bench_function("parse", |b| b.iter(|| mztab_m::parse(black_box(&text))));
        group.bench_function("parse_owned", |b| {
            b.iter(|| mztab_m::parse(black_box(&text)).into_owned());
        });
        group.bench_function("stream_64k", |b| {
            b.iter(|| {
                let mut reader = StreamReader::new();
                let mut events = 0_usize;
                for chunk in black_box(bytes).chunks(65_536) {
                    reader.push(chunk, &mut |_| events += 1);
                }
                reader.finish(&mut |_| events += 1);
                events
            });
        });
        let parsed = mztab_m::parse(&text);
        group.bench_function("write", |b| {
            b.iter(|| mztab_m::to_string(black_box(&parsed.mztab)));
        });
        let mut canonical = parsed.mztab.clone();
        canonical.layout = mztab_m::SourceLayout::default();
        group.bench_function("write_canonical", |b| {
            b.iter(|| mztab_m::to_string(black_box(&canonical)));
        });
        let config = Config::default();
        group.bench_function("structure", |b| {
            b.iter(|| validate::structure(black_box(&parsed.mztab), black_box(&config)));
        });
        group.bench_function("semantic", |b| {
            b.iter(|| validate::semantic(black_box(&parsed.mztab), black_box(&config), &NoTerms));
        });
        if let Ok(json_text) = json::to_string(&canonical) {
            group.bench_function("json_write", |b| {
                b.iter(|| json::to_string(black_box(&canonical)));
            });
            group.throughput(Throughput::Bytes(json_text.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("json_parse", json_text.len()),
                &json_text,
                |b, t| {
                    b.iter(|| json::parse(black_box(t)));
                },
            );
        }
        group.finish();
    }
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    stages(&mut criterion);
    criterion.final_summary();
}
