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

Text = Union[str, bytes]

def validate(
    text: Text,
    *,
    vocabularies: Sequence[str] = ...,
    layer: str = "semantic",
    severities: Mapping[str, str] = ...,
    format: Optional[str] = None,
) -> list[Finding]: ...
def convert(text: Text, to: str, *, lossy: bool = False, format: Optional[str] = None) -> str: ...
def canonical(text: Text, *, format: Optional[str] = None) -> str: ...
def inspect(text: Text, *, format: Optional[str] = None) -> dict[str, Union[str, int, None]]: ...
def codes() -> list[CodeInfo]: ...
def vocabulary_size(text: str) -> int: ...
