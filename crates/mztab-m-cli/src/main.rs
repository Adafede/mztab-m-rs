//! The `mztab-m` binary: the real file system and standard streams for
//! [`mztab_m_cli::run`].

use std::io::{Read as _, Write as _};
use std::process::ExitCode;

struct Std;

#[expect(
    clippy::needless_pass_by_value,
    reason = "a function for `map_err`, which passes the error by value"
)]
fn message(error: std::io::Error) -> String {
    error.to_string()
}

impl mztab_m_cli::Io for Std {
    fn read(&mut self, path: &str) -> Result<Vec<u8>, String> {
        if path == "-" {
            let mut bytes = Vec::new();
            return std::io::stdin()
                .read_to_end(&mut bytes)
                .map(|_| bytes)
                .map_err(message);
        }
        std::fs::read(path).map_err(message)
    }

    fn write(&mut self, path: &str, text: &str) -> Result<(), String> {
        if path == "-" {
            self.out(text);
            return Ok(());
        }
        std::fs::write(path, text).map_err(message)
    }

    fn out(&mut self, text: &str) {
        // A closed pipe (`| head`) is not an error worth reporting.
        drop(std::io::stdout().lock().write_all(text.as_bytes()));
    }

    fn err(&mut self, text: &str) {
        drop(std::io::stderr().lock().write_all(text.as_bytes()));
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    ExitCode::from(mztab_m_cli::run(&args, &mut Std))
}
