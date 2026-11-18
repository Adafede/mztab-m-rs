//! Findings as JSON, for tools (the CLI's `--format json`, the WASM demo).

use alloc::string::ToString as _;
use core::fmt;

use super::value::write_string;
use crate::diag::{Detail, Findings, Location};

/// Writes `findings` as a JSON array of objects:
///
/// ```json
/// {"code": "MZT-S003", "severity": "error", "layer": "syntax",
///  "location": {"line": 3, "column": 5, "cell": 2},
///  "summary": "invalid number", "detail": "...", "spec": "https://...",
///  "jmztab": [1018]}
/// ```
///
/// `location` is `{"line", "column", "cell"}` (cell may be `null`),
/// `{"pointer": "/a/0"}` (JSON input), `{"path": "sml[1][2]"}` (a model
/// built in code), or `null` (the whole document). `detail` is `null` when
/// the summary says it all.
pub(super) fn write(findings: &Findings, out: &mut dyn fmt::Write) -> fmt::Result {
    out.write_char('[')?;
    for (n, finding) in findings.iter().enumerate() {
        if n > 0 {
            out.write_char(',')?;
        }
        let code = finding.code;
        out.write_str("{\"code\":")?;
        write_string(out, code.id())?;
        write!(
            out,
            ",\"severity\":\"{}\",\"layer\":\"{}\",\"location\":",
            finding.severity,
            code.layer()
        )?;
        match &finding.location {
            Location::Text { line, column, cell } => {
                write!(out, "{{\"line\":{line},\"column\":{column},\"cell\":")?;
                match cell {
                    Some(cell) => write!(out, "{cell}}}")?,
                    None => out.write_str("null}")?,
                }
            }
            Location::Json(_) => {
                out.write_str("{\"pointer\":")?;
                write_string(out, &finding.location.to_string())?;
                out.write_char('}')?;
            }
            Location::Model(_) => {
                out.write_str("{\"path\":")?;
                write_string(out, &finding.location.to_string())?;
                out.write_char('}')?;
            }
            Location::Document => out.write_str("null")?,
        }
        out.write_str(",\"summary\":")?;
        write_string(out, code.summary())?;
        out.write_str(",\"detail\":")?;
        match &finding.detail {
            Detail::None => out.write_str("null")?,
            detail => write_string(out, &detail.to_string())?,
        }
        out.write_str(",\"spec\":")?;
        write_string(out, &code.spec_ref().to_string())?;
        out.write_str(",\"jmztab\":[")?;
        for (n, j) in code.jmztab().iter().enumerate() {
            if n > 0 {
                out.write_char(',')?;
            }
            write!(out, "{j}")?;
        }
        out.write_str("]}")?;
    }
    out.write_char(']')
}
