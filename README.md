# stellar-crowdfund

[![CI](https://github.com/Alimzy/stellar-crowdfund/actions/workflows/ci.yml/badge.svg)](https://github.com/Alimzy/stellar-crowdfund/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

All-or-nothing crowdfunding for **any SEP-41 token** on
[Soroban](https://developers.stellar.org/docs/build/smart-contracts/overview).
A creator sets a goal and a deadline. Contributors pledge until the deadline. If the goal is
reached the creator withdraws everything; if not, or if the creator cancels, every contributor
reclaims exactly what they put in. One deployed contract holds any number of isolated campaigns.

- **All or nothing.** Funds only reach the creator when the goal is met by the deadline.
- **Exact refunds, one time.** Failed and cancelled campaigns refund each contributor their own
  total; the record is deleted on refund, so a second refund is impossible.
- **Fee-on-transfer safe.** Every contribution measures the contract's token balance before and
  after the transfer and rejects anything that did not arrive in full.
- **Many campaigns, many tokens.** Each campaign names its own token; accounting is per campaign.
- **No admin.** No owner, upgrade key or pause switch. Nobody can change the rules after deploy.
- **Storage-lifetime safe.** Every state-changing entrypoint extends instance, campaign and
  contribution TTL; `bump` and `bump_contribution` are permissionless.
- **Typed errors and events** with stable codes and exact payloads (asserted in tests).

> Status: implemented and tested (24 tests incl. a model-based test). See
> [FEATURE-STATUS.md](FEATURE-STATUS.md) for what is and is not verified, and how.

## Lifecycle

```
            contribute (now < deadline)
                 +-----+
                 v     |
  create --> [ Open ] -+--- cancel (creator, while open) --------> [ Cancelled ] --refund--> exact amounts
                 |
        deadline passes
          /            \
 raised >= goal     raised < goal
        |                |
  [ Succeeded ]      [ Failed ] --refund--> exact amounts
        |
  withdraw (creator)
        |
  [ Withdrawn ]
```

## Contract API

| Function | Auth | What it does |
|---|---|---|
| `create_campaign(creator, token, goal, deadline) -> u64` | creator | Starts a campaign; returns its id |
| `contribute(id, contributor, amount) -> i128` | contributor | Pledges tokens before the deadline; returns the new total |
| `withdraw(id) -> i128` | creator | After the deadline, if the goal was met: pays out everything raised |
| `refund(id, contributor) -> i128` | contributor | Reclaims a contribution from a failed or cancelled campaign |
| `cancel(id)` | creator | Abandons an open campaign; refunds open immediately |
| `bump(id)` / `bump_contribution(id, contributor)` | none | Extends storage lifetime |
| `get_campaign(id)`, `status(id)`, `contribution(id, who)`, `campaign_count()` | none | Read-only views |

`deadline` is ledger-time seconds. Contributions are accepted while `now < deadline`; at
`now == deadline` the campaign is already closed.

Errors: `InvalidAmount=1`, `InvalidGoal=2`, `InvalidDeadline=3`, `CampaignNotFound=4`,
`CampaignNotOpen=5`, `CampaignStillOpen=6`, `GoalNotReached=7`, `AlreadyWithdrawn=8`,
`CampaignCancelled=9`, `RefundNotAvailable=10`, `NothingToRefund=11`,
`UnexpectedTransferAmount=12`, `MathOverflow=13`.

Events: `campaign_created`, `contributed`, `withdrawn`, `refunded`, `cancelled`; exact topics
and data are in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Quick start

```bash
git clone https://github.com/Alimzy/stellar-crowdfund && cd stellar-crowdfund
make test      # unit, integration and model-based tests
make check     # fmt, clippy, tests
make build     # optimized WASM (needs the wasm32v1-none target)
```

Requirements: Rust 1.85 (pinned in `rust-toolchain.toml`; the declared MSRV, 1.84, is checked in
CI) and, for deployment, the [Stellar CLI](https://developers.stellar.org/docs/tools/cli).

### Run the demo on testnet

```bash
./scripts/demo-testnet.sh
```

The demo deploys the contract, runs one successful campaign (two contributors, creator
withdraws) and one failed campaign (contributor refunds), then writes `docs/testnet-proof.md`
and fills in the table below automatically. It takes about two minutes because it waits for
real deadlines to pass.

## Testnet proof

<!-- proof:start -->
_Not generated yet. Run `./scripts/demo-testnet.sh`._
<!-- proof:end -->

## Repository layout

```
contracts/crowdfund/src/
  lib.rs       entrypoints
  rules.rs     pure lifecycle rules (no Env)
  storage.rs   typed storage access and TTL policy
  types.rs     Campaign, Status, DataKey
  events.rs    CampaignCreated / Contributed / Withdrawn / Refunded / Cancelled
  error.rs     Error enum (stable codes)
  test.rs      integration, mock-token and model-based tests
docs/          ARCHITECTURE.md, SECURITY-MODEL.md
scripts/       demo-testnet.sh, update-proof.py, create-issues.sh (idempotent)
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md): data model, lifecycle, TTL policy, events
- [Security model](docs/SECURITY-MODEL.md): trust assumptions and known limits
- [Feature status](FEATURE-STATUS.md): what is verified, and how
- [Contributing](CONTRIBUTING.md) and [Security policy](SECURITY.md)

## License

MIT, see [LICENSE](LICENSE).
