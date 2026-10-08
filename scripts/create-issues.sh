#!/usr/bin/env bash
# Creates the starter issue backlog with the gh CLI. Safe to run repeatedly:
# issues whose title already exists (open or closed) are skipped.
#
#   DRY_RUN=1 bash scripts/create-issues.sh   # show what would be created
#   bash scripts/create-issues.sh             # create labels + missing issues
#
# Works from any directory: it moves to the repository root itself.
set -euo pipefail
cd "$(dirname "$0")/.."

DRY_RUN="${DRY_RUN:-0}"
EXISTING=""

if [ "$DRY_RUN" != "1" ]; then
  command -v gh >/dev/null || { echo "gh CLI not found" >&2; exit 1; }
  gh auth status >/dev/null 2>&1 || { echo "run: gh auth login" >&2; exit 1; }

  label() { gh label create "$1" --color "$2" --description "$3" --force >/dev/null; }
  label trivial            "c2e0c6" "A few lines or docs; under an hour"
  label medium             "fbca04" "Self-contained change with tests; a few hours"
  label high               "d93f0b" "Touches contract logic or security; discuss design first"
  label ci                 "0e8a16" "Continuous integration"
  label security           "b60205" "Security-relevant"
  label testing            "1d76db" "Tests and verification"
  label documentation      "0075ca" "Docs"
  label enhancement        "a2eeef" "New feature or request"
  label "good first issue" "7057ff" "Good for newcomers"
  label "help wanted"      "008672" "Extra attention is welcome"

  EXISTING=$(gh issue list --state all --limit 500 --json title -q '.[].title')
fi

CREATED=0
SKIPPED=0

issue() { # issue <title> <labels> <body>
  if [ "$DRY_RUN" = "1" ]; then
    echo "[dry run] $1  [$2]"
  elif printf '%s\n' "$EXISTING" | grep -Fxq -- "$1"; then
    echo "[skip] already exists: $1"
    SKIPPED=$((SKIPPED + 1))
  else
    gh issue create --title "$1" --label "$2" --body "$3"
    EXISTING="$EXISTING"$'\n'"$1"
    CREATED=$((CREATED + 1))
  fi
}

issue "Test restoring an archived campaign and contribution" "medium,testing" \
"Persistent entries are archived once their TTL passes and must be restored before use.

**Do**
- Advance the ledger past \`ENTRY_BUMP\`, show that \`refund\` fails on the archived entries, restore them, and refund successfully.

**Done when** the test documents the restore flow and \`docs/ARCHITECTURE.md\` links to it."

issue "Add an optional minimum contribution per campaign" "medium,enhancement" \
"Dust pledges cost storage and add noise.

**Do**
- Add a \`min_contribution\` field set at creation; reject smaller pledges with a new error code (propose the code in the issue first).
- Update the model-based test, docs, \`FEATURE-STATUS.md\` and \`CHANGELOG.md\`.

**Done when** the model test covers the new rule."

issue "Add an optional hard cap per campaign" "medium,enhancement" \
"Today contributions may exceed the goal without limit.

**Do**
- Add an optional \`cap\`; a pledge that would exceed it is rejected (or reduced; discuss in the issue).
- Update the model, tests and docs.

**Done when** the model test covers the cap and the boundary values."

issue "Add a payout address distinct from the creator" "medium,enhancement" \
"Creators may want funds sent to a treasury or multisig rather than the signing key.

**Do**
- Add a \`payout\` address set at creation, used by \`withdraw\`; the creator still authorizes.
- Include it in \`campaign_created\`, tests and docs.

**Done when** the withdraw test pays the payout address and the event test asserts the new field."

issue "Milestone-based tranche release" "high,enhancement" \
"All-or-nothing release is simple but coarse.

**Do**
- Write a design proposal in the issue first: tranche definition, who approves, how refunds work for unreleased funds.
- Implement behind the same guarantees: the contract holds exactly what is owed, refunds are exact.

**Done when** the model-based test covers tranches and the security model is updated."

issue "Run cargo-mutants in CI and fix survivors" "medium,ci,testing" \
"Hand-made mutations were all caught when this repo was created; automate it.

**Do**
- Add a CI job running \`cargo mutants\` on the contract crate (scheduled or on demand if too slow).
- Add tests for any surviving mutant.

**Done when** the job runs and survivors are zero or each is justified in the PR."

issue "Benchmark the resource footprint of each entrypoint" "medium,testing" \
"Soroban charges for CPU, memory and ledger footprint.

**Do**
- Add tests that record \`env.cost_estimate()\` budgets for each entrypoint and fail when they grow beyond a stated margin.

**Done when** a regression in cost turns CI red."

issue "Reproducible WASM build and release workflow" "medium,ci" \
"Reviewers should be able to match the deployed WASM hash to source.

**Do**
- Add a tag-triggered workflow that builds the WASM, prints its sha256, and attaches both to a GitHub release.
- Document how to reproduce the hash locally.

**Done when** a tagged release carries the artifact and the hash."

issue "Add a TypeScript example and typecheck it in CI" "medium,documentation" \
"**Do**
- Add \`examples/ts/\` using @stellar/stellar-sdk to create a campaign, contribute, read \`status\`, and withdraw or refund on testnet.
- Run \`tsc --noEmit\` on it in CI so it cannot rot.

**Done when** CI typechecks the example and the README links to it."

issue "Add Dependabot and CODEOWNERS" "trivial,ci,good first issue" \
"**Do**
- Add \`.github/dependabot.yml\` for cargo and github-actions (weekly) and a \`CODEOWNERS\` file.

**Done when** Dependabot opens its first update PR."

issue "Write docs/CLI.md with a stellar CLI example for every entrypoint" "trivial,documentation,good first issue" \
"**Do**
- Document one copy-pasteable \`stellar contract invoke\` example per entrypoint, using the values from \`scripts/demo-testnet.sh\`.

**Done when** every entrypoint in the README API table has an example."

issue "Generate the event ABI table from the contract spec" "medium,documentation" \
"The event table in \`docs/ARCHITECTURE.md\` is hand-written and can drift.

**Do**
- Add a test or script that reads the contract spec (or event structs) and fails when the documented table differs.

**Done when** renaming an event field without updating the docs fails CI."

if [ "$DRY_RUN" != "1" ]; then
  echo "Done: $CREATED created, $SKIPPED skipped."
fi
