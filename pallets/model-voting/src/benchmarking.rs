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
// ./target/release/node-template benchmark pallet --chain=dev --wasm-execution=compiled --pallet=pallet_model_voting --extrinsic=* --steps=50 --repeat=20 --output="pallets/model-voting/src/weights.rs" --template ./.maintain/frame-weight-template.hbs

use super::*;
use frame_benchmarking::{account, benchmarks, whitelist_account, BenchmarkError};
use frame_support::{
	assert_noop, assert_ok, assert_err,
	traits::{Currency, EnsureOrigin, Get, OnInitialize, UnfilteredDispatchable},
};
use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
use crate::Pallet as ModelVoting;
use crate::{
  ModelPeer, PropsType, ModelVote, VotesBalance, ReservableCurrency, PropCount, VoteType,
  Votes, ActiveProposals, Proposals, PropsStatus, Quorum
};
use frame_support::dispatch::Vec;
use scale_info::prelude::{vec, format};
use pallet_network::{MinStakeBalance, MinModelPeers};
// use pallet_balances::*;

const SEED: u32 = 0;
const DEFAULT_IP: &str = "172.2.54.234";
const DEFAULT_PORT: u16 = 5000;
const DEFAULT_DEPOSIT_AMOUNT: u128 = 10000000000000000000000; // 10,000
const DEFAULT_MODEL_PATH: &str = "hf/llama2";
const DEFAULT_EXISTING_MODEL_PATH: &str = "hf/baluga";
const DEFAUT_VOTE_AMOUNT: u128 = 1000e+18 as u128;

fn peer(id: u32) -> PeerId {
  let peer_id = format!("QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N{id}"); 
	PeerId(peer_id.into())
}

fn default_model_path() -> Vec<u8> {
  DEFAULT_MODEL_PATH.into()
}

fn default_ip() -> Vec<u8> {
  DEFAULT_IP.into()
}

pub fn u64_to_block<T: frame_system::Config>(input: u64) -> BlockNumberFor<T> {
	input.try_into().ok().expect("REASON")
}

pub fn block_to_u64<T: frame_system::Config>(input: BlockNumberFor<T>) -> u64 {
	input.try_into().ok().expect("REASON")
}

fn funded_account<T: Config>(name: &'static str, index: u32) -> T::AccountId {
	let caller: T::AccountId = account(name, index, SEED);
	let deposit_amount: u128 = T::ModelVote::get_model_initialization_cost();
  T::Currency::deposit_creating(&caller, deposit_amount.try_into().ok().expect("REASON"));
	caller
}

fn build_model_peers<T: Config>(start: u32, end: u32, deposit_amount: u128) -> Vec<ModelPeer<T::AccountId>> {
  let mut model_peers: Vec<ModelPeer<T::AccountId>> = Vec::new();
  
  for n in start..end {
    let _ = T::Currency::deposit_creating(&funded_account::<T>("voter", n), deposit_amount.try_into().ok().expect("REASON"));
    let model_peer = ModelPeer {
      account_id: funded_account::<T>("voter", n),
      peer_id: peer(n),
      ip: default_ip(),
      port: DEFAULT_PORT,
    };
    model_peers.push(model_peer);
  }
  model_peers
}

fn post_proposal_concluded<T: Config>(proposal_index: u32, proposer: T::AccountId) {
  let active_proposals = ActiveProposals::<T>::get();
  assert_eq!(active_proposals, proposal_index);

  // --- Ensure cannot call twice
  assert_err!(
    ModelVoting::<T>::execute(
      RawOrigin::Signed(proposer.clone()).into(),
      proposal_index,
    ),
    Error::<T>::Concluded
  );

  // --- Ensure cannot cast vote
  assert_err!(
    ModelVoting::<T>::cast_vote(
      RawOrigin::Signed(proposer.clone()).into(),
      proposal_index,
      DEFAUT_VOTE_AMOUNT.try_into().ok().expect("REASON"),
      VoteType::Yay,
    ),
    Error::<T>::VotingNotOpen
  );

  // --- Ensure cannot cancel proposal
  assert_err!(
    ModelVoting::<T>::cancel_proposal(
      RawOrigin::Signed(proposer.clone()).into(),
      proposal_index,
    ),
    Error::<T>::Concluded
  );
}

fn post_success_proposal_activate_ensures<T: Config>(path: Vec<u8>, proposal_index: u32, proposer: T::AccountId, proposal_start_block: u64) {
  let proposal = Proposals::<T>::get(proposal_index);
  assert_eq!(proposal.path, path);
  assert_eq!(proposal.proposal_status, PropsStatus::Active);
  assert_eq!(proposal.proposal_type, PropsType::Activate);
  // assert_eq!(proposal.model_peers, path);
  assert_eq!(proposal.max_block, proposal_start_block + block_to_u64::<T>(T::VotingPeriod::get()));

  let model_initialization_cost = T::ModelVote::get_model_initialization_cost();
  // assert_eq!(VotesBalance::<T>::get(proposal_index, proposer), model_initialization_cost.clone());

  // let reserved_balance = <pallet_balances::Pallet<T> as ReservableCurrency<T>>::reserved_balance(&proposer);
  // let reserved_balance = ReservableCurrency::reserved_balance(&proposer);
  // assert_eq!(reserved_balance, model_initialization_cost.clone());

  let active_proposals = ActiveProposals::<T>::get();
  assert_eq!(active_proposals, proposal_index + 1);
}

fn post_activate_cancel_ensures<T: Config>(proposal_index: u32, proposer: T::AccountId, path: Vec<u8>) {
  let is_active = T::ModelVote::get_model_path_exist(path);
  // assert_eq!(is_active, None);

  let proposal = Proposals::<T>::get(proposal_index);
  assert_eq!(proposal.proposal_status, PropsStatus::Cancelled);

  post_proposal_concluded::<T>(proposal_index, proposer);
}

fn post_cast_vote_ensures<T: Config>(proposal_index: u32, voter: u32) {
  assert_err!(
    ModelVoting::<T>::unreserve(
      RawOrigin::Signed(funded_account::<T>("voter", voter)).into(),
      proposal_index, 
    ),
    Error::<T>::ProposalInvalid
  );
}

fn build_propose_activate<T: Config>(path: Vec<u8>, start: u32, end: u32, deposit_amount: u128) -> u32 {
  let model_peers = build_model_peers::<T>(start, end, deposit_amount);
  let proposer = funded_account::<T>("account", 0);

  assert_ok!(
    ModelVoting::<T>::propose(
      RawOrigin::Signed(proposer.clone()).into(),
      default_model_path(), 
      model_peers,
      PropsType::Activate,
    )
  );
  0
}

fn build_cast_vote<T: Config>(proposal_index: u32, start: u32, end: u32, vote: VoteType) {
  for n in start..end {
    let voter = funded_account::<T>("voter", n);
    let _ = T::Currency::deposit_creating(&funded_account::<T>("voter", n), DEFAUT_VOTE_AMOUNT.try_into().ok().expect("REASON"),);
    assert_ok!(
      ModelVoting::<T>::cast_vote(
        RawOrigin::Signed(voter).into(),
        proposal_index,
        DEFAUT_VOTE_AMOUNT.try_into().ok().expect("REASON"),
        vote.clone(),
      )
    );
  }
}

benchmarks! {
  propose {
    let prop_count = PropCount::<T>::get();
    let min_stake = T::ModelVote::get_min_stake_balance();
    let min_model_peers: u32 = T::ModelVote::get_min_model_peers();
		let proposer = funded_account::<T>("account", 0);
    let model_peers = build_model_peers::<T>(0, min_model_peers, min_stake);
	}: propose(RawOrigin::Signed(proposer.clone()), default_model_path(), model_peers, PropsType::Activate)
	verify {
    assert_eq!(1, 1);
		post_success_proposal_activate_ensures::<T>(
      default_model_path(), 
      prop_count, 
      proposer.clone(), 
      block_to_u64::<T>(frame_system::Pallet::<T>::block_number())
    )
	}

  cast_vote {
    let prop_count = PropCount::<T>::get();
    let min_stake = T::ModelVote::get_min_stake_balance();
    let min_model_peers: u32 = T::ModelVote::get_min_model_peers();
		let voter = funded_account::<T>("voter", 0);
    let model_peers = build_model_peers::<T>(0, min_model_peers, min_stake);
    let proposal_index = build_propose_activate::<T>(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);
	}: cast_vote(RawOrigin::Signed(voter.clone()), proposal_index, DEFAUT_VOTE_AMOUNT.try_into().ok().expect("REASON"), VoteType::Yay)
	verify {
    assert_eq!(1, 1);
    post_cast_vote_ensures::<T>(proposal_index, 0)
  }

  execute {
    let prop_count = PropCount::<T>::get();
    let min_stake = T::ModelVote::get_min_stake_balance();
    let min_model_peers: u32 = T::ModelVote::get_min_model_peers();
		let voter = funded_account::<T>("voter", 0);
    let model_peers = build_model_peers::<T>(0, min_model_peers, min_stake);
    let proposal_index = build_propose_activate::<T>(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);
    build_cast_vote::<T>(proposal_index, 0, min_model_peers, VoteType::Yay);
	}: execute(RawOrigin::Signed(voter.clone()), proposal_index)
	verify {
    assert_eq!(1, 1);
  }

  cancel_proposal {
    let prop_count = PropCount::<T>::get();
    let min_stake = T::ModelVote::get_min_stake_balance();
    let min_model_peers: u32 = T::ModelVote::get_min_model_peers();
		let voter = funded_account::<T>("voter", 0);
    let model_peers = build_model_peers::<T>(0, min_model_peers, min_stake);
    let proposal_index = build_propose_activate::<T>(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);
	}: cancel_proposal(RawOrigin::Signed(voter.clone()), proposal_index)
	verify {
    assert_eq!(1, 1);
  }

  unreserve {
    let prop_count = PropCount::<T>::get();
    let min_stake = T::ModelVote::get_min_stake_balance();
    let min_model_peers: u32 = T::ModelVote::get_min_model_peers();
		let voter = funded_account::<T>("voter", 0);
    let model_peers = build_model_peers::<T>(0, min_model_peers, min_stake);
    let proposal_index = build_propose_activate::<T>(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);
	}: unreserve(RawOrigin::Signed(voter.clone()), proposal_index)
	verify {
    assert_eq!(1, 1);
  }

  impl_benchmark_test_suite!(
		ModelVoting,
		crate::mock::new_test_ext(),
		crate::mock::Test
	);
}