//! Seeded, deterministic generator of mzTab-M 2.1 documents for benchmarks.
//!
//! Documents are yielded line by line to support bounded-memory streaming.
//!
//! ```
//! use mztab_m_bench::Spec;
//!
//! let spec = Spec { assays: 4, rows: 3, seed: 7 };
//! let text = spec.document();
//! assert!(text.starts_with("MTD\tmzTab-version\t2.1.0-M\n"));
//! assert_eq!(text, spec.document(), "deterministic");
//! ```

use core::fmt::Write as _;

/// Proton mass in daltons (`m_p / u`), CODATA 2022:
/// <https://physics.nist.gov/cgi-bin/cuu/Value?mpu>.
const PROTON_MASS_DA: f64 = 1.007_276_466_578_9;

/// Reference metabolite template paired with exact monoisotopic neutral mass.
struct Metabolite {
    hmdb_id: &'static str,
    formula: &'static str,
    name: &'static str,
    neutral_mass_da: f64,
}

const METABOLITES: &[Metabolite] = &[
    Metabolite {
        hmdb_id: "HMDB0000122",
        formula: "C6H12O6",
        name: "D-Glucose",
        neutral_mass_da: 180.0634,
    },
    Metabolite {
        hmdb_id: "HMDB0001847",
        formula: "C8H10N4O2",
        name: "Caffeine",
        neutral_mass_da: 194.0804,
    },
    Metabolite {
        hmdb_id: "HMDB0000161",
        formula: "C3H7NO2",
        name: "L-Alanine",
        neutral_mass_da: 89.0477,
    },
    Metabolite {
        hmdb_id: "HMDB0000159",
        formula: "C9H11NO2",
        name: "L-Phenylalanine",
        neutral_mass_da: 165.0790,
    },
    Metabolite {
        hmdb_id: "HMDB0000094",
        formula: "C6H8O7",
        name: "Citric acid",
        neutral_mass_da: 192.0270,
    },
    Metabolite {
        hmdb_id: "HMDB0000929",
        formula: "C11H12N2O2",
        name: "L-Tryptophan",
        neutral_mass_da: 204.0899,
    },
];

/// Independent stream identifiers for PRNG separation per row.
const STREAM_ABUNDANCE: u64 = 1;
const STREAM_SCORE: u64 = 2;
const STREAM_CHROMATOGRAPHY: u64 = 3;
const STREAM_MASS_ERROR: u64 = 4;

/// Generator parameters: `rows` small molecules, each with one feature and one
/// evidence row (profile `M+S+F+E`), quantified across `assays.max(1)` assays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spec {
    /// Assays and matching MS runs (values below 1 are clamped to 1).
    pub assays: u32,
    /// Rows in each of SML, SMF, and SME.
    pub rows: u32,
    /// Deterministic PRNG seed.
    pub seed: u64,
}

const HEADER_METADATA: &[&str] = &[
    "MTD\tmzTab-version\t2.1.0-M",
    "MTD\tmzTab-ID\tSYNTHETIC",
    "MTD\tmzTab-profile\tM+S+F+E",
    "MTD\ttitle\tSynthetic benchmark document",
    "MTD\tsoftware[1]\t[MS, MS:1002879, Progenesis QI, 3.0]",
    "MTD\tquantification_method\t[MS, MS:1001834, LC-MS label-free quantitation analysis, ]",
];

const TRAILING_METADATA: &[&str] = &[
    "MTD\tcv[1]-label\tMS",
    "MTD\tcv[1]-full_name\tPSI-MS controlled vocabulary",
    "MTD\tcv[1]-version\t4.1.0",
    "MTD\tcv[1]-uri\thttps://purl.obolibrary.org/obo/ms.obo",
    "MTD\tcv[2]-label\tSTATO",
    "MTD\tcv[2]-full_name\tStatistics Ontology",
    "MTD\tcv[2]-version\tRC1.4",
    "MTD\tcv[2]-uri\thttp://purl.obolibrary.org/obo/stato.owl",
    "MTD\tcv[3]-label\tPRIDE",
    "MTD\tcv[3]-full_name\tPRIDE Controlled Vocabulary",
    "MTD\tcv[3]-version\t2023",
    "MTD\tcv[3]-uri\thttps://github.com/PRIDE-Utilities/pride-ontology",
    "MTD\tdatabase[1]\t[, , HMDB, ]",
    "MTD\tdatabase[1]-prefix\tHMDB",
    "MTD\tdatabase[1]-version\t5.0",
    "MTD\tdatabase[1]-uri\thttps://hmdb.ca",
    "MTD\tsmall_molecule-quantification_unit\t[PRIDE, PRIDE:0000330, Arbitrary quantification unit, ]",
    "MTD\tsmall_molecule_feature-quantification_unit\t[PRIDE, PRIDE:0000330, Arbitrary quantification unit, ]",
    "MTD\tid_confidence_measure[1]\t[MS, MS:1002890, fragmentation score, ]",
];

const SML_COLUMNS: &[&str] = &[
    "SML_ID",
    "SMF_ID_REFS",
    "database_identifier",
    "chemical_formula",
    "smiles",
    "inchi",
    "chemical_name",
    "uri",
    "theoretical_neutral_mass",
    "adduct_ions",
    "reliability",
    "best_id_confidence_measure",
    "best_id_confidence_value",
];

const SMF_COLUMNS: &[&str] = &[
    "SMF_ID",
    "SME_ID_REFS",
    "SME_ID_REF_ambiguity_code",
    "adduct_ion",
    "isotopomer",
    "exp_mass_to_charge",
    "charge",
    "retention_time_in_seconds",
    "retention_time_in_seconds_start",
    "retention_time_in_seconds_end",
];

const SME_COLUMNS: &[&str] = &[
    "SME_ID",
    "evidence_input_id",
    "database_identifier",
    "chemical_formula",
    "smiles",
    "inchi",
    "chemical_name",
    "uri",
    "derivatized_form",
    "adduct_ion",
    "exp_mass_to_charge",
    "charge",
    "theoretical_mass_to_charge",
    "spectra_ref",
    "identification_method",
    "ms_level",
    "id_confidence_measure[1]",
    "rank",
];

const fn table_columns(prefix: &str) -> &'static [&'static str] {
    match prefix.as_bytes() {
        b"SMH" => SML_COLUMNS,
        b"SFH" => SMF_COLUMNS,
        _ => SME_COLUMNS,
    }
}

/// Deterministic `SplitMix64` generator.
#[derive(Clone, Debug)]
struct Rng(u64);

impl Rng {
    const fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Quantized uniform integer in `[0, bound)` converted losslessly to `f64`.
    fn uniform_f64(&mut self, bound: u32) -> f64 {
        let drawn = u32::try_from(self.next() % u64::from(bound)).unwrap_or(0);
        f64::from(drawn)
    }

    /// Quantized assay intensity in `[100.0000, 100_100.0000)` rounded to 4 decimals.
    fn abundance(&mut self) -> f64 {
        (1_000_000.0 + self.uniform_f64(1_000_000_000)) / 10_000.0
    }

    /// Score in `[0.1000, 1.0000]` rounded to 4 decimals.
    fn score(&mut self) -> f64 {
        (1_000.0 + self.uniform_f64(9_001)) / 10_000.0
    }
}

/// Exact Welford online accumulator for mean and sample standard error.
#[derive(Default)]
struct RunningStats {
    count: u32,
    mean: f64,
    m2: f64,
}

impl RunningStats {
    fn push(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / f64::from(self.count);
        let delta2 = value - self.mean;
        self.m2 = delta.mul_add(delta2, self.m2);
    }

    fn standard_error(&self) -> Option<f64> {
        if self.count < 2 {
            return None;
        }
        let variance = self.m2 / f64::from(self.count - 1);
        Some((variance / f64::from(self.count)).sqrt())
    }
}

impl Spec {
    const fn effective_assays(&self) -> u32 {
        if self.assays == 0 { 1 } else { self.assays }
    }

    const fn study_variables(&self) -> u32 {
        if self.effective_assays() >= 2 { 2 } else { 1 }
    }

    /// The whole document as one string.
    #[must_use]
    pub fn document(&self) -> String {
        self.lines().collect()
    }

    /// The document's lines, each ending in `\n`.
    pub fn lines(&self) -> impl Iterator<Item = String> + '_ {
        self.metadata_lines()
            .chain(core::iter::once(String::from("\n")))
            .chain(core::iter::once(self.header("SMH")))
            .chain((1..=self.rows).map(move |n| self.sml(n)))
            .chain(core::iter::once(String::from("\n")))
            .chain(core::iter::once(self.header("SFH")))
            .chain((1..=self.rows).map(move |n| self.smf(n)))
            .chain(core::iter::once(String::from("\n")))
            .chain(core::iter::once(self.header("SEH")))
            .chain((1..=self.rows).map(move |n| self.sme(n)))
    }

    /// Lines into chunks of at least `size` bytes (the last may be shorter),
    /// as a file read in blocks would arrive.
    pub fn chunks(&self, size: usize) -> impl Iterator<Item = String> + '_ {
        let mut lines = self.lines();
        core::iter::from_fn(move || {
            let first = lines.next()?;
            let mut chunk = String::with_capacity(size);
            chunk.push_str(&first);
            while chunk.len() < size
                && let Some(line) = lines.next()
            {
                chunk.push_str(&line);
            }
            Some(chunk)
        })
    }

    /// Independent `SplitMix64` state for row `n` and stream `stream`.
    const fn rng(&self, n: u32, stream: u64) -> Rng {
        let mut z = self.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (n as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
            ^ stream.wrapping_mul(0x94D0_49BB_1331_11EB);
        if z == 0 {
            z = 0x9E37_79B9_7F4A_7C15;
        }
        Rng(z)
    }

    fn metadata_lines(&self) -> impl Iterator<Item = String> + '_ {
        let assays = self.effective_assays();
        let sv_count = self.study_variables();
        let header = HEADER_METADATA.iter().map(|s| format!("{s}\n"));
        let runs = (1..=assays).flat_map(|n| {
            [
                format!("MTD\tms_run[{n}]-location\tfile:///data/run{n}.mzML\n"),
                format!("MTD\tms_run[{n}]-format\t[MS, MS:1000584, mzML format, ]\n"),
                format!("MTD\tms_run[{n}]-id_format\t[MS, MS:1000776, scan number only nativeID format, ]\n"),
                format!("MTD\tms_run[{n}]-scan_polarity[1]\t[MS, MS:1000130, positive scan, ]\n"),
            ]
        });
        let assay_lines = (1..=assays).flat_map(|n| {
            [
                format!("MTD\tassay[{n}]\tassay {n}\n"),
                format!("MTD\tassay[{n}]-ms_run_ref\tms_run[{n}]\n"),
            ]
        });
        let sv_lines = (1..=sv_count).flat_map(move |v| {
            let refs: Vec<String> = (1..=assays)
                .filter(|a| if v == 1 { !a.is_multiple_of(2) } else { a.is_multiple_of(2) })
                .map(|a| format!("assay[{a}]"))
                .collect();
            [
                format!("MTD\tstudy_variable[{v}]\tgroup {v}\n"),
                format!(
                    "MTD\tstudy_variable[{v}]-assay_refs\t{}\n",
                    refs.join("|")
                ),
                format!("MTD\tstudy_variable[{v}]-description\tgroup {v}\n"),
                format!(
                    "MTD\tstudy_variable[{v}]-average_function\t[MS, MS:1002962, mean, ]\n"
                ),
                format!(
                    "MTD\tstudy_variable[{v}]-variation_function\t[MS, MS:1002885, standard error, ]\n"
                ),
            ]
        });
        let sv_refs = if sv_count == 1 {
            String::from("study_variable[1]")
        } else {
            String::from("study_variable[1]|study_variable[2]")
        };
        let sv_group = [
            String::from(
                "MTD\tstudy_variable_group[1]\t[STATO, STATO:0000087, independent variable, ]\n",
            ),
            format!("MTD\tstudy_variable_group[1]-study_variable_refs\t{sv_refs}\n"),
        ]
        .into_iter();
        let trailing = TRAILING_METADATA.iter().map(|s| format!("{s}\n"));
        header
            .chain(runs)
            .chain(assay_lines)
            .chain(sv_lines)
            .chain(sv_group)
            .chain(trailing)
    }

    fn header(&self, prefix: &str) -> String {
        let assays = self.effective_assays();
        let sv_count = self.study_variables();
        let mut line = String::from(prefix);
        for column in table_columns(prefix) {
            line.push('\t');
            line.push_str(column);
        }
        if prefix != "SEH" {
            for n in 1..=assays {
                write!(line, "\tabundance_assay[{n}]").unwrap_or_default();
            }
        }
        if prefix == "SMH" {
            for v in 1..=sv_count {
                write!(line, "\tabundance_study_variable[{v}]").unwrap_or_default();
            }
            for v in 1..=sv_count {
                write!(line, "\tabundance_variation_study_variable[{v}]").unwrap_or_default();
            }
        }
        line.push('\n');
        line
    }

    fn molecule(n: u32) -> &'static Metabolite {
        const FALLBACK: &Metabolite = match METABOLITES.first() {
            Some(m) => m,
            None => unreachable!(),
        };
        let idx = (n.saturating_sub(1) as usize) % METABOLITES.len();
        METABOLITES.get(idx).unwrap_or(FALLBACK)
    }

    fn mz_pair(&self, n: u32) -> (f64, f64) {
        let mol = Self::molecule(n);
        let theo = mol.neutral_mass_da + PROTON_MASS_DA;
        let mut rng = self.rng(n, STREAM_MASS_ERROR);
        // Mass error in [-5, +5] ppm before both m/z values are rounded to 4 dp.
        let ppm = (rng.uniform_f64(10_001) - 5_000.0) / 1_000.0;
        let exp = theo * ppm.mul_add(1e-6, 1.0);
        (exp, theo)
    }

    fn sml(&self, n: u32) -> String {
        let assays = self.effective_assays();
        let sv_count = self.study_variables();
        let mol = Self::molecule(n);
        let score = self.rng(n, STREAM_SCORE).score();
        let mut line = format!(
            "SML\t{n}\t{n}\tHMDB:{}\t{}\tnull\tnull\t{}\tnull\t{:.4}\t[M+H]1+\t2\t[MS, MS:1002890, fragmentation score, ]\t{score:.4}",
            mol.hmdb_id, mol.formula, mol.name, mol.neutral_mass_da
        );
        let mut ab_rng = self.rng(n, STREAM_ABUNDANCE);
        let mut sv_stats = [RunningStats::default(), RunningStats::default()];
        for a in 1..=assays {
            let value = ab_rng.abundance();
            write!(line, "\t{value:.4}").unwrap_or_default();
            let group = usize::from(a.is_multiple_of(2));
            if let Some(stat) = sv_stats.get_mut(group) {
                stat.push(value);
            }
        }
        for stat in sv_stats.iter().take(sv_count as usize) {
            write!(line, "\t{:.4}", stat.mean).unwrap_or_default();
        }
        for stat in sv_stats.iter().take(sv_count as usize) {
            match stat.standard_error() {
                Some(se) => write!(line, "\t{se:.4}").unwrap_or_default(),
                None => line.push_str("\tnull"),
            }
        }
        line.push('\n');
        line
    }

    fn smf(&self, n: u32) -> String {
        let assays = self.effective_assays();
        let (exp_mz, _) = self.mz_pair(n);
        let mut chrom_rng = self.rng(n, STREAM_CHROMATOGRAPHY);
        // Retention time in [15.0000, 615.0000) s with [3.0000, 15.0000] s half-width.
        let rt = 15.0 + chrom_rng.uniform_f64(6_000_000) / 10_000.0;
        let half_width = 3.0 + chrom_rng.uniform_f64(120_001) / 10_000.0;
        let rt_start = (rt - half_width).max(0.0);
        let rt_end = rt + half_width;
        let mut line = format!(
            "SMF\t{n}\t{n}\tnull\t[M+H]1+\tnull\t{exp_mz:.4}\t1\t{rt:.4}\t{rt_start:.4}\t{rt_end:.4}"
        );
        let mut ab_rng = self.rng(n, STREAM_ABUNDANCE);
        for _ in 1..=assays {
            let value = ab_rng.abundance();
            write!(line, "\t{value:.4}").unwrap_or_default();
        }
        line.push('\n');
        line
    }

    fn sme(&self, n: u32) -> String {
        let assays = self.effective_assays();
        let mol = Self::molecule(n);
        let (exp_mz, theo_mz) = self.mz_pair(n);
        let score = self.rng(n, STREAM_SCORE).score();
        let run = (n - 1) % assays + 1;
        format!(
            "SME\t{n}\tinput {n}\tHMDB:{}\t{}\tnull\tnull\t{}\tnull\tnull\t[M+H]1+\t{exp_mz:.4}\t1\t{theo_mz:.4}\tms_run[{run}]:scan={n}\t[MS, MS:1001477, SpectraST, ]\t[MS, MS:1000511, ms level, 2]\t{score:.4}\t1\n",
            mol.hmdb_id, mol.formula, mol.name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Spec;
    use mztab_m::cv::NoTerms;
    use mztab_m::validate::{self, Config};

    #[test]
    fn generated_documents_are_valid_and_sized() {
        for assays in [0, 1, 6] {
            let spec = Spec {
                assays,
                rows: 20,
                seed: 42,
            };
            let text = spec.document();
            let findings = validate::all(&text, &Config::default(), &NoTerms);
            assert!(
                findings.is_empty(),
                "assays={assays} had findings: {findings:#?}"
            );
            let model = mztab_m::parse(&text).mztab;
            assert_eq!(model.smf.as_ref().map(|t| t.rows.len()), Some(20), "rows");
            let chunks: Vec<String> = spec.chunks(1000).collect();
            assert!(
                chunks.iter().rev().skip(1).all(|c| c.len() >= 1000),
                "sized"
            );
            assert_eq!(chunks.concat(), text, "chunks cover the document");
        }
        let spec = Spec {
            assays: 6,
            rows: 20,
            seed: 42,
        };
        let other = Spec { seed: 43, ..spec };
        assert_ne!(other.document(), spec.document(), "the seed matters");
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one row walk checks abundances, statistics and m/z together"
    )]
    fn synthetic_chemistry_and_stats_are_consistent() {
        let spec = Spec {
            assays: 5,
            rows: 6,
            seed: 9,
        };
        let text = spec.document();
        let model = mztab_m::parse(&text).mztab;
        let sml = model.sml.expect("sml").rows;
        let smf = model.smf.expect("smf").rows;
        let sme = model.sme.expect("sme").rows;
        for ((row_sml, row_smf), row_sme) in sml.iter().zip(&smf).zip(&sme) {
            assert_eq!(
                row_sml.abundance.get(..5),
                Some(&row_smf.abundance[..]),
                "assay abundances match between SML and SMF"
            );
            assert_eq!(
                Some(&row_sml.best_id_confidence_value),
                row_sme.id_confidence.first(),
                "SML best confidence matches SME confidence"
            );
            assert_eq!(
                row_smf.exp_mass_to_charge, row_sme.exp_mass_to_charge,
                "SMF and SME share experimental m/z"
            );

            let rt = row_smf
                .retention_time_in_seconds
                .value()
                .expect("rt")
                .to_f64();
            let start = row_smf
                .retention_time_in_seconds_start
                .value()
                .expect("rt_start")
                .to_f64();
            let end = row_smf
                .retention_time_in_seconds_end
                .value()
                .expect("rt_end")
                .to_f64();
            assert!(start < rt && rt < end, "nonzero RT window");

            let g1: Vec<f64> = [0_usize, 2, 4]
                .into_iter()
                .filter_map(|a| {
                    row_sml
                        .abundance
                        .get(a)?
                        .value()
                        .map(mztab_m::Decimal::to_f64)
                })
                .collect();
            assert_eq!(g1.len(), 3, "three assays in group 1");
            let sv1 = row_sml
                .abundance
                .get(5)
                .and_then(|c| c.value())
                .expect("sv1")
                .to_f64();
            let mean1 = g1.iter().sum::<f64>() / 3.0;
            assert!((sv1 - mean1).abs() < 1e-3, "sv1 mean matches assays");

            let g2: Vec<f64> = [1_usize, 3]
                .into_iter()
                .filter_map(|a| {
                    row_sml
                        .abundance
                        .get(a)?
                        .value()
                        .map(mztab_m::Decimal::to_f64)
                })
                .collect();
            let cell = |i: usize| {
                row_sml
                    .abundance
                    .get(i)
                    .and_then(|c| c.value())
                    .expect("sv cell")
                    .to_f64()
            };
            let mean2 = g2.iter().sum::<f64>() / 2.0;
            assert!((cell(6) - mean2).abs() < 1e-3, "sv2 mean matches assays");
            // Standard error of the mean with the n-1 sample variance.
            let se = |g: &[f64], mean: f64| {
                let n = f64::from(u32::try_from(g.len()).expect("small group"));
                (g.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0) / n).sqrt()
            };
            assert!(
                (cell(7) - se(&g1, mean1)).abs() < 1e-3,
                "sv1 standard error"
            );
            assert!(
                (cell(8) - se(&g2, mean2)).abs() < 1e-3,
                "sv2 standard error"
            );

            let neutral = row_sml
                .theoretical_neutral_mass
                .value()
                .and_then(|list| list.first())
                .and_then(|c| c.value())
                .expect("neutral mass")
                .to_f64();
            let theo = row_sme
                .theoretical_mass_to_charge
                .value()
                .expect("theoretical m/z")
                .to_f64();
            let exp = row_sme
                .exp_mass_to_charge
                .value()
                .expect("experimental m/z")
                .to_f64();
            // [M+H]+ m/z = M + m_p; 4-dp rounding of M and m/z bounds the gap.
            assert!(
                (theo - (neutral + super::PROTON_MASS_DA)).abs() < 1e-4,
                "[M+H]+ m/z"
            );
            // 5 ppm plus 1e-4 of rounding on each m/z value.
            let ppm = (exp - theo) / theo * 1e6;
            assert!(
                ppm.abs() <= (2e-4 / theo).mul_add(1e6, 5.0),
                "mass error {ppm} ppm"
            );
        }
    }

    /// The exact output, pinned: benchmark inputs must not drift. Set
    /// `BLESS=1` to rewrite `benches/golden.mztab` after an intended change.
    #[test]
    fn output_is_pinned() {
        let spec = Spec {
            assays: 5,
            rows: 3,
            seed: 9,
        };
        let text = spec.document();
        let golden = concat!(env!("CARGO_MANIFEST_DIR"), "/golden.mztab");
        if std::env::var_os("BLESS").is_some() {
            std::fs::write(golden, &text).expect("write golden");
        }
        let expected = std::fs::read_to_string(golden).unwrap_or_default();
        assert!(
            expected == text,
            "generator output changed; BLESS=1 if intended"
        );
        let first = text.lines().next().map_or(0, str::len) + 1;
        assert_eq!(
            spec.chunks(first).next().map(|c| c.len()),
            Some(first),
            "a chunk stops as soon as it reaches the size"
        );
    }
}
