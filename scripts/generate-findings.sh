#!/usr/bin/env bash
# Regenerate docs/findings.md from mztab_m::Code::ALL.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"
BLESS=1 cargo test -p mztab-m --test findings_doc --locked
