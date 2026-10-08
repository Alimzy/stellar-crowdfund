use soroban_sdk::{contracttype, Address};

/// One crowdfunding campaign. A contract instance holds many, each isolated.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Campaign {
    /// Account that created the campaign and may cancel or withdraw.
    pub creator: Address,
    /// SEP-41 token contributions are made in.
    pub token: Address,
    /// Amount that must be raised by the deadline for the campaign to succeed.
    pub goal: i128,
    /// Ledger-time seconds. Contributions are accepted while `now < deadline`.
    pub deadline: u64,
    /// Total contributed so far.
    pub raised: i128,
    /// Set once when the creator withdraws a successful campaign.
    pub withdrawn: bool,
    /// Set once when the creator cancels an open campaign.
    pub cancelled: bool,
}

/// Lifecycle state, derived from the campaign and the current ledger time.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Status {
    /// Before the deadline and not cancelled: contributions accepted.
    Open = 0,
    /// Deadline passed with `raised >= goal`; the creator may withdraw.
    Succeeded = 1,
    /// The creator has withdrawn the funds.
    Withdrawn = 2,
    /// Deadline passed with `raised < goal`; contributors may refund.
    Failed = 3,
    /// Cancelled while open; contributors may refund.
    Cancelled = 4,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Number of campaigns ever created; also the next campaign id.
    Count,
    /// A campaign, keyed by id.
    Campaign(u64),
    /// What `Address` has contributed to campaign `u64`.
    Contribution(u64, Address),
}
