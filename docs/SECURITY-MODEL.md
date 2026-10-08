# Security model

This contract has **not** been audited. Treat it as reference-quality code.

## Assets at risk

Tokens held by the contract on behalf of campaigns.

## Trust assumptions

1. **The token honours `transfer` and `balance`.** The balance-delta guard rejects tokens that
   deliver a different amount than requested, but it cannot defend against a token that lies
   about `balance`, or that changes behaviour later (for example an issuer freezing the
   contract's account or clawing back funds). Use well-known tokens such as native XLM through
   its Stellar Asset Contract.
2. **Ledger time is honest within normal bounds.** Deadlines read `ledger().timestamp()`;
   do not rely on second-level precision.
3. **Creators and contributors keep their keys.** There is no recovery path or admin override.

## What the contract guarantees

| Guarantee | Enforced by |
|---|---|
| The creator only receives funds if the goal was met by the deadline | `withdraw` requires `Succeeded` |
| Contributors of a failed or cancelled campaign can reclaim exactly their pledge, once | `refund` requires Failed/Cancelled, deletes the record |
| Funds cannot be both withdrawn and refunded | `Withdrawn` and `Cancelled` are mutually exclusive; model-based test |
| The contract holds exactly what is owed after every operation | model-based test checks the balance after each step |
| Only the right party can act | `require_auth` on stored or supplied addresses; signer test |
| Campaigns cannot affect each other | separate storage keys and tokens; isolation test |
| Non-standard transfers cannot create a shortfall | balance-delta check; fee-token test |
| No partial state on token failure | failing-token test across contribute, withdraw and refund |
| Overflow cannot wrap | `checked_add`; release profile has `overflow-checks = true` |

## Known limitations

- **Creator can cancel until the deadline**, even after the goal is met. Contributors trust the
  creator not to do that; they always get refunds if it happens.
- **No deadline extension, no partial release, no milestones.** Funds are all-or-nothing.
- **Funds after success stay in the contract until the creator withdraws.** There is no
  timeout; a creator who loses their key leaves the pledges locked.
- **Token issuer powers** (freeze, clawback, authorization flags) can block `withdraw` or
  `refund` for that token. This is outside the contract's control.
- **Archival**: untouched entries expire after their TTL and must be restored before use; call
  `bump` and `bump_contribution` for long campaigns.
- **Dust and decimals** are the token's concern; amounts are raw `i128` units.

## Reporting

See [SECURITY.md](../SECURITY.md).
