#![allow(clippy::needless_range_loop)]

extern crate std;

use proptest::prelude::*;
use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{
        storage::Instance as _, storage::Persistent as _, Address as _, Events as _, Ledger,
    },
    token::{StellarAssetClient, TokenClient},
    vec, Address, Env, IntoVal, Map, Symbol, Val, Vec,
};

use crate::storage::{ENTRY_BUMP, INSTANCE_BUMP};
use crate::types::DataKey;
use crate::{CrowdfundContract, CrowdfundContractClient, Error, Status};

const T0: u64 = 1_000;
const MINT: i128 = 1_000_000;

// ------------------------------------------------------------ mock token

/// Minimal token with switchable misbehaviour, for testing how the crowdfund
/// contract copes with tokens that are not well behaved.
///   mode 0: normal   mode 1: charges a 10% fee on every transfer   mode 2: panics
#[contract]
pub struct MockToken;

#[contracttype]
enum MockKey {
    Balance(Address),
    Mode,
}

#[contractimpl]
impl MockToken {
    pub fn mint(env: Env, to: Address, amount: i128) {
        let key = MockKey::Balance(to);
        let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        env.storage().persistent().set(&key, &(current + amount));
    }

    pub fn set_mode(env: Env, mode: u32) {
        env.storage().instance().set(&MockKey::Mode, &mode);
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&MockKey::Balance(id))
            .unwrap_or(0)
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        let mode: u32 = env.storage().instance().get(&MockKey::Mode).unwrap_or(0);
        if mode == 2 {
            panic!("transfer disabled");
        }
        let received = if mode == 1 {
            amount - amount / 10
        } else {
            amount
        };
        let from_bal = Self::balance(env.clone(), from.clone());
        let to_bal = Self::balance(env.clone(), to.clone());
        env.storage()
            .persistent()
            .set(&MockKey::Balance(from), &(from_bal - amount));
        env.storage()
            .persistent()
            .set(&MockKey::Balance(to), &(to_bal + received));
    }
}

// ----------------------------------------------------------------- fixture

struct Ctx {
    env: Env,
    contract: Address,
    token: Address,
    creator: Address,
    alice: Address,
    bob: Address,
}

impl Ctx {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = T0);
        let admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract_v2(admin).address();
        let creator = Address::generate(&env);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let sac = StellarAssetClient::new(&env, &token);
        sac.mint(&alice, &MINT);
        sac.mint(&bob, &MINT);
        let contract = env.register(CrowdfundContract, ());
        Ctx {
            env,
            contract,
            token,
            creator,
            alice,
            bob,
        }
    }

    fn client(&self) -> CrowdfundContractClient<'_> {
        CrowdfundContractClient::new(&self.env, &self.contract)
    }

    fn token(&self) -> TokenClient<'_> {
        TokenClient::new(&self.env, &self.token)
    }

    fn at(&self, timestamp: u64) {
        self.env.ledger().with_mut(|l| l.timestamp = timestamp);
    }

    /// Goal 1_000, deadline T0 + 100.
    fn campaign(&self) -> u64 {
        self.client()
            .create_campaign(&self.creator, &self.token, &1_000, &(T0 + 100))
    }

    fn age(&self, ledgers: u32) {
        self.env.ledger().with_mut(|l| l.sequence_number += ledgers);
    }
}

/// Build the (contract, topics, data) triple the SDK records for an event.
fn expected_event(
    env: &Env,
    contract: &Address,
    name: &str,
    topics: &[Val],
    data: &[(&str, Val)],
) -> (Address, Vec<Val>, Val) {
    let mut t: Vec<Val> = Vec::new(env);
    t.push_back(Symbol::new(env, name).into_val(env));
    for topic in topics {
        t.push_back(*topic);
    }
    let mut sorted: std::vec::Vec<&(&str, Val)> = data.iter().collect();
    sorted.sort_by_key(|(k, _)| *k);
    let mut m: Map<Symbol, Val> = Map::new(env);
    for (k, v) in sorted {
        m.set(Symbol::new(env, k), *v);
    }
    (contract.clone(), t, m.into_val(env))
}

// ------------------------------------------------------------ creation

#[test]
fn create_records_campaign_and_ids_are_sequential() {
    let c = Ctx::new();
    let a = c.campaign();
    let b = c
        .client()
        .create_campaign(&c.creator, &c.token, &5, &(T0 + 1));

    assert_eq!((a, b), (0, 1));
    assert_eq!(c.client().campaign_count(), 2);

    let s = c.client().get_campaign(&a);
    assert_eq!(s.goal, 1_000);
    assert_eq!(s.deadline, T0 + 100);
    assert_eq!(s.raised, 0);
    assert!(!s.withdrawn && !s.cancelled);
    assert_eq!(c.client().status(&a), Status::Open);
}

#[test]
fn invalid_creation_parameters_are_rejected() {
    let c = Ctx::new();
    let try_create = |goal: i128, deadline: u64| {
        c.client()
            .try_create_campaign(&c.creator, &c.token, &goal, &deadline)
    };
    assert_eq!(try_create(0, T0 + 10), Err(Ok(Error::InvalidGoal)));
    assert_eq!(try_create(-1, T0 + 10), Err(Ok(Error::InvalidGoal)));
    assert_eq!(try_create(10, T0), Err(Ok(Error::InvalidDeadline)));
    assert_eq!(try_create(10, 0), Err(Ok(Error::InvalidDeadline)));
    assert_eq!(c.client().campaign_count(), 0);
}

// ---------------------------------------------------------- contributions

#[test]
fn contributions_accumulate_per_contributor_and_move_funds() {
    let c = Ctx::new();
    let id = c.campaign();

    assert_eq!(c.client().contribute(&id, &c.alice, &300), 300);
    assert_eq!(c.client().contribute(&id, &c.bob, &200), 500);
    assert_eq!(c.client().contribute(&id, &c.alice, &50), 550);

    assert_eq!(c.client().contribution(&id, &c.alice), 350);
    assert_eq!(c.client().contribution(&id, &c.bob), 200);
    assert_eq!(c.client().get_campaign(&id).raised, 550);
    assert_eq!(c.token().balance(&c.contract), 550);
    assert_eq!(c.token().balance(&c.alice), MINT - 350);
}

#[test]
fn zero_and_negative_contributions_are_rejected() {
    let c = Ctx::new();
    let id = c.campaign();
    assert_eq!(
        c.client().try_contribute(&id, &c.alice, &0),
        Err(Ok(Error::InvalidAmount))
    );
    assert_eq!(
        c.client().try_contribute(&id, &c.alice, &-5),
        Err(Ok(Error::InvalidAmount))
    );
    assert_eq!(c.token().balance(&c.contract), 0);
}

#[test]
fn contributing_stops_exactly_at_the_deadline() {
    let c = Ctx::new();
    let id = c.campaign();

    c.at(T0 + 99);
    assert_eq!(c.client().contribute(&id, &c.alice, &10), 10);

    c.at(T0 + 100);
    assert_eq!(
        c.client().try_contribute(&id, &c.alice, &10),
        Err(Ok(Error::CampaignNotOpen))
    );
}

#[test]
fn contributions_may_exceed_the_goal() {
    let c = Ctx::new();
    let id = c.campaign();
    c.client().contribute(&id, &c.alice, &900);
    c.client().contribute(&id, &c.bob, &900);
    assert_eq!(c.client().get_campaign(&id).raised, 1_800);

    c.at(T0 + 100);
    assert_eq!(c.client().withdraw(&id), 1_800);
}

// --------------------------------------------------------------- success

#[test]
fn successful_campaign_pays_the_creator_once_after_the_deadline() {
    let c = Ctx::new();
    let id = c.campaign();
    c.client().contribute(&id, &c.alice, &600);
    c.client().contribute(&id, &c.bob, &400); // exactly the goal

    assert_eq!(
        c.client().try_withdraw(&id),
        Err(Ok(Error::CampaignStillOpen))
    );

    c.at(T0 + 100);
    assert_eq!(c.client().status(&id), Status::Succeeded);
    assert_eq!(c.client().withdraw(&id), 1_000);
    assert_eq!(c.token().balance(&c.creator), 1_000);
    assert_eq!(c.token().balance(&c.contract), 0);
    assert_eq!(c.client().status(&id), Status::Withdrawn);

    assert_eq!(
        c.client().try_withdraw(&id),
        Err(Ok(Error::AlreadyWithdrawn))
    );
    assert_eq!(
        c.client().try_refund(&id, &c.alice),
        Err(Ok(Error::RefundNotAvailable))
    );
}

// --------------------------------------------------------------- failure

#[test]
fn failed_campaign_refunds_exact_amounts_and_only_once() {
    let c = Ctx::new();
    let id = c.campaign();
    c.client().contribute(&id, &c.alice, &300);
    c.client().contribute(&id, &c.bob, &200); // 500 < goal

    assert_eq!(
        c.client().try_refund(&id, &c.alice),
        Err(Ok(Error::RefundNotAvailable)),
        "no refunds while the campaign is still open"
    );

    c.at(T0 + 100);
    assert_eq!(c.client().status(&id), Status::Failed);
    assert_eq!(c.client().try_withdraw(&id), Err(Ok(Error::GoalNotReached)));

    assert_eq!(c.client().refund(&id, &c.alice), 300);
    assert_eq!(c.token().balance(&c.alice), MINT);
    assert_eq!(
        c.client().try_refund(&id, &c.alice),
        Err(Ok(Error::NothingToRefund))
    );
    assert_eq!(c.client().contribution(&id, &c.alice), 0);

    assert_eq!(c.client().refund(&id, &c.bob), 200);
    assert_eq!(c.token().balance(&c.contract), 0);

    let stranger = Address::generate(&c.env);
    assert_eq!(
        c.client().try_refund(&id, &stranger),
        Err(Ok(Error::NothingToRefund))
    );
}

// ---------------------------------------------------------- cancellation

#[test]
fn cancelling_opens_refunds_immediately() {
    let c = Ctx::new();
    let id = c.campaign();
    c.client().contribute(&id, &c.alice, &700);
    c.client().contribute(&id, &c.bob, &700); // goal met, still cancellable

    c.client().cancel(&id);
    assert_eq!(c.client().status(&id), Status::Cancelled);
    assert_eq!(
        c.client().try_contribute(&id, &c.alice, &1),
        Err(Ok(Error::CampaignNotOpen))
    );

    assert_eq!(c.client().refund(&id, &c.alice), 700);
    assert_eq!(c.client().refund(&id, &c.bob), 700);
    assert_eq!(c.token().balance(&c.contract), 0);

    c.at(T0 + 1_000);
    assert_eq!(
        c.client().try_withdraw(&id),
        Err(Ok(Error::CampaignCancelled))
    );
    assert_eq!(c.client().try_cancel(&id), Err(Ok(Error::CampaignNotOpen)));
}

#[test]
fn cannot_cancel_after_the_deadline() {
    let c = Ctx::new();
    let id = c.campaign();
    c.client().contribute(&id, &c.alice, &1_000);
    c.at(T0 + 100);
    assert_eq!(c.client().try_cancel(&id), Err(Ok(Error::CampaignNotOpen)));
    assert_eq!(c.client().withdraw(&id), 1_000);
}

// -------------------------------------------------------------- isolation

#[test]
fn campaigns_and_tokens_are_isolated() {
    let c = Ctx::new();
    let other_token = c.env.register(MockToken, ());
    let mock = MockTokenClient::new(&c.env, &other_token);
    mock.mint(&c.alice, &5_000);

    let a = c.campaign();
    let b = c
        .client()
        .create_campaign(&c.creator, &other_token, &100, &(T0 + 50));

    c.client().contribute(&a, &c.alice, &400);
    c.client().contribute(&b, &c.alice, &150);

    assert_eq!(c.client().contribution(&a, &c.alice), 400);
    assert_eq!(c.client().contribution(&b, &c.alice), 150);
    assert_eq!(c.token().balance(&c.contract), 400);
    assert_eq!(mock.balance(&c.contract), 150);

    c.at(T0 + 50);
    assert_eq!(c.client().withdraw(&b), 150);
    assert_eq!(c.client().status(&a), Status::Open);
    assert_eq!(c.token().balance(&c.contract), 400);
}

#[test]
fn unknown_campaign_ids_fail_cleanly() {
    let c = Ctx::new();
    assert_eq!(
        c.client().try_contribute(&9, &c.alice, &1),
        Err(Ok(Error::CampaignNotFound))
    );
    assert_eq!(
        c.client().try_withdraw(&9),
        Err(Ok(Error::CampaignNotFound))
    );
    assert_eq!(
        c.client().try_refund(&9, &c.alice),
        Err(Ok(Error::CampaignNotFound))
    );
    assert_eq!(c.client().try_cancel(&9), Err(Ok(Error::CampaignNotFound)));
    assert_eq!(c.client().try_bump(&9), Err(Ok(Error::CampaignNotFound)));
    assert_eq!(c.client().try_status(&9), Err(Ok(Error::CampaignNotFound)));
    assert_eq!(
        c.client().try_contribution(&9, &c.alice),
        Err(Ok(Error::CampaignNotFound))
    );
}

// ------------------------------------------------------------ authorization

#[test]
fn each_entrypoint_requires_exactly_the_right_signer() {
    let c = Ctx::new();
    let id = c.campaign();
    assert_eq!(c.env.auths().len(), 1);
    assert_eq!(c.env.auths()[0].0, c.creator);

    c.client().contribute(&id, &c.alice, &1_000);
    assert_eq!(c.env.auths().len(), 1);
    assert_eq!(c.env.auths()[0].0, c.alice);

    c.at(T0 + 100);
    c.client().withdraw(&id);
    assert_eq!(c.env.auths().len(), 1);
    assert_eq!(c.env.auths()[0].0, c.creator);

    let failing = c
        .client()
        .create_campaign(&c.creator, &c.token, &5_000, &(T0 + 200));
    c.client().contribute(&failing, &c.bob, &10);
    c.at(T0 + 200);
    c.client().refund(&failing, &c.bob);
    assert_eq!(c.env.auths().len(), 1);
    assert_eq!(c.env.auths()[0].0, c.bob);

    c.at(T0 + 201);
    let open = c
        .client()
        .create_campaign(&c.creator, &c.token, &5, &(T0 + 900));
    c.client().cancel(&open);
    assert_eq!(c.env.auths().len(), 1);
    assert_eq!(c.env.auths()[0].0, c.creator);

    c.client().bump(&open);
    assert_eq!(c.env.auths().len(), 0, "bump needs no authorization");
}

// -------------------------------------------------------------------- TTL

#[test]
fn every_state_changing_entrypoint_extends_all_touched_entries() {
    let c = Ctx::new();
    let id = c.campaign();
    let ttl = |c: &Ctx| {
        c.env.as_contract(&c.contract, || {
            (
                c.env.storage().instance().get_ttl(),
                c.env.storage().persistent().get_ttl(&DataKey::Campaign(id)),
            )
        })
    };
    let contribution_ttl = |c: &Ctx, who: &Address| {
        c.env.as_contract(&c.contract, || {
            c.env
                .storage()
                .persistent()
                .get_ttl(&DataKey::Contribution(id, who.clone()))
        })
    };
    let day: u32 = 17_280;

    assert_eq!(ttl(&c), (INSTANCE_BUMP, ENTRY_BUMP));

    // contribute extends instance, campaign and the contribution itself
    c.age(20 * day);
    c.client().contribute(&id, &c.alice, &100);
    assert_eq!(ttl(&c), (INSTANCE_BUMP, ENTRY_BUMP));
    assert_eq!(contribution_ttl(&c, &c.alice), ENTRY_BUMP);

    // permissionless bump of the campaign
    c.age(20 * day);
    c.client().bump(&id);
    assert_eq!(ttl(&c), (INSTANCE_BUMP, ENTRY_BUMP));

    // permissionless bump of one contribution
    c.age(20 * day);
    assert!(contribution_ttl(&c, &c.alice) < ENTRY_BUMP);
    c.client().bump_contribution(&id, &c.alice);
    assert_eq!(contribution_ttl(&c, &c.alice), ENTRY_BUMP);
    assert_eq!(ttl(&c), (INSTANCE_BUMP, ENTRY_BUMP));

    // cancel
    c.age(20 * day);
    c.client().cancel(&id);
    assert_eq!(ttl(&c), (INSTANCE_BUMP, ENTRY_BUMP));

    // refund
    c.age(20 * day);
    c.client().refund(&id, &c.alice);
    assert_eq!(ttl(&c), (INSTANCE_BUMP, ENTRY_BUMP));

    // withdraw, on a second campaign
    let second = c
        .client()
        .create_campaign(&c.creator, &c.token, &10, &(T0 + 100));
    c.client().contribute(&second, &c.bob, &10);
    c.at(T0 + 100);
    c.age(20 * day);
    c.client().withdraw(&second);
    let second_ttl = c.env.as_contract(&c.contract, || {
        c.env
            .storage()
            .persistent()
            .get_ttl(&DataKey::Campaign(second))
    });
    assert_eq!(second_ttl, ENTRY_BUMP);
    assert_eq!(
        c.env
            .as_contract(&c.contract, || c.env.storage().instance().get_ttl()),
        INSTANCE_BUMP
    );
}

#[test]
fn bump_contribution_without_a_contribution_fails() {
    let c = Ctx::new();
    let id = c.campaign();
    assert_eq!(
        c.client().try_bump_contribution(&id, &c.alice),
        Err(Ok(Error::NothingToRefund))
    );
}

// ----------------------------------------------------------------- events

#[test]
fn every_event_has_the_exact_documented_topics_and_data() {
    let c = Ctx::new();
    let env = &c.env;
    let contract = &c.contract;
    let observed = |c: &Ctx| c.env.events().all().filter_by_contract(&c.contract);

    let id = c.campaign();
    assert_eq!(
        observed(&c),
        vec![
            env,
            expected_event(
                env,
                contract,
                "campaign_created",
                &[0u64.into_val(env), c.creator.into_val(env)],
                &[
                    ("token", c.token.into_val(env)),
                    ("goal", 1_000i128.into_val(env)),
                    ("deadline", (T0 + 100).into_val(env)),
                ],
            )
        ]
    );

    c.client().contribute(&id, &c.alice, &400);
    assert_eq!(
        observed(&c),
        vec![
            env,
            expected_event(
                env,
                contract,
                "contributed",
                &[0u64.into_val(env), c.alice.into_val(env)],
                &[
                    ("amount", 400i128.into_val(env)),
                    ("raised", 400i128.into_val(env)),
                ],
            )
        ]
    );

    c.client().cancel(&id);
    assert_eq!(
        observed(&c),
        vec![
            env,
            expected_event(
                env,
                contract,
                "cancelled",
                &[0u64.into_val(env), c.creator.into_val(env)],
                &[("raised", 400i128.into_val(env))],
            )
        ]
    );

    c.client().refund(&id, &c.alice);
    assert_eq!(
        observed(&c),
        vec![
            env,
            expected_event(
                env,
                contract,
                "refunded",
                &[0u64.into_val(env), c.alice.into_val(env)],
                &[("amount", 400i128.into_val(env))],
            )
        ]
    );

    let paid = c
        .client()
        .create_campaign(&c.creator, &c.token, &10, &(T0 + 100));
    c.client().contribute(&paid, &c.bob, &10);
    c.at(T0 + 100);
    c.client().withdraw(&paid);
    assert_eq!(
        observed(&c),
        vec![
            env,
            expected_event(
                env,
                contract,
                "withdrawn",
                &[1u64.into_val(env), c.creator.into_val(env)],
                &[("amount", 10i128.into_val(env))],
            )
        ]
    );
}

#[test]
fn failed_calls_emit_no_events() {
    let c = Ctx::new();
    let id = c.campaign();
    let _ = c.client().try_refund(&id, &c.alice);
    assert_eq!(
        c.env
            .events()
            .all()
            .filter_by_contract(&c.contract)
            .events()
            .len(),
        0
    );
}

// ------------------------------------------------- misbehaving tokens

fn mock_ctx() -> (Ctx, Address, u64) {
    let c = Ctx::new();
    let token = c.env.register(MockToken, ());
    let mock = MockTokenClient::new(&c.env, &token);
    mock.mint(&c.alice, &10_000);
    let id = c
        .client()
        .create_campaign(&c.creator, &token, &1_000, &(T0 + 100));
    (c, token, id)
}

#[test]
fn fee_charging_token_is_rejected_and_nothing_changes() {
    let (c, token, id) = mock_ctx();
    let mock = MockTokenClient::new(&c.env, &token);

    mock.set_mode(&1);
    assert_eq!(
        c.client().try_contribute(&id, &c.alice, &500),
        Err(Ok(Error::UnexpectedTransferAmount))
    );
    assert_eq!(c.client().get_campaign(&id).raised, 0);
    assert_eq!(c.client().contribution(&id, &c.alice), 0);
    assert_eq!(
        mock.balance(&c.alice),
        10_000,
        "the failed transfer was rolled back"
    );
    assert_eq!(mock.balance(&c.contract), 0);

    mock.set_mode(&0);
    assert_eq!(c.client().contribute(&id, &c.alice, &500), 500);
}

#[test]
fn a_reverting_token_leaves_no_partial_state_in_any_entrypoint() {
    let (c, token, id) = mock_ctx();
    let mock = MockTokenClient::new(&c.env, &token);

    // contribute
    mock.set_mode(&2);
    assert!(c.client().try_contribute(&id, &c.alice, &100).is_err());
    assert_eq!(c.client().get_campaign(&id).raised, 0);
    assert_eq!(c.client().contribution(&id, &c.alice), 0);

    // withdraw
    mock.set_mode(&0);
    c.client().contribute(&id, &c.alice, &1_000);
    c.at(T0 + 100);
    mock.set_mode(&2);
    assert!(c.client().try_withdraw(&id).is_err());
    assert!(!c.client().get_campaign(&id).withdrawn);
    assert_eq!(c.client().status(&id), Status::Succeeded);
    mock.set_mode(&0);
    assert_eq!(c.client().withdraw(&id), 1_000);

    // refund
    let failing = c
        .client()
        .create_campaign(&c.creator, &token, &9_999, &(T0 + 200));
    c.client().contribute(&failing, &c.alice, &300);
    c.at(T0 + 200);
    mock.set_mode(&2);
    assert!(c.client().try_refund(&failing, &c.alice).is_err());
    assert_eq!(c.client().contribution(&failing, &c.alice), 300);
    mock.set_mode(&0);
    assert_eq!(c.client().refund(&failing, &c.alice), 300);
}

// ------------------------------------------------------ model-based test

#[derive(Clone, Debug)]
enum Op {
    Contribute { c: usize, u: usize, amt: i128 },
    Refund { c: usize, u: usize },
    Withdraw { c: usize },
    Cancel { c: usize },
    Advance(u64),
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => (0..2usize, 0..3usize, 1..=1_000i128)
            .prop_map(|(c, u, amt)| Op::Contribute { c, u, amt }),
        2 => (0..2usize, 0..3usize).prop_map(|(c, u)| Op::Refund { c, u }),
        2 => (0..2usize).prop_map(|c| Op::Withdraw { c }),
        1 => (0..2usize).prop_map(|c| Op::Cancel { c }),
        3 => (1..400u64).prop_map(Op::Advance),
    ]
}

/// Independent re-implementation of the rules, deliberately written without
/// sharing code with the contract.
#[derive(Clone, Default)]
struct ModelCampaign {
    goal: i128,
    deadline: u64,
    raised: i128,
    refunded: i128,
    withdrawn: bool,
    cancelled: bool,
    contrib: [i128; 3],
}

impl ModelCampaign {
    fn status(&self, now: u64) -> Status {
        if self.cancelled {
            Status::Cancelled
        } else if now < self.deadline {
            Status::Open
        } else if self.withdrawn {
            Status::Withdrawn
        } else if self.raised >= self.goal {
            Status::Succeeded
        } else {
            Status::Failed
        }
    }

    fn outstanding(&self) -> i128 {
        if self.withdrawn {
            0
        } else {
            self.raised - self.refunded
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// Random interleavings of contributions, refunds, withdrawals,
    /// cancellations and time travel across two campaigns and three users.
    /// Every call's outcome must match the model, and after every step:
    /// the contract holds exactly what is still owed, each user's balance
    /// equals their start minus net pledged, and the creator holds exactly
    /// what was withdrawn.
    #[test]
    fn contract_matches_the_model_under_random_operations(
        ops in proptest::collection::vec(op_strategy(), 1..40)
    ) {
        let c = Ctx::new();
        let users = [c.alice.clone(), c.bob.clone(), Address::generate(&c.env)];
        StellarAssetClient::new(&c.env, &c.token).mint(&users[2], &MINT);

        let mut model = [ModelCampaign::default(), ModelCampaign::default()];
        let goals = [1_500i128, 4_000];
        let deadlines = [T0 + 600, T0 + 1_200];
        for i in 0..2 {
            let id = c.client().create_campaign(&c.creator, &c.token, &goals[i], &deadlines[i]);
            prop_assert_eq!(id, i as u64);
            model[i].goal = goals[i];
            model[i].deadline = deadlines[i];
        }

        let mut now = T0;
        let mut creator_expected: i128 = 0;

        for op in ops {
            match op {
                Op::Advance(dt) => {
                    now += dt;
                    c.at(now);
                }
                Op::Contribute { c: ci, u, amt } => {
                    let open = model[ci].status(now) == Status::Open;
                    let got = c.client().try_contribute(&(ci as u64), &users[u], &amt);
                    if open {
                        model[ci].raised += amt;
                        model[ci].contrib[u] += amt;
                        prop_assert_eq!(got, Ok(Ok(model[ci].raised)));
                    } else {
                        prop_assert_eq!(got, Err(Ok(Error::CampaignNotOpen)));
                    }
                }
                Op::Refund { c: ci, u } => {
                    let st = model[ci].status(now);
                    let got = c.client().try_refund(&(ci as u64), &users[u]);
                    if st != Status::Failed && st != Status::Cancelled {
                        prop_assert_eq!(got, Err(Ok(Error::RefundNotAvailable)));
                    } else if model[ci].contrib[u] == 0 {
                        prop_assert_eq!(got, Err(Ok(Error::NothingToRefund)));
                    } else {
                        let amount = model[ci].contrib[u];
                        model[ci].contrib[u] = 0;
                        model[ci].refunded += amount;
                        prop_assert_eq!(got, Ok(Ok(amount)));
                    }
                }
                Op::Withdraw { c: ci } => {
                    let st = model[ci].status(now);
                    let got = c.client().try_withdraw(&(ci as u64));
                    let expected = match st {
                        Status::Succeeded => Ok(Ok(model[ci].raised)),
                        Status::Open => Err(Ok(Error::CampaignStillOpen)),
                        Status::Withdrawn => Err(Ok(Error::AlreadyWithdrawn)),
                        Status::Failed => Err(Ok(Error::GoalNotReached)),
                        Status::Cancelled => Err(Ok(Error::CampaignCancelled)),
                    };
                    if st == Status::Succeeded {
                        creator_expected += model[ci].raised;
                        model[ci].withdrawn = true;
                    }
                    prop_assert_eq!(got, expected);
                }
                Op::Cancel { c: ci } => {
                    let open = model[ci].status(now) == Status::Open;
                    let got = c.client().try_cancel(&(ci as u64));
                    if open {
                        model[ci].cancelled = true;
                        prop_assert_eq!(got, Ok(Ok(())));
                    } else {
                        prop_assert_eq!(got, Err(Ok(Error::CampaignNotOpen)));
                    }
                }
            }

            // Invariants after every step.
            let owed: i128 = model.iter().map(|m| m.outstanding()).sum();
            prop_assert_eq!(c.token().balance(&c.contract), owed);
            prop_assert_eq!(c.token().balance(&c.creator), creator_expected);
            for u in 0..3 {
                let net: i128 = model.iter().map(|m| m.contrib[u]).sum::<i128>();
                let paid_in: i128 = MINT - c.token().balance(&users[u]);
                // paid_in is pledged minus refunded, which is what is still recorded.
                prop_assert_eq!(paid_in, net);
            }
            for i in 0..2 {
                prop_assert_eq!(c.client().status(&(i as u64)), model[i].status(now));
                prop_assert_eq!(c.client().get_campaign(&(i as u64)).raised, model[i].raised);
                for u in 0..3 {
                    prop_assert_eq!(
                        c.client().contribution(&(i as u64), &users[u]),
                        model[i].contrib[u]
                    );
                }
            }
        }
    }
}
