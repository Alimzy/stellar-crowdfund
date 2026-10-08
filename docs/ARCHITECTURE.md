# Architecture

## Goals

1. Hold pledges for many campaigns in one contract with no campaign able to affect another.
2. Keep the money rules small enough to verify by reading: one pure function (`rules::status`)
   decides what is allowed, and every entrypoint asks it.
3. Make "contract holds exactly what is owed" a property the tests check after every step.

## Data model

```
Instance storage              Persistent storage
----------------              ------------------
Count -> u64                  Campaign(id)            -> Campaign
                              Contribution(id, addr)  -> i128
```

`Campaign`: `creator`, `token`, `goal`, `deadline`, `raised`, `withdrawn`, `cancelled`.

`Count` is the next campaign id; ids are sequential and never reused. A `Contribution` entry
exists only while an amount is owed to that contributor and is deleted on refund.

## Lifecycle rules (`rules.rs`)

```
status(c, now) =
    Cancelled   if c.cancelled
    Open        if now < c.deadline
    Withdrawn   if c.withdrawn
    Succeeded   if c.raised >= c.goal
    Failed      otherwise
```

| Entrypoint | Allowed in | Otherwise fails with |
|---|---|---|
| `contribute` | Open | `CampaignNotOpen` |
| `cancel` | Open | `CampaignNotOpen` |
| `withdraw` | Succeeded | `CampaignStillOpen`, `AlreadyWithdrawn`, `GoalNotReached`, `CampaignCancelled` |
| `refund` | Failed, Cancelled | `RefundNotAvailable`, then `NothingToRefund` |

Because `cancel` needs `Open` and `withdraw` needs `Succeeded`, `Cancelled` and `Withdrawn` can
never both hold. The model-based test re-implements this table independently and compares it
with the contract after every random operation.

## Flows and ordering

**contribute**: auth contributor, validate amount, require Open, *then* transfer, measure the
balance delta, and only then update state, extend TTLs and emit. The delta can only be measured
after the transfer, so this is the one place where the interaction comes first. A failed call
reverts the whole invocation, including the transfer. Soroban does not allow a contract to be
re-entered during its own invocation, so a malicious token cannot call back in between.

**withdraw**: auth creator, require Succeeded, set `withdrawn`, extend TTLs, transfer `raised`
to the creator, emit.

**refund**: auth contributor, require Failed or Cancelled, read the contribution, delete it,
extend TTLs, transfer it back, emit.

**cancel**: auth creator, require Open, set `cancelled`, emit.

## Fee-on-transfer guard

```
before = token.balance(contract)
token.transfer(contributor, contract, amount)
after  = token.balance(contract)
require after - before == amount     else Error::UnexpectedTransferAmount
```

A token that charges a fee, rebases, or otherwise delivers a different amount makes the call
fail and roll back. Without this check, the contract could record `amount` while holding less,
and the last refund or the withdrawal would fail.

## Storage lifetime (TTL) policy

| Entry | Extended by | Bumped to | When fewer than |
|---|---|---|---|
| Instance | every state-changing entrypoint, `bump`, `bump_contribution` | ~30 days | ~29 days |
| `Campaign(id)` | same | ~90 days | ~89 days |
| `Contribution(id, who)` | `contribute`, `bump_contribution` | ~90 days | ~89 days |

Read-only views never extend TTL. Soroban archives expired persistent entries rather than
deleting them, but restoring one costs a transaction, so the policy avoids needing it. For a
campaign that runs longer than ~90 days, anyone can call `bump` and `bump_contribution`.
Constants live in `storage.rs`; `every_state_changing_entrypoint_extends_all_touched_entries`
pins the behaviour.

## Authorization

| Entrypoint | Required signer |
|---|---|
| `create_campaign`, `withdraw`, `cancel` | the campaign's creator |
| `contribute`, `refund` | the contributor named in the call |
| `bump`, `bump_contribution`, views | none |

There is no admin, owner, upgrade key or pause switch.

## Events

| Event | Topics | Data |
|---|---|---|
| `campaign_created` | `id`, `creator` | `token`, `goal`, `deadline` |
| `contributed` | `id`, `contributor` | `amount`, `raised` (campaign total after) |
| `withdrawn` | `id`, `creator` | `amount` |
| `refunded` | `id`, `contributor` | `amount` |
| `cancelled` | `id`, `creator` | `raised` (total contributors can reclaim) |

Data is a map keyed by field name. `every_event_has_the_exact_documented_topics_and_data`
asserts these shapes, so a rename or reorder breaks a test.

## Design decisions

- **All or nothing.** Partial funding never reaches the creator; this keeps the trust model
  to a single question, "was the goal met by the deadline".
- **Pull-based refunds.** Contributors refund themselves; no loop over contributors, so cost
  does not grow with the number of backers.
- **Delete on refund.** Removing the record is both the double-refund guard and storage cleanup.
- **Pure rules module.** The one table that matters is testable without a ledger.
- **Per-campaign token.** A misbehaving token can only affect campaigns that use it.
