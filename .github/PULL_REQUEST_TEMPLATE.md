## Summary

<!-- One or two sentences describing what changed and why. Link the issue if there is one. -->

## Checklist

- [ ] `./mk ci` passes locally (or `./mk ci-fast` if mutation testing cannot run locally)
- [ ] `./mk site` passes if docs, findings reference, or the browser demo changed
- [ ] `./mk bindings` passes if Python or R bindings changed
- [ ] Commit messages follow Conventional Commits (`./mk commits`)
- [ ] New or changed behaviour is covered by deterministic tests (`./mk mutants-diff`)
- [ ] Any non-obvious spec reading is recorded in `DECISIONS.md`
- [ ] Lint exceptions use `#[expect(..., reason = "...")]` and are recorded in `docs/DEBT.md` (never `#[allow]`)
- [ ] AI assistance, if used, is disclosed here (which parts and how verified)
