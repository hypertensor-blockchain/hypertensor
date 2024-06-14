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

#![cfg(test)]
use crate::mock::*;
use crate::Error;
use crate::ModelPeerData;
// use frame_support::traits::OriginTrait;
use sp_core::OpaquePeerId as PeerId;
use frame_support::{
	assert_noop, assert_ok, assert_err
};
use log::info;
use sp_core::{H256, U256};
// use parity_scale_codec::Decode;
use frame_support::traits::Currency;
use crate::{
  ModelPeerConsensusResults, AccountPenaltyCount, TotalStake, 
  StakeVaultBalance, ModelPaths, ConsensusBlocksInterval, PeerRemovalThreshold,
  MinRequiredUnstakeEpochs, MaxAccountPenaltyCount, MinModelPeers,
  ModelConsensusUnconfirmedThreshold, ModelPeersData, ModelPeerAccount,
  ModelAccount, ModelConsensusEpochsErrors, RemoveModelPeerEpochPercentage,
  PeerConsensusEpochSubmitted, MinRequiredPeerConsensusInclusionEpochs,
  PeerConsensusEpochUnconfirmed, AccountModelStake, MinStakeBalance,
  ModelTotalConsensusSubmits, PeerAgainstConsensusRemovalThreshold,
  ModelConsensusEpochUnconfirmedCount, ModelsInConsensus,
  MaxModelConsensusUnconfirmedConsecutiveEpochs, ModelConsensusUnconfirmedConsecutiveEpochsCount,
  DishonestyVotingPeriod, ModelPeerDishonestyVote
};
use frame_support::weights::Pays;

type AccountIdOf<Test> = <Test as frame_system::Config>::AccountId;
// type PeerIdOf<Test> = PeerId;

fn account(id: u32) -> AccountIdOf<Test> {
	[id as u8; 32].into()
}

// it is possible to use `use libp2p::PeerId;` with `PeerId::random()`
// https://github.com/paritytech/substrate/blob/033d4e86cc7eff0066cd376b9375f815761d653c/frame/node-authorization/src/mock.rs#L90
// fn peer(id: u8) -> PeerId {
// 	PeerId(vec![id])
// }

fn peer(id: u32) -> PeerId {
   
	// let peer_id = format!("12D3KooWD3eckifWpRn9wQpMG9R9hX3sD158z7EqHWmweQAJU5SA{id}");
  let peer_id = format!("QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N{id}"); 
	PeerId(peer_id.into())
}
// bafzbeie5745rpv2m6tjyuugywy4d5ewrqgqqhfnf445he3omzpjbx5xqxe
// QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N
// 12D3KooWD3eckifWpRn9wQpMG9R9hX3sD158z7EqHWmweQAJU5SA

fn get_min_stake_balance() -> u128 {
	MinStakeBalance::<Test>::get()
}

const PERCENTAGE_FACTOR: u128 = 10000;
const DEFAULT_SCORE: u128 = 5000;
const CONSENSUS_STEPS: u64 = 2;

fn build_model(model_path: Vec<u8>) {
  assert_ok!(
    Network::vote_model(
      RuntimeOrigin::signed(account(0)), 
      model_path.clone(),
    )
  );

  assert_ok!(
    Network::add_model(
      RuntimeOrigin::signed(account(0)),
      model_path.clone(),
    ) 
  );
}

// Returns total staked on model
fn build_model_peers(model_id: u32, start: u32, end: u32, deposit_amount: u128, amount: u128) -> u128 {
  let mut amount_staked = 0;
  for n in start..end {
    let _ = Balances::deposit_creating(&account(n), deposit_amount);
    amount_staked += amount;
    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(n)),
        model_id,
        peer(n),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );
    post_successful_add_model_peer_asserts(n, model_id, amount);
  }
  amount_staked
}

fn build_for_submit_consensus_data(model_id: u32, start: u32, end: u32, start_data: u32, end_data: u32) {
  let model_peer_data_vec = model_peer_data(start_data, end_data);

  for n in start..end {
    assert_ok!(
      Network::submit_consensus_data(
        RuntimeOrigin::signed(account(n)),
        model_id,
        model_peer_data_vec.clone(),
      ) 
    );
  }
}

fn make_model_submittable() {
  // increase blocks
  let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
  let min_required_model_consensus_submit_epochs: u64 = Network::min_required_model_consensus_submit_epochs();
  System::set_block_number(System::block_number() + consensus_blocks_interval * min_required_model_consensus_submit_epochs);
}

// increase the blocks past the consensus steps and remove model peer blocks span
fn make_consensus_data_submittable() {
  // increase blocks
  let current_block_number = System::block_number();
  let model_peer_removal_percentage = RemoveModelPeerEpochPercentage::<Test>::get();
  let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();

  let start_block_can_remove_peer = consensus_blocks_interval as u128 * model_peer_removal_percentage / PERCENTAGE_FACTOR;

  let max_remove_model_peer_block = start_block_can_remove_peer as u64 + (current_block_number - (current_block_number % consensus_blocks_interval));

  if current_block_number < max_remove_model_peer_block {
    System::set_block_number(max_remove_model_peer_block + 1);
  }
}

fn make_model_peer_included() {
  let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
  let min_required_consensus_inclusion_epochs = MinRequiredPeerConsensusInclusionEpochs::<Test>::get();
  System::set_block_number(System::block_number() + consensus_blocks_interval * min_required_consensus_inclusion_epochs);
}

fn make_model_peer_consensus_data_submittable() {
  // increase blocks
  let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
  let min_required_peer_consensus_submit_epochs: u64 = Network::min_required_peer_consensus_submit_epochs();
  System::set_block_number(System::block_number() + consensus_blocks_interval * min_required_peer_consensus_submit_epochs);
  make_consensus_data_submittable();
}

fn make_model_peer_removable() {
  // increase blocks
  let current_block_number = System::block_number();
  let model_peer_removal_percentage = RemoveModelPeerEpochPercentage::<Test>::get();
  let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();

  let block_span_can_remove_peer = (consensus_blocks_interval as u128 * model_peer_removal_percentage / PERCENTAGE_FACTOR) as u64;

  let start_removal_block = (CONSENSUS_STEPS + (current_block_number - (current_block_number % consensus_blocks_interval))) as u64;

  let end_removal_block = block_span_can_remove_peer + (current_block_number - (current_block_number % consensus_blocks_interval));
  
  if current_block_number < start_removal_block {
    System::set_block_number(start_removal_block);
  } else if current_block_number > end_removal_block {
    System::set_block_number(start_removal_block + consensus_blocks_interval);
  }
}

// fn model_peer_data(start: u8, end: u8) -> Vec<ModelPeerData<<Test as frame_system::Config>::AccountId>> {
fn model_peer_data(start: u32, end: u32) -> Vec<ModelPeerData> {
  // initialize peer consensus data array
  let mut model_peer_data: Vec<ModelPeerData> = Vec::new();
  for n in start..end {
    // let peer_model_peer_data: ModelPeerData<<Test as frame_system::Config>::AccountId> = ModelPeerData {
    //   // account_id: account(n),
    //   peer_id: peer(n),
    //   score: DEFAULT_SCORE,
    // };
    let peer_model_peer_data: ModelPeerData = ModelPeerData {
      peer_id: peer(n),
      score: DEFAULT_SCORE,
    };
    model_peer_data.push(peer_model_peer_data);
  }
  model_peer_data
}

// fn model_peer_data_invalid_scores(start: u8, end: u8) -> Vec<ModelPeerData<<Test as frame_system::Config>::AccountId>> {
fn model_peer_data_invalid_scores(start: u32, end: u32) -> Vec<ModelPeerData> {
  // initialize peer consensus data array
  // let mut model_peer_data: Vec<ModelPeerData<<Test as frame_system::Config>::AccountId>> = Vec::new();
  let mut model_peer_data: Vec<ModelPeerData> = Vec::new();
  for n in start..end {
    // let peer_model_peer_data: ModelPeerData<<Test as frame_system::Config>::AccountId> = ModelPeerData {
    //   // account_id: account(n),
    //   peer_id: peer(n),
    //   score: 10000000000,
    // };
    let peer_model_peer_data: ModelPeerData = ModelPeerData {
      peer_id: peer(n),
      score: 10000000000,
    };
    model_peer_data.push(peer_model_peer_data);
  }
  model_peer_data
}

fn post_successful_add_model_peer_asserts(
  n: u32, 
  model_id: u32, 
  amount: u128
) {
  assert_eq!(Network::account_model_stake(account(n), model_id), amount);
  assert_eq!(Network::total_account_stake(account(n)), amount);    
  assert_eq!(Network::total_model_peers(model_id), (n + 1) as u32);
}

// check data after adding multiple peers
// each peer must have equal staking amount per model
fn post_successful_add_model_peers_asserts(
  total_peers: u32,
  stake_per_peer: u128,  
  model_id: u32, 
) {
  let amount_staked = total_peers as u128 * stake_per_peer;
  assert_eq!(Network::total_model_stake(model_id), amount_staked);
}

fn post_remove_model_peer_ensures(n: u32, model_id: u32) {
  // ensure ModelPeersData removed
  let model_peer_data = ModelPeersData::<Test>::try_get(model_id, account(n));
  assert_eq!(model_peer_data, Err(()));

  // ensure ModelPeerAccount removed
  let model_peer_account = ModelPeerAccount::<Test>::try_get(model_id, peer(n));
  assert_eq!(model_peer_account, Err(()));

  // ensure ModelPeerConsensusResults removed
  let model_peer_consensus_results = ModelPeerConsensusResults::<Test>::try_get(model_id, account(n));
  assert_eq!(model_peer_consensus_results, Err(()));

  // ensure ModelAccount u64 updated to current block
  let model_accounts = ModelAccount::<Test>::get(model_id.clone());
  let model_account = model_accounts.get(&account(n));
  assert_eq!(model_accounts.get(&account(n)), Some(&System::block_number()));
}

fn post_remove_unstake_ensures(n: u32, model_id: u32) {
  // ensure ModelAccount is removed after unstaking to 0
  let model_accounts = ModelAccount::<Test>::get(model_id.clone());
  let model_account = model_accounts.get(&account(n));
  assert_eq!(model_accounts.get(&account(n)), None);
}

// The following should be ensured after form_consensus is rate
// This should work regardless if there are consensus issues or not
fn post_successful_form_consensus_ensures(model_id: u32) {
  let peer_consensus_epoch_submitted = PeerConsensusEpochSubmitted::<Test>::iter().count();
  assert_eq!(peer_consensus_epoch_submitted, 0);
  let peer_consensus_epoch_confirmed = PeerConsensusEpochUnconfirmed::<Test>::iter().count();
  assert_eq!(peer_consensus_epoch_confirmed, 0);
  let model_total_consensus_submits = ModelTotalConsensusSubmits::<Test>::iter().count();
  assert_eq!(model_total_consensus_submits, 0);
  let model_consensus_epoch_unconfirmed_count = ModelConsensusEpochUnconfirmedCount::<Test>::try_get(model_id.clone());
  assert_eq!(model_consensus_epoch_unconfirmed_count, Err(()));
}

fn post_successful_generate_emissions_ensures() {
  let models_in_consensus = ModelsInConsensus::<Test>::try_get();
  assert_eq!(models_in_consensus, Err(()));

  let models_in_consensus = ModelsInConsensus::<Test>::get();
  assert_eq!(models_in_consensus.len(), 0);


  let model_peer_consensus_results = ModelPeerConsensusResults::<Test>::iter().count();
  assert_eq!(model_peer_consensus_results, 0);
}

fn post_successful_dishonesty_proposal_ensures(proposer: u32, votee: u32, model_id: u32) {
  let model_peer_dishonesty_vote = ModelPeerDishonestyVote::<Test>::get(model_id.clone(), account(votee));

  assert_eq!(model_peer_dishonesty_vote.model_id, model_id.clone());
  assert_eq!(model_peer_dishonesty_vote.peer_id, peer(votee).into());
  assert_eq!(model_peer_dishonesty_vote.total_votes, 1);
  assert_eq!(model_peer_dishonesty_vote.votes[0], account(proposer));
  assert_ne!(model_peer_dishonesty_vote.start_block, 0);
}

fn add_model_peer(
  account_id: u32, 
  model_id: u32,
  peer_id: u32,
  ip: String,
  port: u16,
  amount: u128
) -> Result<(), sp_runtime::DispatchError> {
  Network::add_model_peer(
    RuntimeOrigin::signed(account(account_id)),
    model_id,
    peer(peer_id),
    ip.into(),
    port,
    amount,
  )
}

#[test]
fn test_add_model() {
  new_test_ext().execute_with(|| {

    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());

    assert_eq!(Network::total_models(), 1);

    let model_path_2: Vec<u8> = "petals-team-2/StableBeluga2".into();

    build_model(model_path_2.clone());

    assert_eq!(Network::total_models(), 2);

  })
}

#[test]
fn test_add_model_err() {
  new_test_ext().execute_with(|| {

    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    assert_err!(
      Network::add_model(
        RuntimeOrigin::signed(account(0)),
        model_path.clone(),
      ),
      Error::<Test>::ModelNotVotedIn
    );

    build_model(model_path.clone());

    assert_eq!(Network::total_models(), 1);

    assert_err!(
      Network::add_model(
        RuntimeOrigin::signed(account(0)),
        model_path.clone(),
      ),
      Error::<Test>::ModelExist
    );
  })
}

#[test]
fn test_remove_model() {
  new_test_ext().execute_with(|| {

    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());

    assert_eq!(Network::total_models(), 1);

    assert_ok!(
      Network::vote_model_out(
        RuntimeOrigin::signed(account(0)), 
        model_path.clone()
      )
    );

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    assert_ok!(
      Network::remove_model(
        RuntimeOrigin::signed(account(0)),
        model_id,
      ) 
    );

    assert_eq!(Network::total_models(), 1);
  })
}

#[test]
fn test_remove_model_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());

    assert_eq!(Network::total_models(), 1);

    assert_err!(
      Network::remove_model(
        RuntimeOrigin::signed(account(0)),
        255,
      ),
      Error::<Test>::ModelNotExist
    );
  })
}

#[test]
fn test_add_model_max_models_err() {
  new_test_ext().execute_with(|| {
    let n_models: u32 = Network::max_models() + 1;

    for m in 0..n_models {
      let model_path = format!("petals-team-{m}/StableBeluga");  
      assert_ok!(
        Network::vote_model(
          RuntimeOrigin::signed(account(0)), 
          model_path.clone().into()
        )
      );
  
      if m+1 < n_models {
        assert_ok!(
          Network::add_model(
            RuntimeOrigin::signed(account(0)),
            model_path.clone().into(),
          ) 
        );
      } else {
        assert_err!(
          Network::add_model(
            RuntimeOrigin::signed(account(0)),
            model_path.clone().into(),
          ),
          Error::<Test>::MaxModels
        );
      }
    }
  })
}

#[test]
fn test_add_model_peer_max_peers_err() {
  new_test_ext().execute_with(|| {
    let n_peers: u32 = Network::max_model_peers() + 1;
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());

    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let mut total_staked: u128 = 0;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);
    for n in 0..n_peers {
      let _ = Balances::deposit_creating(&account(n), deposit_amount);

      if n+1 < n_peers {
        total_staked += amount;
        assert_ok!(
          add_model_peer(
            n, 
            model_id.clone(),
            n,
            "172.20.54.234".into(),
            8888,
            amount
          )  
        );

        // assert_ok!(
        //   Network::add_model_peer(
        //     RuntimeOrigin::signed(account(n)),
        //     model_id.clone(),
        //     peer(n),
        //     "172.20.54.234".into(),
        //     8888,
        //     amount,
        //   ) 
        // );
        assert_eq!(Network::total_model_peers(1), (n + 1) as u32);
        assert_eq!(Network::account_model_stake(account(n), 1), amount);
        assert_eq!(Network::total_account_stake(account(n)), amount);
      } else {
        assert_err!(
          Network::add_model_peer(
            RuntimeOrigin::signed(account(n)),
            model_id.clone(),
            peer(n),
            "172.20.54.234".into(),
            8888,
            amount,
          ),
          Error::<Test>::ModelPeersMax
        );
      }
    }

    assert_eq!(Network::total_stake(), total_staked);
    assert_eq!(Network::total_model_stake(1), total_staked);
  });
}

#[test]
fn test_add_model_peer_model_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount: u128 = 1000;
    assert_err!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        0,
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ),
      Error::<Test>::ModelNotExist
    );

    assert_eq!(Network::total_model_peers(1), 0);

  })
}

#[test]
fn test_add_model_peer_model_account_ineligible_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();
    let max_account_penalty_count = MaxAccountPenaltyCount::<Test>::get();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount: u128 = 1000;

    AccountPenaltyCount::<Test>::insert(account(0), max_account_penalty_count + 1);

    assert_err!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ),
      Error::<Test>::AccountIneligible
    );

    assert_eq!(Network::total_model_peers(1), 0);
  })
}

#[test]
fn test_add_model_peer_not_exists_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );
    assert_eq!(Network::total_model_peers(1), 1);

    // add new peer_id under same account error
    assert_err!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(1),
        "172.20.54.234".into(),
        8888,
        amount,
      ),
      Error::<Test>::ModelPeerExist
    );

    assert_eq!(Network::total_model_peers(1), 1);

    // add same peer_id under new account error
    assert_err!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(1)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ),
      Error::<Test>::PeerIdExist
    );

    assert_eq!(Network::total_model_peers(1), 1);

    // add new peer_id under same account error
    assert_err!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(1),
        "172.20.54.234".into(),
        8888,
        amount,
      ),
      Error::<Test>::ModelPeerExist
    );
  })
}

#[test]
fn test_add_model_peer_stake_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 100000;
    let amount: u128 = 1;

    let _ = Balances::deposit_creating(&account(0), deposit_amount);
    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ),
      Error::<Test>::MinStakeNotReached
    );

  })
}

#[test]
fn test_add_model_peer_stake_not_enough_balance_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 999999999999999999999;
    let amount: u128 =         1000000000000000000000;

    // let deposit_amount: u128 = 999;
    // let amount: u128 = 1000;

    let _ = Balances::deposit_creating(&account(0), deposit_amount);
    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ),
      Error::<Test>::NotEnoughBalanceToStake
    );

  })
}

#[test]
fn test_add_model_peer_invalid_peer_id_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let n_peers: u32 = Network::max_model_peers();

    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let _ = Balances::deposit_creating(&account(0), deposit_amount);
    amount_staked += amount;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let peer_id = format!("2");
    let peer: PeerId = PeerId(peer_id.into());
    assert_err!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer,
        "172.20.54.234".into(),
        8888,
        amount,
      ),
      Error::<Test>::InvalidPeerId
    );
  })
}

#[test]
fn test_add_model_peer_invalid_ip_address_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let n_peers: u32 = Network::max_model_peers();

    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let _ = Balances::deposit_creating(&account(0), deposit_amount);
    amount_staked += amount;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(0),
        "0.0.0".into(),
        8888,
        amount,
      ),
      Error::<Test>::InvalidIpAddress
    );

    assert_err!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(0),
        "127.0.0.1".into(),
        8888,
        amount,
      ),
      Error::<Test>::InvalidIpAddress
    );

  })
}

#[test]
fn test_add_model_peer_remove_readd_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      )
    );

    assert_ok!(
      Network::remove_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
      )
    );

    assert_err!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ), 
      Error::<Test>::RequiredUnstakeEpochsNotMet
    );
  });
}

#[test]
fn test_add_model_peer_remove_readd() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      )
    );

    assert_ok!(
      Network::remove_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
      )
    );

    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<Test>::get();

    System::set_block_number(System::block_number() + consensus_blocks_interval * min_required_unstake_epochs);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );
  });
}

#[test]
fn test_add_model_peer_remove_stake_partial_readd() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      )
    );

    // increase account model stake to simulate rewards
    AccountModelStake::<Test>::insert(&account(0), model_id.clone(), amount + 100);

    assert_ok!(
      Network::remove_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
      )
    );

    // once blocks have been increased, account can either remove stake in part or in full or readd model peer
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<Test>::get();

    System::set_block_number(System::block_number() + consensus_blocks_interval * min_required_unstake_epochs);

    assert_ok!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        amount,
      )
    );

    // should be able to readd after unstaking
    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );
  });
}

#[test]
fn test_add_model_peer_remove_stake_readd() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      )
    );

    assert_ok!(
      Network::remove_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
      )
    );

    // once blocks have been increased, account can either remove stake in part or in full or readd model peer
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<Test>::get();
    System::set_block_number(System::block_number() + consensus_blocks_interval * min_required_unstake_epochs);

    let remaining_account_stake_balance: u128 = AccountModelStake::<Test>::get(&account(0), model_id.clone());

    assert_ok!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        remaining_account_stake_balance,
      )
    );

    // should be able to readd after unstaking
    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );
  });
}

#[test]
fn test_add_model_peer() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let n_peers: u32 = Network::max_model_peers();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    assert_eq!(Network::total_model_stake(model_id.clone()), amount_staked);
  })
}

#[test]
fn test_update_model_peer_peer_id_existing_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      )
    );

    assert_err!(
      Network::update_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(0),
      ),
      Error::<Test>::PeerIdExist
    );
  });
}

#[test]
fn test_update_model_peer_invalid_epoch_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    // Update the interval in case its too small
    // if the removal/update span is 2 blocks or less 
    // the consensus steps logic will conflict and testing won't pass
    // For anyone reading this, this doesn't impact logic but we expect
    // a specific Error to be returned
    ConsensusBlocksInterval::<Test>::set(100);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      )
    );

    make_model_peer_removable();

    System::set_block_number(System::block_number() - CONSENSUS_STEPS);

    assert_err!(
      Network::update_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(1),
      ),
      Error::<Test>::InvalidRemoveOrUpdateModelPeerBlock
    );
  });
}

#[test]
fn test_update_model_peer_during_invalid_block_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      )
    );

    make_model_peer_consensus_data_submittable();

    assert_err!(
      Network::update_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(1),
      ),
      Error::<Test>::InvalidRemoveOrUpdateModelPeerBlock
    );
  });
}

#[test]
fn test_update_model_peer_during_submit_epoch_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      )
    );

    make_model_peer_removable();

    assert_err!(
      Network::update_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(1),
      ),
      Error::<Test>::PeerConsensusSubmitEpochNotReached
    );
  });
}

#[test]
fn test_update_model_peer() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      )
    );

    make_model_peer_consensus_data_submittable();

    make_model_peer_removable();

    assert_ok!(
      Network::update_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id,
        peer(1),
      )
    );
  });
}

#[test]
fn test_update_port_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);  

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let _ = Balances::deposit_creating(&account(0), deposit_amount);
    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      )
    );
    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);    
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);
    assert_eq!(Network::total_model_peers(1), 1);

    // invalid account
    assert_err!(
      Network::update_port(
        RuntimeOrigin::signed(account(255)),
        model_id.clone(),
        65535,
      ),
      Error::<Test>::ModelPeerNotExist
    );

    // invalid model
    assert_err!(
      Network::update_port(
        RuntimeOrigin::signed(account(0)),
        255,
        8889,
      ),
      Error::<Test>::ModelNotExist
    );


  })
}

#[test]
fn test_update_port() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    assert_eq!(Network::total_models(), 1);  

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let _ = Balances::deposit_creating(&account(0), deposit_amount);
    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );
    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);    
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);
    assert_eq!(Network::total_model_peers(1), 1);

    assert_ok!(
      Network::update_port(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        65535,
      )
    );
  })
}

#[test]
fn test_submit_consensus_min_required_model_epochs() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();
    let min_model_peers: u32 = Network::min_model_peers();
    let n_peers: u32 = min_model_peers;

    build_model(model_path.clone());

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    // System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);

    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    assert_eq!(Network::total_stake(), amount_staked);

    // make_model_peer_removable();

    // System::set_block_number(System::block_number() + CONSENSUS_STEPS + 1);

    make_consensus_data_submittable();

    // initialize peer consensus data array
    let model_peer_data_vec = model_peer_data(0, n_peers);

    assert_err!(
      Network::submit_consensus_data(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        model_peer_data_vec.clone(),
      ),
      Error::<Test>::ModelInitializeRequirement
    );

    // let scd = Network::submit_consensus_data(
    //   RuntimeOrigin::signed(account(0)),
    //   model_id.clone(),
    //   model_peer_data_vec.clone(),
    // );

    // assert_eq!(scd.unwrap().pays_fee, Pays::Yes);
  });
}

#[test]
fn test_submit_consensus_min_required_peer_epochs() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();
    let min_model_peers: u32 = Network::min_model_peers();
    let n_peers: u32 = min_model_peers;

    build_model(model_path.clone());
    make_model_submittable();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    make_consensus_data_submittable();

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);

    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    assert_eq!(Network::total_stake(), amount_staked);

    // initialize peer consensus data array
    let model_peer_data_vec = model_peer_data(0, n_peers);

    assert_err!(
      Network::submit_consensus_data(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        model_peer_data_vec.clone(),
      ),
      Error::<Test>::PeerConsensusSubmitEpochNotReached
    );
  });
}

#[test]
fn test_submit_consensus_min_model_peers_err() {
  new_test_ext().execute_with(|| {
    // add first model
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_model(model_path.clone());
    make_model_submittable();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      )
    );
    post_successful_add_model_peer_asserts(0, model_id.clone(), amount);

    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);

    make_model_peer_consensus_data_submittable();
        
    // initialize peer consensus data array
    let model_peer_data_vec = model_peer_data(0, 1);

    assert_err!(
      Network::submit_consensus_data(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        model_peer_data_vec.clone(),
      ),
      Error::<Test>::ModelPeersMin
    );
  });
}


#[test]
fn test_submit_consensus_len_err() {
  new_test_ext().execute_with(|| {
    // add first model
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_model(model_path.clone());
    make_model_submittable();

    let n_peers: u32 = Network::max_model_peers();
    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;
    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    // initialize peer consensus data array
    let model_peer_data_vec = model_peer_data(0, n_peers + 1);

    assert_err!(
      Network::submit_consensus_data(
        RuntimeOrigin::signed(account(0)),
        model_id,
        model_peer_data_vec.clone(),
      ),
      Error::<Test>::ConsensusDataInvalidLen
    );
  });
}

#[test]
fn test_submit_consensus_min_peers_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let n_peers: u32 = Network::min_model_peers() - 1;


    build_model(model_path.clone());
    make_model_submittable();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    // initialize peer consensus data array
    let model_peer_data_vec = model_peer_data(0, n_peers);

    for n in 0..n_peers {
      assert_err!(
        Network::submit_consensus_data(
          RuntimeOrigin::signed(account(n)),
          model_id,
          model_peer_data_vec.clone(),
        ) ,
        Error::<Test>::ModelPeersMin
      );
    }
  });
}


#[test]
fn test_submit_consensus_already_submitted() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let n_peers: u32 = Network::max_model_peers();

    build_model(model_path.clone());
    make_model_submittable();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);
    
    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    // initialize peer consensus data array
    let model_peer_data_vec = model_peer_data(0, n_peers);

    make_model_peer_consensus_data_submittable();

    build_for_submit_consensus_data(model_id.clone(), 0, n_peers, 0, n_peers);

    for n in 0..n_peers {
      assert_err!(
        Network::submit_consensus_data(
          RuntimeOrigin::signed(account(n)),
          model_id.clone(),
          model_peer_data_vec.clone(),
        ) ,
        Error::<Test>::ConsensusDataAlreadySubmitted
      );
    }

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );    

    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );

    assert_eq!(Network::total_model_peers(model_id.clone()), n_peers as u32);
    post_successful_form_consensus_ensures(model_id.clone())
  });
}

#[test]
fn test_submit_consensus_account_err() {
  new_test_ext().execute_with(|| {
    // add first model
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_model(model_path.clone());
    make_model_submittable();

    let n_peers: u32 = Network::max_model_peers();
    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    // initialize peer consensus data array
    let model_peer_data_vec = model_peer_data(0, n_peers + 1);

    assert_err!(
      Network::submit_consensus_data(
        RuntimeOrigin::signed(account(255)),
        model_id.clone(),
        model_peer_data_vec.clone(),
      ),
      Error::<Test>::ModelPeerNotExist
    );
  });
}

#[test]
fn test_submit_consensus_model_err() {
  new_test_ext().execute_with(|| {
    // add first model
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_model(model_path.clone());
    make_model_submittable();

    let n_peers: u32 = Network::max_model_peers();
    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    // initialize peer consensus data array
    let model_peer_data_vec = model_peer_data(0, n_peers + 1);
    
    let model_path_fake: Vec<u8> = "petals-team/StableBeluga3".into();

    assert_err!(
      Network::submit_consensus_data(
        RuntimeOrigin::signed(account(0)),
        0,
        model_peer_data_vec.clone(),
      ),
      Error::<Test>::ModelNotExist
    );
  });
}

#[test]
fn test_submit_consensus_data_invalid_consensus_block() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let n_peers: u32 = Network::max_model_peers();

    build_model(model_path.clone());
    make_model_submittable();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    // initialize peer consensus data array
    let model_peer_data_vec = model_peer_data(0, n_peers);

    // increase blocks to consensus step block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );    
    
    // submit peer consensus data per each peer
    for n in 0..n_peers {
      assert_err!(
        Network::submit_consensus_data(
          RuntimeOrigin::signed(account(n)),
          model_id,
          model_peer_data_vec.clone(),
        ),
        Error::<Test>::InvalidSubmitConsensusBlock
      );
    }
  });
}

#[test]
fn test_submit_consensus_data() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let n_peers: u32 = Network::max_model_peers();

    build_model(model_path.clone());
    make_model_submittable();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    // initialize peer consensus data array
    let model_peer_data_vec = model_peer_data(0, n_peers);

    make_model_peer_consensus_data_submittable();

    build_for_submit_consensus_data(model_id.clone(), 0, n_peers, 0, n_peers);

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );    
    
    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );

    post_successful_form_consensus_ensures(model_id.clone());

    assert_eq!(Network::total_model_peers(model_id.clone()), n_peers as u32);
  });
}

#[test]
fn test_submit_consensus_data_invalid_score() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let n_peers: u32 = Network::max_model_peers();

    build_model(model_path.clone());
    make_model_submittable();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);
    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    // initialize peer consensus data array
    let model_peer_data_vec = model_peer_data_invalid_scores(0, n_peers);

    make_model_peer_consensus_data_submittable();

    // submit peer consensus data per each peer
    for n in 0..n_peers {
      assert_err!(
        Network::submit_consensus_data(
          RuntimeOrigin::signed(account(n)),
          model_id,
          model_peer_data_vec.clone(),
        ),
        Error::<Test>::InvalidScore
      );
    }

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );    
    
    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );

    post_successful_form_consensus_ensures(model_id.clone());

    assert_eq!(Network::total_model_peers(1), n_peers as u32);
  });
}

#[test]
fn test_submit_consensus_data_dishonest() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let n_peers: u32 = Network::max_model_peers();

    build_model(model_path.clone());
    make_model_submittable();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);
    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );


    // initialize peer consensus data array
    // let model_peer_data_vec = model_peer_data(0, n_peers);

    make_model_peer_consensus_data_submittable();

    // submit peer consensus data per each peer with data minus the last peer
    build_for_submit_consensus_data(model_id.clone(), 0, n_peers-1, 0, n_peers);

    // last peer is against first peer
    let model_peer_data_against = model_peer_data(1, n_peers);

    assert_ok!(
      Network::submit_consensus_data(
        RuntimeOrigin::signed(account(n_peers-1)),
        model_id.clone(),
        model_peer_data_against,
      ) 
    );

    // if any peers are left out, they submitted as unsuccessful and unsuccessful_consensus
    // the ModelPeerConsensusResults count should always be the count of total model peers
    let submissions = ModelPeerConsensusResults::<Test>::iter_key_prefix(model_id.clone());
		let len = submissions.count();
		assert_eq!(
			len, 
			n_peers as usize, 
			"ModelPeerConsensusResults len mismatch."
		);

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );    
    
    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );

    assert_eq!(Network::total_model_peers(1), n_peers as u32);

    assert_eq!(AccountPenaltyCount::<Test>::get(account(n_peers-1)), 1 as u32);

    post_successful_form_consensus_ensures(model_id.clone());
  });
}

#[test]
fn test_submit_consensus_data_remove_peer() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let model_peer_consensus_submit_percent_requirement = Network::model_peer_consensus_submit_percent_requirement();

    let n_peers: u32 = Network::max_model_peers();
    // Get amount of peers that need to keep a peer absent so they are removed through consensus
    let n_consensus_peers: u32 = (n_peers as f64 * (model_peer_consensus_submit_percent_requirement as f64 / 10000.0)).ceil() as u32;
    // starting index of peers that should be removed
    let n_peers_should_be_removed = n_peers - (n_peers - n_consensus_peers);
    
    build_model(model_path.clone());
    make_model_submittable();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    // submit peer consensus data per each submitting peer
    // this removes any peer after `n_consensus_peers`
    build_for_submit_consensus_data(model_id.clone(), 0, n_consensus_peers, 0, n_consensus_peers);

    // if any peers are left out, they submitted as unsuccessful and unsuccessful_consensus
    // the ModelPeerConsensusResults count should always be the count of total model peers
    let submissions = ModelPeerConsensusResults::<Test>::iter_key_prefix(model_id.clone());
		let len = submissions.count();
		assert_eq!(
			len, 
			n_peers as usize, 
			"ModelPeerConsensusResults len mismatch."
		);

    assert_eq!(Network::model_peer_consensus_results(1, account(0)).successful_consensus.len(), n_consensus_peers as usize);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).successful, n_consensus_peers as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).unsuccessful_consensus.len(), 0 as usize);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).unsuccessful, 0 as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).total_submits, n_consensus_peers as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).account_id, account(0));
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).peer_id, peer(0));

    
    // n_consensus_peers index should be removed
    assert_eq!(Network::model_peer_consensus_results(1, account(n_consensus_peers)).unsuccessful_consensus.len(), n_consensus_peers as usize);
    assert_eq!(Network::model_peer_consensus_results(1, account(n_consensus_peers)).unsuccessful, n_consensus_peers as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(n_consensus_peers)).account_id, account(n_consensus_peers));
    assert_eq!(Network::model_peer_consensus_results(1, account(n_consensus_peers)).peer_id, peer(n_consensus_peers));

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );    
    
    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );

    post_successful_form_consensus_ensures(model_id.clone());

    // peers should be removed
    assert_eq!(Network::total_model_peers(1), n_consensus_peers as u32);

    // ensure all expected to be removed peers are removed and data is represented correctly
    for n in n_consensus_peers..n_peers {
      post_remove_model_peer_ensures(n, model_id.clone());
    }
  });
}

#[test]
fn test_submit_consensus_data_consensus_submit_percent_requirement() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let model_peer_consensus_submit_percent_requirement = Network::model_peer_consensus_submit_percent_requirement();

    let n_peers: u32 = Network::max_model_peers();
    let n_required_peers: u32 = (n_peers as f64 * (model_peer_consensus_submit_percent_requirement as f64 / 10000.0)).floor() as u32;
    // Get 1 less of peers required to submit consensus data so consensus isn't calculated
    let n_consensus_peers: u32 = n_required_peers - 1;


    build_model(model_path.clone());
    make_model_submittable();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    // initialize peer consensus data array
    // let model_peer_data_vec = model_peer_data(0, n_peers);

    make_model_peer_consensus_data_submittable();

    // submit peer consensus data with not enough peers with data on each peer
    build_for_submit_consensus_data(model_id.clone(), 0, n_consensus_peers, 0, n_peers);

    // if any peers are left out, they submitted as unsuccessful and unsuccessful_consensus
    // the ModelPeerConsensusResults count should always be the count of total model peers
    let submissions = ModelPeerConsensusResults::<Test>::iter_key_prefix(model_id.clone());
		let len = submissions.count();
		assert_eq!(
			len, 
			n_peers as usize, 
			"ModelPeerConsensusResults len mismatch."
		);

    // peer consensus data is identical so data should all match, checking first and last peer 
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).successful_consensus.len(), n_consensus_peers as usize);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).successful, n_consensus_peers as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).unsuccessful_consensus.len(), 0 as usize);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).unsuccessful, 0 as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).total_submits, n_consensus_peers as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).account_id, account(0));
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).peer_id, peer(0));
    // assert_eq!(Network::model_peer_consensus_results(1, account(n_consensus_peers)).unsuccessful_consensus.len(), n_consensus_peers as usize);
    // assert_eq!(Network::model_peer_consensus_results(1, account(n_consensus_peers)).unsuccessful, n_consensus_peers as u32);
    // assert_eq!(Network::model_peer_consensus_results(1, account(n_consensus_peers)).account_id, account(n_consensus_peers));
    // assert_eq!(Network::model_peer_consensus_results(1, account(n_consensus_peers)).peer_id, peer(n_consensus_peers));

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );    
    
    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );

    // nothing should change
    // consensus is remo ed
    assert_eq!(Network::total_model_peers(model_id.clone()), n_peers);
    // assert_eq!(Network::model_peer_consensus_results(1, account(0)).successful_consensus.len(), 0 as usize);

    // models ModelConsensusEpochsErrors should now increment 1
    // when not enough peers submit data or do not unconfirm data errors should increment
    assert_eq!(ModelConsensusEpochsErrors::<Test>::get(model_id.clone()), 1);

    post_successful_form_consensus_ensures(model_id.clone())

  });
}

#[test]
fn test_generate_emissions() {
  new_test_ext().execute_with(|| {
    // minimum required stake vault to generate emissions is
    // min = peer_count * 10000
    StakeVaultBalance::<Test>::mutate(|n: &mut u128| *n += 4000000000000000000000);
    
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let n_peers: u32 = Network::max_model_peers();

    build_model(model_path.clone());
    make_model_submittable();

    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000; // 1000.00 tokens

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);
    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    // initialize peer consensus data array
    // let model_peer_data_vec = model_peer_data(0, n_peers);

    make_model_peer_consensus_data_submittable();

    // submit peer consensus data per each peer
    build_for_submit_consensus_data(model_id.clone(), 0, n_peers, 0, n_peers);

    // if any peers are left out, they submitted as unsuccessful and unsuccessful_consensus
    // the ModelPeerConsensusResults count should always be the count of total model peers
    let submissions = ModelPeerConsensusResults::<Test>::iter_key_prefix(model_id.clone());
		let len = submissions.count();
		assert_eq!(
			len, 
			n_peers as usize, 
			"ModelPeerConsensusResults len mismatch."
		);

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );    
    
    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );

    post_successful_form_consensus_ensures(model_id.clone());

    assert_eq!(Network::total_model_peers(1), n_peers as u32);

    // stake data should exist
    let total_stake: u128 = TotalStake::<Test>::get();
    let total_vault_balance: u128 = StakeVaultBalance::<Test>::get();
    assert_ne!(total_stake, 0);
    assert_ne!(total_vault_balance, 0);


    // Set to correct generate emissions block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval) + 1)
    );    
    
    assert_ok!(
      Network::do_generate_emissions(RuntimeOrigin::signed(account(0))) 
    );

    post_successful_generate_emissions_ensures();

    // ModelPeerConsensusResults is removed on successful emissions generation
    let submissions = ModelPeerConsensusResults::<Test>::iter_key_prefix(model_id.clone());
		let len = submissions.count();
		assert_eq!(
			len, 
			0, 
			"ModelPeerConsensusResults len mismatch."
		);

    // ensure balances have increased
    for n in 0..n_peers {
      let stake_balance = Network::account_model_stake(account(n), model_id);
      assert_ne!(amount, stake_balance);
    }

    let expected_max_post_vault_balance: u128 = (amount_staked as f64 * 0.01) as u128;
    let post_total_vault_balance: u128 = StakeVaultBalance::<Test>::get();
    assert!(post_total_vault_balance <= expected_max_post_vault_balance, "post_total_vault_balance {:?} expected_max_post_vault_balance {:?}", post_total_vault_balance, expected_max_post_vault_balance);
    // assert_ln!(post_total_vault_balance <= expected_max_post_vault_balance);

    // Expect 0 because all numbers are divisible
    // let post_total_vault_balance: u128 = StakeVaultBalance::<Test>::get();
    // assert_eq!(post_total_vault_balance, 0);

    // purposefully !assert
    // assert_eq!(post_total_vault_balance, 1);

  });
}

#[test]
fn test_generate_emissions_all_math() {
  new_test_ext().execute_with(|| {
    StakeVaultBalance::<Test>::mutate(|n: &mut u128| *n += 480000000000000000000u128);

    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let n_models: u32 = Network::max_models();
    let n_peers: u32 = Network::max_model_peers();

    for m in 0..n_models {
			let model_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
      build_model(model_path.clone());
    }

    make_model_submittable();

    let deposit_amount: u128 = 1000000000000000000000000;
    let mut amount_staked: u128 = 0;

    let stake_vault_balance: u128 = StakeVaultBalance::<Test>::get();

    // ensure 1 of the amounts are above the max weight threshold
    let amount_stake: Vec<u128> = vec![
      45275256795453966845740,
      5027522345254264457400,
      1000000000000000000000,
      1275223523454396574000,
      3275245742724396659000,
      3275245234234536237400,
      3275245742453467247500,
      3275245742345396234500,
      3275245742362396360000,
      3275245742342396323000,
      45275256795453966845740,
      5027522345254264457400,
      1000000000000000000000,
      1275223523454396574000,
      3275245742724396659000,
      3275245234234536237400,
      3275245742453467247500,
      3275245742345396234500,
      3275245742362396360000,
      3275245742342396323000,
      45275256795453966845740,
      5027522345254264457400,
      1000000000000000000000,
      1275223523454396574000,
      3275245742724396659000,
      3275245234234536237400,
      3275245742453467247500,
      3275245742345396234500,
      3275245742362396360000,
      3275245742342396323000,
      45275256795453966845740,
      5027522345254264457400,
      1000000000000000000000,
      1275223523454396574000,
      3275245742724396659000,
      3275245234234536237400,
      3275245742453467247500,
      3275245742345396234500,
      3275245742362396360000,
      3275245742342396323000,
      45275256795453966845740,
      5027522345254264457400,
      1000000000000000000000,
      1275223523454396574000,
      3275245742724396659000,
      3275245234234536237400,
      3275245742453467247500,
      3275245742345396234500,
      3275245742362396360000,
      3275245742342396323000,
      45275256795453966845740,
      5027522345254264457400,
      1000000000000000000000,
      1275223523454396574000,
      3275245742724396659000,
      3275245234234536237400,
      3275245742453467247500,
      3275245742345396234500,
      3275245742362396360000,
      3275245742342396323000,
      3275245742453467247500,
      3275245742345396234500,
      3275245742362396360000,
      3275245742342396323000,
    ];

    let mut stake_sum: u128 = 0;
    let mut i: u32 = 0;
    for stake in amount_stake.clone() {
      stake_sum += stake * n_peers as u128;
      i += 1;

      if i >= n_models {
        break
      }
    }

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    // add model peers
    for m in 0..n_models {
			let model_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
      let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();
      let amount: u128 = amount_stake[m as usize] as u128;

      // amount_staked += build_model_peers(model_id.clone(), 0, n_peers as u8, amount + deposit_amount, amount);

      for n in 0..n_peers {
        let _ = Balances::deposit_creating(&account(n), amount + deposit_amount);
        amount_staked += amount;
        assert_ok!(
          Network::add_model_peer(
            RuntimeOrigin::signed(account(n)),
            model_id.clone(),
            peer(n),
            "172.20.54.234".into(),
            8888,
            amount,
          ) 
        );
      } 
      assert_eq!(Network::total_model_peers(model_id.clone()), (n_peers) as u32);
    }

    // assert stake is correct to what's expected
    let total_stake: u128 = TotalStake::<Test>::get();
    assert_eq!(total_stake, stake_sum);

    // initialize peer consensus data array
    let model_peer_data_vec = model_peer_data(0, n_peers);

    make_model_peer_consensus_data_submittable();

    // submit peer consensus data per model per each peer
    for m in 0..n_models {
			let model_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
      let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();
      // submit peer consensus data per each peer
      build_for_submit_consensus_data(model_id.clone(), 0, n_peers, 0, n_peers);
      // if any peers are left out, they submitted as unsuccessful and unsuccessful_consensus
      // the ModelPeerConsensusResults count should always be the count of total model peers
      let submissions = ModelPeerConsensusResults::<Test>::iter_key_prefix(model_id.clone());
      let len = submissions.count();
      assert_eq!(
        len, 
        n_peers as usize, 
        "ModelPeerConsensusResults len mismatch."
      );  
    }

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );    

    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );

    for m in 0..n_models {
      let model_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
      let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();
      post_successful_form_consensus_ensures(model_id.clone());
    }

    // // stake data should exist
    let total_stake: u128 = TotalStake::<Test>::get();
    let total_vault_balance: u128 = StakeVaultBalance::<Test>::get();
    assert_ne!(total_stake, 0);
    assert_ne!(total_vault_balance, 0);

    // Set to correct generate emissions block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval) + 1)
    );    

    assert_ok!(
      Network::do_generate_emissions(RuntimeOrigin::signed(account(0))) 
    );

    post_successful_generate_emissions_ensures();
    
    // ensure balances have increased
    for m in 0..n_models {
      let model_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
      let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();
      let amount: u128 = amount_stake[m as usize] as u128;
      for n in 0..n_peers {
        let stake_balance = Network::account_model_stake(account(n), model_id);
        assert_ne!(amount, stake_balance);
      }
    }

    // when weights are imbalanced vs. max reward weight, the total weight may
    // be under 100.0. We use 99% as leeway to ensure it's working
    // We can assume the algorithm will be more accurate than 99% depending on the
    // starting staking numbers
    // We use 1% of the stake vault balance will be remaining after rewards
    let expected_max_post_vault_balance: u128 = (amount_staked as f64 * 0.01) as u128;
    let post_total_vault_balance: u128 = StakeVaultBalance::<Test>::get();

    assert!(post_total_vault_balance <= expected_max_post_vault_balance);

    // purposefully !assert
    // assert_eq!(post_total_vault_balance, 1);

  });
}

#[test]
fn test_remove_peer_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_model(model_path.clone());
    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::remove_model_peer(
        RuntimeOrigin::signed(account(255)),
        0,
      ),
      Error::<Test>::ModelNotExist
    );

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );
    post_successful_add_model_peer_asserts(0, model_id.clone(), amount);

    post_successful_add_model_peers_asserts(
      1,
      amount,
      model_id.clone(),
    );

    assert_eq!(Network::total_stake(), amount);

    assert_err!(
      Network::remove_model_peer(
        RuntimeOrigin::signed(account(255)),
        model_id.clone(),
      ),
      Error::<Test>::ModelPeerNotExist
    );

    assert_eq!(Network::total_model_peers(1), 1);

  });
}

#[test]
fn test_remove_peer_is_included_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_model(model_path.clone());
    // System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::remove_model_peer(
        RuntimeOrigin::signed(account(255)),
        0,
      ),
      Error::<Test>::ModelNotExist
    );

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );
    post_successful_add_model_peer_asserts(0, model_id.clone(), amount);

    post_successful_add_model_peers_asserts(
      1,
      amount,
      model_id.clone(),
    );

    assert_eq!(Network::total_stake(), amount);

    make_model_peer_included();

    assert_err!(
      Network::remove_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
      ),
      Error::<Test>::InvalidRemoveOrUpdateModelPeerBlock
    );

    assert_eq!(Network::total_model_peers(1), 1);

  });
}


#[test]
fn test_remove_peer_unstake_epochs_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();

    System::set_block_number(System::block_number() + consensus_blocks_interval);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );
    post_successful_add_model_peer_asserts(0, model_id.clone(), amount);
    assert_eq!(Network::total_model_peers(1), 1);
    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);

    // make_model_peer_removable();


    System::set_block_number(System::block_number() + consensus_blocks_interval);

    assert_ok!(
      Network::remove_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
      ) 
    );

    post_remove_model_peer_ensures(0, model_id.clone());

    assert_eq!(Network::total_model_peers(1), 0);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        amount,
      ),
      Error::<Test>::RequiredUnstakeEpochsNotMet,
    );
    
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<Test>::get();
    System::set_block_number(System::block_number() + consensus_blocks_interval * min_required_unstake_epochs);
    
    assert_ok!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        amount,
      )
    );
  });
}

#[test]
fn test_remove_peer_unstake_total_balance() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());
    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );
    post_successful_add_model_peer_asserts(0, model_id.clone(), amount);
    assert_eq!(Network::total_model_peers(1), 1);
    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);

    assert_ok!(
      Network::remove_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
      ) 
    );

    post_remove_model_peer_ensures(0, model_id.clone());

    assert_eq!(Network::total_model_peers(1), 0);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);
    
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<Test>::get();
    System::set_block_number(System::block_number() + consensus_blocks_interval * min_required_unstake_epochs);
    
    let remaining_account_stake_balance: u128 = AccountModelStake::<Test>::get(&account(0), model_id.clone());

    assert_ok!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        remaining_account_stake_balance,
      )
    );

    post_remove_unstake_ensures(0, model_id.clone());
  });
}


#[test]
fn test_remove_peer() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_model(model_path.clone());
    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );
    post_successful_add_model_peer_asserts(0, model_id.clone(), amount);

    assert_eq!(Network::total_model_peers(1), 1);
    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);

    // make_model_peer_removable();
    // should be able to be removed is initialization period doesn't reach inclusion epochs

    assert_ok!(
      Network::remove_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
      ) 
    );
    post_remove_model_peer_ensures(0, model_id.clone());
    assert_eq!(Network::total_model_peers(1), 0);

  });
}

#[test]
fn test_add_to_stake_err() {
  new_test_ext().execute_with(|| {
    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::add_to_stake(
        RuntimeOrigin::signed(account(0)),
        0,
        amount,
      ),
      Error::<Test>::ModelNotExist,
    );

    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_model(model_path.clone());
    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );
    post_successful_add_model_peer_asserts(0, model_id.clone(), amount);

    assert_eq!(Network::total_model_peers(1), 1);
    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::add_to_stake(
        RuntimeOrigin::signed(account(255)),
        model_id.clone(),
        amount,
      ),
      Error::<Test>::ModelPeerNotExist,
    );

  });
}

#[test]
fn test_add_to_stake() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_model(model_path.clone());
    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );
    post_successful_add_model_peer_asserts(0, model_id.clone(), amount);

    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);
    assert_eq!(Network::total_model_peers(1), 1);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_to_stake(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        amount,
      ) 
    );

    assert_eq!(Network::account_model_stake(account(0), 1), amount + amount);
    assert_eq!(Network::total_account_stake(account(0)), amount + amount);
    assert_eq!(Network::total_stake(), amount + amount);
    assert_eq!(Network::total_model_stake(1), amount + amount);


  });
}

#[test]
fn test_remove_stake_err() {
  new_test_ext().execute_with(|| {
    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    // attempt to remove on non-existent model_id
    assert_err!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(255)),
        0,
        amount,
      ),
      Error::<Test>::ModelPeerNotExist,
    );

    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_model(model_path.clone());
    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );
    post_successful_add_model_peer_asserts(0, model_id.clone(), amount);

    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);
    assert_eq!(Network::total_model_peers(1), 1);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(255)),
        model_id.clone(),
        amount,
      ),
      Error::<Test>::ModelPeerNotExist,
    );

    assert_err!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        0,
      ),
      Error::<Test>::RequiredUnstakeEpochsNotMet,
    );

    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<Test>::get();
    System::set_block_number(System::block_number() + consensus_blocks_interval * min_required_unstake_epochs);

    assert_err!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        0,
      ),
      Error::<Test>::NotEnoughStaketoWithdraw,
    );

    assert_err!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        amount+1,
      ),
      Error::<Test>::NotEnoughStaketoWithdraw,
    );

    assert_err!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        amount,
      ),
      Error::<Test>::MinStakeNotReached,
    );

  });
}

#[test]
fn test_remove_stake() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_model(model_path.clone());
    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(0),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );
    post_successful_add_model_peer_asserts(0, model_id.clone(), amount);

    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);      
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);
    assert_eq!(Network::total_model_peers(1), 1);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    // add double amount to stake
    assert_ok!(
      Network::add_to_stake(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        amount,
      ) 
    );

    assert_eq!(Network::account_model_stake(account(0), 1), amount + amount);
    assert_eq!(Network::total_account_stake(account(0)), amount + amount);
    assert_eq!(Network::total_stake(), amount + amount);
    assert_eq!(Network::total_model_stake(1), amount + amount);

    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<Test>::get();
    System::set_block_number(System::block_number() + consensus_blocks_interval * min_required_unstake_epochs);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    // remove amount ontop
    assert_ok!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        amount,
      )
    );

    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);

  });
}

#[test]
fn test_form_consensus_unconfirm_consensus() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let n_peers: u32 = Network::max_model_peers();
    let unconfirm_threshold = ModelConsensusUnconfirmedThreshold::<Test>::get();
    let n_peers_unconfirm: u32 = (n_peers as f64 * (unconfirm_threshold as f64 / 10000.0)).ceil() as u32;

    build_model(model_path.clone());
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    // for n in 0..n_peers {
    //   let _ = Balances::deposit_creating(&account(n), deposit_amount);
    //   amount_staked += amount;
    //   assert_ok!(
    //     Network::add_model_peer(
    //       RuntimeOrigin::signed(account(n)),
    //       model_id.clone(),
    //       peer(n),
    //       "172.20.54.234".into(),
		// 			8888,
    //       amount,
    //     ) 
    //   );
    //   post_successful_add_model_peer_asserts(n, model_id.clone(), amount);
    // }
    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    // submit peer consensus data per each peer
    build_for_submit_consensus_data(model_id.clone(), 0, n_peers, 0, n_peers);

    // unconfirm consensus data up to the required threshold
    for n in 0..n_peers_unconfirm {
      assert_ok!(
        Network::unconfirm_consensus_data(
          RuntimeOrigin::signed(account(n)),
          model_id,
        ) 
      );
    }

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );

    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );
    

    // unconfirming consensus data should remove all ModelPeerConsensusResults
    let submissions = ModelPeerConsensusResults::<Test>::iter_key_prefix(model_id.clone());
		let len = submissions.count();
		assert_eq!(
			len, 
			0, 
			"ModelPeerConsensusResults len mismatch."
		);

    let max_model_model_unconfirmed_epochs = MaxModelConsensusUnconfirmedConsecutiveEpochs::<Test>::get();

    // Should have increased uncofirmed epochs count
    let model_uncofirmed_epochs = ModelConsensusUnconfirmedConsecutiveEpochsCount::<Test>::get(model_id.clone());
		assert_eq!(model_uncofirmed_epochs, 1, "ModelConsensusUnconfirmedConsecutiveEpochsCount incorrect.");

    if model_uncofirmed_epochs > max_model_model_unconfirmed_epochs {
      let model_epoch_errors = ModelConsensusEpochsErrors::<Test>::get(model_id.clone());
      assert_eq!(model_epoch_errors, 1, "ModelConsensusEpochsErrors incorrect.");  
    } else {
      let model_epoch_errors = ModelConsensusEpochsErrors::<Test>::get(model_id.clone());
      assert_eq!(model_epoch_errors, 0, "ModelConsensusEpochsErrors incorrect.");  
    }

    post_successful_form_consensus_ensures(model_id.clone());
  });
}

#[test]
fn test_submit_data_consensus_as_err_then_unconfirm_consensus_err() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let n_peers: u32 = Network::max_model_peers();
    let unconfirm_threshold = ModelConsensusUnconfirmedThreshold::<Test>::get();
    let n_peers_unconfirm: u32 = (n_peers as f64 * (unconfirm_threshold as f64 / 10000.0)).ceil() as u32;

    build_model(model_path.clone());
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    let model_peer_data_vec = model_peer_data(0, n_peers);

    // submit consensus data through `unconfirm_consensus_data`
    // assert_ok!(
    //   Network::submit_consensus_data(
    //     RuntimeOrigin::signed(account(0)),
    //     model_id,
    //     model_peer_data_vec.clone(),
    //   ) 
    // );

    assert_ok!(
      Network::unconfirm_consensus_data(
        RuntimeOrigin::signed(account(0)),
        model_id,
      )
    );

    // cannot call `unconfirm_consensus_data` if already submitted consensus data with an error state
    assert_err!(
      Network::unconfirm_consensus_data(
        RuntimeOrigin::signed(account(0)),
        model_id,
      ),
      Error::<Test>::ConsensusDataAlreadyUnconfirmed
    );
  });
}

#[test]
fn test_form_consensus() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());

    let n_peers: u32 = Network::max_model_peers();
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    build_for_submit_consensus_data(model_id.clone(), 0, n_peers, 0, n_peers);

    assert_eq!(Network::model_peer_consensus_results(1, account(0)).successful_consensus.len(), n_peers as usize);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).successful, n_peers as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).total_submits, n_peers as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).account_id, account(0));
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).peer_id, peer(0));
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).unsuccessful_consensus.len(), 0 as usize);
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).unsuccessful, 0 as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).account_id, account(n_peers-1));
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).peer_id, peer(n_peers-1));

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );    

    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );

    post_successful_form_consensus_ensures(model_id.clone());
  });
}

#[test]
fn test_form_consensus_with_3_peers() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    MinModelPeers::<Test>::set(3);

    build_model(model_path.clone());

    let n_peers: u32 = 3;
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    build_for_submit_consensus_data(model_id.clone(), 0, n_peers, 0, n_peers);

    assert_eq!(Network::model_peer_consensus_results(1, account(0)).successful_consensus.len(), n_peers as usize);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).successful, n_peers as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).total_submits, n_peers as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).account_id, account(0));
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).peer_id, peer(0));
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).unsuccessful_consensus.len(), 0 as usize);
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).unsuccessful, 0 as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).account_id, account(n_peers-1));
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).peer_id, peer(n_peers-1));

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );    

    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );

    post_successful_form_consensus_ensures(model_id.clone());
  });
}


#[test]
fn test_form_consensus_with_4_peers() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    MinModelPeers::<Test>::set(4);

    build_model(model_path.clone());

    let n_peers: u32 = 4;
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    build_for_submit_consensus_data(model_id.clone(), 0, n_peers, 0, n_peers);

    assert_eq!(Network::model_peer_consensus_results(1, account(0)).successful_consensus.len(), n_peers as usize);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).successful, n_peers as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).total_submits, n_peers as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).account_id, account(0));
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).peer_id, peer(0));
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).unsuccessful_consensus.len(), 0 as usize);
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).unsuccessful, 0 as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).account_id, account(n_peers-1));
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).peer_id, peer(n_peers-1));

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );    

    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );

    post_successful_form_consensus_ensures(model_id.clone());
  });
}

#[test]
fn test_form_consensus_with_5_peers() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    MinModelPeers::<Test>::set(5);

    build_model(model_path.clone());

    let n_peers: u32 = 5;
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    build_for_submit_consensus_data(model_id.clone(), 0, n_peers, 0, n_peers);

    assert_eq!(Network::model_peer_consensus_results(1, account(0)).successful_consensus.len(), n_peers as usize);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).successful, n_peers as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).total_submits, n_peers as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).account_id, account(0));
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).peer_id, peer(0));
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).unsuccessful_consensus.len(), 0 as usize);
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).unsuccessful, 0 as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).account_id, account(n_peers-1));
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).peer_id, peer(n_peers-1));

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );    

    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );

    post_successful_form_consensus_ensures(model_id.clone());
  });
}

#[test]
fn test_form_consensus_remove_peer() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());

    let peer_removal_threshold = PeerRemovalThreshold::<Test>::get();

    let n_peers: u32 = Network::max_model_peers();
    let n_required_against_peers: u32 = (n_peers as f64 * (peer_removal_threshold as f64 / 10000.0)).ceil() as u32 + 1;
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    let model_peer_data_vec = model_peer_data(0, n_peers-1);

    for n in 0..n_required_against_peers {
      assert_ok!(
        Network::submit_consensus_data(
          RuntimeOrigin::signed(account(n)),
          model_id,
          model_peer_data_vec.clone(),
        ) 
      );  
    }

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );    

    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );

    post_successful_form_consensus_ensures(model_id.clone());

    post_remove_model_peer_ensures((n_peers - 1) as u32, model_id.clone());
  });
}

#[test]
fn test_form_consensus_peer_submission_epochs() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let n_peers: u32 = Network::max_model_peers();
    
    build_model(model_path.clone());
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    build_for_submit_consensus_data(model_id.clone(), 0, n_peers, 0, n_peers);

    assert_eq!(Network::model_peer_consensus_results(1, account(0)).successful_consensus.len(), n_peers as usize);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).successful, n_peers as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).total_submits, n_peers as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).account_id, account(0));
    assert_eq!(Network::model_peer_consensus_results(1, account(0)).peer_id, peer(0));
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).unsuccessful_consensus.len(), 0 as usize);
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).unsuccessful, 0 as u32);
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).account_id, account(n_peers-1));
    assert_eq!(Network::model_peer_consensus_results(1, account(n_peers-1)).peer_id, peer(n_peers-1));

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );

    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );

    post_successful_form_consensus_ensures(model_id.clone());
  });
}

#[test]
fn test_percent_mul() {
  new_test_ext().execute_with(|| {
    let value = Network::percent_mul(53, 3000);

    assert_eq!(value, 15, "percent_mul didn't round down");

    let value = Network::percent_mul_round_up(53, 3000);

    assert_eq!(value, 16, "percent_mul_round_up didn't round up");

    let value = Network::percent_mul(28000000e+18 as u128, PERCENTAGE_FACTOR);

    assert_ne!(value, 0, "percent_mul didn't round down");
    assert_ne!(value, u128::MAX, "percent_mul didn't round down");

    let value = Network::percent_mul_round_up(28000000e+18 as u128, PERCENTAGE_FACTOR);

    assert_ne!(value, 0, "percent_mul_round_up didn't round down");
    assert_ne!(value, u128::MAX, "percent_mul_round_up didn't round down");
  });
}

#[test]
fn test_percent_div() {
  new_test_ext().execute_with(|| {
    let value = Network::percent_div(1, 3000);

    assert_eq!(value, 3, "percent_div didn't round down");

    let value = Network::percent_div_round_up(1, 3000);

    assert_eq!(value, 4, "percent_div_round_up didn't round up");
  });
}

#[test]
fn test_submit_consensus_data_remove_peer_peer_against_consensus_removal_threshold() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let model_peer_consensus_submit_percent_requirement = Network::model_peer_consensus_submit_percent_requirement();
    let peer_against_consensus_removal_threshold: u128 = PeerAgainstConsensusRemovalThreshold::<Test>::get();

    let n_peers: u32 = Network::max_model_peers();
    // Get amount of peers that need to keep a peer absent so they are removed through consensus
    let n_consensus_peers: u32 = (n_peers as f64 * (model_peer_consensus_submit_percent_requirement as f64 / 10000.0)).ceil() as u32;
    // starting index of peers that should be removed
    let n_peers_should_be_removed = n_peers - (n_peers - n_consensus_peers);
    
    let n_peers_threshold = Network::percent_mul_round_up(n_peers as u128, peer_against_consensus_removal_threshold);

    build_model(model_path.clone());
    make_model_submittable();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    // submit peer consensus data per each peer with data minus submitting on behalf of the last peer
    build_for_submit_consensus_data(model_id.clone(), 0, n_peers-1, 0, n_peers);

    // last peer is against first peers threshold of peers to be removed
    // when being against these percentage of peers
    let model_peer_data_against = model_peer_data((n_peers_threshold - 1) as u32, n_peers);

    assert_ok!(
      Network::submit_consensus_data(
        RuntimeOrigin::signed(account(n_peers-1)),
        model_id.clone(),
        model_peer_data_against,
      ) 
    );
    
    // if any peers are left out, they submitted as unsuccessful and unsuccessful_consensus
    // the ModelPeerConsensusResults count should always be the count of total model peers
    let submissions = ModelPeerConsensusResults::<Test>::iter_key_prefix(model_id.clone());
		let len = submissions.count();
		assert_eq!(
			len, 
			n_peers as usize, 
			"ModelPeerConsensusResults len mismatch."
		);

    // Set to correct consensus block
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();
    System::set_block_number(
      consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
    );    
    
    assert_ok!(
      Network::form_consensus(RuntimeOrigin::signed(account(0))) 
    );

    assert_eq!(Network::total_model_peers(model_id.clone()), (n_peers - 1) as u32);

    assert_eq!(AccountPenaltyCount::<Test>::get(account(n_peers-1)), (n_peers_threshold) as u32);

    post_remove_model_peer_ensures((n_peers - 1) as u32, model_id.clone());

    post_successful_form_consensus_ensures(model_id.clone());
  });
}

#[test]
fn test_form_consensus_remove_ineligible_model_peer() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let n_peers: u32 = Network::max_model_peers();

    build_model(model_path.clone());
    make_model_submittable();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, deposit_amount, amount);
    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    let max_account_penalty_count: u32 = MaxAccountPenaltyCount::<Test>::get();
    let consensus_blocks_interval = ConsensusBlocksInterval::<Test>::get();

    for n in 0..max_account_penalty_count {
      let dishonest_peer_account_penalty_count = AccountPenaltyCount::<Test>::get(account(n_peers-1));

      let total_model_peers: u32 = Network::total_model_peers(model_id.clone());

      if total_model_peers < n_peers {
        break
      }

      if n > 0 {
        System::set_block_number(System::block_number() + consensus_blocks_interval);
      }

      // initialize peer consensus data array
      make_model_peer_consensus_data_submittable();

      // submit peer consensus data per each peer with data minus the last peer
      build_for_submit_consensus_data(model_id.clone(), 0, n_peers-1, 0, n_peers);

      // last peer is against first peer
      let model_peer_data_against = model_peer_data(1, n_peers);

      assert_ok!(
        Network::submit_consensus_data(
          RuntimeOrigin::signed(account(n_peers-1)),
          model_id.clone(),
          model_peer_data_against,
        ) 
      );

      // if any peers are left out, they submitted as unsuccessful and unsuccessful_consensus
      // the ModelPeerConsensusResults count should always be the count of total model peers
      let submissions = ModelPeerConsensusResults::<Test>::iter_key_prefix(model_id.clone());
      let len = submissions.count();
      assert_eq!(
        len, 
        n_peers as usize, 
        "ModelPeerConsensusResults len mismatch."
      );

      // Set to correct consensus block
      System::set_block_number(
        consensus_blocks_interval + (System::block_number() - (System::block_number() % consensus_blocks_interval))
      );    
      
      assert_ok!(
        Network::form_consensus(RuntimeOrigin::signed(account(0))) 
      );

      post_successful_form_consensus_ensures(model_id.clone());
    }

    assert_eq!(Network::total_model_peers(model_id.clone()), (n_peers - 1) as u32);

    assert!(AccountPenaltyCount::<Test>::get(account(n_peers-1)) >= max_account_penalty_count);
  });
}

#[test]
fn test_propose_model_peer_dishonest_proposer_model_error() {
  new_test_ext().execute_with(|| {
    assert_err!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        1,
        peer(0)
      ),
      Error::<Test>::ModelNotExist
    );
  });
}

#[test]
fn test_propose_model_peer_dishonest_proposer_model_peer_exists_error() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let dishonesty_voting_period = DishonestyVotingPeriod::<Test>::get();

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    build_model(model_path.clone());

    let n_peers: u32 = 2;
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    assert_err!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(n_peers + 1)),
        model_id.clone(),
        peer(n_peers + 1)
      ),
      Error::<Test>::ModelPeerNotExist
    );

  });
}

#[test]
fn test_propose_model_peer_dishonest_proposer_not_submittable() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let dishonesty_voting_period = DishonestyVotingPeriod::<Test>::get();

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    build_model(model_path.clone());

    let n_peers: u32 = Network::max_model_peers();
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    assert_err!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(1)
      ),
      Error::<Test>::PeerConsensusSubmitEpochNotReached
    );

  });
}

#[test]
fn test_propose_model_peer_dishonest_votee_peer_id_exists() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let dishonesty_voting_period = DishonestyVotingPeriod::<Test>::get();

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    build_model(model_path.clone());

    let n_peers: u32 = Network::min_model_peers();
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    assert_err!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(n_peers + 1)
      ),
      Error::<Test>::PeerIdNotExist
    );

  });
}

#[test]
fn test_propose_model_peer_dishonest_min_model_peers_error() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let dishonesty_voting_period = DishonestyVotingPeriod::<Test>::get();

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    build_model(model_path.clone());

    let n_peers: u32 = Network::min_model_peers() - 1;
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    assert_err!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(1)
      ),
      Error::<Test>::ModelPeersMin
    );

  });
}

#[test]
fn test_propose_model_peer_dishonest() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());

    let n_peers: u32 = Network::max_model_peers();
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    let dishonesty_voting_period = DishonestyVotingPeriod::<Test>::get();

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    make_model_peer_consensus_data_submittable();

    assert_ok!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(1)
      )
    );

    post_successful_dishonesty_proposal_ensures(0, 1, model_id.clone());
  });
}

#[test]
fn test_vote_model_peer_dishonest_model_exists_error() {
  new_test_ext().execute_with(|| {
    assert_err!(
      Network::vote_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        1,
        peer(1)
      ),
      Error::<Test>::ModelNotExist
    );
  });
}

#[test]
fn test_vote_model_peer_dishonest_model_peer_exists_error() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let dishonesty_voting_period = DishonestyVotingPeriod::<Test>::get();

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    build_model(model_path.clone());

    let n_peers: u32 = Network::max_model_peers() - 1;
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    assert_ok!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(1)
      )
    );

    assert_err!(
      Network::vote_model_peer_dishonest(
        RuntimeOrigin::signed(account(n_peers + 1)),
        model_id.clone(),
        peer(1)
      ),
      Error::<Test>::ModelPeerNotExist
    );

  });
}

#[test]
fn test_vote_model_peer_dishonest_proposer_not_submittable() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let dishonesty_voting_period = DishonestyVotingPeriod::<Test>::get();

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    build_model(model_path.clone());

    let max_model_peers: u32 = Network::max_model_peers();

    let n_peers: u32 = max_model_peers - 1;
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    assert_ok!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(1)
      )
    );

    let _ = Balances::deposit_creating(&account(max_model_peers), deposit_amount);

    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(max_model_peers)),
        model_id.clone(),
        peer(max_model_peers),
        "172.20.54.234".into(),
        8888,
        amount,
      ) 
    );

    assert_err!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(max_model_peers)),
        model_id.clone(),
        peer(1)
      ),
      Error::<Test>::PeerConsensusSubmitEpochNotReached
    );

  });
}

#[test]
fn test_vote_model_peer_dishonest_min_model_peers_error() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let dishonesty_voting_period = DishonestyVotingPeriod::<Test>::get();

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    build_model(model_path.clone());

    let n_peers: u32 = Network::min_model_peers();
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    make_model_peer_consensus_data_submittable();

    assert_ok!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(1)
      )
    );

    make_model_peer_removable();
    assert_ok!(
      Network::remove_model_peer(
        RuntimeOrigin::signed(account(n_peers-1)),
        model_id.clone(),
      ) 
    );

    assert_err!(
      Network::vote_model_peer_dishonest(
        RuntimeOrigin::signed(account(2)),
        model_id.clone(),
        peer(1)
      ),
      Error::<Test>::ModelPeersMin
    );
  });
}

#[test]
fn test_vote_model_peer_dishonest_peer_id_exists_error() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());

    let n_peers: u32 = Network::min_model_peers();
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    let dishonesty_voting_period = DishonestyVotingPeriod::<Test>::get();

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    make_model_peer_consensus_data_submittable();

    assert_ok!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(1)
      )
    );

    post_successful_dishonesty_proposal_ensures(0, 1, model_id.clone());

    assert_err!(
      Network::vote_model_peer_dishonest(
        RuntimeOrigin::signed(account(1)),
        model_id.clone(),
        peer(n_peers + 1)
      ),
      Error::<Test>::PeerIdNotExist
    );

  });
}

#[test]
fn test_vote_model_peer_dishonest_not_proposed_error() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());

    let n_peers: u32 = Network::min_model_peers();
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    let dishonesty_voting_period = DishonestyVotingPeriod::<Test>::get();

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    make_model_peer_consensus_data_submittable();

    assert_err!(
      Network::vote_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(1)
      ),
      Error::<Test>::DishonestyVoteNotProposed
    );

  });
}

#[test]
fn test_vote_model_peer_dishonest_period_over_error() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());

    let n_peers: u32 = Network::max_model_peers();
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    let dishonesty_voting_period = DishonestyVotingPeriod::<Test>::get();

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    make_model_peer_consensus_data_submittable();

    assert_ok!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(1)
      )
    );

    post_successful_dishonesty_proposal_ensures(0, 1, model_id.clone());

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    assert_err!(
      Network::vote_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(1)
      ),
      Error::<Test>::DishonestyVotingPeriodOver
    );

  });
}

#[test]
fn test_vote_model_peer_dishonest_duplicate_error() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());

    let n_peers: u32 = Network::max_model_peers();
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    let dishonesty_voting_period = DishonestyVotingPeriod::<Test>::get();

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    make_model_peer_consensus_data_submittable();

    assert_ok!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(1)
      )
    );

    post_successful_dishonesty_proposal_ensures(0, 1, model_id.clone());

    assert_ok!(
      Network::vote_model_peer_dishonest(
        RuntimeOrigin::signed(account(2)),
        model_id.clone(),
        peer(1)
      )
    );

    assert_err!(
      Network::vote_model_peer_dishonest(
        RuntimeOrigin::signed(account(2)),
        model_id.clone(),
        peer(1)
      ),
      Error::<Test>::DishonestyVotingDuplicate
    );
  });
}


#[test]
fn test_vote_model_peer_dishonest_period_passed_error() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());

    let n_peers: u32 = Network::max_model_peers();
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    let dishonesty_voting_period = DishonestyVotingPeriod::<Test>::get();

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    make_model_peer_consensus_data_submittable();

    assert_ok!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(1)
      )
    );

    post_successful_dishonesty_proposal_ensures(0, 1, model_id.clone());

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    assert_err!(
      Network::vote_model_peer_dishonest(
        RuntimeOrigin::signed(account(2)),
        model_id.clone(),
        peer(1)
      ),
      Error::<Test>::DishonestyVotingPeriodOver
    );
  });
}

#[test]
fn test_vote_model_peer_dishonest() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());

    let n_peers: u32 = Network::max_model_peers();
    
    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    let dishonesty_voting_period = DishonestyVotingPeriod::<Test>::get();

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    make_model_peer_consensus_data_submittable();

    assert_ok!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(1)
      )
    );

    post_successful_dishonesty_proposal_ensures(0, 1, model_id.clone());

    assert_ok!(
      Network::vote_model_peer_dishonest(
        RuntimeOrigin::signed(account(2)),
        model_id.clone(),
        peer(1)
      )
    );
  });
}

#[test]
fn test_vote_model_peer_dishonest_consensus() {
  new_test_ext().execute_with(|| {

    let peer_removal_threshold = PeerRemovalThreshold::<Test>::get();

    let model_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_model(model_path.clone());

    let n_peers: u32 = Network::max_model_peers();
    let n_required_voting_peers: u32 = (n_peers as f64 * (peer_removal_threshold as f64 / 10000.0)).ceil() as u32 + 1;
    log::error!("n_required_voting_peers {:?}", n_required_voting_peers);
    log::info!("n_required_voting_peers {:?}", n_required_voting_peers);

    // increase blocks
    make_model_submittable();

    let model_id = ModelPaths::<Test>::get(model_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount_staked = build_model_peers(model_id.clone(), 0, n_peers, amount + deposit_amount, amount);

    assert_eq!(Network::total_stake(), amount_staked);
    post_successful_add_model_peers_asserts(
      n_peers.into(),
      amount,
      model_id.clone(),
    );

    let dishonesty_voting_period = DishonestyVotingPeriod::<Test>::get();

    System::set_block_number(System::block_number() + dishonesty_voting_period);

    make_model_peer_consensus_data_submittable();

    assert_ok!(
      Network::propose_model_peer_dishonest(
        RuntimeOrigin::signed(account(0)),
        model_id.clone(),
        peer(n_peers-1)
      )
    );

    post_successful_dishonesty_proposal_ensures(0, n_peers-1, model_id.clone());

    for n in 1..n_required_voting_peers {
      assert_ok!(
        Network::vote_model_peer_dishonest(
          RuntimeOrigin::signed(account(n)),
          model_id.clone(),
          peer(n_peers-1)
        )
      );
    }

    post_remove_model_peer_ensures(n_peers-1, model_id.clone());  
  });
}
