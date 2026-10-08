use soroban_sdk::{contractevent, Address};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignCreated {
    #[topic]
    pub id: u64,
    #[topic]
    pub creator: Address,
    pub token: Address,
    pub goal: i128,
    pub deadline: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Contributed {
    #[topic]
    pub id: u64,
    #[topic]
    pub contributor: Address,
    pub amount: i128,
    /// Campaign total after this contribution.
    pub raised: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Withdrawn {
    #[topic]
    pub id: u64,
    #[topic]
    pub creator: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Refunded {
    #[topic]
    pub id: u64,
    #[topic]
    pub contributor: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cancelled {
    #[topic]
    pub id: u64,
    #[topic]
    pub creator: Address,
    /// Total that contributors can now reclaim.
    pub raised: i128,
}
