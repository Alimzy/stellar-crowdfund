use soroban_sdk::contracterror;

/// Every failure mode of the contract. Codes are part of the public ABI:
/// never renumber an existing variant, only append new ones.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Amounts must be strictly positive.
    InvalidAmount = 1,
    /// The funding goal must be strictly positive.
    InvalidGoal = 2,
    /// The deadline must be in the future.
    InvalidDeadline = 3,
    /// No campaign exists with the given id.
    CampaignNotFound = 4,
    /// The action needs an open campaign (before the deadline, not cancelled).
    CampaignNotOpen = 5,
    /// The deadline has not passed yet.
    CampaignStillOpen = 6,
    /// The deadline passed without reaching the goal.
    GoalNotReached = 7,
    /// The creator already withdrew the funds.
    AlreadyWithdrawn = 8,
    /// The campaign was cancelled by its creator.
    CampaignCancelled = 9,
    /// Refunds exist only for failed or cancelled campaigns.
    RefundNotAvailable = 10,
    /// This account has no contribution to refund.
    NothingToRefund = 11,
    /// The token moved a different amount than requested (fee-on-transfer,
    /// rebasing or otherwise non-standard token).
    UnexpectedTransferAmount = 12,
    /// An intermediate calculation overflowed `i128`.
    MathOverflow = 13,
}
