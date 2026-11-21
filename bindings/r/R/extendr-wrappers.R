# The registered Rust routines (see src/rust/src/lib.rs). Each returns
# list(value, error); `unwrap()` raises the error as an R error.
#' @useDynLib mztabm, .registration = TRUE
NULL

unwrap <- function(outcome) {
  if (!is.null(outcome$error)) stop(outcome$error, call. = FALSE)
  outcome$value
}

mztabm_validate <- function(text, vocabularies, layer, severities, format) {
  unwrap(.Call(wrap__mztabm_validate, text, vocabularies, layer, severities, format))
}
mztabm_convert <- function(text, to, lossy, format) {
  unwrap(.Call(wrap__mztabm_convert, text, to, lossy, format))
}
mztabm_canonical <- function(text, format) unwrap(.Call(wrap__mztabm_canonical, text, format))
mztabm_inspect <- function(text, format) unwrap(.Call(wrap__mztabm_inspect, text, format))
mztabm_codes <- function() .Call(wrap__mztabm_codes)
mztabm_spec_commit <- function() .Call(wrap__mztabm_spec_commit)
