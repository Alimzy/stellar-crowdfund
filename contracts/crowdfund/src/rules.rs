//! Pure lifecycle rules, kept free of `Env` so they are trivially testable.

use crate::types::{Campaign, Status};

/// Derive the lifecycle status of `c` at ledger time `now`.
///
/// Precedence: cancelled, then still open, then withdrawn, then goal check.
/// `Withdrawn` and `Cancelled` are mutually exclusive by construction:
/// cancelling needs `Open`, withdrawing needs `Succeeded`.
pub fn status(c: &Campaign, now: u64) -> Status {
    if c.cancelled {
        Status::Cancelled
    } else if now < c.deadline {
        Status::Open
    } else if c.withdrawn {
        Status::Withdrawn
    } else if c.raised >= c.goal {
        Status::Succeeded
    } else {
        Status::Failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn campaign(env: &Env, raised: i128, withdrawn: bool, cancelled: bool) -> Campaign {
        Campaign {
            creator: Address::generate(env),
            token: Address::generate(env),
            goal: 100,
            deadline: 1_000,
            raised,
            withdrawn,
            cancelled,
        }
    }

    #[test]
    fn open_until_the_deadline_even_if_goal_is_met() {
        let env = Env::default();
        let c = campaign(&env, 500, false, false);
        assert_eq!(status(&c, 0), Status::Open);
        assert_eq!(status(&c, 999), Status::Open);
    }

    #[test]
    fn deadline_instant_is_already_closed() {
        let env = Env::default();
        assert_eq!(
            status(&campaign(&env, 100, false, false), 1_000),
            Status::Succeeded
        );
        assert_eq!(
            status(&campaign(&env, 99, false, false), 1_000),
            Status::Failed
        );
    }

    #[test]
    fn goal_exactly_met_succeeds() {
        let env = Env::default();
        assert_eq!(
            status(&campaign(&env, 100, false, false), 2_000),
            Status::Succeeded
        );
    }

    #[test]
    fn withdrawn_and_cancelled_override_the_goal_check() {
        let env = Env::default();
        assert_eq!(
            status(&campaign(&env, 100, true, false), 2_000),
            Status::Withdrawn
        );
        assert_eq!(
            status(&campaign(&env, 100, false, true), 0),
            Status::Cancelled
        );
        assert_eq!(
            status(&campaign(&env, 100, false, true), 2_000),
            Status::Cancelled
        );
    }
}
