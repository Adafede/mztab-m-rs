# Stage the workspace crates `mztab-m`, `mztab-m-obo`, and `mztab-m-api` inside
# `src/rust/crates/` when building a self-contained R source tarball (e.g. via
# `devtools::install()`, `pkgbuild::build()`, or `pak`). Direct checkouts
# compile `../../../../crates/` directly via `bindings/r/src/Makevars` and do
# not need this directory committed.

TOML_HEADER_RE <- "^\\s*(\\[\\[?)([^\\]]+)(\\]\\]?)\\s*(#.*)?$"

parse_toml_header <- function(line) {
  if (!grepl(TOML_HEADER_RE, line, perl = TRUE)) {
    return(NULL)
  }
  list(
    is_array = sub(TOML_HEADER_RE, "\\1", line, perl = TRUE) == "[[",
    section = trimws(sub(TOML_HEADER_RE, "\\2", line, perl = TRUE)),
    trailing = sub(TOML_HEADER_RE, "\\4", line, perl = TRUE)
  )
}

append_section_header <- function(out, header_line) {
  if (length(out) > 0L) {
    out <- c(out, "")
  }
  c(out, header_line)
}

append_body_line <- function(state, line) {
  if (grepl("^\\s*($|#)", line, perl = TRUE)) {
    state$pending <- c(
      state$pending,
      if (grepl("^\\s*$", line, perl = TRUE)) "" else line
    )
  } else {
    if (length(state$pending) > 0L) {
      state$out <- c(state$out, state$pending)
      state$pending <- character()
    }
    state$out <- c(state$out, line)
  }
  state
}

parse_workspace_package <- function(root_lines) {
  in_ws_pkg <- FALSE
  vals <- list()

  for (line in root_lines) {
    hdr <- parse_toml_header(line)
    if (!is.null(hdr)) {
      in_ws_pkg <- !hdr$is_array && hdr$section == "workspace.package"
      next
    }
    if (!in_ws_pkg || grepl("^\\s*($|#)", line, perl = TRUE)) {
      next
    }
    if (grepl("^\\s*([A-Za-z0-9_-]+)\\s*=\\s*(.+)$", line, perl = TRUE)) {
      key <- sub("^\\s*([A-Za-z0-9_-]+)\\s*=\\s*.*$", "\\1", line, perl = TRUE)
      val <- trimws(sub("^\\s*[A-Za-z0-9_-]+\\s*=\\s*(.+)$", "\\1", line, perl = TRUE))
      vals[[key]] <- val
    }
  }

  vals
}

extract_workspace_lints <- function(root_lines) {
  state <- list(out = character(), pending = character())
  keep <- FALSE

  for (line in root_lines) {
    hdr <- parse_toml_header(line)
    if (!is.null(hdr)) {
      state$pending <- character()
      if (!hdr$is_array && startsWith(hdr$section, "workspace.lints.")) {
        promoted <- sub("^workspace\\.", "", hdr$section, perl = TRUE)
        keep <- TRUE
        header_line <- paste0("[", promoted, "]", if (nzchar(hdr$trailing)) paste0(" ", hdr$trailing) else "")
        state$out <- append_section_header(state$out, header_line)
      } else {
        keep <- FALSE
      }
    } else if (keep) {
      state <- append_body_line(state, line)
    }
  }

  state$out
}

build_standalone_manifest <- function(crate_toml, ws_pkg, lint_lines) {
  lines <- readLines(crate_toml, warn = FALSE)
  ws_field_re <- "^\\s*([A-Za-z0-9_-]+)\\.workspace\\s*=\\s*true\\s*(#.*)?$"

  state <- list(out = character(), pending = character())
  current_section <- ""
  keep_section <- FALSE
  skip_include <- FALSE

  for (line in lines) {
    hdr <- parse_toml_header(line)
    if (!is.null(hdr)) {
      current_section <- hdr$section
      skip_include <- FALSE
      state$pending <- character()

      is_target_non_dev <- !hdr$is_array &&
        grepl("^target\\.", hdr$section, perl = TRUE) &&
        !grepl("\\.dev-dependencies(\\.|$)", hdr$section, perl = TRUE)
      is_kept_table <- grepl(
        "^(package$|lib(\\.|$)|features(\\.|$)|(build-)?dependencies(\\.|$))",
        hdr$section,
        perl = TRUE
      )
      keep_section <- (!hdr$is_array || hdr$section == "lib") &&
        (is_kept_table || is_target_non_dev)

      if (keep_section) {
        state$out <- append_section_header(state$out, line)
      }
    } else if (keep_section) {
      if (skip_include) {
        if (grepl("\\]\\s*(#.*)?$", line, perl = TRUE)) {
          skip_include <- FALSE
        }
        next
      }
      if (current_section == "package") {
        if (grepl("^\\s*include\\s*=\\s*\\[", line, perl = TRUE)) {
          state$pending <- character()
          if (!grepl("\\]\\s*(#.*)?$", line, perl = TRUE)) {
            skip_include <- TRUE
          }
          next
        }
        if (grepl(ws_field_re, line, perl = TRUE)) {
          key <- sub(ws_field_re, "\\1", line, perl = TRUE)
          val <- ws_pkg[[key]]
          if (is.null(val) || is.na(val) || !nzchar(val)) {
            stop("Unresolved [workspace.package] key '", key, "' in: ", crate_toml, call. = FALSE)
          }
          line <- paste0(key, " = ", val)
        }
      }
      state <- append_body_line(state, line)
    }
  }

  out <- state$out
  if (length(lint_lines) > 0L) {
    if (length(out) > 0L) {
      out <- c(out, "")
    }
    out <- c(out, lint_lines)
  }

  while (length(out) > 0L && !nzchar(trimws(out[length(out)]))) {
    out <- out[-length(out)]
  }

  if (!any(out == "[package]")) {
    stop("Failed to extract [package] from: ", crate_toml, call. = FALSE)
  }
  if (any(grepl("\\.workspace\\s*=\\s*true", out, perl = TRUE))) {
    stop("Unresolved .workspace = true reference remained in: ", crate_toml, call. = FALSE)
  }
  if (any(grepl("^\\s*\\[\\[?workspace(\\b|\\.|\\])", out, perl = TRUE))) {
    stop("Unexpected [workspace] table in staged manifest for: ", crate_toml, call. = FALSE)
  }

  out
}

write_lf_lines <- function(lines, path) {
  con <- file(path, open = "wb")
  on.exit(close(con), add = TRUE)
  writeLines(lines, con, sep = "\n")
}

copy_crate <- function(name, repo_root, stage_root, ws_pkg, lint_lines) {
  crate_root <- file.path(repo_root, "crates", name)
  crate_toml <- file.path(crate_root, "Cargo.toml")
  if (!file.exists(crate_toml)) {
    stop("Missing workspace crate manifest: ", crate_toml, call. = FALSE)
  }

  readme_src <- file.path(crate_root, "README.md")
  if (!file.exists(readme_src)) {
    stop("Missing crate README.md required by include_str!: ", readme_src, call. = FALSE)
  }

  manifest_lines <- build_standalone_manifest(crate_toml, ws_pkg, lint_lines)

  dst_crate <- file.path(stage_root, name)
  dst_src <- file.path(dst_crate, "src")
  if (!dir.create(dst_src, recursive = TRUE, showWarnings = FALSE) &&
      !dir.exists(dst_src)) {
    stop("Failed to create staged source directory: ", dst_src, call. = FALSE)
  }

  write_lf_lines(manifest_lines, file.path(dst_crate, "Cargo.toml"))

  if (!file.copy(readme_src, file.path(dst_crate, "README.md"), overwrite = TRUE, copy.date = TRUE)) {
    stop("Failed to copy ", readme_src, call. = FALSE)
  }

  src_root <- file.path(crate_root, "src")
  rel_files <- list.files(src_root, recursive = TRUE, all.files = FALSE, no.. = TRUE)
  for (rel in rel_files) {
    src_file <- file.path(src_root, rel)
    dst_file <- file.path(dst_src, rel)
    dst_parent <- dirname(dst_file)
    if (!dir.exists(dst_parent)) {
      dir.create(dst_parent, recursive = TRUE, showWarnings = FALSE)
    }
    if (!file.copy(src_file, dst_file, overwrite = TRUE, copy.date = TRUE)) {
      stop("Failed to copy ", src_file, " to ", dst_file, call. = FALSE)
    }
  }

  build_rs <- file.path(crate_root, "build.rs")
  if (file.exists(build_rs)) {
    dst_build_rs <- file.path(dst_crate, "build.rs")
    if (!file.copy(build_rs, dst_build_rs, overwrite = TRUE, copy.date = TRUE)) {
      stop("Failed to copy ", build_rs, " to ", dst_build_rs, call. = FALSE)
    }
  }
}

main <- function() {
  if (file.exists("DESCRIPTION") &&
      file.exists("../../Cargo.toml") &&
      dir.exists("../../crates/mztab-m/src")) {
    pkg_dir <- "."
    repo_root <- "../.."
  } else if (file.exists("bindings/r/DESCRIPTION") &&
             file.exists("Cargo.toml") &&
             dir.exists("crates/mztab-m/src")) {
    pkg_dir <- "bindings/r"
    repo_root <- "."
  } else if (file.exists("src/rust/crates/mztab-m/Cargo.toml")) {
    return(invisible())
  } else {
    stop(
      "Cannot locate mztab-m-rs repository root (`Cargo.toml` and `crates/mztab-m/src/`). ",
      "Run `bootstrap.R` from `bindings/r/` or the repository root.",
      call. = FALSE
    )
  }

  root_toml <- file.path(repo_root, "Cargo.toml")
  root_lines <- readLines(root_toml, warn = FALSE)
  ws_pkg <- parse_workspace_package(root_lines)
  lint_lines <- extract_workspace_lints(root_lines)

  stage_root <- file.path(pkg_dir, "src", "rust", "crates")
  if (file.exists(stage_root) || dir.exists(stage_root)) {
    unlink(stage_root, recursive = TRUE, force = TRUE)
  }

  for (name in c("mztab-m", "mztab-m-obo", "mztab-m-api")) {
    copy_crate(name, repo_root, stage_root, ws_pkg, lint_lines)
  }

  invisible()
}

main()
