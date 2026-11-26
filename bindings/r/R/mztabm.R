# The package's functions: defaults and data frames around the Rust routines.

as_text <- function(text) {
  if (is.raw(text)) {
    return(text)
  }
  if (!is.character(text) || length(text) != 1L || is.na(text)) {
    stop("`text` must be one non-NA character string or a raw vector", call. = FALSE)
  }
  enc2utf8(text)
}

check_scalar_string <- function(x, arg) {
  if (!is.character(x) || length(x) != 1L || is.na(x)) {
    stop(sprintf("`%s` must be one non-NA character string", arg), call. = FALSE)
  }
  enc2utf8(x)
}

check_format <- function(format) {
  if (is.null(format)) {
    return(NULL)
  }
  check_scalar_string(format, "format")
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
  if (!is.character(vocabularies) || anyNA(vocabularies)) {
    stop("`vocabularies` must be a character vector without NA", call. = FALSE)
  }
  vocabularies <- enc2utf8(vocabularies)
  layer <- check_scalar_string(layer, "layer")
  if (!is.character(severities) || anyNA(severities)) {
    stop("`severities` must be a named character vector without NA", call. = FALSE)
  }
  if (length(severities)) {
    codes <- names(severities)
    if (is.null(codes) || anyNA(codes) || any(!nzchar(codes))) {
      stop("`severities` must be a named character vector with non-empty, non-NA names", call. = FALSE)
    }
    codes <- enc2utf8(codes)
    levels <- enc2utf8(unname(severities))
  } else {
    codes <- character()
    levels <- character()
  }
  columns <- mztabm_validate(as_text(text), vocabularies, layer, codes, levels, check_format(format))
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
  to <- check_scalar_string(to, "to")
  if (!is.logical(lossy) || length(lossy) != 1L || is.na(lossy)) {
    stop("`lossy` must be TRUE or FALSE", call. = FALSE)
  }
  mztabm_convert(as_text(text), to, lossy, check_format(format))
}

#' The canonical form of an mzTab-M document, in its own format
#'
#' @inheritParams validate
#' @return The canonical document, as one string.
#' @export
canonical <- function(text, format = NULL) {
  mztabm_canonical(as_text(text), check_format(format))
}

#' Summarise an mzTab-M document
#'
#' @inheritParams validate
#' @return A named list: version, profiles, element counts, rows per table
#'   and the reader's finding counts.
#' @export
inspect <- function(text, format = NULL) {
  mztabm_inspect(as_text(text), check_format(format))
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

#' The pinned mzTab-M specification version
#'
#' @return The specification version, as one string.
#' @export
spec_version <- function() mztabm_spec_version()

#' Count terms in an OBO 1.4 vocabulary
#'
#' @param text OBO 1.4 text as one non-NA string.
#' @return The number of unique terms parsed from `text`.
#' @export
vocabulary_size <- function(text) {
  mztabm_vocabulary_size(check_scalar_string(text, "text"))
}
