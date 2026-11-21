> Provenance: review of DECISIONS.md written by an AI coding agent on
> 2026-11-21, for the maintainer. Spec quotes were checked against the
> pinned submodule (`e66c19e`). Nothing in DECISIONS.md was changed.

# Review of the spec decisions

DECISIONS.md has 9 OPEN entries (D-002, D-010–D-015, D-020, D-030) plus 3
OPEN rows in the jmzTab-m table (J-09, J-11, J-14), and 37 PROPOSED. The
OPEN ones are already implemented on their proposals (D-065), so deciding
them mostly means confirming or changing a severity in `validate/`.

## The main finding: D-002 says prose wins, but several prose MUSTs are warnings

D-002 sets the precedence: spec prose (RFC 2119), then the field reference,
then the JSON schema. Four proposals then turn a prose MUST into a warning,
mostly because the spec's own examples would fail:

| Decision | Prose (checked) | Implemented as |
|---|---|---|
| D-013 | "At least one `study_variable_group` MUST be defined." (format spec, line 271) | absence = warning (`T021`) |
| D-015 | "The SML element reliability MUST be assigned a value" (line 336) | `null` = warning (`T029`) |
| D-020 | "The fields in the metadata section MUST be reported in the order listed below." (field reference, line 34) | wrong order = warning |
| D-010 | "The order of columns MUST follow the order specified below." (field reference, lines 1284, 1617, 1829) | wrong order = warning (`T025`) |

Each softening is defensible, but together they contradict D-002 as
written. Recommendation: **amend D-002** to say "prose wins; a MUST that the
spec's own published examples break is a warning, and is reported
upstream", and list these four under it. The alternative, making them
errors, turns most of the 2.1 example files red. Either way, a `--strict`
preset (or a documented `--severity` set) that raises them to errors would
serve people preparing submissions.

Note for D-013: the "exactly one group per study variable" sentence is in
the field reference (line 933), not the format spec as the entry says.

## OPEN entries

| ID | Proposal (as implemented) | Assessment | Recommendation |
|---|---|---|---|
| D-002 | Prose > field reference > JSON schema | Sound order; see the main finding | Decide, with the amendment above |
| D-010 | Missing stable column = error; order and misplaced `opt_` = warning | The "all columns MANDATORY" sentence (lines 1275, 1608, 1820) is unambiguous; per-column `Mandatory: False` looks like a generator default | Decide as implemented; report the per-column flags upstream |
| D-011 | SMF `charge` nullable | Three of four sources (field description, JSON schema, examples README) say nullable; only the change log says otherwise | Decide as implemented |
| D-012 | Null `best_id_confidence_value` = information | Matches the JSON schema's `recommended`; corpus uses null widely | Decide as implemented |
| D-013 | No group in 2.1 = warning; with groups, each SV in exactly one = error | See main finding | Decide as warning under amended D-002 |
| D-014 | 2.1 mandatory set as listed in D-065; `software` required; `publication` optional | Reasonable; the `publication` = mandatory flag in the field reference is very likely a generator slip | Decide; file the upstream report |
| D-015 | Null reliability = warning; value outside 1–4 is an error only without a declared scheme | See main finding. The second half is good | Decide under amended D-002 |
| D-020 | Metadata order = warning | See main finding; also, jmzTab-m behaviour is still unmeasured (oracle) | Decide as warning |
| D-030 | Oracle in CI only, outputs committed | The harness exists but has never run (no remote yet). A JDK 25 is installed locally, so a one-off local run is also possible | Decide; run it once the repository is public |
| J-09 | A header line means the table is present | Follows the spec sentence at line 471 | Decide as implemented |
| J-11 | Profile-conditional fields follow the spec, not jmzTab-m | Consistent with D-014 | Decide |
| J-14 | See D-010 | — | Decide with D-010 |

## PROPOSED entries worth a closer look

- **D-022 (`M+S+F`).** The spec permits `M+S+F` ("NOT RECOMMENDED"), but
  declaring it is an error because the `mzTab-profile` enum omits it. A
  file that does what the prose permits gets an error. Suggest a warning,
  and an upstream report about the enum.
- **D-068 / D-077 (CV-mapping corrections) are not applied evenly.**
  `ms_level_must` was corrected to allow its root term because the field
  reference's example uses it. `best_id_confidence_measure_must` was left
  alone, so the root `MS:1002888` "small molecule confidence measure" is
  rejected — and the spec's own 2.1 example
  (`example_study_variable_group.mztab`) and the masster file both use it
  that way. Either allow the root there too (consistent with `ms_level`),
  or keep it and say why the two cases differ.
- **D-051 (row id 0).** "Whether `0` is legal is left to the validator
  (pending)" — still pending. JSON schema indices start at 1 elsewhere.
  Decide (suggest a warning) or remove the "pending".
- **D-041 is out of date.** It says `AGENTS.md`/`CLAUDE.md` are not adopted
  and CITATION is deferred to M9. AGENTS.md and CLAUDE.md now exist, and
  CITATION is still not done. Update the entry.
- **D-075 (units "deliberately not checked").** True for parameter values,
  but `colunit-*` lines carry unit terms (`[UO, UO:0000010, second, ]`)
  that could be checked as UO descendants once UO is loaded. Worth a
  sentence, or an issue.
- **D-021 / J-02 (scientific notation is an error).** Correct by the spec
  ("explicitly not supported", line 296). Expect this to be the most
  common complaint from users of tools that print `1e-5`; the decision
  should stay, but the finding's message could say how to fix it.

## The rest

The remaining PROPOSED entries (D-003, D-016–D-019, D-040, D-050,
D-052–D-064, D-066, D-067, D-069–D-074, D-076, J-01–J-06, J-10, J-12,
J-13, J-15–J-18) are either "as built" records of implementation design or
straightforward readings of the spec. I found nothing in them that
contradicts the pinned spec text I checked. They can be marked `DECIDED`
in bulk, keeping the as-built ones as documentation.

## Upstream reports these decisions imply

Several entries say "report upstream"; none has been filed. Collected:
the JSON schema regex defects (D-016), the per-column `Mandatory: False`
flags (D-010), `publication` marked mandatory (D-014),
`study_variable_group[n]-datatype` typed as Parameter (D-057), the
`M+S+F` enum gap (D-022), the mapping-file root terms and `scopePath`
slip (D-068), the `peak area` example accession `MS:1001113` (D-077).
