// Copyright (C) Hypertensor.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use crate as pallet_network;
use frame_support::{
  parameter_types,
  traits::Everything,
};
use frame_system as system;
use sp_core::{ConstU128, ConstU32, ConstU64, H256, U256};
use sp_runtime::BuildStorage;
use frame_support::sp_tracing;
use sp_runtime::{
	traits::{
		BlakeTwo256, IdentifyAccount, Verify, IdentityLookup, AccountIdLookup
	},
	MultiSignature
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test
	{
    System: system,
    Balances: pallet_balances,
    Network: pallet_network,
	}
);

pub type BalanceCall = pallet_balances::Call<Test>;

parameter_types! {
  pub const BlockHashCount: u64 = 250;
  pub const SS58Prefix: u8 = 42;
}

// pub type AccountId = U256;

pub type Signature = MultiSignature;

pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

// The address format for describing accounts.
pub type Address = AccountId;

// Balance of an account.
pub type Balance = u128;

// An index to a block.
#[allow(dead_code)]
pub type BlockNumber = u64;

pub const EXISTENTIAL_DEPOSIT: u128 = 500;

impl pallet_balances::Config for Test {
  type Balance = Balance;
  type RuntimeEvent = RuntimeEvent;
  type DustRemoval = ();
  type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
  type AccountStore = System;
  type MaxLocks = ();
  type WeightInfo = ();
  type MaxReserves = ();
  type ReserveIdentifier = [u8; 8];
  type RuntimeHoldReason = ();
  type FreezeIdentifier = ();
  type MaxHolds = ();
  type MaxFreezes = ();
}

impl system::Config for Test {
  type BaseCallFilter = Everything;
  type BlockWeights = ();
  type BlockLength = ();
  type Block = Block;
  type DbWeight = ();
  type RuntimeOrigin = RuntimeOrigin;
  type RuntimeCall = RuntimeCall;
  type Nonce = u64;
  type Hash = H256;
  type Hashing = BlakeTwo256;
  // type AccountId = U256;
  type AccountId = AccountId;
  // type Lookup = IdentityLookup<Self::AccountId>;
  type Lookup = AccountIdLookup<AccountId, ()>;
  type RuntimeEvent = RuntimeEvent;
  type BlockHashCount = BlockHashCount;
  type Version = ();
  type PalletInfo = PalletInfo;
  type AccountData = pallet_balances::AccountData<u128>;
  type OnNewAccount = ();
  type OnKilledAccount = ();
  type SystemWeightInfo = ();
  type SS58Prefix = SS58Prefix;
  type OnSetCode = ();
  type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl Config for Test {
  type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
  type Currency = Balances;
  type StringLimit = ConstU32<100>;
	type InitialTxRateLimit = ConstU64<0>;
}

// pub fn new_test_ext() -> sp_io::TestExternalities {
//   sp_tracing::try_init_simple();
//   frame_system::GenesisConfig::default()
//     .build_storage()
//     .unwrap()
//     .into()
// }

pub fn new_test_ext() -> sp_io::TestExternalities {
  // pallet_network::GenesisConfig::<Test> {
  //   model_path: vec![];
  //   model_peers: vec![];
  //   accounts: vec![];
  //   blank: Some();
  // }
  // pallet_network::StakeVaultBalance::<Test>::mutate(|n: &mut u128| *n += 10000u128);
	frame_system::GenesisConfig::<Test>::default()
		.build_storage()
		.unwrap()
		.into()
}
