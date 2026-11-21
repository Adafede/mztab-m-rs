# Security Policy

## Scope

`mztab-m` parses untrusted input. Security issues include:

- a panic, abort, infinite loop or unbounded memory growth on any input, in
  `parse`, `parse_bytes`, `StreamReader`, or the JSON reader;
- output from the writer that a conforming reader would interpret differently
  from the model that was written (injection through tabs, newlines or
  `|`/`,` separators in values);
- flaws in the CLI's file handling, the build scripts (`mk`, `mk.local`), or
  the CI workflows in [`.github/workflows/`](./.github/workflows/).

## Supported versions

Nothing has been released yet. Fixes go to `main` only.

## Reporting a Vulnerability

Please do not report security vulnerabilities in public GitHub issues.

1. Open a private report through
   [GitHub Security Advisories](https://github.com/Adafede/mztab-m-rs/security/advisories/new).
2. Include a minimal input that reproduces the problem, the crate version or
   commit, and what you expected to happen versus what happened.

If private advisories are not available, write to
TODO(human): security contact address.
