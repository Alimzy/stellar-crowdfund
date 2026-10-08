## What and why

<!-- One or two sentences. Link the issue: Closes #N -->

## Checklist

- [ ] `make check` passes locally
- [ ] Tests added or updated (money-affecting changes need a test that fails without the change)
- [ ] Model-based test updated if the lifecycle changed
- [ ] `FEATURE-STATUS.md` / `CHANGELOG.md` updated if behaviour changed
- [ ] No `unwrap()`/`expect()` in contract code; amounts use checked arithmetic
- [ ] Pre-existing problems found along the way are noted here, not fixed in this PR
