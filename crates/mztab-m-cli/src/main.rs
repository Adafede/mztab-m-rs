//! The `mztab-m` binary: the real file system and standard streams for
//! [`mztab_m_cli::run`].

use std::io::{ErrorKind, Read as _, Write as _};
use std::process::ExitCode;

struct Std;

fn write_stdout(out: &mut impl std::io::Write, text: &str) -> Result<(), String> {
    match out.write_all(text.as_bytes()) {
        Ok(()) => Ok(()),
        // A closed pipe (`| head`) is expected when downstream exits early.
        Err(e) if e.kind() == ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn decode_args(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Vec<String>, ()> {
    args.into_iter()
        .map(|arg| arg.into_string().map_err(|_| ()))
        .collect()
}

fn run_main(
    args: impl IntoIterator<Item = std::ffi::OsString>,
    io: &mut dyn mztab_m_cli::Io,
) -> ExitCode {
    let Ok(args) = decode_args(args) else {
        io.err("error: command line arguments must be valid UTF-8\n");
        return ExitCode::from(mztab_m_cli::USAGE);
    };
    ExitCode::from(mztab_m_cli::run(&args, io))
}

impl mztab_m_cli::Io for Std {
    fn read(&mut self, path: &str) -> Result<Vec<u8>, String> {
        let result = if path == "-" {
            let mut bytes = Vec::new();
            std::io::stdin().read_to_end(&mut bytes).map(|_| bytes)
        } else {
            std::fs::read(path)
        };
        result.map_err(|e| e.to_string())
    }

    fn write(&mut self, path: &str, text: &str) -> Result<(), String> {
        if path == "-" {
            return write_stdout(&mut std::io::stdout().lock(), text);
        }
        std::fs::write(path, text).map_err(|e| e.to_string())
    }

    fn out(&mut self, text: &str) {
        drop(write_stdout(&mut std::io::stdout().lock(), text));
    }

    fn err(&mut self, text: &str) {
        drop(std::io::stderr().lock().write_all(text.as_bytes()));
    }
}

fn main() -> ExitCode {
    run_main(std::env::args_os().skip(1), &mut Std)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::io::{self, ErrorKind, Write};
    use std::process::ExitCode;

    use super::{decode_args, run_main, write_stdout};

    struct MockWrite(Option<ErrorKind>);

    impl Write for MockWrite {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0
                .map_or(Ok(buf.len()), |kind| Err(io::Error::from(kind)))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct CaptureErr(String);

    impl mztab_m_cli::Io for CaptureErr {
        fn read(&mut self, _: &str) -> Result<Vec<u8>, String> {
            Err("unused".into())
        }

        fn write(&mut self, _: &str, _: &str) -> Result<(), String> {
            Err("unused".into())
        }

        fn out(&mut self, _: &str) {}

        fn err(&mut self, text: &str) {
            self.0.push_str(text);
        }
    }

    #[test]
    fn stdout_ignores_broken_pipe_only() {
        let mut ok_sink = MockWrite(None);
        assert_eq!(write_stdout(&mut ok_sink, "ok"), Ok(()), "ok");
        assert!(ok_sink.flush().is_ok(), "flush ok");
        assert_eq!(
            write_stdout(&mut MockWrite(Some(ErrorKind::BrokenPipe)), "x"),
            Ok(()),
            "broken pipe ignored"
        );
        assert!(
            write_stdout(&mut MockWrite(Some(ErrorKind::PermissionDenied)), "x").is_err(),
            "other errors propagate"
        );
    }

    #[test]
    fn args_utf8_checked() {
        use mztab_m_cli::Io as _;
        let mut capture = CaptureErr(String::new());
        assert!(capture.read("-").is_err(), "unused read");
        assert!(capture.write("-", "").is_err(), "unused write");
        assert_eq!(
            run_main([OsString::from("--version")], &mut capture),
            ExitCode::from(mztab_m_cli::OK),
            "valid run_main"
        );
        assert_eq!(
            decode_args([OsString::from("--version")]),
            Ok(vec!["--version".to_owned()]),
            "valid UTF-8"
        );
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            let mut io = CaptureErr(String::new());
            let bad = OsString::from_vec(vec![0xff]);
            assert_eq!(
                run_main([bad], &mut io),
                ExitCode::from(mztab_m_cli::USAGE),
                "non-UTF-8 returns USAGE"
            );
            assert!(io.0.contains("valid UTF-8"), "{}", io.0);
        }
    }
}
