# Feature status

"Verified" means a test or a command output demonstrates it. Anything not verified is listed as such.

| Feature | Implemented | Verified by |
|---|---|---|
| Create campaign and validation | Yes | `create_records_campaign_and_ids_are_sequential`, `invalid_creation_parameters_are_rejected` |
| Contributions accumulate and move funds | Yes | `contributions_accumulate_per_contributor_and_move_funds` |
| Deadline boundary (closed at `now == deadline`) | Yes | `contributing_stops_exactly_at_the_deadline`, `rules::tests::*` |
| Success: creator withdraws once after the deadline | Yes | `successful_campaign_pays_the_creator_once_after_the_deadline` |
| Failure: exact refunds, once | Yes | `failed_campaign_refunds_exact_amounts_and_only_once` |
| Cancellation opens refunds, blocks withdraw | Yes | `cancelling_opens_refunds_immediately`, `cannot_cancel_after_the_deadline` |
| Over-funding allowed and fully withdrawn | Yes | `contributions_may_exceed_the_goal` |
| Campaign and token isolation | Yes | `campaigns_and_tokens_are_isolated` |
| Correct signer per entrypoint | Yes | `each_entrypoint_requires_exactly_the_right_signer` |
| TTL extension on every touched entry | Yes | `every_state_changing_entrypoint_extends_all_touched_entries` |
| Exact event topics and data | Yes | `every_event_has_the_exact_documented_topics_and_data`, `failed_calls_emit_no_events` |
| Fee-on-transfer token rejected, nothing changes | Yes | `fee_charging_token_is_rejected_and_nothing_changes` |
| Reverting token leaves no partial state | Yes | `a_reverting_token_leaves_no_partial_state_in_any_entrypoint` |
| Contract holds exactly what is owed, under random operations | Yes | `contract_matches_the_model_under_random_operations` (model-based) |
| Test suite catches regressions | Checked once by hand | Six deliberate bugs (fee guard removed, deadline off by one, goal `>` vs `>=`, refund keeps record, withdraw skips flag, TTL extension skipped) each failed the suite. Not automated yet (open issue: cargo-mutants in CI) |
| `cargo fmt`, `clippy -D warnings`, `cargo doc -D warnings` | Yes | Run locally on Rust 1.85 |
| Declared MSRV 1.84 builds code and tests | Yes | `cargo check --locked --all-targets` on rustc 1.84.1; `msrv` CI job repeats it |
| Optimized WASM build and size | Yes | `stellar contract build` gave 17,081 bytes (hash `8321b009...`); the `build-wasm` CI job repeats it against a 21,000-byte budget |
| Testnet deployment and demo | Script ready | **Not run yet.** Run `scripts/demo-testnet.sh`; it fills the README proof table |
| Dependency bans, sources and license allow-list | Yes | `cargo deny check bans sources licenses` passes locally (cargo-deny 0.16.4); the `deny` CI job repeats it |
| Dependency security advisories | Not verified | CI job is informational; the advisory database could not be fetched locally |
| Restore of archived entries | Not tested | Open issue |
| Third-party audit | No | Not audited |

## Known limits

- The creator can cancel until the deadline, even after the goal is met.
- No milestones, deadline extension, minimum contribution or hard cap yet (open issues).
- Token issuer powers (freeze, clawback) can block `withdraw` or `refund` for that token.
