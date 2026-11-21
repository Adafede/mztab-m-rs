//! The `mztab-m` binary: the real file system and standard streams for
//! [`mztab_m_cli::run`].

use std::io::{Read as _, Write as _};
use std::process::ExitCode;

struct Std;

impl mztab_m_cli::Io for Std {
    fn read(&mut self, path: &str) -> Result<Vec<u8>, String> {
        if path == "-" {
            let mut bytes = Vec::new();
            return std::io::stdin()
                .read_to_end(&mut bytes)
                .map(|_| bytes)
                .map_err(|e| e.to_string());
        }
        std::fs::read(path).map_err(|e| e.to_string())
    }

    fn write(&mut self, path: &str, text: &str) -> Result<(), String> {
        if path == "-" {
            self.out(text);
            return Ok(());
        }
        std::fs::write(path, text).map_err(|e| e.to_string())
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
