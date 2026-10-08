# Contributing

Thanks for helping. Small, focused pull requests are easiest to review.

## Setup

```bash
git clone https://github.com/Alimzy/stellar-crowdfund && cd stellar-crowdfund
make check
```

`make check` runs `cargo fmt --check`, `cargo clippy -D warnings`, all tests and a warning-free
`cargo doc`. CI runs the same plus an MSRV check and the WASM build, so a green `make check`
is the bar for a pull request.

## Workflow

1. Pick an issue and comment so work is not duplicated.
2. Branch from `main`: `git switch -c feat/short-name`.
3. Make the change with tests. Money-affecting changes need a test that fails without them.
4. Update `FEATURE-STATUS.md` and `CHANGELOG.md` when behaviour changes.
5. Open a PR using the template and link the issue (`Closes #N`).

## Issue complexity labels

| Label | Meaning |
|---|---|
| `trivial` | A few lines or docs; under an hour |
| `medium` | A self-contained change with tests; a few hours |
| `high` | Touches contract logic or security, or needs design discussion first |

## Code standards

- No `unsafe`, no `unwrap()` or `expect()` in contract code; return an `Error`.
- Arithmetic on amounts uses checked operations.
- Every state-changing entrypoint extends the TTL of every entry it touches.
- Lifecycle decisions go through `rules::status`; do not re-derive them in entrypoints.
- Error codes are ABI: never renumber, only append.
- New behaviour needs a model update in the model-based test if it changes the lifecycle.

## WASM size budget

`MAX_WASM_BYTES` in the `Makefile` and CI is **21,000 bytes**: the first optimized build was 17,081 bytes, CI builds with plain cargo at about 19,194 bytes before the Stellar CLI optimization pass, so the headroom is about 9%. CI fails when the build exceeds it.

## Commit style

Short imperative subject, for example `Add minimum contribution check`.
