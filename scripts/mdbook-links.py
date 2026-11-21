#!/usr/bin/env python3
"""mdBook preprocessor that rewrites repo-relative links to mdBook pages or GitHub."""

import json
import os
import posixpath
import re
import sys

REPO_BASE = "https://github.com/Adafede/mztab-m-rs"
REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Map normalized repo-root paths to (book_page, default_fragment).
SITE_PAGES = {
    "STATUS.md": ("introduction.md", "#status"),
    "DECISIONS.md": ("decisions.md", ""),
    "docs/DESIGN.md": ("design.md", ""),
    "docs/BENCHMARKS.md": ("benchmarks.md", ""),
    "docs/findings.md": ("findings.md", ""),
    "CONTRIBUTING.md": ("contributing.md", ""),
    "SECURITY.md": ("security.md", ""),
    "CHANGELOG.md": ("changelog.md", ""),
    "bindings/python/README.md": ("bindings.md", "#python-mztab-m"),
    "bindings/r/README.md": ("bindings.md", "#r-mztabm"),
}

# Directory in the repository where each chapter's included content lives.
CHAPTER_BASE_DIRS = {
    "design.md": "docs",
    "benchmarks.md": "docs",
}

SITE_LOCAL = {
    "introduction.md",
    "install.md",
    "quickstart.md",
    "library.md",
    "library-obo.md",
    "library-api.md",
    "bindings.md",
    "findings.md",
    "decisions.md",
    "design.md",
    "benchmarks.md",
    "contributing.md",
    "security.md",
    "changelog.md",
}

REPO_TOP_FILES = {
    "AGENTS.md",
    "CLAUDE.md",
    "CODE_OF_CONDUCT.md",
    "LICENSE-APACHE",
    "LICENSE-MIT",
    "NOTICE",
    "README.md",
    "CITATION.cff",
}

REPO_DIRS = (
    ".github/",
    "benches/",
    "bindings/",
    "crates/",
    "docs/",
    "fuzz/",
    "scripts/",
    "third_party/",
)

FENCE_RE = re.compile(r"^\s{0,3}(`{3,}|~{3,})")


def rewrite_target(target: str, base_dir: str) -> str:
    if target.startswith(("http://", "https://", "mailto:", "#")):
        return target
    if target.startswith(("demo/", "api/")):
        return target

    path_part, sep, frag = target.partition("#")
    fragment = f"{sep}{frag}" if sep else ""

    if path_part in SITE_LOCAL:
        return target

    is_dir = path_part.endswith("/")
    joined = posixpath.join(base_dir, path_part) if base_dir else path_part
    norm = posixpath.normpath(joined)
    if norm == ".":
        return target

    if norm in SITE_PAGES:
        page, default_frag = SITE_PAGES[norm]
        return f"{page}{fragment or default_frag}"

    if norm in REPO_TOP_FILES or any(
        norm.startswith(prefix) or norm == prefix.rstrip("/") for prefix in REPO_DIRS
    ):
        is_dir = is_dir or os.path.isdir(os.path.join(REPO_ROOT, norm))
        kind = "tree" if is_dir else "blob"
        suffix = "/" if is_dir else ""
        return f"{REPO_BASE}/{kind}/main/{norm}{suffix}{fragment}"

    return target


def skip_inline_code(line: str, pos: int) -> int:
    ticks = 0
    n = len(line)
    while pos + ticks < n and line[pos + ticks] == "`":
        ticks += 1
    end = line.find("`" * ticks, pos + ticks)
    if end == -1:
        return pos + ticks
    return end + ticks


def rewrite_line(line: str, base_dir: str) -> str:
    out = []
    i = 0
    n = len(line)
    while i < n:
        ch = line[i]
        if ch == "`":
            j = skip_inline_code(line, i)
            out.append(line[i:j])
            i = j
            continue
        if ch == "[" and (i == 0 or line[i - 1] != "\\"):
            # Scan forward for matching `]` allowing inline code spans inside the label.
            j = i + 1
            while j < n:
                if line[j] == "`":
                    j = skip_inline_code(line, j)
                    continue
                if line[j] == "\\":
                    j += 2
                    continue
                if line[j] == "]":
                    break
                if line[j] == "[":
                    break
                j += 1
            if j < n and line[j] == "]" and j + 1 < n and line[j + 1] == "(":
                k = j + 2
                while k < n and line[k] not in (")", " ", "\t", "\n"):
                    k += 1
                if k < n and line[k] == ")" and k > j + 2:
                    label = line[i + 1 : j]
                    target = line[j + 2 : k]
                    out.append(f"[{label}]({rewrite_target(target, base_dir)})")
                    i = k + 1
                    continue
        out.append(ch)
        i += 1
    return "".join(out)


def process_markdown(text: str, chapter_path: str) -> str:
    base_dir = CHAPTER_BASE_DIRS.get(chapter_path, "")
    out = []
    fence = None
    for line in text.splitlines(keepends=True):
        m = FENCE_RE.match(line)
        if m:
            marker = m.group(1)
            stripped = line.strip()
            if fence is None:
                fence = marker[0] * len(marker)
            elif len(stripped) >= len(fence) and set(stripped) == {fence[0]}:
                fence = None
            out.append(line)
        elif fence is not None:
            out.append(line)
        else:
            out.append(rewrite_line(line, base_dir))
    return "".join(out)


def walk_items(items: list) -> None:
    for item in items:
        chapter = item.get("Chapter")
        if chapter is not None:
            path = chapter.get("path") or ""
            chapter["content"] = process_markdown(chapter.get("content", ""), path)
            walk_items(chapter.get("sub_items", []))


def main() -> int:
    if len(sys.argv) >= 2 and sys.argv[1] == "supports":
        renderer = sys.argv[2] if len(sys.argv) >= 3 else ""
        return 0 if renderer == "html" else 1

    _context, book = json.load(sys.stdin)
    walk_items(book.get("sections", []) or book.get("items", []))
    json.dump(book, sys.stdout)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
