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

// https://blog.oak.tech/tutorial-benchmarking-for-parity-substrate-pallet-development-9cb68bf87ea2
// https://github.com/paritytech/substrate/blob/master/.maintain/frame-weight-template.hbs
// Executed Command:
// ./target/release/node-template benchmark pallet --chain=dev --wasm-execution=compiled --pallet=pallet_network --extrinsic=* --steps=50 --repeat=20 --output="pallets/network/src/weights.rs" --template ./.maintain/frame-weight-template.hbs

// ./target/release/node-template benchmark pallet --chain=dev --wasm-execution=compiled --pallet=pallet_network --extrinsic=* --steps=5 --repeat=2 --output="pallets/network/src/weights.rs" --template ./.maintain/frame-weight-template.hbs

// cargo build --release --features runtime-benchmarks
// cargo test --release --features runtime-benchmarks
// cargo build --package pallet-network --features runtime-benchmarks
use super::*;
use frame_benchmarking::{account, benchmarks, whitelist_account, BenchmarkError};
use frame_support::{
	assert_noop, assert_ok,
	traits::{Currency, EnsureOrigin, Get, OnInitialize, UnfilteredDispatchable},
};
use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
use crate::Pallet as Network;
use frame_support::dispatch::Vec;
use sp_core::OpaquePeerId as PeerId;
use scale_info::prelude::vec;
use scale_info::prelude::format;
use crate::{SubnetPaths, MinRequiredUnstakeEpochs, TotalStake};

const PERCENTAGE_FACTOR: u128 = 10000;
const SEED: u32 = 0;
const DEFAULT_SCORE: u128 = 10000;
// Steps to complete consensus
// 1 += form consensus
// 1 += generate emissions
const CONSENSUS_STEPS: u64 = 2;

fn funded_initializer<T: Config>(name: &'static str, index: u32) -> T::AccountId {
	let caller: T::AccountId = account(name, index, SEED);
	// Give the account half of the maximum value of the `Balance` type.
	// Otherwise some transfers will fail with an overflow error.
	let deposit_amount: u128 = T::SubnetInitializationCost::get();
	T::Currency::deposit_creating(&caller, deposit_amount.try_into().ok().expect("REASON"));
	caller
}

fn funded_account<T: Config>(name: &'static str, index: u32) -> T::AccountId {
	let caller: T::AccountId = account(name, index, SEED);
	// Give the account half of the maximum value of the `Balance` type.
	// Otherwise some transfers will fail with an overflow error.
	let deposit_amount: u128 = get_min_stake_balance::<T>() + 10000;
	T::Currency::deposit_creating(&caller, deposit_amount.try_into().ok().expect("REASON"));
	caller
}

fn get_min_stake_balance<T: Config>() -> u128 {
	MinStakeBalance::<T>::get()
}

// increase the blocks past the consensus steps and remove subnet peer blocks span
fn make_consensus_data_submittable<T: Config>() {
  // increase blocks
	let current_block_number = get_current_block_as_u64::<T>();
  let subnet_node_removal_percentage = RemoveSubnetNodeEpochPercentage::<T>::get();
  let epoch_length = T::EpochLength::get();

  let block_can_remove_peer = epoch_length as u128 * subnet_node_removal_percentage / PERCENTAGE_FACTOR;

  let max_remove_subnet_node_block = block_can_remove_peer as u64 + (current_block_number - (current_block_number % epoch_length));

  if current_block_number < max_remove_subnet_node_block {
		frame_system::Pallet::<T>::set_block_number(u64_to_block::<T>(max_remove_subnet_node_block + 1));
  }
}

fn make_subnet_node_consensus_data_submittable<T: Config>() {
  // increase blocks
	let current_block_number = get_current_block_as_u64::<T>();
  let epoch_length = T::EpochLength::get();
  let min_required_peer_consensus_submit_epochs: u64 = Network::<T>::min_required_peer_consensus_submit_epochs();
	let required_block = current_block_number + epoch_length * min_required_peer_consensus_submit_epochs;
	frame_system::Pallet::<T>::set_block_number(u64_to_block::<T>(required_block));

	make_consensus_data_submittable::<T>();
}

fn make_subnet_node_removable<T: Config>() {
  // increase blocks
  let current_block_number = get_current_block_as_u64::<T>();
  let subnet_node_removal_percentage = RemoveSubnetNodeEpochPercentage::<T>::get();
  let epoch_length = T::EpochLength::get();

  let block_span_can_remove_peer = (epoch_length as u128 * subnet_node_removal_percentage / PERCENTAGE_FACTOR) as u64;

  let start_removal_block = (CONSENSUS_STEPS + (current_block_number - (current_block_number % epoch_length))) as u64;

  let end_removal_block = block_span_can_remove_peer + (current_block_number - (current_block_number % epoch_length));
  
  if current_block_number < start_removal_block {
		frame_system::Pallet::<T>::set_block_number(u64_to_block::<T>(start_removal_block));
  } else if current_block_number > end_removal_block {
		frame_system::Pallet::<T>::set_block_number(u64_to_block::<T>(start_removal_block + epoch_length));
  }
}

fn make_subnet_initialized<T: Config>() {
	let current_block_number = get_current_block_as_u64::<T>();
	let epoch_length = T::EpochLength::get();
	let min_required_model_consensus_submit_epochs: u64 = Network::<T>::min_required_model_consensus_submit_epochs();
	frame_system::Pallet::<T>::set_block_number(u64_to_block::<T>(current_block_number + epoch_length * min_required_model_consensus_submit_epochs));
}

fn peer(id: u8) -> PeerId {
  let peer_id = format!("QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N{id}"); 
	PeerId(peer_id.into())
}

// fn subnet_node_data<T: Config>(start: u8, end: u8) -> Vec<SubnetNodeData<T::AccountId>> {
//   // initialize peer consensus data array
//   let mut subnet_node_data: Vec<SubnetNodeData<T::AccountId>> = Vec::new();
//   for n in start..end {
//     let peer_subnet_node_data: SubnetNodeData<T::AccountId> = SubnetNodeData {
//       account_id: account("peer", n.into(), SEED),
//       peer_id: peer(n),
//       score: DEFAULT_SCORE,
//     };
//     subnet_node_data.push(peer_subnet_node_data);
//   }
//   subnet_node_data
// }

fn subnet_node_data<T: Config>(start: u8, end: u8) -> Vec<SubnetNodeData> {
	// initialize peer consensus data array
	let mut subnet_node_data: Vec<SubnetNodeData> = Vec::new();
	for n in start..end {
		let peer_subnet_node_data: SubnetNodeData = SubnetNodeData {
			peer_id: peer(n),
			score: DEFAULT_SCORE,
		};
		subnet_node_data.push(peer_subnet_node_data);
	}
	subnet_node_data
}
	
pub fn u64_to_block<T: frame_system::Config>(input: u64) -> BlockNumberFor<T> {
	input.try_into().ok().expect("REASON")
}

pub fn get_current_block_as_u64<T: frame_system::Config>() -> u64 {
	TryInto::try_into(<frame_system::Pallet<T>>::block_number())
		.ok()
		.expect("blockchain will not exceed 2^64 blocks; QED.")
}

benchmarks! {
	add_subnet_node {
		// add subnet
		let subnet_path: Vec<u8> = "petals-team-2/StableBeluga2".into();
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);

		let funded_initializer = funded_account::<T>("funded_initializer", 0);

		let add_subnet_data = PreSubnetData {
			path: subnet_path.clone().into(),
			memory_mb: 50000,
		};
		assert_ok!(
			Network::<T>::activate_subnet(
				funded_initializer.clone(),
				funded_initializer.clone(),
				add_subnet_data,
			)
		);
	
		// Network::<T>::vote_model(RawOrigin::Signed(caller.clone()).into(), subnet_path.clone());
		// let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), subnet_path.clone());
		
		make_subnet_initialized::<T>();
		
		let subnet_id = SubnetPaths::<T>::get(subnet_path.clone()).unwrap();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));

		// add subnet peer params
		let stake_amount: u128 = get_min_stake_balance::<T>();
		let peer_account = funded_account::<T>("peer", 0);
		whitelist_account!(peer_account);

		// params
		let total_models = Network::<T>::total_models();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));
	}: add_subnet_node(RawOrigin::Signed(peer_account), subnet_id.clone(), peer(0), stake_amount)
	verify {
		assert_eq!(Network::<T>::total_subnet_nodes(total_models), 1, "TotalSubnetNodes incorrect.");
	}

	update_subnet_node {
		// add subnet
		let subnet_path: Vec<u8> = "petals-team-2/StableBeluga2".into();
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		let funded_initializer = funded_account::<T>("funded_initializer", 0);

		let add_subnet_data = PreSubnetData {
			path: subnet_path.clone().into(),
			memory_mb: 50000,
		};
		assert_ok!(
			Network::<T>::activate_subnet(
				funded_initializer.clone(),
				funded_initializer.clone(),
				add_subnet_data,
			)
		);

		// Network::<T>::vote_model(RawOrigin::Signed(caller.clone()).into(), subnet_path.clone());
		// let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), subnet_path.clone());
		
		make_subnet_initialized::<T>();
		
		let subnet_id = SubnetPaths::<T>::get(subnet_path.clone()).unwrap();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));

		// add subnet peer params
		let stake_amount: u128 = get_min_stake_balance::<T>();
		let peer_account = funded_account::<T>("peer", 0);
		whitelist_account!(peer_account);

		// params
		let total_models = Network::<T>::total_models();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		// log::error!("Block -> {:?}", block);

		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));

		Network::<T>::add_subnet_node(
			RawOrigin::Signed(peer_account.clone()).into(), 
			subnet_id.clone(), 
			peer(0), 
			stake_amount
		);

		make_subnet_node_consensus_data_submittable::<T>();

		make_subnet_node_removable::<T>();

	}: update_subnet_node(RawOrigin::Signed(peer_account.clone()), subnet_id.clone(), peer(1))
	verify {
		assert_eq!(Network::<T>::total_subnet_nodes(total_models), 1, "TotalSubnetNodes incorrect.");
	}

	remove_subnet_node {
		// add subnet
		let subnet_path: Vec<u8> = "petals-team-2/StableBeluga2".into();
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		let funded_initializer = funded_account::<T>("funded_initializer", 0);

		let add_subnet_data = PreSubnetData {
			path: subnet_path.clone().into(),
			memory_mb: 50000,
		};
		assert_ok!(
			Network::<T>::activate_subnet(
				funded_initializer.clone(),
				funded_initializer.clone(),
				add_subnet_data,
			)
		);

		// Network::<T>::vote_model(RawOrigin::Signed(caller.clone()).into(), subnet_path.clone());
		// let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), subnet_path.clone());
		
		make_subnet_initialized::<T>();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));
		
		let subnet_id = SubnetPaths::<T>::get(subnet_path.clone()).unwrap();

		// add subnet peer
		let stake_amount: u128 = get_min_stake_balance::<T>();
		let peer_account = funded_account::<T>("peer", 0);
		whitelist_account!(peer_account);
		Network::<T>::add_subnet_node(
			RawOrigin::Signed(peer_account.clone()).into(), 
			subnet_id.clone(), 
			peer(0), 
			// "172.20.54.234".into(), 
			// 8888, 
			stake_amount
		);


		// params
		let total_models = Network::<T>::total_models();

	}: remove_subnet_node(RawOrigin::Signed(peer_account.clone()), subnet_id.clone())
	verify {
		assert_eq!(Network::<T>::total_subnet_nodes(total_models), 0, "TotalSubnetNodes incorrect.");
	}

	add_to_stake {
		// add subnet
		let subnet_path: Vec<u8> = "petals-team-2/StableBeluga2".into();
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		let funded_initializer = funded_account::<T>("funded_initializer", 0);

		let add_subnet_data = PreSubnetData {
			path: subnet_path.clone().into(),
			memory_mb: 50000,
		};
		assert_ok!(
			Network::<T>::activate_subnet(
				funded_initializer.clone(),
				funded_initializer.clone(),
				add_subnet_data,
			)
		);

		// Network::<T>::vote_model(RawOrigin::Signed(caller.clone()).into(), subnet_path.clone());
		// let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), subnet_path.clone());
		
		make_subnet_initialized::<T>();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));
		
		let subnet_id = SubnetPaths::<T>::get(subnet_path.clone()).unwrap();

		
		// add subnet peer
		let stake_amount: u128 = get_min_stake_balance::<T>();
		let peer_account = funded_account::<T>("peer", 0);
		whitelist_account!(peer_account);
		Network::<T>::add_subnet_node(
			RawOrigin::Signed(peer_account.clone()).into(), 
			subnet_id.clone(), 
			peer(0), 
			// "172.20.54.234".into(), 
			// 8888, 
			stake_amount
		);

		
		let add_to_stake_amount: u128 = 1000;

		// params
		let total_models = Network::<T>::total_models();
		let account_model_stake = Network::<T>::account_model_stake(peer_account.clone(), total_models.clone());
		let total_account_stake = Network::<T>::total_account_stake(peer_account.clone());
		let total_stake = Network::<T>::total_stake();
		let total_model_stake = Network::<T>::total_model_stake(total_models.clone());

		// expected stake results
		let expected_account_model_stake = account_model_stake + add_to_stake_amount;
		let expected_total_account_stake = total_account_stake + add_to_stake_amount;
		let expected_total_stake = total_stake + add_to_stake_amount;
		let expected_total_model_stake = total_model_stake + add_to_stake_amount;

	}: add_to_stake(RawOrigin::Signed(peer_account.clone()), subnet_id.clone(), add_to_stake_amount)
	verify {
		assert_eq!(Network::<T>::account_model_stake(peer_account.clone(), total_models.clone()), expected_account_model_stake, "AccountSubnetStake incorrect.");
		assert_eq!(Network::<T>::total_account_stake(peer_account.clone()), expected_total_account_stake, "TotalAccountStake incorrect.");
		assert_eq!(Network::<T>::total_stake(), expected_total_stake, "TotalStake incorrect.");
		assert_eq!(Network::<T>::total_model_stake(total_models.clone()), expected_total_model_stake, "TotalSubnetStake incorrect.");
		assert_eq!(Network::<T>::total_subnet_nodes(total_models.clone()), 1, "TotalSubnetNodes incorrect.");
	}

	remove_stake {
		// add subnet
		let subnet_path: Vec<u8> = "petals-team-2/StableBeluga2".into();
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		let funded_initializer = funded_account::<T>("funded_initializer", 0);

		let add_subnet_data = PreSubnetData {
			path: subnet_path.clone().into(),
			memory_mb: 50000,
		};
		assert_ok!(
			Network::<T>::activate_subnet(
				funded_initializer.clone(),
				funded_initializer.clone(),
				add_subnet_data,
			)
		);

		// Network::<T>::vote_model(RawOrigin::Signed(caller.clone()).into(), subnet_path.clone());
		// let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), subnet_path.clone());
		
		make_subnet_initialized::<T>();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));

		let subnet_id = SubnetPaths::<T>::get(subnet_path.clone()).unwrap();

		// add subnet peer
		let stake_amount: u128 = get_min_stake_balance::<T>() + 1000;
		let peer_account = funded_account::<T>("peer", 0);
		whitelist_account!(peer_account);
		Network::<T>::add_subnet_node(
			RawOrigin::Signed(peer_account.clone()).into(), 
			subnet_id.clone(), 
			peer(0), 
			// "172.20.54.234".into(), 
			// 8888, 
			stake_amount
		);

		
		let block = frame_system::Pallet::<T>::block_number();

		let epoch_length = T::EpochLength::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<T>::get();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(epoch_length * min_required_unstake_epochs));

		let remove_stake_amount: u128 = 10;
		let expected_stake_amount: u128 = stake_amount - remove_stake_amount;

		// params
		let total_models = Network::<T>::total_models();
		let account_model_stake = Network::<T>::account_model_stake(peer_account.clone(), total_models.clone());
		let total_account_stake = Network::<T>::total_account_stake(peer_account.clone());
		let total_stake = Network::<T>::total_stake();
		let total_model_stake = Network::<T>::total_model_stake(total_models.clone());

		// expected stake results
		let expected_account_model_stake = account_model_stake - remove_stake_amount;
		let expected_total_account_stake = total_account_stake - remove_stake_amount;
		let expected_total_stake = total_stake - remove_stake_amount;
		let expected_total_model_stake = total_model_stake - remove_stake_amount;
	}: remove_stake(RawOrigin::Signed(peer_account.clone()), total_models.clone(), remove_stake_amount)
	verify {
		assert_eq!(Network::<T>::account_model_stake(peer_account.clone(), total_models.clone()), expected_account_model_stake, "AccountSubnetStake incorrect.");
		assert_eq!(Network::<T>::total_account_stake(peer_account.clone()), expected_total_account_stake, "TotalAccountStake incorrect.");
		assert_eq!(Network::<T>::total_stake(), expected_total_stake, "TotalStake incorrect.");
		assert_eq!(Network::<T>::total_model_stake(total_models.clone()), expected_total_model_stake, "TotalSubnetStake incorrect.");
		assert_eq!(Network::<T>::total_subnet_nodes(total_models.clone()), 1, "TotalSubnetNodes incorrect.");
	}

	impl_benchmark_test_suite!(
		Network,
		crate::mock::new_test_ext(),
		crate::mock::Test
	);
}