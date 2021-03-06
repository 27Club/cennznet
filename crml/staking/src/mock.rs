// Copyright 2017-2020 Parity Technologies (UK) Ltd. and Centrality Investments Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Test utilities

use crate::{
	rewards::{self, Module as RewardsModule, Trait as RewardsTrait},
	EraIndex, GenesisConfig, Module, Nominators, RewardDestination, StakerStatus, Trait, ValidatorPrefs,
};
use frame_support::{
	assert_ok, impl_outer_event, impl_outer_origin, parameter_types,
	traits::{Currency, FindAuthor, Get},
	weights::Weight,
	IterableStorageMap, StorageValue,
};
use sp_core::{crypto::key_types, H256};
use sp_io;
use sp_runtime::curve::PiecewiseLinear;
use sp_runtime::testing::{Header, UintAuthorityId};
use sp_runtime::traits::{BlakeTwo256, Convert, IdentityLookup, OpaqueKeys, SaturatedConversion};
use sp_runtime::{KeyTypeId, ModuleId, Perbill};
use sp_staking::{
	offence::{OffenceDetails, OnOffenceHandler},
	SessionIndex,
};
use std::{cell::RefCell, collections::HashSet};

const INIT_TIMESTAMP: u64 = 30_000;

pub type AccountId = u64;
pub type BlockNumber = u64;
pub type Balance = u64;

/// Simple structure that exposes how u64 currency can be represented as... u64.
pub struct CurrencyToVoteHandler;
impl Convert<u64, u64> for CurrencyToVoteHandler {
	fn convert(x: u64) -> u64 {
		x
	}
}
impl Convert<u128, u64> for CurrencyToVoteHandler {
	fn convert(x: u128) -> u64 {
		x.saturated_into()
	}
}

thread_local! {
	static SESSION: RefCell<(Vec<AccountId>, HashSet<AccountId>)> = RefCell::new(Default::default());
	static EXISTENTIAL_DEPOSIT: RefCell<u64> = RefCell::new(0);
	static SLASH_DEFER_DURATION: RefCell<EraIndex> = RefCell::new(0);
}

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
	const KEY_TYPE_IDS: &'static [KeyTypeId] = &[key_types::DUMMY];

	fn on_genesis_session<Ks: OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}

	fn on_new_session<Ks: OpaqueKeys>(
		_changed: bool,
		validators: &[(AccountId, Ks)],
		_queued_validators: &[(AccountId, Ks)],
	) {
		SESSION.with(|x| *x.borrow_mut() = (validators.iter().map(|x| x.0.clone()).collect(), HashSet::new()));
	}

	fn on_disabled(validator_index: usize) {
		SESSION.with(|d| {
			let mut d = d.borrow_mut();
			let value = d.0[validator_index];
			d.1.insert(value);
		})
	}
}

pub fn is_disabled(controller: AccountId) -> bool {
	let stash = Staking::ledger(&controller).unwrap().stash;
	SESSION.with(|d| d.borrow().1.contains(&stash))
}

pub struct ExistentialDeposit;
impl Get<u64> for ExistentialDeposit {
	fn get() -> u64 {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
	}
}

pub struct SlashDeferDuration;
impl Get<EraIndex> for SlashDeferDuration {
	fn get() -> EraIndex {
		SLASH_DEFER_DURATION.with(|v| *v.borrow())
	}
}

impl_outer_origin! {
	pub enum Origin for Test where system = frame_system {}
}

mod staking {
	// Re-export needed for `impl_outer_event!`.
	pub use super::super::*;
}
use frame_system as system;
use pallet_balances as balances;
use pallet_session as session;

impl_outer_event! {
	pub enum MetaEvent for Test {
		system<T>,
		balances<T>,
		session,
		staking<T>,
		rewards<T>,
	}
}

/// Author of block is always 11
pub struct Author11;
impl FindAuthor<u64> for Author11 {
	fn find_author<'a, I>(_digests: I) -> Option<u64>
	where
		I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
	{
		Some(11)
	}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl frame_system::Trait for Test {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Index = u64;
	type Call = ();
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = MetaEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type AvailableBlockRatio = AvailableBlockRatio;
	type MaximumBlockLength = MaximumBlockLength;
	type Version = ();
	type PalletInfo = ();
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}

parameter_types! {
	pub const CreationFee: u64 = 0;
}

impl pallet_balances::Trait for Test {
	type MaxLocks = ();
	type Balance = Balance;
	type Event = MetaEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

parameter_types! {
	pub const Period: BlockNumber = 1;
	pub const Offset: BlockNumber = 0;
	pub const UncleGenerations: u64 = 0;
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(25);
}
impl pallet_session::Trait for Test {
	type Event = MetaEvent;
	type ValidatorId = AccountId;
	type ValidatorIdOf = crate::StashOf<Test>;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = ();
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
	type SessionHandler = TestSessionHandler;
	type Keys = UintAuthorityId;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type WeightInfo = ();
}

impl pallet_session::historical::Trait for Test {
	type FullIdentification = crate::Exposure<AccountId, Balance>;
	type FullIdentificationOf = crate::ExposureOf<Test>;
}
impl pallet_authorship::Trait for Test {
	type FindAuthor = Author11;
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = Rewards;
}
parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Trait for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const HistoricalPayoutEras: u16 = 7;
	pub const PayoutSplitThreshold: u32 = 10;
	pub const FiscalEraLength: u32 = 5;
	pub const TreasuryModuleId: ModuleId = ModuleId(*b"py/trsry");
}
impl RewardsTrait for Test {
	type CurrencyToReward = pallet_balances::Module<Self>;
	type Event = MetaEvent;
	type HistoricalPayoutEras = HistoricalPayoutEras;
	type TreasuryModuleId = TreasuryModuleId;
	type PayoutSplitThreshold = PayoutSplitThreshold;
	type FiscalEraLength = FiscalEraLength;
	type WeightInfo = ();
}

parameter_types! {
	pub const SessionsPerEra: SessionIndex = 3;
	pub const BondingDuration: EraIndex = 3;
	pub const BlocksPerEra: BlockNumber = 3;
}
impl Trait for Test {
	type Currency = pallet_balances::Module<Self>;
	type Time = pallet_timestamp::Module<Self>;
	type CurrencyToVote = CurrencyToVoteHandler;
	type Event = MetaEvent;
	type Slash = ();
	type SessionsPerEra = SessionsPerEra;
	type BlocksPerEra = BlocksPerEra;
	type SlashDeferDuration = SlashDeferDuration;
	type BondingDuration = BondingDuration;
	type SessionInterface = Self;
	type WeightInfo = ();
	type Rewarder = Rewards;
}
pub struct ExtBuilder {
	existential_deposit: u64,
	validator_pool: bool,
	nominate: bool,
	validator_count: u32,
	minimum_validator_count: u32,
	slash_defer_duration: EraIndex,
	fair: bool,
	num_validators: Option<u32>,
	invulnerables: Vec<u64>,
	has_stakers: bool,
	minimum_bond: u64,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			existential_deposit: 1,
			validator_pool: false,
			nominate: true,
			validator_count: 2,
			minimum_validator_count: 0,
			slash_defer_duration: 0,
			fair: true,
			num_validators: None,
			invulnerables: vec![],
			has_stakers: true,
			minimum_bond: 1,
		}
	}
}

impl ExtBuilder {
	pub fn existential_deposit(mut self, existential_deposit: u64) -> Self {
		self.existential_deposit = existential_deposit;
		self
	}
	pub fn validator_pool(mut self, validator_pool: bool) -> Self {
		self.validator_pool = validator_pool;
		self
	}
	pub fn nominate(mut self, nominate: bool) -> Self {
		self.nominate = nominate;
		self
	}
	pub fn validator_count(mut self, count: u32) -> Self {
		self.validator_count = count;
		self
	}
	pub fn minimum_validator_count(mut self, count: u32) -> Self {
		self.minimum_validator_count = count;
		self
	}
	pub fn slash_defer_duration(mut self, eras: EraIndex) -> Self {
		self.slash_defer_duration = eras;
		self
	}
	pub fn fair(mut self, is_fair: bool) -> Self {
		self.fair = is_fair;
		self
	}
	pub fn num_validators(mut self, num_validators: u32) -> Self {
		self.num_validators = Some(num_validators);
		self
	}
	pub fn invulnerables(mut self, invulnerables: Vec<u64>) -> Self {
		self.invulnerables = invulnerables;
		self
	}
	pub fn minimum_bond(mut self, minimum_bond: u64) -> Self {
		self.minimum_bond = minimum_bond;
		self
	}
	pub fn has_stakers(mut self, has: bool) -> Self {
		self.has_stakers = has;
		self
	}
	pub fn set_associated_consts(&self) {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
		SLASH_DEFER_DURATION.with(|v| *v.borrow_mut() = self.slash_defer_duration);
	}
	// Simplified version of `build` taking constant parameters only
	// no account, balance, or staking setup is performed.
	pub fn simple(self) -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		let _ = GenesisConfig::<Test> {
			current_era: 0,
			stakers: vec![],
			validator_count: self.validator_count,
			minimum_validator_count: self.minimum_validator_count,
			invulnerables: self.invulnerables,
			minimum_bond: self.minimum_bond,
			slash_reward_fraction: Perbill::from_percent(10),
			..Default::default()
		}
		.assimilate_storage(&mut storage);

		sp_io::TestExternalities::from(storage)
	}
	pub fn build(self) -> sp_io::TestExternalities {
		self.set_associated_consts();
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		let balance_factor = if self.existential_deposit > 1 { 256 } else { 1 };

		let num_validators = self.num_validators.unwrap_or(self.validator_count);
		let validators = (0..num_validators)
			.map(|x| ((x + 1) * 10 + 1) as u64)
			.collect::<Vec<_>>();

		let _ = pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 10 * balance_factor),
				(2, 20 * balance_factor),
				(3, 300 * balance_factor),
				(4, 400 * balance_factor),
				(10, balance_factor),
				(11, balance_factor * 1000),
				(20, balance_factor),
				(21, balance_factor * 2000),
				(30, balance_factor),
				(31, balance_factor * 2000),
				(40, balance_factor),
				(41, balance_factor * 2000),
				(100, 2000 * balance_factor),
				(101, 2000 * balance_factor),
				// This allow us to have a total_payout different from 0.
				(999, 1_000_000_000_000),
			],
		}
		.assimilate_storage(&mut storage);

		let mut stakers = vec![];
		if self.has_stakers {
			let stake_21 = if self.fair { 1000 } else { 2000 };
			let stake_31 = if self.validator_pool { balance_factor * 1000 } else { 1 };
			let status_41 = if self.validator_pool {
				StakerStatus::<AccountId>::Validator
			} else {
				StakerStatus::<AccountId>::Idle
			};
			let nominated = if self.nominate { vec![11, 21] } else { vec![] };
			stakers = vec![
				// (stash, controller, staked_amount, status)
				(11, 10, balance_factor * 1000, StakerStatus::<AccountId>::Validator),
				(21, 20, stake_21, StakerStatus::<AccountId>::Validator),
				(31, 30, stake_31, StakerStatus::<AccountId>::Validator),
				(41, 40, balance_factor * 1000, status_41),
				// nominator
				(
					101,
					100,
					balance_factor * 500,
					StakerStatus::<AccountId>::Nominator(nominated),
				),
			];
		}
		let _ = GenesisConfig::<Test> {
			current_era: 0,
			stakers: stakers,
			validator_count: self.validator_count,
			minimum_validator_count: self.minimum_validator_count,
			invulnerables: self.invulnerables,
			minimum_bond: self.minimum_bond,
			slash_reward_fraction: Perbill::from_percent(10),
			..Default::default()
		}
		.assimilate_storage(&mut storage);

		let _ = pallet_session::GenesisConfig::<Test> {
			keys: validators.iter().map(|x| (*x, *x, UintAuthorityId(*x))).collect(),
		}
		.assimilate_storage(&mut storage);

		let mut ext = sp_io::TestExternalities::from(storage);
		ext.execute_with(|| {
			let validators = Session::validators();
			SESSION.with(|x| *x.borrow_mut() = (validators.clone(), HashSet::new()));
		});
		// We consider all test to start after timestamp is initialized
		// This must be ensured by having `timestamp::on_initialize` called before
		// `staking::on_initialize`
		ext.execute_with(|| {
			Timestamp::set_timestamp(INIT_TIMESTAMP);
			Rewards::new_fiscal_era();
		});

		ext
	}
}

pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type Session = pallet_session::Module<Test>;
pub type Timestamp = pallet_timestamp::Module<Test>;
pub type Staking = Module<Test>;
pub type Rewards = RewardsModule<Test>;

pub fn check_exposure_all() {
	Staking::current_elected()
		.into_iter()
		.for_each(|acc| check_exposure(acc));
}

pub fn check_nominator_all() {
	<Nominators<Test>>::iter().for_each(|(acc, _)| check_nominator_exposure(acc));
}

/// Check for each selected validator: expo.total = Sum(expo.other) + expo.own
pub fn check_exposure(stash: u64) {
	assert_is_stash(stash);
	let expo = Staking::stakers(&stash);
	assert_eq!(
		expo.total as u128,
		expo.own as u128 + expo.others.iter().map(|e| e.value as u128).sum::<u128>(),
		"wrong total exposure for {:?}: {:?}",
		stash,
		expo,
	);
}

/// Check that for each nominator: slashable_balance > sum(used_balance)
/// Note: we might not consume all of a nominator's balance, but we MUST NOT over spend it.
pub fn check_nominator_exposure(stash: u64) {
	assert_is_stash(stash);
	let mut sum = 0;
	Staking::current_elected()
		.iter()
		.map(|v| Staking::stakers(v))
		.for_each(|e| e.others.iter().filter(|i| i.who == stash).for_each(|i| sum += i.value));
	let nominator_stake = Staking::slashable_balance_of(&stash);
	// a nominator cannot over-spend.
	assert!(
		nominator_stake >= sum,
		"failed: Nominator({}) stake({}) >= sum divided({})",
		stash,
		nominator_stake,
		sum,
	);
}

pub fn assert_is_stash(acc: u64) {
	assert!(Staking::bonded(&acc).is_some(), "Not a stash.");
}

pub fn assert_ledger_consistent(stash: u64) {
	assert_is_stash(stash);
	let ledger = Staking::ledger(stash - 1).unwrap();

	let real_total: Balance = ledger.unlocking.iter().fold(ledger.active, |a, c| a + c.value);
	assert_eq!(real_total, ledger.total);
}

pub(crate) fn bond_validator(stash: AccountId, ctrl: AccountId, val: Balance) {
	let _ = Balances::make_free_balance_be(&stash, val);
	let _ = Balances::make_free_balance_be(&ctrl, val);
	assert_ok!(Staking::bond(
		Origin::signed(stash),
		ctrl,
		val,
		RewardDestination::Controller,
	));
	assert_ok!(Staking::validate(Origin::signed(ctrl), ValidatorPrefs::default()));
}

pub(crate) fn bond_nominator(stash: AccountId, ctrl: AccountId, val: Balance, target: Vec<AccountId>) {
	let _ = Balances::make_free_balance_be(&stash, val);
	let _ = Balances::make_free_balance_be(&ctrl, val);
	assert_ok!(Staking::bond(
		Origin::signed(stash),
		ctrl,
		val,
		RewardDestination::Controller,
	));
	assert_ok!(Staking::nominate(Origin::signed(ctrl), target));
}

pub fn start_session(index: SessionIndex) {
	assert!(Session::current_index() <= index);

	let rotations = index - Session::current_index();
	for _i in 0..rotations {
		Timestamp::set_timestamp(Timestamp::now() + 1000);
		Session::rotate_session();
	}
}

pub fn start_era(era_index: EraIndex) {
	start_session(era_index * SessionsPerEra::get());
}

pub fn advance_session() {
	start_session(Session::current_index() + 1);
}

pallet_staking_reward_curve::build! {
	const I_NPOS: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

pub fn reward_all_elected() {
	let rewards = <Module<Test>>::current_elected()
		.iter()
		.map(|v| (*v, 1))
		.collect::<Vec<_>>();

	Rewards::reward_by_ids(rewards)
}

pub fn validator_controllers() -> Vec<AccountId> {
	Session::validators()
		.into_iter()
		.map(|s| Staking::bonded(&s).expect("no controller for validator"))
		.collect()
}

pub fn on_offence_in_era(
	offenders: &[OffenceDetails<AccountId, pallet_session::historical::IdentificationTuple<Test>>],
	slash_fraction: &[Perbill],
	era: EraIndex,
) {
	let bonded_eras = crate::BondedEras::get();
	for &(bonded_era, start_session) in bonded_eras.iter() {
		if bonded_era == era {
			let _ = Staking::on_offence(offenders, slash_fraction, start_session);
			return;
		} else if bonded_era > era {
			break;
		}
	}

	if Staking::current_era() == era {
		let _ = Staking::on_offence(offenders, slash_fraction, Staking::current_era_start_session_index());
	} else {
		panic!("cannot slash in era {}", era);
	}
}

pub fn on_offence_now(
	offenders: &[OffenceDetails<AccountId, pallet_session::historical::IdentificationTuple<Test>>],
	slash_fraction: &[Perbill],
) {
	let now = Staking::current_era();
	on_offence_in_era(offenders, slash_fraction, now)
}
