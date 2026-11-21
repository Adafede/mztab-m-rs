# The package's functions: defaults and data frames around the Rust routines.

as_text <- function(text) {
  if (is.raw(text)) text <- rawToChar(text)
  if (!is.character(text) || length(text) != 1L) {
    stop("`text` must be one character string (or a raw vector)")
  }
  text
}

#' Validate an mzTab-M document
#'
#' @param text The document (TSV or JSON) as one string or a raw vector.
#' @param vocabularies OBO texts (such as the contents of `psi-ms.obo`) for
#'   the semantic layer.
#' @param layer The last layer to run: `"syntax"`, `"structure"` or
#'   `"semantic"`.
#' @param severities Named character vector: code to `"error"`,
#'   `"warning"`, `"info"` or `"off"`.
#' @param format `"tsv"`, `"json"`, or `NULL` to detect it.
#' @return A data frame, one row per finding: `code`, `severity`, `layer`,
#'   `line`, `column`, `cell`, `location`, `summary`, `detail`, `spec`.
#' @export
validate <- function(text, vocabularies = character(), layer = "semantic",
                     severities = character(), format = NULL) {
  settings <- if (length(severities)) paste0(names(severities), "=", severities) else character()
  columns <- mztabm_validate(as_text(text), as.character(vocabularies), layer, settings,
                             if (is.null(format)) "" else format)
  as.data.frame(columns, stringsAsFactors = FALSE)
}

#' Convert an mzTab-M document between TSV and JSON
#'
#' @inheritParams validate
#' @param to `"tsv"` or `"json"`.
#' @param lossy Drop what the specification does not define instead of
#'   failing.
#' @return The converted document, as one string.
#' @export
convert <- function(text, to, lossy = FALSE, format = NULL) {
  mztabm_convert(as_text(text), to, lossy, if (is.null(format)) "" else format)
}

#' The canonical form of an mzTab-M document, in its own format
#'
#' @inheritParams validate
#' @return The canonical document, as one string.
#' @export
canonical <- function(text, format = NULL) {
  mztabm_canonical(as_text(text), if (is.null(format)) "" else format)
}

#' Summarise an mzTab-M document
#'
#' @inheritParams validate
#' @return A named list: version, profiles, element counts, rows per table
#'   and the reader's finding counts.
#' @export
inspect <- function(text, format = NULL) {
  mztabm_inspect(as_text(text), if (is.null(format)) "" else format)
}

#' The finding codes
#'
#' @return A data frame: `code`, `layer`, `requirement`, `summary`, `spec`.
#' @export
codes <- function() as.data.frame(mztabm_codes(), stringsAsFactors = FALSE)

#' The pinned mzTab-M specification commit
#'
#' @return The commit hash, as one string.
#' @export
spec_commit <- function() mztabm_spec_commit()
