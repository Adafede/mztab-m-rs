//! A seeded, deterministic generator of valid mzTab-M 2.1 documents of any
//! size, for the benchmarks (PLAN §7).
//!
//! A document is produced line by line, so a gigabyte can be streamed
//! without being held in memory.
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

/// What to generate: `rows` small molecules, each with one feature and one
/// evidence (profile `M+S+F+E`), quantified in `assays` assays split over
/// two study variables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spec {
    /// Assays (and MS runs, one each): the width of the abundance columns.
    pub assays: u32,
    /// Rows in each of SML, SMF and SME.
    pub rows: u32,
    /// The seed of the value generator.
    pub seed: u64,
}

/// Study variables: assays alternate between the two.
const STUDY_VARIABLES: u32 = 2;

/// A small xorshift generator: deterministic across platforms.
#[derive(Clone, Debug)]
struct Rng(u64);

impl Rng {
    const fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// A decimal in `[0, scale)` with four places.
    fn decimal(&mut self, scale: u64) -> String {
        let n = self.next() % (scale * 10_000);
        format!("{}.{:04}", n / 10_000, n % 10_000)
    }
}

impl Spec {
    /// The whole document as one string.
    #[must_use]
    pub fn document(&self) -> String {
        self.lines().collect()
    }

    /// The document's lines, each ending in `\n`.
    pub fn lines(&self) -> impl Iterator<Item = String> + '_ {
        self.metadata()
            .into_iter()
            .chain(core::iter::once(String::from("\n")))
            .chain(core::iter::once(self.header("SMH")))
            .chain((1..=self.rows).map(move |n| self.sml(n, &mut self.rng(n, 1))))
            .chain(core::iter::once(String::from("\n")))
            .chain(core::iter::once(self.header("SFH")))
            .chain((1..=self.rows).map(move |n| self.smf(n, &mut self.rng(n, 2))))
            .chain(core::iter::once(String::from("\n")))
            .chain(core::iter::once(self.header("SEH")))
            .chain((1..=self.rows).map(move |n| self.sme(n, &mut self.rng(n, 3))))
    }

    /// Lines into chunks of at least `size` bytes (the last may be shorter),
    /// as a file read in blocks would arrive.
    pub fn chunks(&self, size: usize) -> impl Iterator<Item = String> + '_ {
        let mut lines = self.lines().peekable();
        core::iter::from_fn(move || {
            lines.peek()?;
            let mut chunk = String::with_capacity(size);
            while chunk.len() < size {
                match lines.next() {
                    Some(line) => chunk.push_str(&line),
                    None => break,
                }
            }
            Some(chunk)
        })
    }

    /// The generator of row `n` of table `table`: rows are independent, so
    /// any row can be produced on its own.
    const fn rng(&self, n: u32, table: u64) -> Rng {
        let row = (n as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        let mixed = self.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ row ^ table;
        Rng(mixed | 1)
    }

    fn metadata(&self) -> Vec<String> {
        let mut m = vec![
            "MTD\tmzTab-version\t2.1.0-M".to_owned(),
            "MTD\tmzTab-ID\tSYNTHETIC".to_owned(),
            "MTD\tmzTab-profile\tM+S+F+E".to_owned(),
            "MTD\ttitle\tSynthetic benchmark document".to_owned(),
            "MTD\tsoftware[1]\t[MS, MS:1002879, Progenesis QI, 3.0]".to_owned(),
            "MTD\tquantification_method\t[MS, MS:1001834, LC-MS label-free quantitation analysis, ]".to_owned(),
        ];
        for n in 1..=self.assays {
            m.push(format!(
                "MTD\tms_run[{n}]-location\tfile:///data/run{n}.mzML"
            ));
            m.push(format!(
                "MTD\tms_run[{n}]-format\t[MS, MS:1000584, mzML format, ]"
            ));
            m.push(format!(
                "MTD\tms_run[{n}]-id_format\t[MS, MS:1000774, multiple peak list nativeID format, ]"
            ));
            m.push(format!(
                "MTD\tms_run[{n}]-scan_polarity[1]\t[MS, MS:1000130, positive scan, ]"
            ));
        }
        for n in 1..=self.assays {
            m.push(format!("MTD\tassay[{n}]\tassay {n}"));
            m.push(format!("MTD\tassay[{n}]-ms_run_ref\tms_run[{n}]"));
        }
        for v in 1..=STUDY_VARIABLES {
            let refs: Vec<String> = (1..=self.assays)
                .filter(|a| a % STUDY_VARIABLES == v % STUDY_VARIABLES)
                .map(|a| format!("assay[{a}]"))
                .collect();
            m.push(format!("MTD\tstudy_variable[{v}]\tgroup {v}"));
            m.push(format!(
                "MTD\tstudy_variable[{v}]-assay_refs\t{}",
                refs.join("|")
            ));
            m.push(format!("MTD\tstudy_variable[{v}]-description\tgroup {v}"));
        }
        m.extend([
            "MTD\tstudy_variable_group[1]\t[STATO, STATO:0000087, independent variable, ]".to_owned(),
            "MTD\tstudy_variable_group[1]-study_variable_refs\tstudy_variable[1]|study_variable[2]"
                .to_owned(),
            "MTD\tcv[1]-label\tMS".to_owned(),
            "MTD\tcv[1]-full_name\tPSI-MS controlled vocabulary".to_owned(),
            "MTD\tcv[1]-version\t4.1.0".to_owned(),
            "MTD\tcv[1]-uri\thttps://purl.obolibrary.org/obo/ms.obo".to_owned(),
            "MTD\tcv[2]-label\tSTATO".to_owned(),
            "MTD\tcv[2]-full_name\tStatistics Ontology".to_owned(),
            "MTD\tcv[2]-version\tRC1.4".to_owned(),
            "MTD\tcv[2]-uri\thttp://purl.obolibrary.org/obo/stato.owl".to_owned(),
            "MTD\tcv[3]-label\tPRIDE".to_owned(),
            "MTD\tcv[3]-full_name\tPRIDE Controlled Vocabulary".to_owned(),
            "MTD\tcv[3]-version\t2023".to_owned(),
            "MTD\tcv[3]-uri\thttps://github.com/PRIDE-Utilities/pride-ontology".to_owned(),
            "MTD\tdatabase[1]\t[, , HMDB, ]".to_owned(),
            "MTD\tdatabase[1]-prefix\thmdb".to_owned(),
            "MTD\tdatabase[1]-version\t5.0".to_owned(),
            "MTD\tdatabase[1]-uri\thttps://hmdb.ca".to_owned(),
            "MTD\tsmall_molecule-quantification_unit\t[PRIDE, PRIDE:0000330, Arbitrary quantification unit, ]".to_owned(),
            "MTD\tsmall_molecule_feature-quantification_unit\t[PRIDE, PRIDE:0000330, Arbitrary quantification unit, ]".to_owned(),
            "MTD\tid_confidence_measure[1]\t[MS, MS:1002890, fragmentation score, ]".to_owned(),
        ]);
        m.into_iter().map(|line| line + "\n").collect()
    }

    fn header(&self, prefix: &str) -> String {
        let mut line = String::from(prefix);
        let columns: &[&str] = match prefix {
            "SMH" => &[
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
            ],
            "SFH" => &[
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
            ],
            _ => &[
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
            ],
        };
        for column in columns {
            line.push('\t');
            line.push_str(column);
        }
        if prefix != "SEH" {
            for n in 1..=self.assays {
                write!(line, "\tabundance_assay[{n}]").unwrap_or_default();
            }
        }
        if prefix == "SMH" {
            for v in 1..=STUDY_VARIABLES {
                write!(line, "\tabundance_study_variable[{v}]").unwrap_or_default();
            }
            for v in 1..=STUDY_VARIABLES {
                write!(line, "\tabundance_variation_study_variable[{v}]").unwrap_or_default();
            }
        }
        line.push('\n');
        line
    }

    fn sml(&self, n: u32, rng: &mut Rng) -> String {
        let mass = rng.decimal(1000);
        let score = rng.decimal(1);
        let mut line = format!(
            "SML\t{n}\t{n}\tHMDB:HMDB{n:07}\tC6H12O6\tnull\tnull\tcompound {n}\tnull\t{mass}\t[M+H]1+\t2\t[MS, MS:1002890, fragmentation score, ]\t{score}"
        );
        push_decimals(&mut line, rng, self.assays, 1_000_000);
        // Study-variable abundances, then their variations.
        push_decimals(&mut line, rng, STUDY_VARIABLES, 1_000_000);
        push_decimals(&mut line, rng, STUDY_VARIABLES, 100);
        line.push('\n');
        line
    }

    fn smf(&self, n: u32, rng: &mut Rng) -> String {
        let mz = rng.decimal(1000);
        let rt = rng.decimal(600);
        let mut line = format!("SMF\t{n}\t{n}\tnull\t[M+H]1+\tnull\t{mz}\t1\t{rt}\t{rt}\t{rt}");
        push_decimals(&mut line, rng, self.assays, 1_000_000);
        line.push('\n');
        line
    }

    fn sme(&self, n: u32, rng: &mut Rng) -> String {
        let mz = rng.decimal(1000);
        let score = rng.decimal(1);
        let run = (n - 1) % self.assays + 1;
        format!(
            "SME\t{n}\tinput {n}\tHMDB:HMDB{n:07}\tC6H12O6\tnull\tnull\tcompound {n}\tnull\tnull\t[M+H]1+\t{mz}\t1\t{mz}\tms_run[{run}]:index={n}\t[MS, MS:1001477, SpectraST, ]\t[MS, MS:1000511, ms level, 2]\t{score}\t1\n"
        )
    }
}

/// Appends `count` tab-separated random decimals up to `scale` onto `line`.
fn push_decimals(line: &mut String, rng: &mut Rng, count: u32, scale: u64) {
    for _ in 0..count {
        line.push('\t');
        line.push_str(&rng.decimal(scale));
    }
}

#[cfg(test)]
mod tests {
    use super::Spec;
    use mztab_m::cv::NoTerms;
    use mztab_m::validate::{self, Config};

    #[test]
    fn generated_documents_are_valid_and_sized() {
        let spec = Spec {
            assays: 6,
            rows: 20,
            seed: 42,
        };
        let text = spec.document();
        let findings = validate::all(&text, &Config::default(), &NoTerms);
        let errors: Vec<String> = findings
            .iter()
            .filter(|f| f.severity >= mztab_m::Severity::Warning)
            .map(ToString::to_string)
            .collect();
        assert!(errors.is_empty(), "{errors:#?}");
        let model = mztab_m::parse(&text).mztab;
        assert_eq!(model.smf.map(|t| t.rows.len()), Some(20), "rows");
        let chunks: Vec<String> = spec.chunks(1000).collect();
        assert!(
            chunks.iter().rev().skip(1).all(|c| c.len() >= 1000),
            "sized"
        );
        assert_eq!(chunks.concat(), text, "chunks cover the document");
        let other = Spec { seed: 43, ..spec };
        assert_ne!(other.document(), text, "the seed matters");
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
