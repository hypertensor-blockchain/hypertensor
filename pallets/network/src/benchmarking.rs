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
use crate::{ModelPaths, ConsensusBlocksInterval, MinRequiredUnstakeEpochs, TotalStake};

const PERCENTAGE_FACTOR: u128 = 10000;
const SEED: u32 = 0;
const DEFAULT_SCORE: u128 = 10000;
// Steps to complete consensus
// 1 += form consensus
// 1 += generate emissions
const CONSENSUS_STEPS: u64 = 2;

fn funded_account<T: Config>(name: &'static str, index: u32) -> T::AccountId {
	let caller: T::AccountId = account(name, index, SEED);
	// Give the account half of the maximum value of the `Balance` type.
	// Otherwise some transfers will fail with an overflow error.
	let deposit_amount: u128 = 10000000000000000000000;
	T::Currency::deposit_creating(&caller, deposit_amount.try_into().ok().expect("REASON"));
	caller
}

// increase the blocks past the consensus steps and remove model peer blocks span
fn make_consensus_data_submittable<T: Config>() {
  // increase blocks
	let current_block_number = get_current_block_as_u64::<T>();
  let model_peer_removal_percentage = RemoveModelPeerEpochPercentage::<T>::get();
  let consensus_blocks_interval = ConsensusBlocksInterval::<T>::get();

  let block_can_remove_peer = consensus_blocks_interval as u128 * model_peer_removal_percentage / PERCENTAGE_FACTOR;

  let max_remove_model_peer_block = block_can_remove_peer as u64 + (current_block_number - (current_block_number % consensus_blocks_interval));

  if current_block_number < max_remove_model_peer_block {
		frame_system::Pallet::<T>::set_block_number(u64_to_block::<T>(max_remove_model_peer_block));
  }
}

fn make_model_peer_consensus_data_submittable<T: Config>() {
  // increase blocks
	let current_block_number = get_current_block_as_u64::<T>();
  let consensus_blocks_interval = ConsensusBlocksInterval::<T>::get();
  let min_required_peer_consensus_submit_epochs: u64 = Network::<T>::min_required_peer_consensus_submit_epochs();
	let required_block = current_block_number + consensus_blocks_interval * min_required_peer_consensus_submit_epochs;
	frame_system::Pallet::<T>::set_block_number(u64_to_block::<T>(required_block));

	make_consensus_data_submittable::<T>();
}

fn make_model_initialized<T: Config>() {
	let current_block_number = get_current_block_as_u64::<T>();
	let consensus_blocks_interval = ConsensusBlocksInterval::<T>::get();
	let min_required_model_consensus_submit_epochs: u64 = Network::<T>::min_required_model_consensus_submit_epochs();
	frame_system::Pallet::<T>::set_block_number(u64_to_block::<T>(current_block_number + consensus_blocks_interval * min_required_model_consensus_submit_epochs));
}

fn peer(id: u8) -> PeerId {
  let peer_id = format!("QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N{id}"); 
	PeerId(peer_id.into())
}

// fn model_peer_data<T: Config>(start: u8, end: u8) -> Vec<ModelPeerData<T::AccountId>> {
//   // initialize peer consensus data array
//   let mut model_peer_data: Vec<ModelPeerData<T::AccountId>> = Vec::new();
//   for n in start..end {
//     let peer_model_peer_data: ModelPeerData<T::AccountId> = ModelPeerData {
//       account_id: account("peer", n.into(), SEED),
//       peer_id: peer(n),
//       score: DEFAULT_SCORE,
//     };
//     model_peer_data.push(peer_model_peer_data);
//   }
//   model_peer_data
// }

fn model_peer_data<T: Config>(start: u8, end: u8) -> Vec<ModelPeerData> {
	// initialize peer consensus data array
	let mut model_peer_data: Vec<ModelPeerData> = Vec::new();
	for n in start..end {
		let peer_model_peer_data: ModelPeerData = ModelPeerData {
			peer_id: peer(n),
			score: DEFAULT_SCORE,
		};
		model_peer_data.push(peer_model_peer_data);
	}
	model_peer_data
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
	submit_consensus_data {
		let consensus_blocks_interval = ConsensusBlocksInterval::<T>::get();

		// add model
		let model_path: Vec<u8> = "petals-team-2/StableBeluga2".into();
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		Network::<T>::vote_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone());
		let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone());

		make_model_initialized::<T>();

		let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();

		// increase blocks
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));

		// add model peers
		let n_peers: u8 = (Network::<T>::max_model_peers()) as u8;
		let stake_amount: u128 = 1000;
		let first_peer_account = funded_account::<T>("peer", 0);
		let first_peer_id = peer(0);
		for n in 0..n_peers {
			let peer_account = funded_account::<T>("peer", n.into());
			whitelist_account!(peer_account);
			Network::<T>::add_model_peer(RawOrigin::Signed(peer_account.clone()).into(), model_id.clone(), peer(n), "172.20.54.234".into(), 8888, stake_amount);
    }
		
		make_model_peer_consensus_data_submittable::<T>();

		// let block = frame_system::Pallet::<T>::block_number();
		// let min_required_peer_consensus_submit_epochs: u64 = Network::<T>::min_required_peer_consensus_submit_epochs();
		// frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(consensus_blocks_interval * min_required_peer_consensus_submit_epochs));

		// build consensus
		let model_peer_data = model_peer_data::<T>(0, n_peers);

		// params
		let total_models = Network::<T>::total_models();

		// increase blocks past consensus steps
		make_consensus_data_submittable::<T>();

		// let block = frame_system::Pallet::<T>::block_number();
		// frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));
	}: submit_consensus_data(RawOrigin::Signed(first_peer_account.clone()), model_id.clone(), false, model_peer_data)
	verify {
		assert_eq!(Network::<T>::total_model_peers(total_models), n_peers as u32, "TotalModelPeers incorrect.");
		let submission = ModelPeerConsensusResults::<T>::iter_key_prefix(total_models);
		let len = submission.count();
		assert_eq!(
			len, 
			n_peers as usize, 
			"ModelPeerConsensusResults len mismatch."
		);
	}

  add_model {
		let total_models = Network::<T>::total_models();

		let model_path: Vec<u8> = "petals-team-2/StableBeluga2".into();
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		Network::<T>::vote_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone());

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));
	}: add_model(RawOrigin::Signed(caller.clone()), model_path.clone())
	verify {
		assert_eq!(Network::<T>::total_models(), total_models + 1, "TotalModels incorrect.");
		// assert_eq!(Network::<T>::models(model_path.clone()), Some((total_models + 1) as u32), "Models path stored incorrectly.");
	}

	add_model_peer {
		// add model
		let model_path: Vec<u8> = "petals-team-2/StableBeluga2".into();
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		Network::<T>::vote_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone());
		let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone());
		
		make_model_initialized::<T>();
		
		let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));

		// add model peer params
		let stake_amount: u128 = 10000000000000000000;
		let peer_account = funded_account::<T>("peer", 0);
		whitelist_account!(peer_account);

		// params
		let total_models = Network::<T>::total_models();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));
	}: add_model_peer(RawOrigin::Signed(peer_account), model_id.clone(), peer(0), "172.20.54.234".into(), 8888, stake_amount)
	verify {
		assert_eq!(Network::<T>::total_model_peers(total_models), 1, "TotalModelPeers incorrect.");
	}

	remove_model_peer {
		// add model
		let model_path: Vec<u8> = "petals-team-2/StableBeluga2".into();
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		Network::<T>::vote_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone());
		let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone());
		
		make_model_initialized::<T>();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));
		
		let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();

		// add model peer
		let stake_amount: u128 = 1000;
		let peer_account = funded_account::<T>("peer", 0);
		whitelist_account!(peer_account);
		let __ = Network::<T>::add_model_peer(RawOrigin::Signed(peer_account.clone()).into(), model_id.clone(), peer(0), "172.20.54.234".into(), 8888, stake_amount);
		
		// params
		let total_models = Network::<T>::total_models();

	}: remove_model_peer(RawOrigin::Signed(peer_account.clone()), model_id.clone())
	verify {
		assert_eq!(Network::<T>::total_model_peers(total_models), 0, "TotalModelPeers incorrect.");
	}

	update_port {
		// add model
		let model_path: Vec<u8> = "petals-team-2/StableBeluga2".into();
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		Network::<T>::vote_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone());
		let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone());
		
		make_model_initialized::<T>();

		let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));
		
		// add model peer
		let stake_amount: u128 = 1000;
		let peer_account = funded_account::<T>("peer", 0);
		whitelist_account!(peer_account);
		let __ = Network::<T>::add_model_peer(RawOrigin::Signed(peer_account.clone()).into(), model_id.clone(), peer(0), "172.20.54.234".into(), 8888, stake_amount);
		
		// params
		let total_models = Network::<T>::total_models();
	}: update_port(RawOrigin::Signed(peer_account.clone()), model_id.clone(), 8889)
	verify {
		assert_eq!(Network::<T>::total_model_peers(total_models), 1, "TotalModelPeers incorrect.");
	}

	add_to_stake {
		// add model
		let model_path: Vec<u8> = "petals-team-2/StableBeluga2".into();
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		Network::<T>::vote_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone());
		let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone());
		
		make_model_initialized::<T>();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));
		
		let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();

		
		// add model peer
		let stake_amount: u128 = 1000;
		let peer_account = funded_account::<T>("peer", 0);
		whitelist_account!(peer_account);
		let __ = Network::<T>::add_model_peer(RawOrigin::Signed(peer_account.clone()).into(), model_id.clone(), peer(0), "172.20.54.234".into(), 8888, stake_amount);
		
		// params
		let total_models = Network::<T>::total_models();
		let account_model_stake = Network::<T>::account_model_stake(peer_account.clone(), total_models.clone());
		let total_account_stake = Network::<T>::total_account_stake(peer_account.clone());
		let total_stake = Network::<T>::total_stake();
		let total_model_stake = Network::<T>::total_model_stake(total_models.clone());

		// expected stake results
		let expected_account_model_stake = account_model_stake + stake_amount;
		let expected_total_account_stake = total_account_stake + stake_amount;
		let expected_total_stake = total_stake + stake_amount;
		let expected_total_model_stake = total_model_stake + stake_amount;

	}: add_to_stake(RawOrigin::Signed(peer_account.clone()), model_id.clone(), stake_amount)
	verify {
		assert_eq!(Network::<T>::account_model_stake(peer_account.clone(), total_models.clone()), expected_account_model_stake, "AccountModelStake incorrect.");
		assert_eq!(Network::<T>::total_account_stake(peer_account.clone()), expected_total_account_stake, "TotalAccountStake incorrect.");
		assert_eq!(Network::<T>::total_stake(), expected_total_stake, "TotalStake incorrect.");
		assert_eq!(Network::<T>::total_model_stake(total_models.clone()), expected_total_model_stake, "TotalModelStake incorrect.");
		assert_eq!(Network::<T>::total_model_peers(total_models.clone()), 1, "TotalModelPeers incorrect.");
	}

	remove_stake {
		// add model
		let model_path: Vec<u8> = "petals-team-2/StableBeluga2".into();
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		Network::<T>::vote_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone());
		let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone());
		
		make_model_initialized::<T>();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));

		let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();

		// add model peer
		let stake_amount: u128 = 1000;
		let peer_account = funded_account::<T>("peer", 0);
		whitelist_account!(peer_account);
		let __ = Network::<T>::add_model_peer(RawOrigin::Signed(peer_account.clone()).into(), model_id.clone(), peer(0), "172.20.54.234".into(), 8888, stake_amount);
		
		let block = frame_system::Pallet::<T>::block_number();

		let consensus_blocks_interval = ConsensusBlocksInterval::<T>::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<T>::get();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(consensus_blocks_interval * min_required_unstake_epochs));

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
		assert_eq!(Network::<T>::account_model_stake(peer_account.clone(), total_models.clone()), expected_account_model_stake, "AccountModelStake incorrect.");
		assert_eq!(Network::<T>::total_account_stake(peer_account.clone()), expected_total_account_stake, "TotalAccountStake incorrect.");
		assert_eq!(Network::<T>::total_stake(), expected_total_stake, "TotalStake incorrect.");
		assert_eq!(Network::<T>::total_model_stake(total_models.clone()), expected_total_model_stake, "TotalModelStake incorrect.");
		assert_eq!(Network::<T>::total_model_peers(total_models.clone()), 1, "TotalModelPeers incorrect.");
	}

	form_consensus_data {
		// params from genesis
		let total_models = Network::<T>::total_models();
		let max_models = Network::<T>::max_models();
		let consensus_blocks_interval = ConsensusBlocksInterval::<T>::get();

		// add model
		let m_models: u32 = max_models;
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		for m in total_models..m_models {
			let model_path = format!("petals-team-{m}/StableBeluga");
			Network::<T>::vote_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone().into());
			let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone().into());
    }
		
		make_model_initialized::<T>();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));

		// add model peers
		let n_peers: u8 = (Network::<T>::max_model_peers()) as u8;
		let stake_amount: u128 = 1000;
		for m in total_models..m_models {
			let model_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
			let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();

			for n in 0..n_peers {
				let peer_account = funded_account::<T>("peer", n.into());
				whitelist_account!(peer_account);
				let __ = Network::<T>::add_model_peer(RawOrigin::Signed(peer_account.clone()).into(), model_id.clone(), peer(n), "172.20.54.234".into(), 8888, stake_amount);
			}	
    }

		// increase blocks for peers to submit data
		make_model_peer_consensus_data_submittable::<T>();
		// let block = frame_system::Pallet::<T>::block_number();
		// let min_required_peer_consensus_submit_epochs: u64 = Network::<T>::min_required_peer_consensus_submit_epochs();
		// frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(consensus_blocks_interval * min_required_peer_consensus_submit_epochs));

		// build consensus
		let model_peer_data = model_peer_data::<T>(0, n_peers);

		// increase blocks to surpass consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));

		for m in total_models..m_models {
			let model_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
			let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();

			for n in 0..n_peers {
				let peer_account = funded_account::<T>("peer", n.into());
				whitelist_account!(peer_account);
				Network::<T>::submit_consensus_data(
					RawOrigin::Signed(peer_account.clone()).into(), 
					model_id.clone().into(), 
					false,
					model_peer_data.clone()
				);
			}
    }

		let consensus_blocks_interval = u64_to_block::<T>(ConsensusBlocksInterval::<T>::get());
		let block = frame_system::Pallet::<T>::block_number();
    frame_system::Pallet::<T>::set_block_number(
      consensus_blocks_interval + (block - (block % consensus_blocks_interval))
    );    
	}: form_consensus(RawOrigin::Signed(caller.clone()))
	verify {
		assert_eq!(Network::<T>::total_models(), max_models, "TotalModels incorrect.");
		let submission = ModelPeerConsensusResults::<T>::iter_key_prefix(9);
		let len = submission.count();
		assert_eq!(
			len, 
			n_peers as usize, 
			"ModelPeerConsensusResults len mismatch."
		);

		for m in total_models..m_models {
			let submission = ModelPeerConsensusResults::<T>::iter_key_prefix(m+1);
			let len = submission.count();
			assert_eq!(
				len, 
				n_peers as usize, 
				"ModelPeerConsensusResults len mismatch."
			);
			for n in 0..n_peers {
				assert_eq!(Network::<T>::total_model_peers(m+1), n_peers as u32, "TotalModelPeers incorrect.");
			}	
    }
	}

	// form_consensus_data_2 {
	// 	// params from genesis
	// 	let total_models = Network::<T>::total_models();

	// 	let model_path: Vec<u8> = "petals-team-2/StableBeluga2".into();

	// 	// add model
	// 	let caller = funded_account::<T>("caller", 0);
	// 	whitelist_account!(caller);
	// 	let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone().into());
	// let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();
	// 	// add model peers
	// 	let n_peers: u8 = (Network::<T>::max_model_peers()) as u8;
	// 	let stake_amount: u128 = 1000;
	// 	for n in 0..n_peers {
	// 		let peer_account = funded_account::<T>("peer", n.into());
	// 		whitelist_account!(peer_account);
	// 		let __ = Network::<T>::add_model_peer(RawOrigin::Signed(peer_account.clone()).into(), model_id.clone().into(), peer(n), "172.20.54.234".into(), 8888, stake_amount);
	// 	}	

	// 	// build consensus
	// 	let model_peer_data_vec = model_peer_data::<T>(0, n_peers);

	// 	// increase blocks
	// 	let min_required_peer_consensus_submit_epochs: u64 = Network::<T>::min_required_peer_consensus_submit_epochs() + CONSENSUS_STEPS;
	// 	let block = frame_system::Pallet::<T>::block_number();
	// 	frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(min_required_peer_consensus_submit_epochs));

	// 	for n in 0..n_peers-1 {
	// 		let peer_account = funded_account::<T>("peer", n.into());
	// 		whitelist_account!(peer_account);
	// 		Network::<T>::submit_consensus_data(RawOrigin::Signed(peer_account.clone()).into(), model_id.clone().into(), model_peer_data_vec.clone());	
	// 	}

	// 	// get consensus data removing the first peer
	// 	let model_peer_data_against = model_peer_data::<T>(1, n_peers);
	// 	// submit peer consensus data removing the first peer
	// 	let peer_account_against = funded_account::<T>("peer", (n_peers-1) as u32);
	// 	Network::<T>::submit_consensus_data(RawOrigin::Signed(peer_account_against.clone()).into(), model_id.clone().into(), model_peer_data_against.clone());	

	// }: form_consensus(RawOrigin::Signed(caller.clone()))
	// verify {
	// 	assert_eq!(Network::<T>::total_models(), total_models+1, "TotalModels incorrect.");
	// 	let submission = ModelPeerConsensusResults::<T>::iter_key_prefix(total_models+1);
	// 	let len = submission.count();
	// 	assert_eq!(
	// 		len, 
	// 		n_peers as usize, 
	// 		"ModelPeerConsensusResults len mismatch."
	// 	);
	// 	assert_eq!(
	// 		Network::<T>::model_peer_penalty_count(funded_account::<T>("peer", (n_peers-1) as u32)), 
	// 		1, 
	// 		"AgainstConsensusCount incorrect."
	// 	);

	// }

	// generate_emissions {
	// 	// params from genesis
	// 	StakeVaultBalance::<T>::mutate(|n: &mut u128| *n += 10000000000000000000);
	// 	let total_models = Network::<T>::total_models();
	// 	let max_models = Network::<T>::max_models();

	// 	// add model
	// 	let m_models: u32 = max_models;
	// 	let caller = funded_account::<T>("caller", 0);
	// 	whitelist_account!(caller);
	// 	for m in total_models..m_models {
	// 		let model_path = format!("petals-team-{m}/StableBeluga");
	// 		let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone().into());
  //   }
		
	// 	// add model peers
	// 	let n_peers: u8 = (Network::<T>::max_model_peers()) as u8;
	// 	let stake_amount: u128 = 10000000000000000000;
	// 	for m in total_models..m_models {
	// 		let model_path = format!("petals-team-{m}/StableBeluga");
	// let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();
	// 		for n in 0..n_peers {
	// 			let peer_account = funded_account::<T>("peer", n.into());
	// 			whitelist_account!(peer_account);
	// 			let __ = Network::<T>::add_model_peer(RawOrigin::Signed(peer_account.clone()).into(), model_id.clone().into(), peer(n), "172.20.54.234".into(), 8888, stake_amount);
	// 		}	
  //   }

	// 	// build consensus
	// 	let model_peer_data = model_peer_data::<T>(0, n_peers);

	// 	// increase blocks
	// 	let min_required_peer_consensus_submit_epochs: u64 = Network::<T>::min_required_peer_consensus_submit_epochs() + CONSENSUS_STEPS;
	// 	let block = frame_system::Pallet::<T>::block_number();
	// 	frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(min_required_peer_consensus_submit_epochs));

	// 	for m in total_models..m_models {
	// 		let model_path = format!("petals-team-{m}/StableBeluga");  
	// let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();
	// 		for n in 0..n_peers {
	// 			let peer_account = funded_account::<T>("peer", n.into());
	// 			whitelist_account!(peer_account);
	// 			Network::<T>::submit_consensus_data(RawOrigin::Signed(peer_account.clone()).into(), model_id.clone().into(), model_peer_data.clone());	
	// 		}	
  //   };

	// 	// call form_consensus
	// 	Network::<T>::form_consensus(RawOrigin::Signed(caller.clone()).into());

	// }: do_generate_emissions(RawOrigin::Signed(caller.clone()))
	// verify {
	// 	assert_eq!(Network::<T>::total_models(), max_models, "TotalModels incorrect.");
	// 	let submission = ModelPeerConsensusResults::<T>::iter_key_prefix(9);
	// 	let len = submission.count();
	// 	// should be removed
	// 	assert_eq!(
	// 		len, 
	// 		0 as usize, 
	// 		"ModelPeerConsensusResults len mismatch."
	// 	);		

	// 	for m in total_models..m_models {
	// 		let submission = ModelPeerConsensusResults::<T>::iter_key_prefix(m+1);
	// 		let len = submission.count();
	// 		assert_eq!(
	// 			len, 
	// 			0 as usize, 
	// 			"ModelPeerConsensusResults len mismatch."
	// 		);
	// 		for n in 0..n_peers {
	// 			assert_eq!(Network::<T>::total_model_peers(m+1), n_peers as u32, "TotalModelPeers incorrect.");
	// 		}	
  //   }
	// }

	// generate_emissions_1_model {
	// 	// params from genesis
	// 	let caller = funded_account::<T>("caller", 0);
	// 	StakeVaultBalance::<T>::mutate(|n: &mut u128| *n += 10000000000000000000);
	// 	let total_models = Network::<T>::total_models();
	// 	let max_models = Network::<T>::max_models();

	// 	// add model
	// 	let model_path: Vec<u8> = "petals-team/StableBeluga2".into();
	// 	whitelist_account!(caller);
	// 	let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone());
	// let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();

	// 	let n_peers: u8 = (Network::<T>::max_model_peers()) as u8;
	// 	let stake_amount: u128 = 10000000000000000000;
	// 	for n in 0..n_peers {
	// 		let peer_account = funded_account::<T>("peer", n.into());
	// 		whitelist_account!(peer_account);
	// 		let __ = Network::<T>::add_model_peer(RawOrigin::Signed(peer_account.clone()).into(), model_id.clone(), peer(n), "172.20.54.234".into(), 8888, stake_amount);
  //   }

	// 	// build consensus
	// 	let model_peer_data = model_peer_data::<T>(0, n_peers);

	// 	// increase blocks
	// 	let min_required_peer_consensus_submit_epochs: u64 = Network::<T>::min_required_peer_consensus_submit_epochs() + CONSENSUS_STEPS;
	// 	let block = frame_system::Pallet::<T>::block_number();
	// 	frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(min_required_peer_consensus_submit_epochs));

	// 	for n in 0..n_peers {
	// 		let peer_account = funded_account::<T>("peer", n.into());
	// 		whitelist_account!(peer_account);
	// 		Network::<T>::submit_consensus_data(RawOrigin::Signed(peer_account.clone()).into(), model_id.clone().into(), model_peer_data.clone());	
	// 	}	

	// 	// call form_consensus
	// 	Network::<T>::form_consensus(RawOrigin::Signed(caller.clone()).into());

	// }: do_generate_emissions(RawOrigin::Signed(caller.clone()))
	// verify {
	// 	assert_eq!(Network::<T>::total_models(), 1, "TotalModels incorrect.");
	// 	let submission = ModelPeerConsensusResults::<T>::iter_key_prefix(1);
	// 	let len = submission.count();
	// 	// should be removed
	// 	assert_eq!(
	// 		len, 
	// 		0 as usize, 
	// 		"ModelPeerConsensusResults len mismatch."
	// 	);		
	// }

	generate_emissions_max_model {
		// params from genesis
		let caller = funded_account::<T>("caller", 0);
		StakeVaultBalance::<T>::mutate(|n: &mut u128| *n += 10000000000000000000);
		let total_models = Network::<T>::total_models();
		let max_models = Network::<T>::max_models();
		let consensus_blocks_interval = ConsensusBlocksInterval::<T>::get();

		// add models
		let m_models: u32 = max_models;
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		for m in total_models..m_models {
			let model_path = format!("petals-team-{m}/StableBeluga");
			Network::<T>::vote_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone().into());
			let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone().into());
    }

		make_model_initialized::<T>();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));
		
		// add model peers
		let n_peers: u8 = (Network::<T>::max_model_peers()) as u8;
		let stake_amount: u128 = 23000000000000000000;
		for m in total_models..m_models {
			let model_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
			let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();

			for n in 0..n_peers {
				let peer_account = funded_account::<T>("peer", n.into());
				whitelist_account!(peer_account);
				let __ = Network::<T>::add_model_peer(RawOrigin::Signed(peer_account.clone()).into(), model_id.clone(), peer(n), "172.20.54.234".into(), 8888, stake_amount);
			}	
    }

		// increase blocks for peers to submit data
		make_model_peer_consensus_data_submittable::<T>();
		// let block = frame_system::Pallet::<T>::block_number();
		// let min_required_peer_consensus_submit_epochs: u64 = Network::<T>::min_required_peer_consensus_submit_epochs();
		// frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(consensus_blocks_interval * min_required_peer_consensus_submit_epochs));

		// build consensus
		let model_peer_data = model_peer_data::<T>(0, n_peers);

		// increase blocks to surpass consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));

		for m in total_models..m_models {
			let model_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
			let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();

			for n in 0..n_peers {
				let peer_account = funded_account::<T>("peer", n.into());
				whitelist_account!(peer_account);
				Network::<T>::submit_consensus_data(RawOrigin::Signed(peer_account.clone()).into(), model_id.clone(), false, model_peer_data.clone());		
			}
			let submission = ModelPeerConsensusResults::<T>::iter_key_prefix(m + 1);
			let len = submission.count();

			// should match size of inclusion peers
			assert_eq!(
				len, 
				n_peers as usize, 
				"ModelPeerConsensusResults len mismatch."
			);	
    }

		let consensus_blocks_interval = u64_to_block::<T>(ConsensusBlocksInterval::<T>::get());
		let block = frame_system::Pallet::<T>::block_number();
    frame_system::Pallet::<T>::set_block_number(
      consensus_blocks_interval + (block - (block % consensus_blocks_interval))
    );    

		// call form_consensus
		Network::<T>::form_consensus(RawOrigin::Signed(caller.clone()).into());

		// Set to correct generate emissions block
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(
			consensus_blocks_interval + (block - (block % consensus_blocks_interval) + u64_to_block::<T>(1))
		);    
	}: do_generate_emissions(RawOrigin::Signed(caller.clone()))
	verify {
		assert_eq!(Network::<T>::total_models(), max_models, "TotalModels incorrect.");
		let submission = ModelPeerConsensusResults::<T>::iter_key_prefix(1);
		let len = submission.count();
		// should be removed on on successful emissions emitting
		assert_eq!(
			len, 
			0 as usize, 
			"ModelPeerConsensusResults len mismatch."
		);		
	}

	generate_emissions_emissions_weights {
		// params from genesis
		let caller = funded_account::<T>("caller", 0);
		StakeVaultBalance::<T>::mutate(|n: &mut u128| *n += 10000000000000000000);
		let total_models = Network::<T>::total_models();
		let max_models = Network::<T>::max_models();

		// add models
		let m_models: u32 = max_models;
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		for m in total_models..m_models {
			let model_path = format!("petals-team-{m}/StableBeluga");
			Network::<T>::vote_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone().into());
			let _ = Network::<T>::add_model(RawOrigin::Signed(caller.clone()).into(), model_path.clone().into());
    }

		make_model_initialized::<T>();

		// increase blocks past consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));
		
		let consensus_blocks_interval = ConsensusBlocksInterval::<T>::get();

		// add model peers
		let n_peers: u8 = (Network::<T>::max_model_peers()) as u8;

		// use stake amounts that will create models(s) that will be greater than the max weight
		let amount_stake: Vec<u128> = vec![
      10000000000000000000,
      8000000000000000000,
      200000000000000000,
      300000000000000000,
      140000000000000000,
      120000000000000000,
      100000000000000000,
      200000000000000000,
      100000000000000000,
      100000000000000000,
			10000000000000000000,
      8000000000000000000,
      200000000000000000,
      300000000000000000,
      140000000000000000,
      120000000000000000,
      100000000000000000,
      200000000000000000,
      100000000000000000,
      100000000000000000,
			10000000000000000000,
      8000000000000000000,
      200000000000000000,
      300000000000000000,
      140000000000000000,
      120000000000000000,
      100000000000000000,
      200000000000000000,
      100000000000000000,
      100000000000000000,
			10000000000000000000,
      8000000000000000000,
      200000000000000000,
      300000000000000000,
      140000000000000000,
      120000000000000000,
      100000000000000000,
      200000000000000000,
      100000000000000000,
      100000000000000000,
			10000000000000000000,
      8000000000000000000,
      200000000000000000,
      300000000000000000,
      140000000000000000,
      120000000000000000,
      100000000000000000,
      200000000000000000,
      100000000000000000,
      100000000000000000,
			10000000000000000000,
      8000000000000000000,
      200000000000000000,
      300000000000000000,
      140000000000000000,
      120000000000000000,
      100000000000000000,
      200000000000000000,
      100000000000000000,
      100000000000000000,
      100000000000000000,
      200000000000000000,
      100000000000000000,
      100000000000000000,
    ];
		let mut amount_staked: u128 = 0;

		for m in total_models..m_models {
			let model_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
			let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();
			let stake_amount: u128 = amount_stake[m as usize] as u128;
			amount_staked += stake_amount;
			for n in 0..n_peers {
				let peer_account = funded_account::<T>("peer", n.into());
				whitelist_account!(peer_account);
				let __ = Network::<T>::add_model_peer(
					RawOrigin::Signed(peer_account.clone()).into(), 
					model_id.clone(), 
					peer(n), 
					"172.20.54.234".into(), 
					8888, 
					stake_amount
				);
			}	
    }

		// increase blocks for peers to submit data
		make_model_peer_consensus_data_submittable::<T>();
		// let block = frame_system::Pallet::<T>::block_number();
		// let min_required_peer_consensus_submit_epochs: u64 = Network::<T>::min_required_peer_consensus_submit_epochs();
		// frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(consensus_blocks_interval * min_required_peer_consensus_submit_epochs));

		// build consensus
		let model_peer_data = model_peer_data::<T>(0, n_peers);

		// increase blocks to surpass consensus steps
		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(block + u64_to_block::<T>(CONSENSUS_STEPS));

		for m in total_models..m_models {
			let model_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
			let model_id = ModelPaths::<T>::get(model_path.clone()).unwrap();

			for n in 0..n_peers {
				let peer_account = funded_account::<T>("peer", n.into());
				whitelist_account!(peer_account);
				Network::<T>::submit_consensus_data(RawOrigin::Signed(peer_account.clone()).into(), model_id.clone(), false, model_peer_data.clone());		
			}
			let submission = ModelPeerConsensusResults::<T>::iter_key_prefix(m + 1);
			let len = submission.count();

			// should match size of inclusion peers
			assert_eq!(
				len, 
				n_peers as usize, 
				"ModelPeerConsensusResults len mismatch."
			);	
    }

		let consensus_blocks_interval = u64_to_block::<T>(ConsensusBlocksInterval::<T>::get());
		let block = frame_system::Pallet::<T>::block_number();
    frame_system::Pallet::<T>::set_block_number(
      consensus_blocks_interval + (block - (block % consensus_blocks_interval))
    );    

		// call form_consensus
		Network::<T>::form_consensus(RawOrigin::Signed(caller.clone()).into());

		let block = frame_system::Pallet::<T>::block_number();
		frame_system::Pallet::<T>::set_block_number(
			consensus_blocks_interval + (block - (block % consensus_blocks_interval) + u64_to_block::<T>(1))
		);
	}: do_generate_emissions(RawOrigin::Signed(caller.clone()))
	verify {
		assert_eq!(Network::<T>::total_models(), max_models, "TotalModels incorrect.");
		let submission = ModelPeerConsensusResults::<T>::iter_key_prefix(1);
		let len = submission.count();
		// should be removed on on successful emissions emitting
		assert_eq!(
			len, 
			0 as usize, 
			"ModelPeerConsensusResults len mismatch."
		);

		let expected_max_post_vault_balance: u128 = (amount_staked as f64 * 0.01) as u128;
    let post_total_vault_balance: u128 = StakeVaultBalance::<T>::get();
		// log::info!("expected_max_post_vault_balance {:?}", expected_max_post_vault_balance);
		// log::error!("expected_max_post_vault_balance {:?}", expected_max_post_vault_balance);
		// log::info!("post_total_vault_balance        {:?}", post_total_vault_balance);
		// log::error!("post_total_vault_balance        {:?}", post_total_vault_balance);
		assert!(post_total_vault_balance > 0, "Incorrect expected stake balance.");
    // assert!(post_total_vault_balance <= expected_max_post_vault_balance, "Incorrect expected stake balance.");
	}

	form_consensus_no_consensus_weight_test {
		let total_models = Network::<T>::total_models();
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);

		let consensus_blocks_interval = u64_to_block::<T>(ConsensusBlocksInterval::<T>::get());
		let block = frame_system::Pallet::<T>::block_number();
    frame_system::Pallet::<T>::set_block_number(
      consensus_blocks_interval + (block - (block % consensus_blocks_interval))
    );
	}: form_consensus_no_consensus_weight_test(RawOrigin::Signed(caller.clone()))
	verify {
		assert_eq!(Network::<T>::total_models(), total_models, "TotalModels incorrect.");
	}


	impl_benchmark_test_suite!(
		Network,
		crate::mock::new_test_ext(),
		crate::mock::Test
	);
}