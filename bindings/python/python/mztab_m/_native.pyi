from typing import Mapping, Optional, Sequence, TypedDict, Union

SPEC_COMMIT: str
SPEC_VERSION: str
__version__: str

class Finding(TypedDict):
    code: str
    severity: str
    layer: str
    line: Optional[int]
    column: Optional[int]
    cell: Optional[int]
    location: str
    summary: str
    detail: Optional[str]
    spec: str

class CodeInfo(TypedDict):
    code: str
    layer: str
    requirement: str
    summary: str
    spec: str

class InspectResult(TypedDict):
    version: Optional[str]
    id: Optional[str]
    title: Optional[str]
    profile: Optional[str]
    tables_profile: Optional[str]
    ms_runs: int
    assays: int
    study_variables: int
    samples: int
    cvs: int
    sml_rows: Optional[int]
    smf_rows: Optional[int]
    sme_rows: Optional[int]
    reader_findings: int
    reader_errors: int

Text = Union[str, bytes, bytearray]

def validate(
    text: Text,
    *,
    vocabularies: Optional[Sequence[str]] = None,
    layer: str = "semantic",
    severities: Optional[Mapping[str, str]] = None,
    format: Optional[str] = None,
) -> list[Finding]: ...
def convert(text: Text, to: str, *, lossy: bool = False, format: Optional[str] = None) -> str: ...
def canonical(text: Text, *, format: Optional[str] = None) -> str: ...
def inspect(text: Text, *, format: Optional[str] = None) -> InspectResult: ...
def codes() -> list[CodeInfo]: ...
def vocabulary_size(text: str) -> int: ...
