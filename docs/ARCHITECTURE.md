> Provenance: authored by an AI coding agent during the structural refactor.

# Architecture

Target structural architecture for `mztab-m-rs`. Every dependency arrow is strictly one-way (acyclic).

## 1. Workspace Layering Rule

```text
Layer 3: Delivery Adapters (I/O, CLI, JS/Python/R FFI)
  mztab-m-cli   mztab-m-wasm   bindings/python   bindings/r
       \             |                |              /
        v            v                v             v
Layer 2: Application Facade (no_std + alloc, text/bytes in -> text/records out)
                       mztab-m-api
                      /           \
                     v             v
Layer 1: Vocabulary Adapter        |
       mztab-m-obo ----------------+
                     \             |
                      v            v
Layer 0: Domain Core (no_std + alloc, zero external dependencies, zero I/O)
                         mztab-m
```

- **Strict one-way rule:** Layer 3 adapters depend **only** on `mztab-m-api` (plus their respective FFI/runtime crates: `wasm-bindgen`, `pyo3`, `extendr-api`, or `std` in `mztab-m-cli`'s `main.rs`). They never import `mztab-m` or `mztab-m-obo` directly.
- **Core purity invariant:** `crates/mztab-m` remains `#![no_std]` + `alloc` with zero runtime dependencies, zero I/O, zero `unsafe`, and zero panics on arbitrary input.

## 2. Crate Responsibilities

| Crate | One-Line Responsibility | Public Surface Policy |
|---|---|---|
| `mztab-m` | Pure mzTab-M domain model, lossless TSV/JSON codecs, and three-layer validator | Curated re-exports at crate root + `cv`, `validate`, `json` submodules; internal helpers `pub(crate)` |
| `mztab-m-obo` | Zero-copy OBO 1.4 parser implementing `mztab_m::cv::TermLookup` | `Ontology` struct + `TermLookup` impl |
| `mztab-m-api` | Stateless application use-cases (`read`, `validate`, `write`, `convert`, `facts`, `codes`) | Stable high-level types (`Format`, `Layer`, `Settings`, `Record`, `CodeRecord`, `Fact`, `Error`) + re-exports needed by adapters |
| `mztab-m-cli` | CLI command parsing, `Io` abstraction, and `std` process entrypoint | `run`, `Io` trait, exit-code constants (`OK`, `FINDINGS`, `USAGE`, `FAILED`) |
| `mztab-m-wasm` | `wasm-bindgen` entrypoints for the browser validation/conversion demo | `validate`, `convert` |
| `bindings/python` | PyO3 `_native` module exposing `mztab-m-api` to Python | `validate`, `convert`, `canonical`, `inspect`, `codes`, `vocabulary_size` |
| `bindings/r` | extendr native bindings exposing `mztab-m-api` to R | `mztabm_*` R wrappers |

## 3. Internal DAG of `mztab-m`

Modules inside `crates/mztab-m/src/` are organized into four strict acyclic tiers:

1. **Tier 1 — Primitives** (depend only on `core` and `alloc`):
   - `id`: 1-based non-zero typed element IDs, `Ix`, `parse_index`, and sorted `IndexMap`.
   - `value` (`cell`, `text`, `decimal`): `Cell<T>`, `Raw`, `Text`, `Decimal`, and lexical number/text errors.
   - `param` (`text`, `parse`): `Param`, `ParamText`, `Accession`, and located parameter parser.
   - `vocab` (`version`, `profile`, `section`, `field`): Closed vocabularies (`Version`, `RuleSet`, `Profile`, `Section`, internal `TableSection`, `AmbiguityCode`, `Datatype`, `PublicationItem`).
2. **Tier 2 — Domain Model & Diagnostics** (depend on Tier 1 only):
   - `meta` (`subkey`, `key`, `value`, `insert`, `entries`): Metadata key grammar (`MtdKey`, `*Key`), `MtdValue`, and insertion/iteration on `Metadata`. `meta::key` depends only on Tier 1; `meta::insert` and `meta::entries` depend on `model`.
   - `model` (`document`, `layout`, `metadata`, `colunit`, `table`): `MzTab`, `SourceLayout`, `Metadata` structs, `ColUnit`, `SpectraRef`, and table/row/column definitions.
   - `diag` (`severity`, `code`, `location`, `finding`): `Severity`, `Requirement`, `Layer`, `SpecRef`, stable `Code` registry, `Path`/`Location`, and `Finding`/`Findings`.
   - `cv` (`rule`, `rules`): `Term`, `TermLookup`, `NoTerms`, and generated CV-mapping `Rule`s.
3. **Tier 3 — Serialization & Event Streaming** (depend on Tiers 1–2):
   - `tsv` (`splitter`, `typing`, `mtd`, `table`): Shared list `Splitter`, raw cell typing, metadata value typing, and table header/row slot typing (zero dependency on `write`).
   - `write` (`error`, `sink`, `render`, `line`, `layout`): `WriteError`, fingerprinting/checking `Out`, `Render`/`RenderRow`, canonical & layout-preserving TSV emission with precomputed table slots.
   - `read` (`event`, `metadata`, `typer`, `builder`, `stream`): `Event`, `Parsed`, whole-input reader, and push-based `StreamReader`.
   - `owned` (`model`): `IntoOwned` trait detaching borrowed models and events to `'static`.
   - `json` (`schema`, `schema_tables`, `value`, `lexer`, `read/*`, `write/*`, `findings`): Feature-gated (`json`) RFC 8259 parser/writer, streaming JSON emitter, and findings JSON serializer.
4. **Tier 4 — Validation Rules** (depend on Tiers 1–3):
   - `validate` (`report`, `patterns`, `metadata/*`, `tables/*`, `semantic/*`): `Config`, `Report` locator, shared format predicates (`is_orcid`, `is_uri`, `is_email`, `is_adduct`, `native_id_ok`, `xsd_matches`), and independent syntax/structure/semantic layers.

## 4. Type-Driven Design & Error Strategy

- **Make invalid states unrepresentable:**
  - `TableSection` (`Sml`, `Smf`, `Sme`) replaces `Section` in all table-only contexts, removing silent `Section::Mtd | Section::Sml` fallbacks.
  - Typed `Command` enum in `mztab-m-cli` (`Help`, `Version`, `Validate`, `Fmt`, `Convert`, `Inspect`) replaces the stringly-typed `Options` god-struct.
  - Standard `core::str::FromStr` implementations on `Version`, `Profile`, `AmbiguityCode`, `Datatype`, `MtdKey`, `Severity`, `Layer`, `Code`, `mztab_m_api::Format`, and `mztab_m_api::Layer` alongside existing `parse` convenience methods for backwards compatibility.
- **Error strategy:**
  - `mztab-m` and `mztab-m-api` remain zero-dependency `no_std`, so typed errors implement `core::fmt::Display` and `core::error::Error` directly rather than pulling in `thiserror` (which would violate the zero-dependency rule for `mztab-m` and add proc-macro compile overhead to `mztab-m-api`).
  - Readers never abort on malformed input: syntax and schema violations become `Finding`s, while unrecoverable output constraints return `WriteError`.
  - `mztab-m-cli` converts `mztab_m_api::Error` and `Io` failures into typed `Failure::Usage` (exit `2`) or `Failure::Failed` (exit `3`).

## 5. Concurrency & Async Strategy

- All crates are strictly synchronous and single-threaded: parsing, validation, and formatting are CPU-bound pure transformations over borrowed memory (`&str` / `&[u8]`) with zero I/O, zero clocks, and zero global or interior-mutable state.
- Memory-bounded processing for large files is provided via the push-based callback `StreamReader`, keeping heap usage proportional to a single line rather than buffering an async stream.
- No async runtime (`tokio`, `async-std`) or shared mutable synchronization (`Arc<Mutex<_>>`, `Rc<RefCell<_>>`) is needed or permitted in core.
