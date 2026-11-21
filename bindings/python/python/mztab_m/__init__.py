"""Read, validate, convert and inspect mzTab-M metabolomics files.

A thin layer over the Rust `mztab-m` library: the same reader, validator
and writer as the `mztab-m` command, in process.

>>> import mztab_m
>>> findings = mztab_m.validate("MTD\\tmzTab-version\\t2.1\\n", layer="syntax")
>>> findings[0]["code"]
'MZT-S006'
"""

from ._native import (
    SPEC_COMMIT,
    SPEC_VERSION,
    __version__,
    canonical,
    codes,
    convert,
    inspect,
    validate,
    vocabulary_size,
)

__all__ = [
    "SPEC_COMMIT",
    "SPEC_VERSION",
    "__version__",
    "canonical",
    "codes",
    "convert",
    "inspect",
    "validate",
    "vocabulary_size",
]
