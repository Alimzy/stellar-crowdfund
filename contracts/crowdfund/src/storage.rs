use soroban_sdk::{Address, Env};

use crate::error::Error;
use crate::types::{Campaign, DataKey};

const DAY_IN_LEDGERS: u32 = 17_280;

/// Instance storage (the campaign counter): bumped to ~30 days whenever fewer
/// than ~29 remain.
pub const INSTANCE_BUMP: u32 = 30 * DAY_IN_LEDGERS;
pub const INSTANCE_THRESHOLD: u32 = INSTANCE_BUMP - DAY_IN_LEDGERS;

/// Campaigns and contributions: bumped to ~90 days whenever fewer than ~89
/// remain. Anyone may call `bump` / `bump_contribution` to keep long ones alive.
pub const ENTRY_BUMP: u32 = 90 * DAY_IN_LEDGERS;
pub const ENTRY_THRESHOLD: u32 = ENTRY_BUMP - DAY_IN_LEDGERS;

pub fn extend_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_THRESHOLD, INSTANCE_BUMP);
}

pub fn extend_campaign(env: &Env, id: u64) {
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Campaign(id), ENTRY_THRESHOLD, ENTRY_BUMP);
}

pub fn extend_contribution(env: &Env, id: u64, who: &Address) {
    env.storage().persistent().extend_ttl(
        &DataKey::Contribution(id, who.clone()),
        ENTRY_THRESHOLD,
        ENTRY_BUMP,
    );
}

pub fn count(env: &Env) -> u64 {
    env.storage().instance().get(&DataKey::Count).unwrap_or(0)
}

pub fn set_count(env: &Env, value: u64) {
    env.storage().instance().set(&DataKey::Count, &value);
}

pub fn load(env: &Env, id: u64) -> Result<Campaign, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Campaign(id))
        .ok_or(Error::CampaignNotFound)
}

pub fn save(env: &Env, id: u64, campaign: &Campaign) {
    env.storage()
        .persistent()
        .set(&DataKey::Campaign(id), campaign);
}

pub fn contribution(env: &Env, id: u64, who: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::Contribution(id, who.clone()))
        .unwrap_or(0)
}

pub fn set_contribution(env: &Env, id: u64, who: &Address, amount: i128) {
    env.storage()
        .persistent()
        .set(&DataKey::Contribution(id, who.clone()), &amount);
}

pub fn remove_contribution(env: &Env, id: u64, who: &Address) {
    env.storage()
        .persistent()
        .remove(&DataKey::Contribution(id, who.clone()));
}

pub fn has_contribution(env: &Env, id: u64, who: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Contribution(id, who.clone()))
}
