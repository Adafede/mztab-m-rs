//! Format predicates shared across structural and semantic validators.

use crate::param::Param;
use crate::value::Decimal;
use crate::vocab::Datatype;

/// Whether `text` matches `^[0-9]{4}-[0-9]{4}-[0-9]{4}-[0-9]{3}[0-9X]$`.
pub(super) fn is_orcid(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 19
        && bytes.iter().enumerate().all(|(i, &b)| match i {
            4 | 9 | 14 => b == b'-',
            18 => b.is_ascii_digit() || b == b'X',
            _ => b.is_ascii_digit(),
        })
}

/// Whether `text` is an RFC 3986 URI: a scheme, `:`, then only characters
/// RFC 3986 allows (a `%` only before two hex digits, at most one `#`).
pub(super) fn is_uri(text: &str) -> bool {
    let Some((scheme, rest)) = text.split_once(':') else {
        return false;
    };
    let scheme_ok = scheme
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic())
        && scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'));
    let bytes = rest.as_bytes();
    let chars_ok = bytes.iter().enumerate().all(|(i, &b)| match b {
        b'%' => bytes
            .get(i + 1..i + 3)
            .is_some_and(|h| h.iter().all(u8::is_ascii_hexdigit)),
        // Unreserved, sub-delims and the delimiters of the URI syntax.
        b => b.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@/?#[]".contains(&b),
    });
    scheme_ok && chars_ok && rest.matches('#').count() <= 1
}

/// Whether `text` looks like an e-mail address: `local@domain`, with a dot
/// inside the domain and no spaces.
pub(super) fn is_email(text: &str) -> bool {
    let Some((local, domain)) = text.split_once('@') else {
        return false;
    };
    let printable = |s: &str| s.bytes().all(|b| b.is_ascii_graphic() && b != b'@');
    !local.is_empty()
        && printable(local)
        && printable(domain)
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

/// Whether `text` is an ISO 8601 date `YYYY-MM-DD`.
pub(super) fn is_date(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, &c)| {
            if i == 4 || i == 7 {
                c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}

/// Whether `text` is `hh:mm:ss`, optionally with fractional seconds and a
/// zone (`Z` or `±hh:mm`).
pub(super) fn is_time(text: &str) -> bool {
    let b = text.as_bytes();
    let Some(rest) = b.get(8..) else { return false };
    let clock = b.iter().take(8).enumerate().all(|(i, &c)| {
        if i == 2 || i == 5 {
            c == b':'
        } else {
            c.is_ascii_digit()
        }
    });
    let rest = match rest.first() {
        Some(b'.') => {
            let digits = rest
                .iter()
                .skip(1)
                .take_while(|c| c.is_ascii_digit())
                .count();
            if digits == 0 {
                return false;
            }
            rest.get(1 + digits..).unwrap_or_default()
        }
        _ => rest,
    };
    let zone = match rest {
        [] | [b'Z'] => true,
        [sign, h1, h2, b':', m1, m2] => {
            matches!(sign, b'+' | b'-') && [h1, h2, m1, m2].iter().all(|c| c.is_ascii_digit())
        }
        _ => false,
    };
    clock && zone
}

/// Whether a study-variable value matches its group's datatype.
pub(super) fn matches_datatype(datatype: Datatype, value: &str) -> bool {
    match datatype {
        Datatype::String | Datatype::AnyUri => true,
        Datatype::Integer => value.parse::<i128>().is_ok() && !value.starts_with('+'),
        Datatype::Decimal => Decimal::parse(value).is_ok_and(|d| d.kind().is_conforming()),
        Datatype::Boolean => matches!(value, "true" | "false"),
        Datatype::Date => is_date(value),
        Datatype::Time => is_time(value),
        Datatype::DateTime => value
            .split_once('T')
            .is_some_and(|(date, time)| is_date(date) && is_time(time)),
        Datatype::Parameter => Param::parse(value).is_ok(),
    }
}

/// Whether `text` matches `^\[\d*M([+-][\w\d]+)*\]\d*[+-]$` (the SML and
/// SME pattern of the JSON Schema, used for SMF too; DECISIONS D-016).
pub(super) fn is_adduct(text: &str) -> bool {
    let Some(rest) = text.strip_prefix('[') else {
        return false;
    };
    let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    let Some(mut rest) = rest.strip_prefix('M') else {
        return false;
    };
    loop {
        match rest.as_bytes().first() {
            Some(b'+' | b'-') => {
                let after = rest.get(1..).unwrap_or_default();
                let word = after.len()
                    - after
                        .trim_start_matches(|c: char| c.is_ascii_alphanumeric() || c == '_')
                        .len();
                if word == 0 {
                    return false;
                }
                rest = after.get(word..).unwrap_or_default();
            }
            Some(b']') => break,
            _ => return false,
        }
    }
    let charge = rest
        .get(1..)
        .unwrap_or_default()
        .trim_start_matches(|c: char| c.is_ascii_digit());
    charge == "+" || charge == "-"
}

/// The keys of the native spectrum id of each `id_format` term in spec
/// table 1, and whether each value is a non-negative integer (`false`: an
/// id, `xsd:IDREF`).
const NATIVE_IDS: &[(&str, &[(&str, bool)])] = &[
    (
        "MS:1000768",
        &[
            ("controllerType", true),
            ("controllerNumber", true),
            ("scan", true),
        ],
    ),
    (
        "MS:1000769",
        &[("function", true), ("process", true), ("scan", true)],
    ),
    (
        "MS:1000770",
        &[
            ("sample", true),
            ("period", true),
            ("cycle", true),
            ("experiment", true),
        ],
    ),
    ("MS:1000771", &[("scan", true)]),
    ("MS:1000772", &[("scan", true)]),
    ("MS:1000773", &[("file", false)]),
    ("MS:1000774", &[("index", true)]),
    ("MS:1000775", &[("file", false)]),
    ("MS:1000776", &[("scan", true)]),
    ("MS:1000777", &[("spectrum", true)]),
];

/// Whether `reference` has the form spec table 1 gives for the `id_format`
/// term `accession`: space-separated `key=value` pairs, in order. `None`
/// for a term the table does not constrain (such as `MS:1001530`, any
/// string).
pub(super) fn native_id_ok(accession: &str, reference: &str) -> Option<bool> {
    let (_, keys) = NATIVE_IDS.iter().find(|(a, _)| *a == accession)?;
    let mut parts = reference.split(' ');
    let matched = keys.iter().all(|(key, number)| {
        parts.next().is_some_and(|part| {
            part.split_once('=').is_some_and(|(k, v)| {
                k == *key && !v.is_empty() && (!*number || v.bytes().all(|b| b.is_ascii_digit()))
            })
        })
    });
    Some(matched && parts.next().is_none())
}

/// Whether `value` is a lexical value of XML Schema type `kind` (by local
/// name); `None` for a type not checked.
pub(super) fn xsd_matches(kind: &str, value: &str) -> Option<bool> {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let signed = |s: &str| digits(s.strip_prefix(['-', '+']).unwrap_or(s));
    Some(match kind {
        "int" | "integer" | "long" | "short" => signed(value),
        "nonNegativeInteger" | "unsignedInt" => digits(value.strip_prefix('+').unwrap_or(value)),
        "positiveInteger" => {
            digits(value.strip_prefix('+').unwrap_or(value))
                && value.bytes().any(|b| (b'1'..=b'9').contains(&b))
        }
        "float" | "double" | "decimal" => {
            matches!(value, "NaN" | "INF" | "-INF") || value.parse::<f64>().is_ok()
        }
        "boolean" => matches!(value, "true" | "false" | "1" | "0"),
        // `string`, `anyURI`, dates and the rest: not checked.
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use super::{
        is_adduct, is_date, is_orcid, is_time, matches_datatype, native_id_ok, xsd_matches,
    };
    use crate::vocab::Datatype;

    #[test]
    fn patterns() {
        assert!(is_orcid("0000-0002-1825-009X"), "X check digit");
        for bad in [
            "0000-0002-1825-00",
            "0000-0002-1825-009x",
            "000000021825-0097",
            "https://orcid.org/0000-0002-1825-0097",
        ] {
            assert!(!is_orcid(bad), "{bad}");
        }
        assert!(
            is_date("2025-03-14") && !is_date("2025-3-14") && !is_date("2025/03/14"),
            "dates"
        );
        for ok in [
            "09:45:30",
            "09:45:30.250Z",
            "09:45:30+02:00",
            "09:45:30-02:00",
        ] {
            assert!(is_time(ok), "{ok}");
        }
        for bad in [
            "09:45",
            "09:45:30.",
            "09:45:30+0200",
            "09-45-30",
            "09:45:30Y",
            "09:45:30*02:00",
        ] {
            assert!(!is_time(bad), "{bad}");
        }
        assert_eq!(native_id_ok("MS:1000776", "scan=1"), Some(true));
        assert_eq!(native_id_ok("MS:1000776", "scan=1 extra=2"), Some(false));
        assert_eq!(native_id_ok("MS:1001530", "anything"), None);
    }

    #[test]
    fn datatypes() {
        let cases = [
            (Datatype::String, "anything", true),
            (Datatype::AnyUri, "x", true),
            (Datatype::Integer, "-12", true),
            (Datatype::Integer, "+12", false),
            (Datatype::Integer, "1.0", false),
            (Datatype::Decimal, "1.5", true),
            (Datatype::Decimal, "1e5", false),
            (Datatype::Boolean, "true", true),
            (Datatype::Boolean, "TRUE", false),
            (Datatype::Date, "2025-03-14", true),
            (Datatype::Time, "09:45:30Z", true),
            (Datatype::DateTime, "2025-03-14T09:45:30", true),
            (Datatype::DateTime, "2025-03-14 09:45:30", false),
            (Datatype::Parameter, "[,,Male,]", true),
            (Datatype::Parameter, "Male", false),
        ];
        for (datatype, value, expected) in cases {
            assert_eq!(
                matches_datatype(datatype, value),
                expected,
                "{datatype} {value}"
            );
        }
    }

    #[test]
    fn adducts() {
        for ok in [
            "[M+H]1+",
            "[M]+",
            "[2M+Na]1+",
            "[M-H2O+H]1+",
            "[M+2H]2+",
            "[M-H]-",
            "[M+NH4]+",
        ] {
            assert!(is_adduct(ok), "{ok}");
        }
        for bad in [
            "M+H", "[M+H]", "[M+]1+", "[X+H]1+", "[M+H]1", "[M+H]1+x", "[M*H]+", "[M+H",
        ] {
            assert!(!is_adduct(bad), "{bad}");
        }
    }

    #[test]
    fn xml_schema_types() {
        let cases = [
            ("int", "-3", Some(true)),
            ("integer", "x", Some(false)),
            ("nonNegativeInteger", "+0", Some(true)),
            ("nonNegativeInteger", "-1", Some(false)),
            ("positiveInteger", "0", Some(false)),
            ("positiveInteger", "07", Some(true)),
            ("double", "1.5e3", Some(true)),
            ("float", "INF", Some(true)),
            ("decimal", "one", Some(false)),
            ("boolean", "1", Some(true)),
            ("boolean", "yes", Some(false)),
            ("string", "anything", None),
            ("QName", "x", None),
        ];
        for (kind, value, expected) in cases {
            assert_eq!(xsd_matches(kind, value), expected, "{kind} {value}");
        }
    }
}
