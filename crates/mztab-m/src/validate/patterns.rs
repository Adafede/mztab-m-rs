//! Format predicates shared across structural and semantic validators.

use crate::param::Param;
use crate::value::decimal::{Decimal, DecimalKind};
use crate::vocab::field::Datatype;

fn two_digits(a: u8, b: u8) -> Option<u32> {
    (a.is_ascii_digit() && b.is_ascii_digit())
        .then(|| u32::from(a - b'0') * 10 + u32::from(b - b'0'))
}

fn digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn signed_digits(s: &str) -> bool {
    digits(s.strip_prefix(['-', '+']).unwrap_or(s))
}

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

fn is_uri_chars(rest: &str) -> bool {
    let bytes = rest.as_bytes();
    let chars_ok = bytes.iter().enumerate().all(|(i, &b)| match b {
        b'%' => bytes
            .get(i + 1..i + 3)
            .is_some_and(|h| h.iter().all(u8::is_ascii_hexdigit)),
        // Unreserved, sub-delims and the delimiters of the URI syntax.
        b => b.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@/?#[]".contains(&b),
    });
    chars_ok && rest.matches('#').count() <= 1
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
    scheme_ok && is_uri_chars(rest)
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

/// Whether `text` is a valid ISO 8601 calendar date `YYYY-MM-DD`.
pub(super) fn is_date(text: &str) -> bool {
    let [y1, y2, y3, y4, b'-', m1, m2, b'-', d1, d2] = *text.as_bytes() else {
        return false;
    };
    let (Some(yh), Some(yl), Some(m), Some(d)) = (
        two_digits(y1, y2),
        two_digits(y3, y4),
        two_digits(m1, m2),
        two_digits(d1, d2),
    ) else {
        return false;
    };
    let year = yh * 100 + yl;
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let max_day = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=max_day).contains(&d)
}

/// Whether `text` is `hh:mm:ss` (`00:00:00`–`23:59:59` or `24:00:00`),
/// optionally with fractional seconds and a timezone (`Z` or `±hh:mm` in
/// `±00:00`–`±14:00`).
pub(super) fn is_time(text: &str) -> bool {
    let [h1, h2, b':', m1, m2, b':', s1, s2, rest @ ..] = text.as_bytes() else {
        return false;
    };
    let (Some(h), Some(m), Some(s)) = (
        two_digits(*h1, *h2),
        two_digits(*m1, *m2),
        two_digits(*s1, *s2),
    ) else {
        return false;
    };
    let (frac_nonzero, rest) = match rest.first() {
        Some(b'.') => {
            let frac = rest.get(1..).unwrap_or_default();
            let count = frac.iter().take_while(|c| c.is_ascii_digit()).count();
            if count == 0 {
                return false;
            }
            let nonzero = frac
                .get(..count)
                .is_some_and(|d| d.iter().any(|&c| c != b'0'));
            (nonzero, frac.get(count..).unwrap_or_default())
        }
        _ => (false, rest),
    };
    let clock = (h < 24 && m < 60 && s < 60) || (h == 24 && m == 0 && s == 0 && !frac_nonzero);
    let zone = match rest {
        [] | [b'Z'] => true,
        [b'+' | b'-', zh1, zh2, b':', zm1, zm2] => two_digits(*zh1, *zh2).is_some_and(|zh| {
            two_digits(*zm1, *zm2).is_some_and(|zm| (zh < 14 && zm < 60) || (zh == 14 && zm == 0))
        }),
        _ => false,
    };
    clock && zone
}

/// Whether a study-variable value matches its group's datatype.
pub(super) fn matches_datatype(datatype: Datatype, value: &str) -> bool {
    match datatype {
        Datatype::String => true,
        Datatype::AnyUri => !value.is_empty() && is_uri_chars(value),
        Datatype::Integer => signed_digits(value),
        Datatype::Decimal => Decimal::parse(value).is_ok_and(|d| d.kind() == DecimalKind::Decimal),
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

#[derive(Clone, Copy)]
enum NativeValue {
    IdRef,
    NonNegativeInt,
    PositiveInt,
}

/// The keys of the native spectrum id of each `id_format` term in spec
/// table 1, and their expected XML Schema value type.
const NATIVE_IDS: &[(&str, &[(&str, NativeValue)])] = &[
    (
        "MS:1000768",
        &[
            ("controllerType", NativeValue::NonNegativeInt),
            ("controllerNumber", NativeValue::PositiveInt),
            ("scan", NativeValue::PositiveInt),
        ],
    ),
    (
        "MS:1000769",
        &[
            ("function", NativeValue::PositiveInt),
            ("process", NativeValue::NonNegativeInt),
            ("scan", NativeValue::NonNegativeInt),
        ],
    ),
    (
        "MS:1000770",
        &[
            ("sample", NativeValue::NonNegativeInt),
            ("period", NativeValue::NonNegativeInt),
            ("cycle", NativeValue::NonNegativeInt),
            ("experiment", NativeValue::NonNegativeInt),
        ],
    ),
    ("MS:1000771", &[("scan", NativeValue::NonNegativeInt)]),
    ("MS:1000772", &[("scan", NativeValue::NonNegativeInt)]),
    ("MS:1000773", &[("file", NativeValue::IdRef)]),
    ("MS:1000774", &[("index", NativeValue::NonNegativeInt)]),
    ("MS:1000775", &[("file", NativeValue::IdRef)]),
    ("MS:1000776", &[("scan", NativeValue::NonNegativeInt)]),
    ("MS:1000777", &[("spectrum", NativeValue::NonNegativeInt)]),
];

/// Whether `reference` has the form spec table 1 gives for the `id_format`
/// term `accession`: space-separated `key=value` pairs, in order. `None`
/// for a term the table does not constrain (such as `MS:1001530`, any
/// string).
pub(super) fn native_id_ok(accession: &str, reference: &str) -> Option<bool> {
    let (_, keys) = NATIVE_IDS.iter().find(|(a, _)| *a == accession)?;
    let mut parts = reference.split(' ');
    let matched = keys.iter().all(|(key, kind)| {
        parts.next().is_some_and(|part| {
            part.split_once('=').is_some_and(|(k, v)| {
                k == *key
                    && match kind {
                        NativeValue::IdRef => !v.is_empty(),
                        NativeValue::NonNegativeInt => digits(v),
                        NativeValue::PositiveInt => {
                            digits(v) && v.bytes().any(|b| (b'1'..=b'9').contains(&b))
                        }
                    }
            })
        })
    });
    Some(matched && parts.next().is_none())
}

/// Whether `value` is a lexical value of XML Schema type `kind` (by local
/// name); `None` for a type not checked.
pub(super) fn xsd_matches(kind: &str, value: &str) -> Option<bool> {
    Some(match kind {
        "short" => value.parse::<i16>().is_ok(),
        "int" => value.parse::<i32>().is_ok(),
        "long" => value.parse::<i64>().is_ok(),
        "unsignedInt" => value.parse::<u32>().is_ok(),
        "integer" => signed_digits(value),
        "nonNegativeInteger" => digits(value.strip_prefix('+').unwrap_or(value)),
        "positiveInteger" => {
            digits(value.strip_prefix('+').unwrap_or(value))
                && value.bytes().any(|b| (b'1'..=b'9').contains(&b))
        }
        "decimal" => Decimal::parse(value).is_ok_and(|d| d.kind() == DecimalKind::Decimal),
        "float" | "double" => {
            matches!(value, "NaN" | "INF" | "-INF" | "+INF")
                || Decimal::parse(value).is_ok_and(|d| {
                    matches!(d.kind(), DecimalKind::Decimal | DecimalKind::Scientific)
                })
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
    use crate::vocab::field::Datatype;

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
            is_date("2025-03-14")
                && is_date("2024-02-29")
                && is_date("2000-02-29")
                && is_date("2025-04-30")
                && is_date("2025-01-31")
                && !is_date("2025-02-29")
                && !is_date("1900-02-29")
                && !is_date("2025-00-10")
                && !is_date("2025-13-01")
                && !is_date("2025-01-00")
                && !is_date("2025-04-31")
                && !is_date("2025-01-32")
                && !is_date("2025-3-14")
                && !is_date("2025-ab-14")
                && !is_date("2025/03/14"),
            "dates"
        );
        for ok in [
            "09:45:30",
            "09:45:30.250Z",
            "09:45:30+02:00",
            "09:45:30-02:00",
            "23:59:59",
            "24:00:00",
            "24:00:00.0",
            "12:00:00+14:00",
            "12:00:00-14:00",
            "12:00:00+13:59",
        ] {
            assert!(is_time(ok), "{ok}");
        }
        for bad in [
            "09:45",
            "09:45:30.",
            "09:45:30+0200",
            "09-45-30",
            "09:ab:30",
            "09:45:30Y",
            "09:45:30*02:00",
            "24:00:00.1",
            "24:01:00",
            "24:00:01",
            "25:00:00",
            "23:60:00",
            "23:00:60",
            "12:00:00+14:01",
            "12:00:00+15:00",
            "12:00:00+13:60",
        ] {
            assert!(!is_time(bad), "{bad}");
        }
        assert_eq!(native_id_ok("MS:1000776", "scan=1"), Some(true));
        assert_eq!(native_id_ok("MS:1000776", "scan=1 extra=2"), Some(false));
        assert_eq!(native_id_ok("MS:1001530", "anything"), None);
        assert_eq!(
            native_id_ok("MS:1000768", "controllerType=0 controllerNumber=1 scan=1"),
            Some(true)
        );
        assert_eq!(
            native_id_ok("MS:1000768", "controllerType=0 controllerNumber=0 scan=1"),
            Some(false)
        );
        assert_eq!(
            native_id_ok("MS:1000768", "controllerType=0 controllerNumber=1 scan=0"),
            Some(false)
        );
        assert_eq!(
            native_id_ok("MS:1000769", "function=1 process=0 scan=0"),
            Some(true)
        );
        assert_eq!(
            native_id_ok("MS:1000769", "function=0 process=0 scan=0"),
            Some(false)
        );
    }

    #[test]
    fn datatypes() {
        let cases = [
            (Datatype::String, "anything", true),
            (Datatype::AnyUri, "x", true),
            (Datatype::AnyUri, "https://example.org/a%20b#c", true),
            (Datatype::AnyUri, "not a uri", false),
            (Datatype::AnyUri, "a%zz", false),
            (Datatype::AnyUri, "", false),
            (Datatype::Integer, "-12", true),
            (Datatype::Integer, "+12", true),
            (
                Datatype::Integer,
                "1234567890123456789012345678901234567890",
                true,
            ),
            (Datatype::Integer, "+", false),
            (Datatype::Integer, "1.0", false),
            (Datatype::Decimal, "1.5", true),
            (Datatype::Decimal, "NaN", false),
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
            ("int", "99999999999999", Some(false)),
            ("short", "32767", Some(true)),
            ("short", "32768", Some(false)),
            ("long", "-9223372036854775808", Some(true)),
            ("unsignedInt", "4294967295", Some(true)),
            ("unsignedInt", "-1", Some(false)),
            ("integer", "x", Some(false)),
            ("integer", "999999999999999999999999999999", Some(true)),
            ("nonNegativeInteger", "+0", Some(true)),
            ("nonNegativeInteger", "-1", Some(false)),
            ("positiveInteger", "0", Some(false)),
            ("positiveInteger", "07", Some(true)),
            ("double", "1.5e3", Some(true)),
            ("double", "inf", Some(false)),
            ("double", "Infinity", Some(false)),
            ("float", "INF", Some(true)),
            ("decimal", "1.5", Some(true)),
            ("decimal", "1.5e3", Some(false)),
            ("decimal", "NaN", Some(false)),
            ("decimal", "INF", Some(false)),
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
