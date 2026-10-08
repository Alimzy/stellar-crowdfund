//! # Crowdfund
//!
//! All-or-nothing crowdfunding for any SEP-41 token on Soroban. A creator sets
//! a goal and a deadline. Contributors pledge tokens until the deadline. If the
//! goal is reached the creator withdraws everything; if not, or if the creator
//! cancels, every contributor can reclaim exactly what they put in.
//!
//! One deployed contract holds any number of fully isolated campaigns.
//!
//! See `docs/ARCHITECTURE.md` for the design and `docs/SECURITY-MODEL.md` for
//! the threat model.
#![no_std]

mod error;
mod events;
mod rules;
mod storage;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, token::TokenClient, Address, Env};

pub use error::Error;
pub use types::{Campaign, Status};

use events::{CampaignCreated, Cancelled, Contributed, Refunded, Withdrawn};

#[contract]
pub struct CrowdfundContract;

#[contractimpl]
impl CrowdfundContract {
    /// Start a campaign. Returns its id (ids start at 0).
    ///
    /// Requires `creator` authorization. `goal` must be positive and
    /// `deadline` (ledger-time seconds) must be in the future.
    pub fn create_campaign(
        env: Env,
        creator: Address,
        token: Address,
        goal: i128,
        deadline: u64,
    ) -> Result<u64, Error> {
        creator.require_auth();

        if goal <= 0 {
            return Err(Error::InvalidGoal);
        }
        if deadline <= env.ledger().timestamp() {
            return Err(Error::InvalidDeadline);
        }

        let id = storage::count(&env);
        let campaign = Campaign {
            creator: creator.clone(),
            token: token.clone(),
            goal,
            deadline,
            raised: 0,
            withdrawn: false,
            cancelled: false,
        };

        storage::save(&env, id, &campaign);
        storage::set_count(&env, id + 1);
        storage::extend_instance(&env);
        storage::extend_campaign(&env, id);

        CampaignCreated {
            id,
            creator,
            token,
            goal,
            deadline,
        }
        .publish(&env);

        Ok(id)
    }

    /// Pledge `amount` to an open campaign. Returns the campaign total after
    /// this contribution. Requires `contributor` authorization.
    ///
    /// The contract measures its own token balance before and after the
    /// transfer and rejects the call unless exactly `amount` arrived, so
    /// fee-on-transfer and rebasing tokens can never create a shortfall.
    pub fn contribute(
        env: Env,
        id: u64,
        contributor: Address,
        amount: i128,
    ) -> Result<i128, Error> {
        contributor.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let mut campaign = storage::load(&env, id)?;
        if rules::status(&campaign, env.ledger().timestamp()) != Status::Open {
            return Err(Error::CampaignNotOpen);
        }

        // Interaction first: the balance delta can only be measured after the
        // transfer. A failure here reverts the whole invocation.
        let this = env.current_contract_address();
        let token = TokenClient::new(&env, &campaign.token);
        let before = token.balance(&this);
        token.transfer(&contributor, &this, &amount);
        let after = token.balance(&this);
        if after.checked_sub(before) != Some(amount) {
            return Err(Error::UnexpectedTransferAmount);
        }

        let previous = storage::contribution(&env, id, &contributor);
        let total = previous.checked_add(amount).ok_or(Error::MathOverflow)?;
        campaign.raised = campaign
            .raised
            .checked_add(amount)
            .ok_or(Error::MathOverflow)?;

        storage::set_contribution(&env, id, &contributor, total);
        storage::save(&env, id, &campaign);
        storage::extend_instance(&env);
        storage::extend_campaign(&env, id);
        storage::extend_contribution(&env, id, &contributor);

        Contributed {
            id,
            contributor,
            amount,
            raised: campaign.raised,
        }
        .publish(&env);

        Ok(campaign.raised)
    }

    /// Creator collects everything raised, once the deadline has passed and the
    /// goal was met. Requires the creator's authorization. Returns the amount.
    pub fn withdraw(env: Env, id: u64) -> Result<i128, Error> {
        let mut campaign = storage::load(&env, id)?;
        campaign.creator.require_auth();

        match rules::status(&campaign, env.ledger().timestamp()) {
            Status::Succeeded => {}
            Status::Open => return Err(Error::CampaignStillOpen),
            Status::Withdrawn => return Err(Error::AlreadyWithdrawn),
            Status::Failed => return Err(Error::GoalNotReached),
            Status::Cancelled => return Err(Error::CampaignCancelled),
        }

        let amount = campaign.raised;
        campaign.withdrawn = true;
        storage::save(&env, id, &campaign);
        storage::extend_instance(&env);
        storage::extend_campaign(&env, id);

        TokenClient::new(&env, &campaign.token).transfer(
            &env.current_contract_address(),
            &campaign.creator,
            &amount,
        );

        Withdrawn {
            id,
            creator: campaign.creator,
            amount,
        }
        .publish(&env);

        Ok(amount)
    }

    /// Reclaim your contribution from a failed or cancelled campaign.
    /// Requires `contributor` authorization. Returns the amount refunded.
    pub fn refund(env: Env, id: u64, contributor: Address) -> Result<i128, Error> {
        contributor.require_auth();

        let campaign = storage::load(&env, id)?;
        match rules::status(&campaign, env.ledger().timestamp()) {
            Status::Failed | Status::Cancelled => {}
            _ => return Err(Error::RefundNotAvailable),
        }

        let amount = storage::contribution(&env, id, &contributor);
        if amount <= 0 {
            return Err(Error::NothingToRefund);
        }

        // Effects before interaction; removing the entry makes a second refund
        // impossible and frees the storage.
        storage::remove_contribution(&env, id, &contributor);
        storage::extend_instance(&env);
        storage::extend_campaign(&env, id);

        TokenClient::new(&env, &campaign.token).transfer(
            &env.current_contract_address(),
            &contributor,
            &amount,
        );

        Refunded {
            id,
            contributor,
            amount,
        }
        .publish(&env);

        Ok(amount)
    }

    /// Creator abandons an open campaign; contributors can refund immediately.
    /// Requires the creator's authorization.
    pub fn cancel(env: Env, id: u64) -> Result<(), Error> {
        let mut campaign = storage::load(&env, id)?;
        campaign.creator.require_auth();

        if rules::status(&campaign, env.ledger().timestamp()) != Status::Open {
            return Err(Error::CampaignNotOpen);
        }

        campaign.cancelled = true;
        storage::save(&env, id, &campaign);
        storage::extend_instance(&env);
        storage::extend_campaign(&env, id);

        Cancelled {
            id,
            creator: campaign.creator,
            raised: campaign.raised,
        }
        .publish(&env);

        Ok(())
    }

    /// Permissionless: extend the storage lifetime of the contract and of
    /// campaign `id`.
    pub fn bump(env: Env, id: u64) -> Result<(), Error> {
        storage::load(&env, id)?;
        storage::extend_instance(&env);
        storage::extend_campaign(&env, id);
        Ok(())
    }

    /// Permissionless: extend the lifetime of one contribution record, so a
    /// refund never needs a restore transaction first.
    pub fn bump_contribution(env: Env, id: u64, contributor: Address) -> Result<(), Error> {
        storage::load(&env, id)?;
        if !storage::has_contribution(&env, id, &contributor) {
            return Err(Error::NothingToRefund);
        }
        storage::extend_instance(&env);
        storage::extend_campaign(&env, id);
        storage::extend_contribution(&env, id, &contributor);
        Ok(())
    }

    /// Full campaign record. Read-only.
    pub fn get_campaign(env: Env, id: u64) -> Result<Campaign, Error> {
        storage::load(&env, id)
    }

    /// Lifecycle status at the current ledger time. Read-only.
    pub fn status(env: Env, id: u64) -> Result<Status, Error> {
        let campaign = storage::load(&env, id)?;
        Ok(rules::status(&campaign, env.ledger().timestamp()))
    }

    /// What `contributor` has pledged and not yet refunded. Read-only.
    pub fn contribution(env: Env, id: u64, contributor: Address) -> Result<i128, Error> {
        storage::load(&env, id)?;
        Ok(storage::contribution(&env, id, &contributor))
    }

    /// How many campaigns this contract has ever created. Read-only.
    pub fn campaign_count(env: Env) -> u64 {
        storage::count(&env)
    }
}
