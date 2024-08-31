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
// use frame_support::traits::OriginTrait;
use sp_core::OpaquePeerId as PeerId;
use frame_support::{
	assert_noop, assert_ok, assert_err
};
use sp_runtime::traits::Header;
use log::info;
use sp_core::{H256, U256};
// use parity_scale_codec::Decode;
use frame_support::traits::{OnInitialize, Currency};
use crate::{
  Error, SubnetNodeData, SubnetNodeConsensusResults, AccountPenaltyCount, TotalStake, 
  StakeVaultBalance, SubnetPaths, NodeRemovalThreshold,
  MinRequiredUnstakeEpochs, MaxAccountPenaltyCount, MinSubnetNodes, TotalSubnetNodes,
  SubnetConsensusUnconfirmedThreshold, SubnetNodesData, SubnetNodeAccount,
  SubnetAccount, SubnetConsensusEpochsErrors, RemoveSubnetNodeEpochPercentage,
  NodeConsensusEpochSubmitted, MinRequiredNodeConsensusInclusionEpochs,
  NodeConsensusEpochUnconfirmed, AccountSubnetStake, MinStakeBalance,
  SubnetTotalConsensusSubmits, NodeAgainstConsensusRemovalThreshold,
  SubnetConsensusEpochUnconfirmedCount, SubnetsInConsensus,
  MaxSubnetConsensusUnconfirmedConsecutiveEpochs, SubnetConsensusUnconfirmedConsecutiveEpochsCount,
  VotingPeriod, MinRequiredNodeAccountantEpochs, ProposalsCount, ChallengePeriod, VoteType,
  AccountSubnetDelegateStakeShares,TotalSubnetDelegateStakeShares, TotalSubnetDelegateStakeBalance,
  MinRequiredDelegateUnstakeEpochs, TotalSubnets, CurrentAccountant2, AccountantDataCount, PropsType,
  AccountantDataNodeParams, SubnetRewardsValidator, SubnetRewardsSubmission, BaseSubnetReward, BaseReward,
  DelegateStakeRewardsPercentage, SubnetNodesClasses, SubnetNodeClass, SubnetNodeClassEpochs,
  SubnetPenaltyCount, MaxSequentialAbsentSubnetNode, SequentialAbsentSubnetNode, PreSubnetData,
  CurrentAccountants, TargetAccountantsLength
};
use frame_support::weights::Pays;
use frame_support::BoundedVec;
use strum::IntoEnumIterator;
use sp_io::crypto::sr25519_sign;
use sp_runtime::{MultiSigner, MultiSignature};
use sp_io::crypto::sr25519_generate;
use frame_support::pallet_prelude::Encode;
use sp_runtime::traits::IdentifyAccount;
use sp_core::Pair;

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

fn build_subnet(subnet_path: Vec<u8>) {
  // assert_ok!(
  //   Network::vote_model(
  //     RuntimeOrigin::signed(account(0)), 
  //     subnet_path.clone(),
  //   )
  // );
  let cost = Network::get_model_initialization_cost(0);
  let _ = Balances::deposit_creating(&account(0), cost+1000);

  let add_subnet_data = PreSubnetData {
    path: subnet_path.clone().into(),
    memory_mb: 50000,
  };
  assert_ok!(
    Network::activate_subnet(
      account(0),
      account(0),
      add_subnet_data,
    )
  );

  // assert_ok!(
  //   Network::add_subnet(
  //     RuntimeOrigin::signed(account(0)),
  //     add_subnet_data,
  //   ) 
  // );
}

// Returns total staked on subnet
fn build_subnet_nodes(subnet_id: u32, start: u32, end: u32, deposit_amount: u128, amount: u128) -> u128 {
  let mut amount_staked = 0;
  for n in start..end {
    let _ = Balances::deposit_creating(&account(n), deposit_amount);
    amount_staked += amount;
    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(n)),
        subnet_id,
        peer(n),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );
    post_successful_add_subnet_node_asserts(n, subnet_id, amount);
  }
  amount_staked
}

// fn build_for_submit_consensus_data(subnet_id: u32, start: u32, end: u32, start_data: u32, end_data: u32) {
//   let subnet_node_data_vec = subnet_node_data(start_data, end_data);

//   for n in start..end {
//     assert_ok!(
//       Network::submit_consensus_data(
//         RuntimeOrigin::signed(account(n)),
//         subnet_id,
//         subnet_node_data_vec.clone(),
//       ) 
//     );
//   }
// }

fn make_model_submittable() {
  // increase blocks
  // let epoch_length = Network::EpochLength::get();
  // let epoch_length = EpochLength::get();
  let epoch_length = EpochLength::get();

  let min_required_model_consensus_submit_epochs: u64 = Network::min_required_model_consensus_submit_epochs();
  System::set_block_number(System::block_number() + epoch_length * min_required_model_consensus_submit_epochs);
}

// increase the blocks past the consensus steps and remove subnet peer blocks span
fn make_consensus_data_submittable() {
  // increase blocks
  let current_block_number = System::block_number();
  let subnet_node_removal_percentage = RemoveSubnetNodeEpochPercentage::<Test>::get();
  let epoch_length = EpochLength::get();

  let start_block_can_remove_peer = epoch_length as u128 * subnet_node_removal_percentage / PERCENTAGE_FACTOR;

  let max_remove_subnet_node_block = start_block_can_remove_peer as u64 + (current_block_number - (current_block_number % epoch_length));

  if current_block_number < max_remove_subnet_node_block {
    System::set_block_number(max_remove_subnet_node_block + 1);
  }
}

fn make_subnet_node_included() {
  let epoch_length = EpochLength::get();
  let min_required_consensus_inclusion_epochs = MinRequiredNodeConsensusInclusionEpochs::<Test>::get();
  System::set_block_number(System::block_number() + epoch_length * min_required_consensus_inclusion_epochs);
}

fn make_subnet_node_consensus_data_submittable() {
  // increase blocks
  let epoch_length = EpochLength::get();
  let min_required_peer_consensus_submit_epochs: u64 = Network::min_required_peer_consensus_submit_epochs();
  System::set_block_number(System::block_number() + epoch_length * min_required_peer_consensus_submit_epochs);
  make_consensus_data_submittable();
}

fn make_subnet_node_dishonesty_consensus_proposable() {
  // increase blocks
  let epoch_length = EpochLength::get();
  let min_required_peer_consensus_submit_epochs: u64 = MinRequiredNodeAccountantEpochs::<Test>::get();
  System::set_block_number(System::block_number() + epoch_length * min_required_peer_consensus_submit_epochs);
}

fn make_subnet_node_removable() {
  // increase blocks
  let current_block_number = System::block_number();
  let subnet_node_removal_percentage = RemoveSubnetNodeEpochPercentage::<Test>::get();
  let epoch_length = EpochLength::get();

  let block_span_can_remove_peer = (epoch_length as u128 * subnet_node_removal_percentage / PERCENTAGE_FACTOR) as u64;

  let start_removal_block = (CONSENSUS_STEPS + (current_block_number - (current_block_number % epoch_length))) as u64;

  let end_removal_block = block_span_can_remove_peer + (current_block_number - (current_block_number % epoch_length));
  
  if current_block_number < start_removal_block {
    System::set_block_number(start_removal_block);
  } else if current_block_number > end_removal_block {
    System::set_block_number(start_removal_block + epoch_length);
  }
}

// fn subnet_node_data(start: u8, end: u8) -> Vec<SubnetNodeData<<Test as frame_system::Config>::AccountId>> {
fn subnet_node_data(start: u32, end: u32) -> Vec<SubnetNodeData> {
  // initialize peer consensus data array
  let mut subnet_node_data: Vec<SubnetNodeData> = Vec::new();
  for n in start..end {
    // let peer_subnet_node_data: SubnetNodeData<<Test as frame_system::Config>::AccountId> = SubnetNodeData {
    //   // account_id: account(n),
    //   peer_id: peer(n),
    //   score: DEFAULT_SCORE,
    // };
    let peer_subnet_node_data: SubnetNodeData = SubnetNodeData {
      peer_id: peer(n),
      score: DEFAULT_SCORE,
    };
    subnet_node_data.push(peer_subnet_node_data);
  }
  subnet_node_data
}

// fn subnet_node_data_invalid_scores(start: u8, end: u8) -> Vec<SubnetNodeData<<Test as frame_system::Config>::AccountId>> {
fn subnet_node_data_invalid_scores(start: u32, end: u32) -> Vec<SubnetNodeData> {
  // initialize peer consensus data array
  // let mut subnet_node_data: Vec<SubnetNodeData<<Test as frame_system::Config>::AccountId>> = Vec::new();
  let mut subnet_node_data: Vec<SubnetNodeData> = Vec::new();
  for n in start..end {
    // let peer_subnet_node_data: SubnetNodeData<<Test as frame_system::Config>::AccountId> = SubnetNodeData {
    //   // account_id: account(n),
    //   peer_id: peer(n),
    //   score: 10000000000,
    // };
    let peer_subnet_node_data: SubnetNodeData = SubnetNodeData {
      peer_id: peer(n),
      score: 10000000000,
    };
    subnet_node_data.push(peer_subnet_node_data);
  }
  subnet_node_data
}

fn post_successful_add_subnet_node_asserts(
  n: u32, 
  subnet_id: u32, 
  amount: u128
) {
  assert_eq!(Network::account_model_stake(account(n), subnet_id), amount);
  assert_eq!(Network::total_account_stake(account(n)), amount);    
  assert_eq!(Network::total_subnet_nodes(subnet_id), (n + 1) as u32);
}

// check data after adding multiple peers
// each peer must have equal staking amount per subnet
fn post_successful_add_subnet_nodes_asserts(
  total_peers: u32,
  stake_per_peer: u128,  
  subnet_id: u32, 
) {
  let amount_staked = total_peers as u128 * stake_per_peer;
  assert_eq!(Network::total_model_stake(subnet_id), amount_staked);
}

fn post_remove_subnet_node_ensures(n: u32, subnet_id: u32) {
  // ensure SubnetNodesData removed
  let subnet_node_data = SubnetNodesData::<Test>::try_get(subnet_id, account(n));
  assert_eq!(subnet_node_data, Err(()));

  // ensure SubnetNodeAccount removed
  let subnet_node_account = SubnetNodeAccount::<Test>::try_get(subnet_id, peer(n));
  assert_eq!(subnet_node_account, Err(()));

  // ensure SubnetNodeConsensusResults removed
  let subnet_node_consensus_results = SubnetNodeConsensusResults::<Test>::try_get(subnet_id, account(n));
  assert_eq!(subnet_node_consensus_results, Err(()));

  // ensure SubnetAccount u64 updated to current block
  let model_accounts = SubnetAccount::<Test>::get(subnet_id.clone());
  let model_account = model_accounts.get(&account(n));
  assert_eq!(model_accounts.get(&account(n)), Some(&System::block_number()));

  for class_id in SubnetNodeClass::iter() {
    let node_sets = SubnetNodesClasses::<Test>::get(subnet_id, class_id);
    assert_eq!(node_sets.get(&account(n)), None);
  }
}

fn post_remove_unstake_ensures(n: u32, subnet_id: u32) {
  // ensure SubnetAccount is removed after unstaking to 0
  let model_accounts = SubnetAccount::<Test>::get(subnet_id.clone());
  let model_account = model_accounts.get(&account(n));
  assert_eq!(model_accounts.get(&account(n)), None);
}

// The following should be ensured after form_consensus is rate
// This should work regardless if there are consensus issues or not
fn post_successful_form_consensus_ensures(subnet_id: u32) {
  let peer_consensus_epoch_submitted = NodeConsensusEpochSubmitted::<Test>::iter().count();
  assert_eq!(peer_consensus_epoch_submitted, 0);
  let peer_consensus_epoch_confirmed = NodeConsensusEpochUnconfirmed::<Test>::iter().count();
  assert_eq!(peer_consensus_epoch_confirmed, 0);
  let model_total_consensus_submits = SubnetTotalConsensusSubmits::<Test>::iter().count();
  assert_eq!(model_total_consensus_submits, 0);
  let model_consensus_epoch_unconfirmed_count = SubnetConsensusEpochUnconfirmedCount::<Test>::try_get(subnet_id.clone());
  assert_eq!(model_consensus_epoch_unconfirmed_count, Err(()));
}

fn post_successful_generate_emissions_ensures() {
  let models_in_consensus = SubnetsInConsensus::<Test>::try_get();
  assert_eq!(models_in_consensus, Err(()));

  let models_in_consensus = SubnetsInConsensus::<Test>::get();
  assert_eq!(models_in_consensus.len(), 0);


  let subnet_node_consensus_results = SubnetNodeConsensusResults::<Test>::iter().count();
  assert_eq!(subnet_node_consensus_results, 0);
}

// fn post_successful_dishonesty_proposal_ensures(proposer: u32, votee: u32, subnet_id: u32) {
//   let subnet_node_dishonesty_vote = SubnetNodeDishonestyVote::<Test>::get(subnet_id.clone(), account(votee));

//   assert_eq!(subnet_node_dishonesty_vote.subnet_id, subnet_id.clone());
//   assert_eq!(subnet_node_dishonesty_vote.peer_id, peer(votee).into());
//   assert_eq!(subnet_node_dishonesty_vote.total_votes, 1);
//   assert_eq!(subnet_node_dishonesty_vote.votes[0], account(proposer));
//   assert_ne!(subnet_node_dishonesty_vote.start_block, 0);
// }

fn add_subnet_node(
  account_id: u32, 
  subnet_id: u32,
  peer_id: u32,
  ip: String,
  port: u16,
  amount: u128
) -> Result<(), sp_runtime::DispatchError> {
  Network::add_subnet_node(
    RuntimeOrigin::signed(account(account_id)),
    subnet_id,
    peer(peer_id),
    amount,
  )
}

#[test]
fn test_add_model() {
  new_test_ext().execute_with(|| {

    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    assert_eq!(Network::total_models(), 1);

    let model_path_2: Vec<u8> = "petals-team-2/StableBeluga2".into();

    build_subnet(model_path_2.clone());

    assert_eq!(Network::total_models(), 2);

  })
}

#[test]
fn test_add_model_err() {
  new_test_ext().execute_with(|| {

    // let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    // let add_subnet_data = PreSubnetData {
    //   path: subnet_path.clone().into(),
    //   memory_mb: 50000,
    // };
  
    // assert_err!(
    //   Network::add_subnet(
    //     RuntimeOrigin::signed(account(0)),
    //     add_subnet_data.clone(),
    //   ),
    //   Error::<Test>::SubnetNotVotedIn
    // );

    // build_subnet(subnet_path.clone());

    // assert_eq!(Network::total_models(), 1);

    // assert_err!(
    //   Network::add_subnet(
    //     RuntimeOrigin::signed(account(0)),
    //     add_subnet_data.clone(),
    //   ),
    //   Error::<Test>::SubnetExist
    // );
  })
}

#[test]
fn test_remove_model() {
  new_test_ext().execute_with(|| {

    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    assert_eq!(Network::total_models(), 1);
    let add_subnet_data = PreSubnetData {
      path: subnet_path.clone().into(),
      memory_mb: 50000,
    };
    assert_ok!(
      Network::deactivate_subnet(
        account(0),
        account(0),
        add_subnet_data,
      )
    );
  
    // assert_ok!(
    //   Network::vote_model_out(
    //     RuntimeOrigin::signed(account(0)), 
    //     subnet_path.clone()
    //   )
    // );

    // let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    // assert_ok!(
    //   Network::remove_model(
    //     RuntimeOrigin::signed(account(0)),
    //     subnet_id,
    //   ) 
    // );

    // Total models should stay constant as its an index value
    assert_eq!(Network::total_models(), 1);
  })
}

#[test]
fn test_remove_model_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    assert_eq!(Network::total_models(), 1);

    assert_err!(
      Network::remove_subnet(
        RuntimeOrigin::signed(account(0)),
        255,
      ),
      Error::<Test>::SubnetNotExist
    );
  })
}

// #[test]
// fn test_add_model_max_models_err() {
//   new_test_ext().execute_with(|| {
//     let n_models: u32 = Network::max_models() + 1;

//     for m in 0..n_models {
//       let subnet_path = format!("petals-team-{m}/StableBeluga");
//       let subnet_path_2 = format!("petals-team-{m}/StableBeluga2");
//       let add_subnet_data = PreSubnetData {
//         path: subnet_path.clone().into(),
//         memory_mb: 50000,
//       };
  
//       assert_ok!(
//         Network::activate_subnet(
//           account(0),
//           account(0),
//           add_subnet_data,
//         )
//       );
  
//       // assert_ok!(
//       //   Network::vote_model(
//       //     RuntimeOrigin::signed(account(0)), 
//       //     subnet_path.clone().into(),
//       //   )
//       // );
//       let add_subnet_data = PreSubnetData {
//         path: subnet_path.clone().into(),
//         memory_mb: 50000,
//       };

//       if m+1 < n_models {
//         assert_ok!(
//           Network::activate_subnet(
//             account(0),
//             account(0),
//             add_subnet_data.clone(),
//           )
//         );  
//         // assert_ok!(
//         //   Network::add_subnet(
//         //     RuntimeOrigin::signed(account(0)),
//         //     add_subnet_data.clone()
//         //   ) 
//         // );
//       } else {
//         assert_err!(
//           Network::activate_subnet(
//             account(0),
//             account(0),
//             add_subnet_data.clone(),
//           ),
//           Error::<Test>::MaxSubnets
//         );  
//       }
//     }
//   })
// }

#[test]
fn test_add_subnet_node_max_peers_err() {
  new_test_ext().execute_with(|| {
    let n_peers: u32 = Network::max_subnet_nodes() + 1;
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let mut total_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);
    for n in 0..n_peers {
      let _ = Balances::deposit_creating(&account(n), deposit_amount);

      if n+1 < n_peers {
        total_staked += amount;
        assert_ok!(
          add_subnet_node(
            n, 
            subnet_id.clone(),
            n,
            "172.20.54.234".into(),
            8888,
            amount
          )  
        );

        // assert_ok!(
        //   Network::add_subnet_node(
        //     RuntimeOrigin::signed(account(n)),
        //     subnet_id.clone(),
        //     peer(n),
        //     "172.20.54.234".into(),
        //     8888,
        //     amount,
        //   ) 
        // );
        assert_eq!(Network::total_subnet_nodes(1), (n + 1) as u32);
        assert_eq!(Network::account_model_stake(account(n), 1), amount);
        assert_eq!(Network::total_account_stake(account(n)), amount);
      } else {
        assert_err!(
          Network::add_subnet_node(
            RuntimeOrigin::signed(account(n)),
            subnet_id.clone(),
            peer(n),
            // "172.20.54.234".into(),
            // 8888,
            amount,
          ),
          Error::<Test>::SubnetNodesMax
        );
      }
    }

    assert_eq!(Network::total_stake(), total_staked);
    assert_eq!(Network::total_model_stake(1), total_staked);
    assert_eq!(TotalSubnetNodes::<Test>::get(1), n_peers-1);
  });
}

#[test]
fn test_add_subnet_node_model_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount: u128 = 1000;
    assert_err!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        0,
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ),
      Error::<Test>::SubnetNotExist
    );

    assert_eq!(Network::total_subnet_nodes(1), 0);

  })
}

#[test]
fn test_add_subnet_node_model_account_ineligible_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();
    let max_account_penalty_count = MaxAccountPenaltyCount::<Test>::get();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let amount: u128 = 1000;

    AccountPenaltyCount::<Test>::insert(account(0), max_account_penalty_count + 1);

    assert_err!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ),
      Error::<Test>::AccountIneligible
    );

    assert_eq!(Network::total_subnet_nodes(1), 0);
  })
}

#[test]
fn test_add_subnet_node_not_exists_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );
    assert_eq!(Network::total_subnet_nodes(1), 1);

    // add new peer_id under same account error
    assert_err!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(1),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ),
      Error::<Test>::SubnetNodeExist
    );

    assert_eq!(Network::total_subnet_nodes(1), 1);

    // add same peer_id under new account error
    assert_err!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(1)),
        subnet_id.clone(),
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ),
      Error::<Test>::PeerIdExist
    );

    assert_eq!(Network::total_subnet_nodes(1), 1);

    // add new peer_id under same account error
    assert_err!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(1),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ),
      Error::<Test>::SubnetNodeExist
    );
  })
}

#[test]
fn test_add_subnet_node_stake_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 100000;
    let amount: u128 = 1;

    let _ = Balances::deposit_creating(&account(0), deposit_amount);
    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ),
      Error::<Test>::MinStakeNotReached
    );

  })
}

#[test]
fn test_add_subnet_node_stake_not_enough_balance_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 999999999999999999999;
    let amount: u128 =         1000000000000000000000;

    // let deposit_amount: u128 = 999;
    // let amount: u128 = 1000;

    let _ = Balances::deposit_creating(&account(255), deposit_amount);
    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(255)),
        subnet_id,
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ),
      Error::<Test>::NotEnoughBalanceToStake
    );

  })
}

#[test]
fn test_add_subnet_node_invalid_peer_id_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let n_peers: u32 = Network::max_subnet_nodes();

    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let _ = Balances::deposit_creating(&account(0), deposit_amount);
    amount_staked += amount;

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let peer_id = format!("2");
    let peer: PeerId = PeerId(peer_id.into());
    assert_err!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer,
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ),
      Error::<Test>::InvalidPeerId
    );
  })
}

// #[test]
// fn test_add_subnet_node_invalid_ip_address_err() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     build_subnet(subnet_path.clone());
//     assert_eq!(Network::total_models(), 1);

//     let n_peers: u32 = Network::max_subnet_nodes();

//     let deposit_amount: u128 = 10000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let _ = Balances::deposit_creating(&account(0), deposit_amount);
//     amount_staked += amount;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     assert_err!(
//       Network::add_subnet_node(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id,
//         peer(0),
//         // "0.0.0".into(),
//         // 8888,
//         amount,
//       ),
//       Error::<Test>::InvalidIpAddress
//     );

//     assert_err!(
//       Network::add_subnet_node(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id,
//         peer(0),
//         // "127.0.0.1".into(),
//         // 8888,
//         amount,
//       ),
//       Error::<Test>::InvalidIpAddress
//     );

//   })
// }

#[test]
fn test_add_subnet_node_remove_readd_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      )
    );

    assert_ok!(
      Network::remove_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
      )
    );

    assert_err!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ), 
      Error::<Test>::RequiredUnstakeEpochsNotMet
    );
  });
}

#[test]
fn test_add_subnet_node_remove_readd() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      )
    );

    assert_ok!(
      Network::remove_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
      )
    );

    let epoch_length = EpochLength::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<Test>::get();

    System::set_block_number(System::block_number() + epoch_length * min_required_unstake_epochs);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );
  });
}

#[test]
fn test_add_subnet_node_remove_stake_partial_readd() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      )
    );

    // increase account subnet stake to simulate rewards
    AccountSubnetStake::<Test>::insert(&account(0), subnet_id.clone(), amount + 100);

    assert_ok!(
      Network::remove_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
      )
    );

    // once blocks have been increased, account can either remove stake in part or in full or readd subnet peer
    let epoch_length = EpochLength::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<Test>::get();

    System::set_block_number(System::block_number() + epoch_length * min_required_unstake_epochs);

    assert_ok!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        amount,
      )
    );

    // should be able to readd after unstaking
    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );
  });
}

#[test]
fn test_add_subnet_node_remove_stake_readd() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      )
    );

    assert_ok!(
      Network::remove_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
      )
    );

    // once blocks have been increased, account can either remove stake in part or in full or readd subnet peer
    let epoch_length = EpochLength::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<Test>::get();
    System::set_block_number(System::block_number() + epoch_length * min_required_unstake_epochs);

    let remaining_account_stake_balance: u128 = AccountSubnetStake::<Test>::get(&account(0), subnet_id.clone());

    assert_ok!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        remaining_account_stake_balance,
      )
    );

    // should be able to readd after unstaking
    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );
  });
}

#[test]
fn test_add_subnet_node() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let n_peers: u32 = Network::max_subnet_nodes();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    let node_set = SubnetNodesClasses::<Test>::get(subnet_id.clone(), SubnetNodeClass::Idle);
    assert_eq!(node_set.len(), n_peers as usize);

    assert_eq!(Network::total_stake(), amount_staked);
    assert_eq!(Network::total_model_stake(subnet_id.clone()), amount_staked);
  })
}

#[test]
fn test_update_subnet_node_peer_id_existing_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      )
    );

    let node_set = SubnetNodesClasses::<Test>::get(subnet_id.clone(), SubnetNodeClass::Idle);
    assert_eq!(node_set.len(), 1);

    assert_err!(
      Network::update_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(0),
      ),
      Error::<Test>::PeerIdExist
    );
  });
}

#[test]
fn test_update_subnet_node_invalid_epoch_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    // Update the epoch_length in case its too small
    // if the removal/update span is 2 blocks or less 
    // the consensus steps logic will conflict and testing won't pass
    // For anyone reading this, this doesn't impact logic but we expect
    // a specific Error to be returned
    // EpochLength::<Test>::set(100);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      )
    );

    make_subnet_node_removable();

    System::set_block_number(System::block_number() - CONSENSUS_STEPS);

    assert_err!(
      Network::update_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(1),
      ),
      Error::<Test>::InvalidRemoveOrUpdateSubnetNodeBlock
    );
  });
}

#[test]
fn test_update_subnet_node_during_invalid_block_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      )
    );

    make_subnet_node_consensus_data_submittable();

    assert_err!(
      Network::update_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(1),
      ),
      Error::<Test>::InvalidRemoveOrUpdateSubnetNodeBlock
    );
  });
}

#[test]
fn test_update_subnet_node_during_submit_epoch_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      )
    );

    make_subnet_node_removable();

    assert_err!(
      Network::update_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(1),
      ),
      Error::<Test>::NodeConsensusSubmitEpochNotReached
    );
  });
}

#[test]
fn test_update_subnet_node() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      )
    );

    make_subnet_node_consensus_data_submittable();

    make_subnet_node_removable();

    assert_ok!(
      Network::update_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(1),
      )
    );
  });
}

// #[test]
// fn test_update_port_err() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     build_subnet(subnet_path.clone());
//     assert_eq!(Network::total_models(), 1);  

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     let _ = Balances::deposit_creating(&account(0), deposit_amount);
//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     assert_ok!(
//       Network::add_subnet_node(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(0),
//         // "172.20.54.234".into(),
//         // 8888,
//         amount,
//       )
//     );
//     assert_eq!(Network::account_model_stake(account(0), 1), amount);
//     assert_eq!(Network::total_account_stake(account(0)), amount);    
//     assert_eq!(Network::total_stake(), amount);
//     assert_eq!(Network::total_model_stake(1), amount);
//     assert_eq!(Network::total_subnet_nodes(1), 1);

//     // invalid account
//     assert_err!(
//       Network::update_port(
//         RuntimeOrigin::signed(account(255)),
//         subnet_id.clone(),
//         65535,
//       ),
//       Error::<Test>::SubnetNodeNotExist
//     );

//     // invalid subnet
//     assert_err!(
//       Network::update_port(
//         RuntimeOrigin::signed(account(0)),
//         255,
//         8889,
//       ),
//       Error::<Test>::SubnetNotExist
//     );


//   })
// }

// #[test]
// fn test_update_port() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     build_subnet(subnet_path.clone());
//     assert_eq!(Network::total_models(), 1);  

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     let _ = Balances::deposit_creating(&account(0), deposit_amount);
//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     assert_ok!(
//       Network::add_subnet_node(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(0),
//         // "172.20.54.234".into(),
//         // 8888,
//         amount,
//       ) 
//     );
//     assert_eq!(Network::account_model_stake(account(0), 1), amount);
//     assert_eq!(Network::total_account_stake(account(0)), amount);    
//     assert_eq!(Network::total_stake(), amount);
//     assert_eq!(Network::total_model_stake(1), amount);
//     assert_eq!(Network::total_subnet_nodes(1), 1);

//     assert_ok!(
//       Network::update_port(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         65535,
//       )
//     );
//   })
// }

// #[test]
// fn test_submit_consensus_min_required_model_epochs() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();
//     let min_subnet_nodes: u32 = Network::min_subnet_nodes();
//     let n_peers: u32 = min_subnet_nodes;

//     build_subnet(subnet_path.clone());

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     // System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     assert_eq!(Network::total_stake(), amount_staked);

//     // make_subnet_node_removable();

//     // System::set_block_number(System::block_number() + CONSENSUS_STEPS + 1);

//     make_consensus_data_submittable();

//     // initialize peer consensus data array
//     let subnet_node_data_vec = subnet_node_data(0, n_peers);

//     assert_err!(
//       Network::submit_consensus_data(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         subnet_node_data_vec.clone(),
//       ),
//       Error::<Test>::SubnetInitializeRequirement
//     );

//     // let scd = Network::submit_consensus_data(
//     //   RuntimeOrigin::signed(account(0)),
//     //   subnet_id.clone(),
//     //   subnet_node_data_vec.clone(),
//     // );

//     // assert_eq!(scd.unwrap().pays_fee, Pays::Yes);
//   });
// }

// #[test]
// fn test_submit_consensus_min_required_peer_epochs() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();
//     let min_subnet_nodes: u32 = Network::min_subnet_nodes();
//     let n_peers: u32 = min_subnet_nodes;

//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     make_consensus_data_submittable();

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     assert_eq!(Network::total_stake(), amount_staked);

//     // initialize peer consensus data array
//     let subnet_node_data_vec = subnet_node_data(0, n_peers);

//     assert_err!(
//       Network::submit_consensus_data(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         subnet_node_data_vec.clone(),
//       ),
//       Error::<Test>::NodeConsensusSubmitEpochNotReached
//     );
//   });
// }

// #[test]
// fn test_submit_consensus_min_subnet_nodes_err() {
//   new_test_ext().execute_with(|| {
//     // add first subnet
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();


//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     let _ = Balances::deposit_creating(&account(0), deposit_amount);

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     assert_ok!(
//       Network::add_subnet_node(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(0),
//         "172.20.54.234".into(),
//         8888,
//         amount,
//       )
//     );
//     post_successful_add_subnet_node_asserts(0, subnet_id.clone(), amount);

//     assert_eq!(Network::total_stake(), amount);
//     assert_eq!(Network::total_model_stake(1), amount);

//     make_subnet_node_consensus_data_submittable();
        
//     // initialize peer consensus data array
//     let subnet_node_data_vec = subnet_node_data(0, 1);

//     assert_err!(
//       Network::submit_consensus_data(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         subnet_node_data_vec.clone(),
//       ),
//       Error::<Test>::SubnetNodesMin
//     );
//   });
// }


// #[test]
// fn test_submit_consensus_len_err() {
//   new_test_ext().execute_with(|| {
//     // add first subnet
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();


//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let n_peers: u32 = Network::max_subnet_nodes();
//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;
//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_consensus_data_submittable();

//     // initialize peer consensus data array
//     let subnet_node_data_vec = subnet_node_data(0, n_peers + 1);

//     assert_err!(
//       Network::submit_consensus_data(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id,
//         subnet_node_data_vec.clone(),
//       ),
//       Error::<Test>::ConsensusDataInvalidLen
//     );
//   });
// }

// #[test]
// fn test_submit_consensus_min_peers_err() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let n_peers: u32 = Network::min_subnet_nodes() - 1;


//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_consensus_data_submittable();

//     // initialize peer consensus data array
//     let subnet_node_data_vec = subnet_node_data(0, n_peers);

//     for n in 0..n_peers {
//       assert_err!(
//         Network::submit_consensus_data(
//           RuntimeOrigin::signed(account(n)),
//           subnet_id,
//           subnet_node_data_vec.clone(),
//         ) ,
//         Error::<Test>::SubnetNodesMin
//       );
//     }
//   });
// }


// #[test]
// fn test_submit_consensus_already_submitted() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let n_peers: u32 = Network::max_subnet_nodes();

//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);
    
//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     // initialize peer consensus data array
//     let subnet_node_data_vec = subnet_node_data(0, n_peers);

//     make_subnet_node_consensus_data_submittable();

//     build_for_submit_consensus_data(subnet_id.clone(), 0, n_peers, 0, n_peers);

//     for n in 0..n_peers {
//       assert_err!(
//         Network::submit_consensus_data(
//           RuntimeOrigin::signed(account(n)),
//           subnet_id.clone(),
//           subnet_node_data_vec.clone(),
//         ) ,
//         Error::<Test>::ConsensusDataAlreadySubmitted
//       );
//     }

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    

//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     assert_eq!(Network::total_subnet_nodes(subnet_id.clone()), n_peers as u32);
//     post_successful_form_consensus_ensures(subnet_id.clone())
//   });
// }

// #[test]
// fn test_submit_consensus_account_err() {
//   new_test_ext().execute_with(|| {
//     // add first subnet
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();


//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let n_peers: u32 = Network::max_subnet_nodes();
//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_consensus_data_submittable();

//     // initialize peer consensus data array
//     let subnet_node_data_vec = subnet_node_data(0, n_peers + 1);

//     assert_err!(
//       Network::submit_consensus_data(
//         RuntimeOrigin::signed(account(255)),
//         subnet_id.clone(),
//         subnet_node_data_vec.clone(),
//       ),
//       Error::<Test>::SubnetNodeNotExist
//     );
//   });
// }

// #[test]
// fn test_submit_consensus_model_err() {
//   new_test_ext().execute_with(|| {
//     // add first subnet
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();


//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let n_peers: u32 = Network::max_subnet_nodes();
//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_consensus_data_submittable();

//     // initialize peer consensus data array
//     let subnet_node_data_vec = subnet_node_data(0, n_peers + 1);
    
//     let model_path_fake: Vec<u8> = "petals-team/StableBeluga3".into();

//     assert_err!(
//       Network::submit_consensus_data(
//         RuntimeOrigin::signed(account(0)),
//         0,
//         subnet_node_data_vec.clone(),
//       ),
//       Error::<Test>::SubnetNotExist
//     );
//   });
// }

// #[test]
// fn test_submit_consensus_data_invalid_consensus_block() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let n_peers: u32 = Network::max_subnet_nodes();

//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     // initialize peer consensus data array
//     let subnet_node_data_vec = subnet_node_data(0, n_peers);

//     // increase blocks to consensus step block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    
    
//     // submit peer consensus data per each peer
//     for n in 0..n_peers {
//       assert_err!(
//         Network::submit_consensus_data(
//           RuntimeOrigin::signed(account(n)),
//           subnet_id,
//           subnet_node_data_vec.clone(),
//         ),
//         Error::<Test>::InvalidSubmitEpochLength
//       );
//     }
//   });
// }

// #[test]
// fn test_submit_consensus_data() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let n_peers: u32 = Network::max_subnet_nodes();

//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     // initialize peer consensus data array
//     let subnet_node_data_vec = subnet_node_data(0, n_peers);

//     make_subnet_node_consensus_data_submittable();

//     build_for_submit_consensus_data(subnet_id.clone(), 0, n_peers, 0, n_peers);

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    
    
//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     post_successful_form_consensus_ensures(subnet_id.clone());

//     assert_eq!(Network::total_subnet_nodes(subnet_id.clone()), n_peers as u32);
//   });
// }

// #[test]
// fn test_submit_consensus_data_invalid_score() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let n_peers: u32 = Network::max_subnet_nodes();

//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);
//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     // initialize peer consensus data array
//     let subnet_node_data_vec = subnet_node_data_invalid_scores(0, n_peers);

//     make_subnet_node_consensus_data_submittable();

//     // submit peer consensus data per each peer
//     for n in 0..n_peers {
//       assert_err!(
//         Network::submit_consensus_data(
//           RuntimeOrigin::signed(account(n)),
//           subnet_id,
//           subnet_node_data_vec.clone(),
//         ),
//         Error::<Test>::InvalidScore
//       );
//     }

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    
    
//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     post_successful_form_consensus_ensures(subnet_id.clone());

//     assert_eq!(Network::total_subnet_nodes(1), n_peers as u32);
//   });
// }

// #[test]
// fn test_submit_consensus_data_dishonest() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let n_peers: u32 = Network::max_subnet_nodes();

//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);
//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );


//     // initialize peer consensus data array
//     // let subnet_node_data_vec = subnet_node_data(0, n_peers);

//     make_subnet_node_consensus_data_submittable();

//     // submit peer consensus data per each peer with data minus the last peer
//     build_for_submit_consensus_data(subnet_id.clone(), 0, n_peers-1, 0, n_peers);

//     // last peer is against first peer
//     let subnet_node_data_against = subnet_node_data(1, n_peers);

//     assert_ok!(
//       Network::submit_consensus_data(
//         RuntimeOrigin::signed(account(n_peers-1)),
//         subnet_id.clone(),
//         subnet_node_data_against,
//       ) 
//     );

//     // if any peers are left out, they submitted as unsuccessful and unsuccessful_consensus
//     // the SubnetNodeConsensusResults count should always be the count of total subnet peers
//     let submissions = SubnetNodeConsensusResults::<Test>::iter_key_prefix(subnet_id.clone());
// 		let len = submissions.count();
// 		assert_eq!(
// 			len, 
// 			n_peers as usize, 
// 			"SubnetNodeConsensusResults len mismatch."
// 		);

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    
    
//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     assert_eq!(Network::total_subnet_nodes(1), n_peers as u32);

//     assert_eq!(AccountPenaltyCount::<Test>::get(account(n_peers-1)), 1 as u32);

//     post_successful_form_consensus_ensures(subnet_id.clone());
//   });
// }

// #[test]
// fn test_submit_consensus_data_remove_peer() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let subnet_node_consensus_submit_percent_requirement = Network::subnet_node_consensus_submit_percent_requirement();

//     let n_peers: u32 = Network::max_subnet_nodes();
//     // Get amount of peers that need to keep a peer absent so they are removed through consensus
//     let n_consensus_peers: u32 = (n_peers as f64 * (subnet_node_consensus_submit_percent_requirement as f64 / 10000.0)).ceil() as u32;
//     // starting index of peers that should be removed
//     let n_peers_should_be_removed = n_peers - (n_peers - n_consensus_peers);
    
//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_consensus_data_submittable();

//     // submit peer consensus data per each submitting peer
//     // this removes any peer after `n_consensus_peers`
//     build_for_submit_consensus_data(subnet_id.clone(), 0, n_consensus_peers, 0, n_consensus_peers);

//     // if any peers are left out, they submitted as unsuccessful and unsuccessful_consensus
//     // the SubnetNodeConsensusResults count should always be the count of total subnet peers
//     let submissions = SubnetNodeConsensusResults::<Test>::iter_key_prefix(subnet_id.clone());
// 		let len = submissions.count();
// 		assert_eq!(
// 			len, 
// 			n_peers as usize, 
// 			"SubnetNodeConsensusResults len mismatch."
// 		);

//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).successful_consensus.len(), n_consensus_peers as usize);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).successful, n_consensus_peers as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).unsuccessful_consensus.len(), 0 as usize);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).unsuccessful, 0 as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).total_submits, n_consensus_peers as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).account_id, account(0));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).peer_id, peer(0));

    
//     // n_consensus_peers index should be removed
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_consensus_peers)).unsuccessful_consensus.len(), n_consensus_peers as usize);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_consensus_peers)).unsuccessful, n_consensus_peers as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_consensus_peers)).account_id, account(n_consensus_peers));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_consensus_peers)).peer_id, peer(n_consensus_peers));

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    
    
//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     post_successful_form_consensus_ensures(subnet_id.clone());

//     // peers should be removed
//     assert_eq!(Network::total_subnet_nodes(1), n_consensus_peers as u32);

//     // ensure all expected to be removed peers are removed and data is represented correctly
//     for n in n_consensus_peers..n_peers {
//       post_remove_subnet_node_ensures(n, subnet_id.clone());
//     }
//   });
// }

// #[test]
// fn test_submit_consensus_data_consensus_submit_percent_requirement() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let subnet_node_consensus_submit_percent_requirement = Network::subnet_node_consensus_submit_percent_requirement();

//     let n_peers: u32 = Network::max_subnet_nodes();
//     let n_required_peers: u32 = (n_peers as f64 * (subnet_node_consensus_submit_percent_requirement as f64 / 10000.0)).floor() as u32;
//     // Get 1 less of peers required to submit consensus data so consensus isn't calculated
//     let n_consensus_peers: u32 = n_required_peers - 1;


//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     // initialize peer consensus data array
//     // let subnet_node_data_vec = subnet_node_data(0, n_peers);

//     make_subnet_node_consensus_data_submittable();

//     // submit peer consensus data with not enough peers with data on each peer
//     build_for_submit_consensus_data(subnet_id.clone(), 0, n_consensus_peers, 0, n_peers);

//     // if any peers are left out, they submitted as unsuccessful and unsuccessful_consensus
//     // the SubnetNodeConsensusResults count should always be the count of total subnet peers
//     let submissions = SubnetNodeConsensusResults::<Test>::iter_key_prefix(subnet_id.clone());
// 		let len = submissions.count();
// 		assert_eq!(
// 			len, 
// 			n_peers as usize, 
// 			"SubnetNodeConsensusResults len mismatch."
// 		);

//     // peer consensus data is identical so data should all match, checking first and last peer 
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).successful_consensus.len(), n_consensus_peers as usize);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).successful, n_consensus_peers as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).unsuccessful_consensus.len(), 0 as usize);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).unsuccessful, 0 as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).total_submits, n_consensus_peers as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).account_id, account(0));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).peer_id, peer(0));
//     // assert_eq!(Network::subnet_node_consensus_results(1, account(n_consensus_peers)).unsuccessful_consensus.len(), n_consensus_peers as usize);
//     // assert_eq!(Network::subnet_node_consensus_results(1, account(n_consensus_peers)).unsuccessful, n_consensus_peers as u32);
//     // assert_eq!(Network::subnet_node_consensus_results(1, account(n_consensus_peers)).account_id, account(n_consensus_peers));
//     // assert_eq!(Network::subnet_node_consensus_results(1, account(n_consensus_peers)).peer_id, peer(n_consensus_peers));

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    
    
//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     // nothing should change
//     // consensus is remo ed
//     assert_eq!(Network::total_subnet_nodes(subnet_id.clone()), n_peers);
//     // assert_eq!(Network::subnet_node_consensus_results(1, account(0)).successful_consensus.len(), 0 as usize);

//     // subnets SubnetConsensusEpochsErrors should now increment 1
//     // when not enough peers submit data or do not unconfirm data errors should increment
//     assert_eq!(SubnetConsensusEpochsErrors::<Test>::get(subnet_id.clone()), 1);

//     post_successful_form_consensus_ensures(subnet_id.clone())

//   });
// }

// #[test]
// fn test_generate_emissions() {
//   new_test_ext().execute_with(|| {
//     // minimum required stake vault to generate emissions is
//     // min = peer_count * 10000
//     StakeVaultBalance::<Test>::mutate(|n: &mut u128| *n += 4000000000000000000000);
    
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let n_peers: u32 = Network::max_subnet_nodes();

//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let deposit_amount: u128 = 10000000000000000000000;
//     let amount: u128 = 1000000000000000000000; // 1000.00 tokens

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);
//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     // initialize peer consensus data array
//     // let subnet_node_data_vec = subnet_node_data(0, n_peers);

//     make_subnet_node_consensus_data_submittable();

//     // submit peer consensus data per each peer
//     build_for_submit_consensus_data(subnet_id.clone(), 0, n_peers, 0, n_peers);

//     // if any peers are left out, they submitted as unsuccessful and unsuccessful_consensus
//     // the SubnetNodeConsensusResults count should always be the count of total subnet peers
//     let submissions = SubnetNodeConsensusResults::<Test>::iter_key_prefix(subnet_id.clone());
// 		let len = submissions.count();
// 		assert_eq!(
// 			len, 
// 			n_peers as usize, 
// 			"SubnetNodeConsensusResults len mismatch."
// 		);

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    
    
//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     post_successful_form_consensus_ensures(subnet_id.clone());

//     assert_eq!(Network::total_subnet_nodes(1), n_peers as u32);

//     // stake data should exist
//     let total_stake: u128 = TotalStake::<Test>::get();
//     let total_vault_balance: u128 = StakeVaultBalance::<Test>::get();
//     assert_ne!(total_stake, 0);
//     assert_ne!(total_vault_balance, 0);


//     // Set to correct generate emissions block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length) + 1)
//     );    
    
//     assert_ok!(
//       Network::do_generate_emissions(RuntimeOrigin::signed(account(0))) 
//     );

//     post_successful_generate_emissions_ensures();

//     // SubnetNodeConsensusResults is removed on successful emissions generation
//     let submissions = SubnetNodeConsensusResults::<Test>::iter_key_prefix(subnet_id.clone());
// 		let len = submissions.count();
// 		assert_eq!(
// 			len, 
// 			0, 
// 			"SubnetNodeConsensusResults len mismatch."
// 		);

//     // ensure balances have increased
//     for n in 0..n_peers {
//       let stake_balance = Network::account_model_stake(account(n), subnet_id);
//       assert_ne!(amount, stake_balance);
//     }

//     let expected_max_post_vault_balance: u128 = (amount_staked as f64 * 0.01) as u128;
//     let post_total_vault_balance: u128 = StakeVaultBalance::<Test>::get();
//     assert!(post_total_vault_balance <= expected_max_post_vault_balance, "post_total_vault_balance {:?} expected_max_post_vault_balance {:?}", post_total_vault_balance, expected_max_post_vault_balance);
//     // assert_ln!(post_total_vault_balance <= expected_max_post_vault_balance);

//     // Expect 0 because all numbers are divisible
//     // let post_total_vault_balance: u128 = StakeVaultBalance::<Test>::get();
//     // assert_eq!(post_total_vault_balance, 0);

//     // purposefully !assert
//     // assert_eq!(post_total_vault_balance, 1);

//   });
// }

// #[test]
// fn test_generate_emissions_all_math() {
//   new_test_ext().execute_with(|| {
//     StakeVaultBalance::<Test>::mutate(|n: &mut u128| *n += 480000000000000000000u128);

//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let n_models: u32 = Network::max_models();
//     let n_peers: u32 = Network::max_subnet_nodes();

//     for m in 0..n_models {
// 			let subnet_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
//       build_subnet(subnet_path.clone());
//     }

//     make_model_submittable();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     let stake_vault_balance: u128 = StakeVaultBalance::<Test>::get();

//     // ensure 1 of the amounts are above the max weight threshold
//     let amount_stake: Vec<u128> = vec![
//       45275256795453966845740,
//       5027522345254264457400,
//       1000000000000000000000,
//       1275223523454396574000,
//       3275245742724396659000,
//       3275245234234536237400,
//       3275245742453467247500,
//       3275245742345396234500,
//       3275245742362396360000,
//       3275245742342396323000,
//       45275256795453966845740,
//       5027522345254264457400,
//       1000000000000000000000,
//       1275223523454396574000,
//       3275245742724396659000,
//       3275245234234536237400,
//       3275245742453467247500,
//       3275245742345396234500,
//       3275245742362396360000,
//       3275245742342396323000,
//       45275256795453966845740,
//       5027522345254264457400,
//       1000000000000000000000,
//       1275223523454396574000,
//       3275245742724396659000,
//       3275245234234536237400,
//       3275245742453467247500,
//       3275245742345396234500,
//       3275245742362396360000,
//       3275245742342396323000,
//       45275256795453966845740,
//       5027522345254264457400,
//       1000000000000000000000,
//       1275223523454396574000,
//       3275245742724396659000,
//       3275245234234536237400,
//       3275245742453467247500,
//       3275245742345396234500,
//       3275245742362396360000,
//       3275245742342396323000,
//       45275256795453966845740,
//       5027522345254264457400,
//       1000000000000000000000,
//       1275223523454396574000,
//       3275245742724396659000,
//       3275245234234536237400,
//       3275245742453467247500,
//       3275245742345396234500,
//       3275245742362396360000,
//       3275245742342396323000,
//       45275256795453966845740,
//       5027522345254264457400,
//       1000000000000000000000,
//       1275223523454396574000,
//       3275245742724396659000,
//       3275245234234536237400,
//       3275245742453467247500,
//       3275245742345396234500,
//       3275245742362396360000,
//       3275245742342396323000,
//       3275245742453467247500,
//       3275245742345396234500,
//       3275245742362396360000,
//       3275245742342396323000,
//     ];

//     let mut stake_sum: u128 = 0;
//     let mut i: u32 = 0;
//     for stake in amount_stake.clone() {
//       stake_sum += stake * n_peers as u128;
//       i += 1;

//       if i >= n_models {
//         break
//       }
//     }

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     // add subnet peers
//     for m in 0..n_models {
// 			let subnet_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
//       let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();
//       let amount: u128 = amount_stake[m as usize] as u128;

//       // amount_staked += build_subnet_nodes(subnet_id.clone(), 0, n_peers as u8, amount + deposit_amount, amount);

//       for n in 0..n_peers {
//         let _ = Balances::deposit_creating(&account(n), amount + deposit_amount);
//         amount_staked += amount;
//         assert_ok!(
//           Network::add_subnet_node(
//             RuntimeOrigin::signed(account(n)),
//             subnet_id.clone(),
//             peer(n),
//             "172.20.54.234".into(),
//             8888,
//             amount,
//           ) 
//         );
//       } 
//       assert_eq!(Network::total_subnet_nodes(subnet_id.clone()), (n_peers) as u32);
//     }

//     // assert stake is correct to what's expected
//     let total_stake: u128 = TotalStake::<Test>::get();
//     assert_eq!(total_stake, stake_sum);

//     // initialize peer consensus data array
//     let subnet_node_data_vec = subnet_node_data(0, n_peers);

//     make_subnet_node_consensus_data_submittable();

//     // submit peer consensus data per subnet per each peer
//     for m in 0..n_models {
// 			let subnet_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
//       let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();
//       // submit peer consensus data per each peer
//       build_for_submit_consensus_data(subnet_id.clone(), 0, n_peers, 0, n_peers);
//       // if any peers are left out, they submitted as unsuccessful and unsuccessful_consensus
//       // the SubnetNodeConsensusResults count should always be the count of total subnet peers
//       let submissions = SubnetNodeConsensusResults::<Test>::iter_key_prefix(subnet_id.clone());
//       let len = submissions.count();
//       assert_eq!(
//         len, 
//         n_peers as usize, 
//         "SubnetNodeConsensusResults len mismatch."
//       );  
//     }

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    

//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     for m in 0..n_models {
//       let subnet_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
//       let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();
//       post_successful_form_consensus_ensures(subnet_id.clone());
//     }

//     // // stake data should exist
//     let total_stake: u128 = TotalStake::<Test>::get();
//     let total_vault_balance: u128 = StakeVaultBalance::<Test>::get();
//     assert_ne!(total_stake, 0);
//     assert_ne!(total_vault_balance, 0);

//     // Set to correct generate emissions block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length) + 1)
//     );    

//     assert_ok!(
//       Network::do_generate_emissions(RuntimeOrigin::signed(account(0))) 
//     );

//     post_successful_generate_emissions_ensures();
    
//     // ensure balances have increased
//     for m in 0..n_models {
//       let subnet_path: Vec<u8> = format!("petals-team-{m}/StableBeluga").into();
//       let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();
//       let amount: u128 = amount_stake[m as usize] as u128;
//       for n in 0..n_peers {
//         let stake_balance = Network::account_model_stake(account(n), subnet_id);
//         assert_ne!(amount, stake_balance);
//       }
//     }

//     // when weights are imbalanced vs. max reward weight, the total weight may
//     // be under 100.0. We use 99% as leeway to ensure it's working
//     // We can assume the algorithm will be more accurate than 99% depending on the
//     // starting staking numbers
//     // We use 1% of the stake vault balance will be remaining after rewards
//     let expected_max_post_vault_balance: u128 = (amount_staked as f64 * 0.01) as u128;
//     let post_total_vault_balance: u128 = StakeVaultBalance::<Test>::get();

//     assert!(post_total_vault_balance <= expected_max_post_vault_balance);

//     // purposefully !assert
//     // assert_eq!(post_total_vault_balance, 1);

//   });
// }

// #[test]
// fn test_generate_emissionsf() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let n_peers: u32 = Network::max_subnet_nodes();

//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let deposit_amount: u128 = 10000000000000000000000;
//     let amount: u128 = 1000000000000000000000; // 1000.00 tokens

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);
//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     // initialize peer consensus data array
//     // let subnet_node_data_vec = subnet_node_data(0, n_peers);

//     make_subnet_node_consensus_data_submittable();

//     // submit peer consensus data per each peer
//     build_for_submit_consensus_data(subnet_id.clone(), 0, n_peers, 0, n_peers);

//     // if any peers are left out, they submitted as unsuccessful and unsuccessful_consensus
//     // the SubnetNodeConsensusResults count should always be the count of total subnet peers
//     let submissions = SubnetNodeConsensusResults::<Test>::iter_key_prefix(subnet_id.clone());
// 		let len = submissions.count();
// 		assert_eq!(
// 			len, 
// 			n_peers as usize, 
// 			"SubnetNodeConsensusResults len mismatch."
// 		);

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    
    
//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     post_successful_form_consensus_ensures(subnet_id.clone());

//     assert_eq!(Network::total_subnet_nodes(1), n_peers as u32);

//     // stake data should exist
//     let total_stake: u128 = TotalStake::<Test>::get();
//     assert_ne!(total_stake, 0);

//     // Set to correct generate emissions block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length) + 1)
//     );    
    
//     assert_ok!(
//       Network::do_generate_emissionsf(RuntimeOrigin::signed(account(0))) 
//     );

//     post_successful_generate_emissions_ensures();

//     // SubnetNodeConsensusResults is removed on successful emissions generation
//     let submissions = SubnetNodeConsensusResults::<Test>::iter_key_prefix(subnet_id.clone());
// 		let len = submissions.count();
// 		assert_eq!(
// 			len, 
// 			0, 
// 			"SubnetNodeConsensusResults len mismatch."
// 		);

//     // ensure balances have increased
//     for n in 0..n_peers {
//       let stake_balance = Network::account_model_stake(account(n), subnet_id);
//       assert_ne!(amount, stake_balance);
//     }
//   });
// }

#[test]
fn test_remove_peer_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_subnet(subnet_path.clone());
    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::remove_subnet_node(
        RuntimeOrigin::signed(account(255)),
        0,
      ),
      Error::<Test>::SubnetNotExist
    );

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );
    post_successful_add_subnet_node_asserts(0, subnet_id.clone(), amount);

    post_successful_add_subnet_nodes_asserts(
      1,
      amount,
      subnet_id.clone(),
    );

    assert_eq!(Network::total_stake(), amount);

    assert_err!(
      Network::remove_subnet_node(
        RuntimeOrigin::signed(account(255)),
        subnet_id.clone(),
      ),
      Error::<Test>::SubnetNodeNotExist
    );

    assert_eq!(Network::total_subnet_nodes(1), 1);

  });
}

#[test]
fn test_remove_peer_is_included_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_subnet(subnet_path.clone());
    // System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::remove_subnet_node(
        RuntimeOrigin::signed(account(255)),
        0,
      ),
      Error::<Test>::SubnetNotExist
    );

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );
    post_successful_add_subnet_node_asserts(0, subnet_id.clone(), amount);

    post_successful_add_subnet_nodes_asserts(
      1,
      amount,
      subnet_id.clone(),
    );

    assert_eq!(Network::total_stake(), amount);

    make_subnet_node_included();

    assert_err!(
      Network::remove_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
      ),
      Error::<Test>::InvalidRemoveOrUpdateSubnetNodeBlock
    );

    assert_eq!(Network::total_subnet_nodes(1), 1);

  });
}


#[test]
fn test_remove_peer_unstake_epochs_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    let epoch_length = EpochLength::get();

    System::set_block_number(System::block_number() + epoch_length);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );
    post_successful_add_subnet_node_asserts(0, subnet_id.clone(), amount);
    assert_eq!(Network::total_subnet_nodes(1), 1);
    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);

    // make_subnet_node_removable();


    System::set_block_number(System::block_number() + epoch_length);

    assert_ok!(
      Network::remove_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
      ) 
    );

    post_remove_subnet_node_ensures(0, subnet_id.clone());

    assert_eq!(Network::total_subnet_nodes(1), 0);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        amount,
      ),
      Error::<Test>::RequiredUnstakeEpochsNotMet,
    );
    
    let epoch_length = EpochLength::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<Test>::get();
    System::set_block_number(System::block_number() + epoch_length * min_required_unstake_epochs);
    
    assert_ok!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        amount,
      )
    );
  });
}

#[test]
fn test_remove_peer_unstake_total_balance() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );
    post_successful_add_subnet_node_asserts(0, subnet_id.clone(), amount);
    assert_eq!(Network::total_subnet_nodes(1), 1);
    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);

    assert_ok!(
      Network::remove_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
      ) 
    );

    post_remove_subnet_node_ensures(0, subnet_id.clone());

    assert_eq!(Network::total_subnet_nodes(1), 0);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);
    
    let epoch_length = EpochLength::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<Test>::get();
    System::set_block_number(System::block_number() + epoch_length * min_required_unstake_epochs);
    
    let remaining_account_stake_balance: u128 = AccountSubnetStake::<Test>::get(&account(0), subnet_id.clone());

    assert_ok!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        remaining_account_stake_balance,
      )
    );

    post_remove_unstake_ensures(0, subnet_id.clone());
  });
}


#[test]
fn test_remove_peer() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_subnet(subnet_path.clone());
    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );
    post_successful_add_subnet_node_asserts(0, subnet_id.clone(), amount);

    assert_eq!(Network::total_subnet_nodes(1), 1);
    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);

    // make_subnet_node_removable();
    // should be able to be removed is initialization period doesn't reach inclusion epochs

    assert_ok!(
      Network::remove_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
      ) 
    );
    post_remove_subnet_node_ensures(0, subnet_id.clone());
    assert_eq!(Network::total_subnet_nodes(1), 0);

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
      Error::<Test>::SubnetNotExist,
    );

    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_subnet(subnet_path.clone());
    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );
    post_successful_add_subnet_node_asserts(0, subnet_id.clone(), amount);

    assert_eq!(Network::total_subnet_nodes(1), 1);
    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::add_to_stake(
        RuntimeOrigin::signed(account(255)),
        subnet_id.clone(),
        amount,
      ),
      Error::<Test>::SubnetNodeNotExist,
    );

  });
}

#[test]
fn test_add_to_stake() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_subnet(subnet_path.clone());
    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );
    post_successful_add_subnet_node_asserts(0, subnet_id.clone(), amount);

    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);
    assert_eq!(Network::total_subnet_nodes(1), 1);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_to_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
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

    // attempt to remove on non-existent subnet_id
    assert_err!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(255)),
        0,
        amount,
      ),
      Error::<Test>::SubnetNodeNotExist,
    );

    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_subnet(subnet_path.clone());
    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );
    post_successful_add_subnet_node_asserts(0, subnet_id.clone(), amount);

    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);
    assert_eq!(Network::total_subnet_nodes(1), 1);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_err!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(255)),
        subnet_id.clone(),
        amount,
      ),
      Error::<Test>::SubnetNodeNotExist,
    );

    assert_err!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        0,
      ),
      Error::<Test>::RequiredUnstakeEpochsNotMet,
    );

    let epoch_length = EpochLength::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<Test>::get();
    System::set_block_number(System::block_number() + epoch_length * min_required_unstake_epochs);

    assert_err!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        0,
      ),
      Error::<Test>::NotEnoughStakeToWithdraw,
    );

    assert_err!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        amount+1,
      ),
      Error::<Test>::NotEnoughStakeToWithdraw,
    );

    assert_err!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        amount,
      ),
      Error::<Test>::MinStakeNotReached,
    );

  });
}

#[test]
fn test_remove_stake() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();


    build_subnet(subnet_path.clone());
    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );
    post_successful_add_subnet_node_asserts(0, subnet_id.clone(), amount);

    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);      
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);
    assert_eq!(Network::total_subnet_nodes(1), 1);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    // add double amount to stake
    assert_ok!(
      Network::add_to_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        amount,
      ) 
    );

    assert_eq!(Network::account_model_stake(account(0), 1), amount + amount);
    assert_eq!(Network::total_account_stake(account(0)), amount + amount);
    assert_eq!(Network::total_stake(), amount + amount);
    assert_eq!(Network::total_model_stake(1), amount + amount);

    let epoch_length = EpochLength::get();
    let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<Test>::get();
    System::set_block_number(System::block_number() + epoch_length * min_required_unstake_epochs);

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    // remove amount ontop
    assert_ok!(
      Network::remove_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        amount,
      )
    );

    assert_eq!(Network::account_model_stake(account(0), 1), amount);
    assert_eq!(Network::total_account_stake(account(0)), amount);
    assert_eq!(Network::total_stake(), amount);
    assert_eq!(Network::total_model_stake(1), amount);

  });
}

// #[test]
// fn test_form_consensus_unconfirm_consensus() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let n_peers: u32 = Network::max_subnet_nodes();
//     let unconfirm_threshold = SubnetConsensusUnconfirmedThreshold::<Test>::get();
//     let n_peers_unconfirm: u32 = (n_peers as f64 * (unconfirm_threshold as f64 / 10000.0)).ceil() as u32;

//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     // for n in 0..n_peers {
//     //   let _ = Balances::deposit_creating(&account(n), deposit_amount);
//     //   amount_staked += amount;
//     //   assert_ok!(
//     //     Network::add_subnet_node(
//     //       RuntimeOrigin::signed(account(n)),
//     //       subnet_id.clone(),
//     //       peer(n),
//     //       "172.20.54.234".into(),
// 		// 			8888,
//     //       amount,
//     //     ) 
//     //   );
//     //   post_successful_add_subnet_node_asserts(n, subnet_id.clone(), amount);
//     // }
//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_consensus_data_submittable();

//     // submit peer consensus data per each peer
//     build_for_submit_consensus_data(subnet_id.clone(), 0, n_peers, 0, n_peers);

//     // unconfirm consensus data up to the required threshold
//     for n in 0..n_peers_unconfirm {
//       assert_ok!(
//         Network::unconfirm_consensus_data(
//           RuntimeOrigin::signed(account(n)),
//           subnet_id,
//         ) 
//       );
//     }

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );

//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );
    

//     // unconfirming consensus data should remove all SubnetNodeConsensusResults
//     let submissions = SubnetNodeConsensusResults::<Test>::iter_key_prefix(subnet_id.clone());
// 		let len = submissions.count();
// 		assert_eq!(
// 			len, 
// 			0, 
// 			"SubnetNodeConsensusResults len mismatch."
// 		);

//     let max_model_model_unconfirmed_epochs = MaxSubnetConsensusUnconfirmedConsecutiveEpochs::<Test>::get();

//     // Should have increased uncofirmed epochs count
//     let model_uncofirmed_epochs = SubnetConsensusUnconfirmedConsecutiveEpochsCount::<Test>::get(subnet_id.clone());
// 		assert_eq!(model_uncofirmed_epochs, 1, "SubnetConsensusUnconfirmedConsecutiveEpochsCount incorrect.");

//     if model_uncofirmed_epochs > max_model_model_unconfirmed_epochs {
//       let model_epoch_errors = SubnetConsensusEpochsErrors::<Test>::get(subnet_id.clone());
//       assert_eq!(model_epoch_errors, 1, "SubnetConsensusEpochsErrors incorrect.");  
//     } else {
//       let model_epoch_errors = SubnetConsensusEpochsErrors::<Test>::get(subnet_id.clone());
//       assert_eq!(model_epoch_errors, 0, "SubnetConsensusEpochsErrors incorrect.");  
//     }

//     post_successful_form_consensus_ensures(subnet_id.clone());
//   });
// }

// #[test]
// fn test_submit_data_consensus_as_err_then_unconfirm_consensus_err() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let n_peers: u32 = Network::max_subnet_nodes();
//     let unconfirm_threshold = SubnetConsensusUnconfirmedThreshold::<Test>::get();
//     let n_peers_unconfirm: u32 = (n_peers as f64 * (unconfirm_threshold as f64 / 10000.0)).ceil() as u32;

//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_consensus_data_submittable();

//     let subnet_node_data_vec = subnet_node_data(0, n_peers);

//     // submit consensus data through `unconfirm_consensus_data`
//     // assert_ok!(
//     //   Network::submit_consensus_data(
//     //     RuntimeOrigin::signed(account(0)),
//     //     subnet_id,
//     //     subnet_node_data_vec.clone(),
//     //   ) 
//     // );

//     assert_ok!(
//       Network::unconfirm_consensus_data(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id,
//       )
//     );

//     // cannot call `unconfirm_consensus_data` if already submitted consensus data with an error state
//     assert_err!(
//       Network::unconfirm_consensus_data(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id,
//       ),
//       Error::<Test>::ConsensusDataAlreadyUnconfirmed
//     );
//   });
// }

// #[test]
// fn test_form_consensus() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = Network::max_subnet_nodes();
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_consensus_data_submittable();

//     build_for_submit_consensus_data(subnet_id.clone(), 0, n_peers, 0, n_peers);

//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).successful_consensus.len(), n_peers as usize);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).successful, n_peers as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).total_submits, n_peers as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).account_id, account(0));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).peer_id, peer(0));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).unsuccessful_consensus.len(), 0 as usize);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).unsuccessful, 0 as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).account_id, account(n_peers-1));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).peer_id, peer(n_peers-1));

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    

//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     post_successful_form_consensus_ensures(subnet_id.clone());
//   });
// }

// #[test]
// fn test_form_consensus_with_3_peers() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     MinSubnetNodes::<Test>::set(3);

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = 3;
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_consensus_data_submittable();

//     build_for_submit_consensus_data(subnet_id.clone(), 0, n_peers, 0, n_peers);

//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).successful_consensus.len(), n_peers as usize);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).successful, n_peers as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).total_submits, n_peers as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).account_id, account(0));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).peer_id, peer(0));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).unsuccessful_consensus.len(), 0 as usize);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).unsuccessful, 0 as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).account_id, account(n_peers-1));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).peer_id, peer(n_peers-1));

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    

//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     post_successful_form_consensus_ensures(subnet_id.clone());
//   });
// }


// #[test]
// fn test_form_consensus_with_4_peers() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     MinSubnetNodes::<Test>::set(4);

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = 4;
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_consensus_data_submittable();

//     build_for_submit_consensus_data(subnet_id.clone(), 0, n_peers, 0, n_peers);

//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).successful_consensus.len(), n_peers as usize);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).successful, n_peers as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).total_submits, n_peers as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).account_id, account(0));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).peer_id, peer(0));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).unsuccessful_consensus.len(), 0 as usize);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).unsuccessful, 0 as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).account_id, account(n_peers-1));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).peer_id, peer(n_peers-1));

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    

//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     post_successful_form_consensus_ensures(subnet_id.clone());
//   });
// }

// #[test]
// fn test_form_consensus_with_5_peers() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     MinSubnetNodes::<Test>::set(5);

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = 5;
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_consensus_data_submittable();

//     build_for_submit_consensus_data(subnet_id.clone(), 0, n_peers, 0, n_peers);

//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).successful_consensus.len(), n_peers as usize);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).successful, n_peers as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).total_submits, n_peers as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).account_id, account(0));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).peer_id, peer(0));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).unsuccessful_consensus.len(), 0 as usize);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).unsuccessful, 0 as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).account_id, account(n_peers-1));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).peer_id, peer(n_peers-1));

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    

//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     post_successful_form_consensus_ensures(subnet_id.clone());
//   });
// }

// #[test]
// fn test_form_consensus_remove_peer() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     build_subnet(subnet_path.clone());

//     let peer_removal_threshold = NodeRemovalThreshold::<Test>::get();

//     let n_peers: u32 = Network::max_subnet_nodes();
//     let n_required_against_peers: u32 = (n_peers as f64 * (peer_removal_threshold as f64 / 10000.0)).ceil() as u32 + 1;
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_consensus_data_submittable();

//     let subnet_node_data_vec = subnet_node_data(0, n_peers-1);

//     for n in 0..n_required_against_peers {
//       assert_ok!(
//         Network::submit_consensus_data(
//           RuntimeOrigin::signed(account(n)),
//           subnet_id,
//           subnet_node_data_vec.clone(),
//         ) 
//       );  
//     }

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    

//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     post_successful_form_consensus_ensures(subnet_id.clone());

//     post_remove_subnet_node_ensures((n_peers - 1) as u32, subnet_id.clone());
//   });
// }

// #[test]
// fn test_form_consensus_peer_submission_epochs() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let n_peers: u32 = Network::max_subnet_nodes();
    
//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_consensus_data_submittable();

//     build_for_submit_consensus_data(subnet_id.clone(), 0, n_peers, 0, n_peers);

//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).successful_consensus.len(), n_peers as usize);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).successful, n_peers as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).total_submits, n_peers as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).account_id, account(0));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(0)).peer_id, peer(0));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).unsuccessful_consensus.len(), 0 as usize);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).unsuccessful, 0 as u32);
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).account_id, account(n_peers-1));
//     assert_eq!(Network::subnet_node_consensus_results(1, account(n_peers-1)).peer_id, peer(n_peers-1));

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );

//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     post_successful_form_consensus_ensures(subnet_id.clone());
//   });
// }

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

// #[test]
// fn test_submit_consensus_data_remove_peer_peer_against_consensus_removal_threshold() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let subnet_node_consensus_submit_percent_requirement = Network::subnet_node_consensus_submit_percent_requirement();
//     let peer_against_consensus_removal_threshold: u128 = NodeAgainstConsensusRemovalThreshold::<Test>::get();

//     let n_peers: u32 = Network::max_subnet_nodes();
//     // Get amount of peers that need to keep a peer absent so they are removed through consensus
//     let n_consensus_peers: u32 = (n_peers as f64 * (subnet_node_consensus_submit_percent_requirement as f64 / 10000.0)).ceil() as u32;
//     // starting index of peers that should be removed
//     let n_peers_should_be_removed = n_peers - (n_peers - n_consensus_peers);
    
//     let n_peers_threshold = Network::percent_mul_round_up(n_peers as u128, peer_against_consensus_removal_threshold);

//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_consensus_data_submittable();

//     // submit peer consensus data per each peer with data minus submitting on behalf of the last peer
//     build_for_submit_consensus_data(subnet_id.clone(), 0, n_peers-1, 0, n_peers);

//     // last peer is against first peers threshold of peers to be removed
//     // when being against these percentage of peers
//     let subnet_node_data_against = subnet_node_data((n_peers_threshold - 1) as u32, n_peers);

//     assert_ok!(
//       Network::submit_consensus_data(
//         RuntimeOrigin::signed(account(n_peers-1)),
//         subnet_id.clone(),
//         subnet_node_data_against,
//       ) 
//     );
    
//     // if any peers are left out, they submitted as unsuccessful and unsuccessful_consensus
//     // the SubnetNodeConsensusResults count should always be the count of total subnet peers
//     let submissions = SubnetNodeConsensusResults::<Test>::iter_key_prefix(subnet_id.clone());
// 		let len = submissions.count();
// 		assert_eq!(
// 			len, 
// 			n_peers as usize, 
// 			"SubnetNodeConsensusResults len mismatch."
// 		);

//     // Set to correct consensus block
//     let epoch_length = EpochLength::get();
//     System::set_block_number(
//       epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//     );    
    
//     assert_ok!(
//       Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//     );

//     assert_eq!(Network::total_subnet_nodes(subnet_id.clone()), (n_peers - 1) as u32);

//     assert_eq!(AccountPenaltyCount::<Test>::get(account(n_peers-1)), (n_peers_threshold) as u32);

//     post_remove_subnet_node_ensures((n_peers - 1) as u32, subnet_id.clone());

//     post_successful_form_consensus_ensures(subnet_id.clone());
//   });
// }

// #[test]
// fn test_form_consensus_remove_ineligible_subnet_node() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let n_peers: u32 = Network::max_subnet_nodes();

//     build_subnet(subnet_path.clone());
//     make_model_submittable();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);
//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     let max_account_penalty_count: u32 = MaxAccountPenaltyCount::<Test>::get();
//     let epoch_length = EpochLength::get();

//     for n in 0..max_account_penalty_count {
//       let dishonest_peer_account_penalty_count = AccountPenaltyCount::<Test>::get(account(n_peers-1));

//       let total_subnet_nodes: u32 = Network::total_subnet_nodes(subnet_id.clone());

//       if total_subnet_nodes < n_peers {
//         break
//       }

//       if n > 0 {
//         System::set_block_number(System::block_number() + epoch_length);
//       }

//       // initialize peer consensus data array
//       make_subnet_node_consensus_data_submittable();

//       // submit peer consensus data per each peer with data minus the last peer
//       build_for_submit_consensus_data(subnet_id.clone(), 0, n_peers-1, 0, n_peers);

//       // last peer is against first peer
//       let subnet_node_data_against = subnet_node_data(1, n_peers);

//       assert_ok!(
//         Network::submit_consensus_data(
//           RuntimeOrigin::signed(account(n_peers-1)),
//           subnet_id.clone(),
//           subnet_node_data_against,
//         ) 
//       );

//       // if any peers are left out, they submitted as unsuccessful and unsuccessful_consensus
//       // the SubnetNodeConsensusResults count should always be the count of total subnet peers
//       let submissions = SubnetNodeConsensusResults::<Test>::iter_key_prefix(subnet_id.clone());
//       let len = submissions.count();
//       assert_eq!(
//         len, 
//         n_peers as usize, 
//         "SubnetNodeConsensusResults len mismatch."
//       );

//       // Set to correct consensus block
//       System::set_block_number(
//         epoch_length + (System::block_number() - (System::block_number() % epoch_length))
//       );    
      
//       assert_ok!(
//         Network::form_consensus(RuntimeOrigin::signed(account(0))) 
//       );

//       post_successful_form_consensus_ensures(subnet_id.clone());
//     }

//     assert_eq!(Network::total_subnet_nodes(subnet_id.clone()), (n_peers - 1) as u32);

//     assert!(AccountPenaltyCount::<Test>::get(account(n_peers-1)) >= max_account_penalty_count);
//   });
// }

// #[test]
// fn test_proposal_proposer_model_error() {
//   new_test_ext().execute_with(|| {
//     assert_err!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(0)),
//         1,
//         peer(1),
//         "test_data".into()
//       ),
//       Error::<Test>::SubnetNotExist
//     );
//   });
// }

// #[test]
// fn test_proposal_proposer_subnet_node_exists_error() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let dishonesty_voting_period = VotingPeriod::<Test>::get();

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = 2;
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_dishonesty_consensus_proposable();

//     assert_err!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(n_peers + 1)),
//         subnet_id.clone(),
//         peer(n_peers + 1),
//         "test_data".into()
//       ),
//       Error::<Test>::SubnetNodeNotExist
//     );

//   });
// }

// #[test]
// fn test_proposal_proposer_not_submittable() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let dishonesty_voting_period = VotingPeriod::<Test>::get();

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = Network::max_subnet_nodes();
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     assert_err!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(1),
//         "test_data".into()
//       ),
//       Error::<Test>::NodeConsensusSubmitEpochNotReached
//     );

//   });
// }

// #[test]
// fn test_proposal_votee_peer_id_exists() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let dishonesty_voting_period = VotingPeriod::<Test>::get();

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = Network::min_subnet_nodes();
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_dishonesty_consensus_proposable();

//     assert_err!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(n_peers + 1),
//         "test_data".into()
//       ),
//       Error::<Test>::PeerIdNotExist
//     );

//   });
// }

// #[test]
// fn test_proposal_min_subnet_nodes_error() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let dishonesty_voting_period = VotingPeriod::<Test>::get();

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = Network::min_subnet_nodes() - 1;
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_dishonesty_consensus_proposable();

//     assert_err!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(1),
//         "test_data".into()
//       ),
//       Error::<Test>::SubnetNodesMin
//     );

//   });
// }

// #[test]
// fn test_proposal() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = Network::max_subnet_nodes();
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     let dishonesty_voting_period = VotingPeriod::<Test>::get();

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     make_subnet_node_dishonesty_consensus_proposable();

//     assert_ok!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(1),
//         "test_data".into()
//       )
//     );

//     post_successful_dishonesty_proposal_ensures(0, 1, subnet_id.clone());
//   });
// }

// #[test]
// fn test_vote_subnet_node_dishonest_model_exists_error() {
//   new_test_ext().execute_with(|| {
//     assert_err!(
//       Network::vote_subnet_node_dishonest(
//         RuntimeOrigin::signed(account(0)),
//         1,
//         peer(1)
//       ),
//       Error::<Test>::SubnetNotExist
//     );
//   });
// }

// #[test]
// fn test_vote_subnet_node_dishonest_subnet_node_exists_error() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let dishonesty_voting_period = VotingPeriod::<Test>::get();

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = Network::max_subnet_nodes() - 1;
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_dishonesty_consensus_proposable();

//     assert_ok!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(1),
//         "test_data".into()
//       )
//     );

//     assert_err!(
//       Network::vote_subnet_node_dishonest(
//         RuntimeOrigin::signed(account(n_peers + 1)),
//         subnet_id.clone(),
//         peer(1)
//       ),
//       Error::<Test>::SubnetNodeNotExist
//     );

//   });
// }

// #[test]
// fn test_vote_subnet_node_dishonest_proposer_not_submittable() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let dishonesty_voting_period = VotingPeriod::<Test>::get();

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     build_subnet(subnet_path.clone());

//     let max_subnet_nodes: u32 = Network::max_subnet_nodes();

//     let n_peers: u32 = max_subnet_nodes - 1;
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_dishonesty_consensus_proposable();

//     assert_ok!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(1),
//         "test_data".into()
//       )
//     );

//     let _ = Balances::deposit_creating(&account(max_subnet_nodes), deposit_amount);

//     assert_ok!(
//       Network::add_subnet_node(
//         RuntimeOrigin::signed(account(max_subnet_nodes)),
//         subnet_id.clone(),
//         peer(max_subnet_nodes),
//         "172.20.54.234".into(),
//         8888,
//         amount,
//       ) 
//     );

//     assert_err!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(max_subnet_nodes)),
//         subnet_id.clone(),
//         peer(1),
//         "test_data".into()
//       ),
//       Error::<Test>::NodeConsensusSubmitEpochNotReached
//     );

//   });
// }

// #[test]
// fn test_vote_subnet_node_dishonest_min_subnet_nodes_error() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     let dishonesty_voting_period = VotingPeriod::<Test>::get();

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = Network::min_subnet_nodes();
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     make_subnet_node_dishonesty_consensus_proposable();

//     assert_ok!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(1),
//         "test_data".into()
//       )
//     );

//     make_subnet_node_removable();
//     assert_ok!(
//       Network::remove_subnet_node(
//         RuntimeOrigin::signed(account(n_peers-1)),
//         subnet_id.clone(),
//       ) 
//     );

//     assert_err!(
//       Network::vote_subnet_node_dishonest(
//         RuntimeOrigin::signed(account(2)),
//         subnet_id.clone(),
//         peer(1)
//       ),
//       Error::<Test>::SubnetNodesMin
//     );
//   });
// }

// #[test]
// fn test_vote_subnet_node_dishonest_peer_id_exists_error() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = Network::min_subnet_nodes();
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     let dishonesty_voting_period = VotingPeriod::<Test>::get();

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     make_subnet_node_dishonesty_consensus_proposable();

//     assert_ok!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(1),
//         "test_data".into()
//       )
//     );

//     post_successful_dishonesty_proposal_ensures(0, 1, subnet_id.clone());

//     assert_err!(
//       Network::vote_subnet_node_dishonest(
//         RuntimeOrigin::signed(account(1)),
//         subnet_id.clone(),
//         peer(n_peers + 1)
//       ),
//       Error::<Test>::PeerIdNotExist
//     );

//   });
// }

// #[test]
// fn test_vote_subnet_node_dishonest_not_proposed_error() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = Network::min_subnet_nodes();
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     let dishonesty_voting_period = VotingPeriod::<Test>::get();

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     make_subnet_node_dishonesty_consensus_proposable();

//     assert_err!(
//       Network::vote_subnet_node_dishonest(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(1)
//       ),
//       Error::<Test>::DishonestyVoteNotProposed
//     );

//   });
// }

// #[test]
// fn test_vote_subnet_node_dishonest_period_over_error() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = Network::max_subnet_nodes();
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     let dishonesty_voting_period = VotingPeriod::<Test>::get();

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     make_subnet_node_dishonesty_consensus_proposable();

//     assert_ok!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(1),
//         "test_data".into()
//       )
//     );

//     post_successful_dishonesty_proposal_ensures(0, 1, subnet_id.clone());

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     assert_err!(
//       Network::vote_subnet_node_dishonest(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(1)
//       ),
//       Error::<Test>::DishonestyVotingPeriodOver
//     );

//   });
// }

// #[test]
// fn test_vote_subnet_node_dishonest_duplicate_error() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = Network::max_subnet_nodes();
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     let dishonesty_voting_period = VotingPeriod::<Test>::get();

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     make_subnet_node_dishonesty_consensus_proposable();

//     assert_ok!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(1),
//         "test_data".into()
//       )
//     );

//     post_successful_dishonesty_proposal_ensures(0, 1, subnet_id.clone());

//     assert_ok!(
//       Network::vote_subnet_node_dishonest(
//         RuntimeOrigin::signed(account(2)),
//         subnet_id.clone(),
//         peer(1)
//       )
//     );

//     assert_err!(
//       Network::vote_subnet_node_dishonest(
//         RuntimeOrigin::signed(account(2)),
//         subnet_id.clone(),
//         peer(1)
//       ),
//       Error::<Test>::DishonestyVotingDuplicate
//     );
//   });
// }


// #[test]
// fn test_vote_subnet_node_dishonest_period_passed_error() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = Network::max_subnet_nodes();
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     let dishonesty_voting_period = VotingPeriod::<Test>::get();

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     make_subnet_node_dishonesty_consensus_proposable();

//     assert_ok!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(1),
//         "test_data".into()
//       )
//     );

//     post_successful_dishonesty_proposal_ensures(0, 1, subnet_id.clone());

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     assert_err!(
//       Network::vote_subnet_node_dishonest(
//         RuntimeOrigin::signed(account(2)),
//         subnet_id.clone(),
//         peer(1)
//       ),
//       Error::<Test>::DishonestyVotingPeriodOver
//     );
//   });
// }

// #[test]
// fn test_vote_subnet_node_dishonest() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = Network::max_subnet_nodes();
    
//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     let dishonesty_voting_period = VotingPeriod::<Test>::get();

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     make_subnet_node_dishonesty_consensus_proposable();

//     assert_ok!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(1),
//         "test_data".into()
//       )
//     );

//     post_successful_dishonesty_proposal_ensures(0, 1, subnet_id.clone());

//     assert_ok!(
//       Network::vote_subnet_node_dishonest(
//         RuntimeOrigin::signed(account(2)),
//         subnet_id.clone(),
//         peer(1)
//       )
//     );
//   });
// }

// #[test]
// fn test_vote_subnet_node_dishonest_consensus() {
//   new_test_ext().execute_with(|| {

//     let peer_removal_threshold = NodeRemovalThreshold::<Test>::get();

//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     build_subnet(subnet_path.clone());

//     let n_peers: u32 = Network::max_subnet_nodes();
//     let n_required_voting_peers: u32 = (n_peers as f64 * (peer_removal_threshold as f64 / 10000.0)).ceil() as u32 + 1;

//     // increase blocks
//     make_model_submittable();

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     assert_eq!(Network::total_stake(), amount_staked);
//     post_successful_add_subnet_nodes_asserts(
//       n_peers.into(),
//       amount,
//       subnet_id.clone(),
//     );

//     let dishonesty_voting_period = VotingPeriod::<Test>::get();

//     System::set_block_number(System::block_number() + dishonesty_voting_period);

//     make_subnet_node_dishonesty_consensus_proposable();

//     assert_ok!(
//       Network::proposal(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id.clone(),
//         peer(n_peers-1),
//         "test_data".into()
//       )
//     );

//     post_successful_dishonesty_proposal_ensures(0, n_peers-1, subnet_id.clone());

//     for n in 1..n_required_voting_peers {
//       assert_ok!(
//         Network::vote_subnet_node_dishonest(
//           RuntimeOrigin::signed(account(n)),
//           subnet_id.clone(),
//           peer(n_peers-1)
//         )
//       );
//     }

//     post_remove_subnet_node_ensures(n_peers-1, subnet_id.clone());  
//   });
// }

#[test]
fn test_add_to_delegate_stake() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let total_model_delegated_stake_shares = TotalSubnetDelegateStakeShares::<Test>::get(subnet_id.clone());
    let total_model_delegated_stake_balance = TotalSubnetDelegateStakeBalance::<Test>::get(subnet_id.clone());

    let mut delegate_stake_to_be_added_as_shares = Network::convert_to_shares(
      amount,
      total_model_delegated_stake_shares,
      total_model_delegated_stake_balance
    );

    if total_model_delegated_stake_shares == 0 {
      delegate_stake_to_be_added_as_shares = delegate_stake_to_be_added_as_shares.saturating_sub(1000);
    }

    assert_ok!(
      Network::add_to_delegate_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        amount,
      ) 
    );

    let delegate_shares = AccountSubnetDelegateStakeShares::<Test>::get(account(0), subnet_id.clone());
    assert_eq!(delegate_shares, delegate_stake_to_be_added_as_shares);
    assert_ne!(delegate_shares, 0);

    let total_model_delegated_stake_shares = TotalSubnetDelegateStakeShares::<Test>::get(subnet_id.clone());
    let total_model_delegated_stake_balance = TotalSubnetDelegateStakeBalance::<Test>::get(subnet_id.clone());

    let mut delegate_balance = Network::convert_to_balance(
      delegate_shares,
      total_model_delegated_stake_shares,
      total_model_delegated_stake_balance
    );
    // The first depositor will lose a percentage of their deposit depending on the size
    // https://docs.openzeppelin.com/contracts/4.x/erc4626#inflation-attack
    assert_eq!(delegate_balance, delegate_stake_to_be_added_as_shares);

  });
}

#[test]
fn test_add_to_delegate_stake_increase_pool_check_balance() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let total_model_delegated_stake_shares = TotalSubnetDelegateStakeShares::<Test>::get(subnet_id.clone());
    let total_model_delegated_stake_balance = TotalSubnetDelegateStakeBalance::<Test>::get(subnet_id.clone());

    let mut delegate_stake_to_be_added_as_shares = Network::convert_to_shares(
      amount,
      total_model_delegated_stake_shares,
      total_model_delegated_stake_balance
    );

    if total_model_delegated_stake_shares == 0 {
      delegate_stake_to_be_added_as_shares = delegate_stake_to_be_added_as_shares.saturating_sub(1000);
    }

    assert_ok!(
      Network::add_to_delegate_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        amount,
      ) 
    );

    let delegate_shares = AccountSubnetDelegateStakeShares::<Test>::get(account(0), subnet_id.clone());
    assert_eq!(delegate_shares, delegate_stake_to_be_added_as_shares);
    assert_ne!(delegate_shares, 0);

    let total_model_delegated_stake_shares = TotalSubnetDelegateStakeShares::<Test>::get(subnet_id.clone());
    let total_model_delegated_stake_balance = TotalSubnetDelegateStakeBalance::<Test>::get(subnet_id.clone());

    let mut delegate_balance = Network::convert_to_balance(
      delegate_shares,
      total_model_delegated_stake_shares,
      total_model_delegated_stake_balance
    );
    // The first depositor will lose a percentage of their deposit depending on the size
    // https://docs.openzeppelin.com/contracts/4.x/erc4626#inflation-attack
    assert_eq!(delegate_balance, delegate_stake_to_be_added_as_shares);

    let increase_delegated_stake_amount: u128 = 1000000000000000000000;
    Network::increase_delegated_stake(
      subnet_id.clone(),
      increase_delegated_stake_amount,
    );

    // ensure balance has increase
    let total_model_delegated_stake_shares = TotalSubnetDelegateStakeShares::<Test>::get(subnet_id.clone());
    let total_model_delegated_stake_balance = TotalSubnetDelegateStakeBalance::<Test>::get(subnet_id.clone());
    let mut post_delegate_balance = Network::convert_to_balance(
      delegate_shares,
      total_model_delegated_stake_shares,
      total_model_delegated_stake_balance
    );
    assert!(delegate_balance < post_delegate_balance);
    assert_ne!(delegate_balance, post_delegate_balance);
  });
}

#[test]
fn test_remove_to_delegate_stake() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let total_model_delegated_stake_shares = TotalSubnetDelegateStakeShares::<Test>::get(subnet_id.clone());
    let total_model_delegated_stake_balance = TotalSubnetDelegateStakeBalance::<Test>::get(subnet_id.clone());

    let mut delegate_stake_to_be_added_as_shares = Network::convert_to_shares(
      amount,
      total_model_delegated_stake_shares,
      total_model_delegated_stake_balance
    );

    if total_model_delegated_stake_shares == 0 {
      delegate_stake_to_be_added_as_shares = delegate_stake_to_be_added_as_shares.saturating_sub(1000);
    }

    assert_ok!(
      Network::add_to_delegate_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        amount,
      ) 
    );

    let delegate_shares = AccountSubnetDelegateStakeShares::<Test>::get(account(0), subnet_id.clone());
    assert_eq!(delegate_shares, delegate_stake_to_be_added_as_shares);
    assert_ne!(delegate_shares, 0);

    let total_model_delegated_stake_shares = TotalSubnetDelegateStakeShares::<Test>::get(subnet_id.clone());
    let total_model_delegated_stake_balance = TotalSubnetDelegateStakeBalance::<Test>::get(subnet_id.clone());

    let mut delegate_balance = Network::convert_to_balance(
      delegate_shares,
      total_model_delegated_stake_shares,
      total_model_delegated_stake_balance
    );
    // The first depositor will lose a percentage of their deposit depending on the size
    // https://docs.openzeppelin.com/contracts/4.x/erc4626#inflation-attack
    assert_eq!(delegate_balance, delegate_stake_to_be_added_as_shares);

    let epoch_length = EpochLength::get();
    let min_required_delegate_unstake_epochs = MinRequiredDelegateUnstakeEpochs::<Test>::get();

    System::set_block_number(System::block_number() + epoch_length * min_required_delegate_unstake_epochs);

    let balance = Balances::free_balance(&account(0));

    assert_ok!(
      Network::remove_delegate_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        delegate_shares,
      )
    );

    let post_balance = Balances::free_balance(&account(0));
    assert_eq!(post_balance, balance + delegate_balance);

  });
}

#[test]
fn test_remove_to_delegate_stake_epochs_not_met_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let _ = Balances::deposit_creating(&account(0), deposit_amount);

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let total_model_delegated_stake_shares = TotalSubnetDelegateStakeShares::<Test>::get(subnet_id.clone());
    let total_model_delegated_stake_balance = TotalSubnetDelegateStakeBalance::<Test>::get(subnet_id.clone());

    let mut delegate_stake_to_be_added_as_shares = Network::convert_to_shares(
      amount,
      total_model_delegated_stake_shares,
      total_model_delegated_stake_balance
    );

    if total_model_delegated_stake_shares == 0 {
      delegate_stake_to_be_added_as_shares = delegate_stake_to_be_added_as_shares.saturating_sub(1000);
    }

    assert_ok!(
      Network::add_to_delegate_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        amount,
      ) 
    );

    let delegate_shares = AccountSubnetDelegateStakeShares::<Test>::get(account(0), subnet_id.clone());
    assert_eq!(delegate_shares, delegate_stake_to_be_added_as_shares);
    assert_ne!(delegate_shares, 0);

    let total_model_delegated_stake_shares = TotalSubnetDelegateStakeShares::<Test>::get(subnet_id.clone());
    let total_model_delegated_stake_balance = TotalSubnetDelegateStakeBalance::<Test>::get(subnet_id.clone());

    let mut delegate_balance = Network::convert_to_balance(
      delegate_shares,
      total_model_delegated_stake_shares,
      total_model_delegated_stake_balance
    );
    // The first depositor will lose a percentage of their deposit depending on the size
    // https://docs.openzeppelin.com/contracts/4.x/erc4626#inflation-attack
    assert_eq!(delegate_balance, delegate_stake_to_be_added_as_shares);

    assert_err!(
      Network::remove_delegate_stake(
        RuntimeOrigin::signed(account(0)),
        subnet_id.clone(),
        delegate_shares,
      ),
      Error::<Test>::RequiredDelegateUnstakeEpochsNotMet
    );
  });
}

// #[test]
// fn test_apr_per_subnet() {
//   new_test_ext().execute_with(|| {
//     let max_subnet_nodes = Network::max_subnet_nodes();
//     let min_stake = MinStakeBalance::<Test>::get();
//     let epoch_length = EpochLength::get();
    
//     let mut previous_apr = f64::MAX;
//     for n in 0..max_subnet_nodes {
//       let apr = Network::get_apr(epoch_length, min_stake*(n as u128 + 1));
//       log::error!("apr          {:?}", apr);
//       log::error!("previous_apr {:?}", apr);

//       assert!(previous_apr > apr, "previous_apr > apr");
//       assert_ne!(apr, 0.0, "apr is 0");
//       previous_apr = apr;
//     }
//   });
// }

// #[test]
// fn test_apr() {
//   new_test_ext().execute_with(|| {
//     let max_models = Network::max_models();
//     let epoch_length = EpochLength::get();
    

//     let total_staked_balance: u128 = 12000000000000000000000;

//     let mut previous_apr = f64::MAX;
//     for n in 0..max_models {
//       TotalSubnets::<Test>::set(n+1);
//       let apr = Network::get_apr(epoch_length, total_staked_balance*(n as u128 + 1));
//       log::error!("apr          {:?}", apr);
//       log::error!("previous_apr {:?}", apr);

//       assert!(previous_apr > apr, "previous_apr > apr");
//       assert_ne!(apr, 0.0, "apr is 0");
//       previous_apr = apr;
//     }
//   });
// }

// #[test]
// fn test_emissions() {
//   new_test_ext().execute_with(|| {
//     let max_models = Network::max_models();
//     let epoch_length = EpochLength::get();
//     let total_staked_balance: u128 = 12000000000000000000000;
//     let mut previous_emissions = u128::MAX;
//     for n in 0..max_models {
//       TotalSubnets::<Test>::set(n+1);
//       let emissions = Network::get_epoch_emissions(epoch_length, total_staked_balance*(n as u128 + 1));
//     }
//   });
// }

// #[test]
// fn test_submit_accountant_data() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();
//     let n_peers: u32 = Network::max_subnet_nodes();
//     build_subnet(subnet_path.clone());

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     make_subnet_node_dishonesty_consensus_proposable();

//     Network::check_and_choose_accountant();

//     let mut current_accountants = CurrentAccountant2::<Test>::get(subnet_id.clone());
//     let accountant = current_accountants.first_key_value().unwrap();
//     let accountant_data_count = AccountantDataCount::<Test>::get(subnet_id.clone());

//     let mut accountant_data_peers: Vec<AccountantDataNodeParams> = Vec::new();

//     assert_err!(
//       Network::submit_accountant_data(
//         RuntimeOrigin::signed(accountant.0.clone()), 
//         subnet_id.clone(),
//         Vec::new(),
//       ),
//       Error::<Test>::InvalidAccountantData
//     );

//     let mut fake_data: Vec<AccountantDataNodeParams> = Vec::new();

//     for n in 0..255 {
//       fake_data.push(
//         AccountantDataNodeParams {
//           peer_id: peer(n),
//           data: BoundedVec::new()
//         }
//       )  
//     }

//     assert_err!(
//       Network::submit_accountant_data(
//         RuntimeOrigin::signed(accountant.0.clone()), 
//         subnet_id.clone(),
//         fake_data,
//       ),
//       Error::<Test>::InvalidAccountantData
//     );

//     for n in 0..n_peers {
//       accountant_data_peers.push(
//         AccountantDataNodeParams {
//           peer_id: peer(n),
//           data: BoundedVec::new()
//         }
//       )  
//     }

//     assert_ok!(
//       Network::submit_accountant_data(
//         RuntimeOrigin::signed(accountant.0.clone()), 
//         subnet_id.clone(),
//         accountant_data_peers.clone(),
//       )  
//     );

//     assert_err!(
//       Network::submit_accountant_data(
//         RuntimeOrigin::signed(accountant.0.clone()), 
//         subnet_id.clone(),
//         accountant_data_peers.clone(),
//       ),
//       Error::<Test>::NotAccountant
//     );

//   });
// }

#[test]
fn test_choose_accountants() {
  new_test_ext().execute_with(|| {
    
    setup_blocks(38);

    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();
    let n_peers: u32 = Network::max_subnet_nodes();
    build_subnet(subnet_path.clone());

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);
    make_model_submittable();

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);

    Network::shift_node_classes(System::block_number(), epoch_length);

    let epoch = System::block_number() / epoch_length;

    let validator = SubnetRewardsValidator::<Test>::get(subnet_id.clone(), epoch as u32);
    assert!(validator == None, "Validator should be None");

    let accountants = CurrentAccountants::<Test>::get(subnet_id.clone(), epoch as u32);
    assert!(accountants == None, "Accountant should be None");

    Network::do_choose_validator_and_accountants(System::block_number(), epoch as u32, epoch_length);

    let validator = SubnetRewardsValidator::<Test>::get(subnet_id.clone(), epoch as u32);
    assert!(validator != None, "Validator is None");

    let accountants = CurrentAccountants::<Test>::get(subnet_id.clone(), epoch as u32);
    assert!(accountants != None, "Accountants is None");
    assert_eq!(accountants.unwrap().len() as u32, TargetAccountantsLength::<Test>::get());


    let subnet_node_data_vec = subnet_node_data(0, n_peers);
    assert_ok!(
      Network::validate(
        RuntimeOrigin::signed(validator.unwrap()), 
        subnet_id.clone(),
        subnet_node_data_vec.clone()
      )
    );

  });
}

fn setup_blocks(blocks: u64) {
  let mut parent_hash = System::parent_hash();

  for i in 1..(blocks + 1) {
    System::reset_events();
    System::initialize(&i, &parent_hash, &Default::default());
    InsecureRandomnessCollectiveFlip::on_initialize(i);

    let header = System::finalize();
    parent_hash = header.hash();
    System::set_block_number(*header.number());
  }
}

#[test]
fn test_randomness() {
  new_test_ext().execute_with(|| {
    setup_blocks(38);
    let gen_rand_num = Network::generate_random_number(1);
    log::error!("test_randomness gen_rand_num {:?}", gen_rand_num);

    let rand_num = Network::get_random_number(96, 0);
    log::error!("test_randomness rand_num {:?}", rand_num);

  });
}

// #[test]
// fn test_propose_dishonesty() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();
//     let n_peers: u32 = Network::max_subnet_nodes();
//     build_subnet(subnet_path.clone());

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, amount + deposit_amount, amount);

//     make_subnet_node_dishonesty_consensus_proposable();

//     Network::check_and_choose_accountant();

//     let mut current_accountants = CurrentAccountant2::<Test>::get(subnet_id.clone());
//     let accountant = current_accountants.first_key_value().unwrap();
//     let accountant_data_count = AccountantDataCount::<Test>::get(subnet_id.clone());

//     let mut accountant_data_peers: Vec<AccountantDataNodeParams> = Vec::new();

//     for n in 0..n_peers {
//       accountant_data_peers.push(
//         AccountantDataNodeParams {
//           peer_id: peer(n),
//           data: BoundedVec::new()
//         }
//       )  
//     }

//     assert_ok!(
//       Network::submit_accountant_data(
//         RuntimeOrigin::signed(accountant.0.clone()), 
//         subnet_id.clone(),
//         accountant_data_peers.clone(),
//       )  
//     );

//     let non_empty_data: Vec<u8> = "__data__".into();

//     // propose_dishonesty
//     assert_ok!(
//       Network::propose_dishonesty(
//         RuntimeOrigin::signed(account(1)), 
//         subnet_id.clone(),
//         peer(0),
//         PropsType::DishonestAccountant,
//         non_empty_data,
//         Some(1),
//       )
//     );
//   });
// }











#[test]
fn test_validate() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    make_model_submittable();

    let n_peers: u32 = Network::max_subnet_nodes();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    // System::set_block_number(System::block_number() + epoch_length);

    amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    make_subnet_node_consensus_data_submittable();

    let epoch_length = EpochLength::get();

    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);

    Network::shift_node_classes(System::block_number(), epoch_length);

    let epoch = System::block_number() / epoch_length;

    log::error!("epoch is -> {:?}", epoch);

    let subnet_node_data_vec = subnet_node_data(0, n_peers);

    // --- Insert validator
    SubnetRewardsValidator::<Test>::insert(subnet_id, epoch as u32, account(0));

    assert_ok!(
      Network::validate(
        RuntimeOrigin::signed(account(0)), 
        subnet_id.clone(),
        subnet_node_data_vec.clone()
      )
    );

    let submission = SubnetRewardsSubmission::<Test>::get(subnet_id.clone(), epoch as u32).unwrap();

    assert_eq!(submission.validator, account(0), "Err: validator");
    assert_eq!(submission.data.len(), subnet_node_data_vec.len(), "Err: data len");
    assert_eq!(submission.sum, DEFAULT_SCORE * n_peers as u128, "Err: sum");
    assert_eq!(submission.attests.len(), 1, "Err: attests");
    assert_eq!(submission.nodes_count, n_peers, "Err: nodes_count");

    assert_err!(
      Network::validate(
        RuntimeOrigin::signed(account(0)), 
        subnet_id.clone(),
        subnet_node_data_vec.clone()
      ),
      Error::<Test>::SubnetRewardsAlreadySubmitted
    );
  });
}

#[test]
fn test_validate_invalid_validator() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    make_model_submittable();

    let n_peers: u32 = Network::max_subnet_nodes();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    // let epoch_length = EpochLength::get();
    // System::set_block_number(System::block_number() + epoch_length);

    amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    make_subnet_node_consensus_data_submittable();

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);

    Network::shift_node_classes(System::block_number(), epoch_length);

    let epoch = System::block_number() / epoch_length;

    let subnet_node_data_vec = subnet_node_data(0, n_peers);

    assert_err!(
      Network::validate(
        RuntimeOrigin::signed(account(0)), 
        subnet_id.clone(),
        subnet_node_data_vec
      ),
      Error::<Test>::InvalidValidator
    );
  });
}

#[test]
fn test_attest() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    make_model_submittable();

    let n_peers: u32 = Network::max_subnet_nodes();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    make_subnet_node_consensus_data_submittable();

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);
    let epoch = System::block_number() / epoch_length;

    let subnet_node_data_vec = subnet_node_data(0, n_peers);

    // --- Insert validator
    SubnetRewardsValidator::<Test>::insert(subnet_id, epoch as u32, account(0));

    assert_ok!(
      Network::validate(
        RuntimeOrigin::signed(account(0)), 
        subnet_id.clone(),
        subnet_node_data_vec.clone()
      )
    );

    // Attest
    for n in 1..n_peers {
      assert_ok!(
        Network::attest(
          RuntimeOrigin::signed(account(n)), 
          subnet_id.clone(),
        )
      );
    }
    
    let submission = SubnetRewardsSubmission::<Test>::get(subnet_id.clone(), epoch as u32).unwrap();

    assert_eq!(submission.validator, account(0));
    assert_eq!(submission.data.len(), subnet_node_data_vec.len());
    assert_eq!(submission.sum, DEFAULT_SCORE * n_peers as u128);
    assert_eq!(submission.attests.len(), n_peers as usize);
    assert_eq!(submission.attests.get(&account(1)), Some(&account(1)));
    assert_eq!(submission.nodes_count, n_peers);
  });
}

#[test]
fn test_attest_no_submission_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    make_model_submittable();

    let n_peers: u32 = Network::max_subnet_nodes();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    make_subnet_node_consensus_data_submittable();

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);
    let epoch = System::block_number() / epoch_length;

    let subnet_node_data_vec = subnet_node_data(0, n_peers);

    // --- Insert validator
    SubnetRewardsValidator::<Test>::insert(subnet_id, epoch as u32, account(0));

    assert_err!(
      Network::attest(
        RuntimeOrigin::signed(account(0)), 
        subnet_id.clone(),
      ),
      Error::<Test>::InvalidSubnetRewardsSubmission
    );
  });
}

#[test]
fn test_attest_already_attested_err() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    make_model_submittable();

    let n_peers: u32 = Network::max_subnet_nodes();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    make_subnet_node_consensus_data_submittable();

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);
    let epoch = System::block_number() / epoch_length;

    let subnet_node_data_vec = subnet_node_data(0, n_peers);

    // --- Insert validator
    SubnetRewardsValidator::<Test>::insert(subnet_id, epoch as u32, account(0));

    assert_ok!(
      Network::validate(
        RuntimeOrigin::signed(account(0)), 
        subnet_id.clone(),
        subnet_node_data_vec.clone()
      )
    );

    // Attest
    for n in 1..n_peers {
      assert_ok!(
        Network::attest(
          RuntimeOrigin::signed(account(n)), 
          subnet_id.clone(),
        )
      );
    }
    
    let submission = SubnetRewardsSubmission::<Test>::get(subnet_id.clone(), epoch as u32).unwrap();

    assert_eq!(submission.validator, account(0));
    assert_eq!(submission.data.len(), subnet_node_data_vec.len());
    assert_eq!(submission.sum, DEFAULT_SCORE * n_peers as u128);
    assert_eq!(submission.attests.len(), n_peers as usize);
    assert_eq!(submission.nodes_count, n_peers);

    for n in 1..n_peers {
      assert_eq!(submission.attests.get(&account(n)), Some(&account(n)));
    }

    // for n in 0..n_peers {
    //   assert_err!(
    //     Network::attest(
    //       RuntimeOrigin::signed(account(n)), 
    //       subnet_id.clone(),
    //     ),
    //     Error::<Test>::AlreadyAttested
    //   );
    // }
  });
}

#[test]
fn test_reward_subnets() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    make_model_submittable();

    let n_peers: u32 = Network::max_subnet_nodes();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    make_subnet_node_consensus_data_submittable();

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);
    let epoch = System::block_number() / epoch_length;

    let subnet_node_data_vec = subnet_node_data(0, n_peers);

    // --- Insert validator
    SubnetRewardsValidator::<Test>::insert(subnet_id, epoch as u32, account(0));

    assert_ok!(
      Network::validate(
        RuntimeOrigin::signed(account(0)), 
        subnet_id.clone(),
        subnet_node_data_vec.clone()
      )
    );

    // Attest
    for n in 1..n_peers {
      assert_ok!(
        Network::attest(
          RuntimeOrigin::signed(account(n)), 
          subnet_id.clone(),
        )
      );
    }
    
    Network::reward_subnets(System::block_number(), epoch as u32, epoch_length);
  });
}

#[test]
fn test_reward_subnets_remove_subnet_node() {
  new_test_ext().execute_with(|| {
    let max_absent = MaxSequentialAbsentSubnetNode::<Test>::get();
    
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    make_model_submittable();

    let n_peers: u32 = Network::max_subnet_nodes();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    make_subnet_node_consensus_data_submittable();

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);

    for num in 0..max_absent+1 {
      System::set_block_number(System::block_number() + epochs * epoch_length + 1);
      Network::shift_node_classes(System::block_number(), epoch_length);
      let epoch = System::block_number() / epoch_length;
  
      let subnet_node_data_vec = subnet_node_data(0, n_peers-1);
    
      // --- Insert validator
      SubnetRewardsValidator::<Test>::insert(subnet_id, epoch as u32, account(0));
  
      assert_ok!(
        Network::validate(
          RuntimeOrigin::signed(account(0)), 
          subnet_id.clone(),
          subnet_node_data_vec.clone()
        )
      );
  
      // Attest
      for n in 1..n_peers-1 {
        assert_ok!(
          Network::attest(
            RuntimeOrigin::signed(account(n)), 
            subnet_id.clone(),
          )
        );
      }
      
      Network::reward_subnets(System::block_number(), epoch as u32, epoch_length);

      let node_absent_count = SequentialAbsentSubnetNode::<Test>::get(subnet_id.clone(), account(n_peers-1));
      assert_eq!(node_absent_count, num+1);

      log::error!("node_absent_count {:?}", node_absent_count);

      if num + 1 > max_absent {
        post_remove_subnet_node_ensures(n_peers-1, subnet_id.clone());
      }

      let submission = SubnetRewardsSubmission::<Test>::get(subnet_id.clone(), epoch as u32).unwrap();

      let base_subnet_reward: u128 = BaseSubnetReward::<Test>::get();
      let delegate_stake_rewards_percentage: u128 = DelegateStakeRewardsPercentage::<Test>::get();
      let subnet_reward: u128 = Network::percent_mul(base_subnet_reward, delegate_stake_rewards_percentage);
  
      let reward_ratio: u128 = Network::percent_div(DEFAULT_SCORE, submission.sum);
      let account_reward: u128 = Network::percent_mul(reward_ratio, subnet_reward);
  
      let base_reward = BaseReward::<Test>::get();
  
      let submission_nodes_count: u128 = submission.nodes_count as u128;
      let submission_attestations: u128 = submission.attests.len() as u128;
      let attestation_percentage: u128 = Network::percent_div(submission_attestations, submission_nodes_count);

      // check each subnet nodes balance increased
      for n in 0..n_peers {
        if n == 0 {
          // validator
          let stake_balance: u128 = AccountSubnetStake::<Test>::get(&account(n), subnet_id.clone());
          let validator_reward: u128 = Network::percent_mul(base_reward, attestation_percentage);
          assert!(stake_balance == amount + (account_reward * (num+1) as u128) + (validator_reward * (num+1) as u128), "Invalid validator staking rewards")  
        } else if n == n_peers - 1 {
          // node removed | should have no rewards
          let stake_balance: u128 = AccountSubnetStake::<Test>::get(&account(n), subnet_id.clone());
          assert!(stake_balance == amount, "Invalid subnet node staking rewards")  
        } else {
          // attestors
          let stake_balance: u128 = AccountSubnetStake::<Test>::get(&account(n), subnet_id.clone());
          assert!(stake_balance == amount + (account_reward * (num+1) as u128), "Invalid subnet node staking rewards")  
        }
      }
    }
  });
}

#[test]
fn test_reward_subnets_absent_node_increment_decrement() {
  new_test_ext().execute_with(|| {
    let max_absent = MaxSequentialAbsentSubnetNode::<Test>::get();
    
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    make_model_submittable();

    let n_peers: u32 = Network::max_subnet_nodes();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    make_subnet_node_consensus_data_submittable();

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);

    for num in 0..10 {
      System::set_block_number(System::block_number() + epochs * epoch_length + 1);
      Network::shift_node_classes(System::block_number(), epoch_length);
      let epoch = System::block_number() / epoch_length;

      if num % 2 == 0 {
        let subnet_node_data_vec = subnet_node_data(0, n_peers-1);
    
        // --- Insert validator
        SubnetRewardsValidator::<Test>::insert(subnet_id, epoch as u32, account(0));
    
        assert_ok!(
          Network::validate(
            RuntimeOrigin::signed(account(0)), 
            subnet_id.clone(),
            subnet_node_data_vec.clone()
          )
        );
    
        // Attest
        for n in 1..n_peers-1 {
          assert_ok!(
            Network::attest(
              RuntimeOrigin::signed(account(n)), 
              subnet_id.clone(),
            )
          );
        }
        
        Network::reward_subnets(System::block_number(), epoch as u32, epoch_length);
  
        let node_absent_count = SequentialAbsentSubnetNode::<Test>::get(subnet_id.clone(), account(n_peers-1));
        assert_eq!(node_absent_count, 1);
      } else {
        let subnet_node_data_vec = subnet_node_data(0, n_peers);
    
        // --- Insert validator
        SubnetRewardsValidator::<Test>::insert(subnet_id, epoch as u32, account(0));
    
        assert_ok!(
          Network::validate(
            RuntimeOrigin::signed(account(0)), 
            subnet_id.clone(),
            subnet_node_data_vec.clone()
          )
        );
    
        // Attest
        for n in 1..n_peers {
          assert_ok!(
            Network::attest(
              RuntimeOrigin::signed(account(n)), 
              subnet_id.clone(),
            )
          );
        }
        
        Network::reward_subnets(System::block_number(), epoch as u32, epoch_length);
  
        let node_absent_count = SequentialAbsentSubnetNode::<Test>::get(subnet_id.clone(), account(n_peers-1));
        assert_eq!(node_absent_count, 0);  
      }
    }
  });
}

#[test]
fn test_reward_subnets_check_balances() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    make_model_submittable();

    let n_peers: u32 = Network::max_subnet_nodes();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = MinStakeBalance::<Test>::get();
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    // let epoch_length = EpochLength::get();
    // System::set_block_number(System::block_number() + epoch_length);

    // let epoch = System::block_number() / epoch_length;

    amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    // make_subnet_node_consensus_data_submittable();

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);
    let epoch = System::block_number() / epoch_length;

    let subnet_node_data_vec = subnet_node_data(0, n_peers);

    // --- Insert validator
    SubnetRewardsValidator::<Test>::insert(subnet_id, epoch as u32, account(0));

    assert_ok!(
      Network::validate(
        RuntimeOrigin::signed(account(0)), 
        subnet_id.clone(),
        subnet_node_data_vec.clone()
      )
    );

    // Attest
    for n in 1..n_peers {
      assert_ok!(
        Network::attest(
          RuntimeOrigin::signed(account(n)), 
          subnet_id.clone(),
        )
      );
    }
    
    let delegate_stake_balance: u128 = TotalSubnetDelegateStakeBalance::<Test>::get(subnet_id.clone());

    Network::reward_subnets(System::block_number(), epoch as u32, epoch_length);

    let post_delegate_stake_balance: u128 = TotalSubnetDelegateStakeBalance::<Test>::get(subnet_id.clone());
    assert!(post_delegate_stake_balance > delegate_stake_balance, "Delegate stake balance should increase");

    let submission = SubnetRewardsSubmission::<Test>::get(subnet_id.clone(), epoch as u32).unwrap();

    let base_subnet_reward: u128 = BaseSubnetReward::<Test>::get();
    let delegate_stake_rewards_percentage: u128 = DelegateStakeRewardsPercentage::<Test>::get();
    let subnet_reward: u128 = Network::percent_mul(base_subnet_reward, delegate_stake_rewards_percentage);

    let reward_ratio: u128 = Network::percent_div(DEFAULT_SCORE, submission.sum);
    let account_reward: u128 = Network::percent_mul(reward_ratio, subnet_reward);

    let base_reward = BaseReward::<Test>::get();

    // check each subnet nodes balance increased
    for n in 0..n_peers {
      if n == 0 {
        // validator
        let stake_balance: u128 = AccountSubnetStake::<Test>::get(&account(n), subnet_id.clone());
        assert!(stake_balance == amount + account_reward + base_reward, "Invalid validator staking rewards")  
      } else {
        // attestors
        let stake_balance: u128 = AccountSubnetStake::<Test>::get(&account(n), subnet_id.clone());
        assert!(stake_balance == amount + account_reward, "Invalid subnet node staking rewards")  
      }
    }
  });
}

#[test]
fn test_reward_subnets_validator_slash() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    make_model_submittable();

    let n_peers: u32 = Network::max_subnet_nodes();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    make_subnet_node_consensus_data_submittable();

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);
    let epoch = System::block_number() / epoch_length;

    let subnet_node_data_vec = subnet_node_data(0, n_peers);

    // --- Insert validator
    SubnetRewardsValidator::<Test>::insert(subnet_id, epoch as u32, account(0));

    assert_ok!(
      Network::validate(
        RuntimeOrigin::signed(account(0)), 
        subnet_id.clone(),
        subnet_node_data_vec.clone()
      )
    );

    // No attests to ensure validator is slashed
    
    let validator_stake_balance: u128 = AccountSubnetStake::<Test>::get(&account(0), subnet_id.clone());

    Network::reward_subnets(System::block_number(), epoch as u32, epoch_length);

    let slashed_validator_stake_balance: u128 = AccountSubnetStake::<Test>::get(&account(0), subnet_id.clone());

    // Ensure validator was slashed
    assert!(validator_stake_balance > slashed_validator_stake_balance, "Validator was not slashed")
  });
}

#[test]
fn test_reward_subnets_subnet_penalty_count() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    make_model_submittable();

    let n_peers: u32 = Network::max_subnet_nodes();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    make_subnet_node_consensus_data_submittable();

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);
    let epoch = System::block_number() / epoch_length;

    let subnet_node_data_vec = subnet_node_data(0, n_peers);

    // --- Insert validator
    SubnetRewardsValidator::<Test>::insert(subnet_id, epoch as u32, account(0));

    assert_ok!(
      Network::validate(
        RuntimeOrigin::signed(account(0)), 
        subnet_id.clone(),
        Vec::new()
      )
    );

    // Attest
    for n in 1..n_peers {
      assert_ok!(
        Network::attest(
          RuntimeOrigin::signed(account(n)), 
          subnet_id.clone(),
        )
      );
    }
    
    Network::reward_subnets(System::block_number(), epoch as u32, epoch_length);

    let subnet_penalty_count = SubnetPenaltyCount::<Test>::get(subnet_id.clone());
    assert_eq!(subnet_penalty_count, 1);

    let account_penalty_count = AccountPenaltyCount::<Test>::get(account(0));
    assert_eq!(account_penalty_count, 0);
  });
}

#[test]
fn test_reward_subnets_account_penalty_count() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    make_model_submittable();

    let n_peers: u32 = Network::max_subnet_nodes();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    make_subnet_node_consensus_data_submittable();

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);
    let epoch = System::block_number() / epoch_length;

    let subnet_node_data_vec = subnet_node_data(0, n_peers);

    // --- Insert validator
    SubnetRewardsValidator::<Test>::insert(subnet_id, epoch as u32, account(0));

    assert_ok!(
      Network::validate(
        RuntimeOrigin::signed(account(0)), 
        subnet_id.clone(),
        Vec::new()
      )
    );

    // No Attest

    Network::reward_subnets(System::block_number(), epoch as u32, epoch_length);

    let subnet_penalty_count = SubnetPenaltyCount::<Test>::get(subnet_id.clone());
    assert_eq!(subnet_penalty_count, 1);

    let account_penalty_count = AccountPenaltyCount::<Test>::get(account(0));
    assert_eq!(account_penalty_count, 1);
  });
}

#[test]
fn test_shift_node_classes() {
  new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    SubnetNodeClassEpochs::<Test>::insert(SubnetNodeClass::Idle, 2);
    SubnetNodeClassEpochs::<Test>::insert(SubnetNodeClass::Included, 4);
    SubnetNodeClassEpochs::<Test>::insert(SubnetNodeClass::Submittable, 6);
    SubnetNodeClassEpochs::<Test>::insert(SubnetNodeClass::Accountant, 8);

    build_subnet(subnet_path.clone());
    assert_eq!(Network::total_models(), 1);

    let n_peers: u32 = Network::max_subnet_nodes();

    let deposit_amount: u128 = 1000000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let mut amount_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    System::set_block_number(System::block_number() + CONSENSUS_STEPS);

    amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    let node_set = SubnetNodesClasses::<Test>::get(subnet_id.clone(), SubnetNodeClass::Idle);
    assert_eq!(node_set.len(), n_peers as usize);

    let epoch_length = EpochLength::get();

    let last_class_id = SubnetNodeClass::iter().last().unwrap();

    let starting_block = System::block_number();

    for class_id in SubnetNodeClass::iter() {
      if class_id == last_class_id {
        continue;
      }
      log::error!("test class_id {:?}", class_id);


      let node_set = SubnetNodesClasses::<Test>::get(subnet_id.clone(), class_id);
      assert_eq!(node_set.len(), n_peers as usize);

      let epochs = SubnetNodeClassEpochs::<Test>::get(class_id.clone());
      System::set_block_number(starting_block + epochs * epoch_length + 1);

      Network::shift_node_classes(System::block_number(), epoch_length);
    }
  })
}

// #[test]
// fn test_add_subnet_node_signature() {
//   new_test_ext().execute_with(|| {
//     let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

//     build_subnet(subnet_path.clone());
//     assert_eq!(Network::total_models(), 1);

//     let n_peers: u32 = Network::max_subnet_nodes();

//     let deposit_amount: u128 = 1000000000000000000000000;
//     let amount: u128 = 1000000000000000000000;
//     let mut amount_staked: u128 = 0;

//     let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

//     System::set_block_number(System::block_number() + CONSENSUS_STEPS);

//     let encoded_peer_id = Encode::encode(&peer(0).0.to_vec());
//     let public = sr25519_generate(0.into(), None);
//     let who_account: AccountIdOf<Test> = MultiSigner::Sr25519(public).into_account().into();
//     let signature =
//       MultiSignature::Sr25519(sr25519_sign(0.into(), &public, &encoded_peer_id).unwrap());

//     assert_ok!(
//       Network::add_subnet_node(
//         RuntimeOrigin::signed(account(0)),
//         subnet_id,
//         peer(0),
//         "172.20.54.234".into(),
//         8888,
//         amount,
//         // signature,
//         // who_account
//       ) 
//     );

//     let node_set = SubnetNodesClasses::<Test>::get(subnet_id.clone(), SubnetNodeClass::Idle);
//     assert_eq!(node_set.len(), n_peers as usize);

//   })
// }

#[test]
fn validate_signature() {
	new_test_ext().execute_with(|| {
		let user_1_pair = sp_core::sr25519::Pair::from_string("//Alice", None).unwrap();
		let user_1_signer = MultiSigner::Sr25519(user_1_pair.public());
    log::error!("user_1_signer {:?}", user_1_signer);
		let user_1 = user_1_signer.clone().into_account();
    log::error!("user_1 {:?}", user_1);
		let peer_id: PeerId = peer(0);
		let encoded_data = Encode::encode(&peer_id);
		let signature = MultiSignature::Sr25519(user_1_pair.sign(&encoded_data));
		assert_ok!(Network::validate_signature(&encoded_data, &signature, &user_1));

		let mut wrapped_data: Vec<u8> = Vec::new();
		wrapped_data.extend(b"<Bytes>");
		wrapped_data.extend(&encoded_data);
		wrapped_data.extend(b"</Bytes>");

		let signature = MultiSignature::Sr25519(user_1_pair.sign(&wrapped_data));
		assert_ok!(Network::validate_signature(&encoded_data, &signature, &user_1));
	})
}

#[test]
fn validate_signature_and_peer() {
	new_test_ext().execute_with(|| {
    // validate signature
		let user_1_pair = sp_core::sr25519::Pair::from_string("//Alice", None).unwrap();
		let user_1_signer = MultiSigner::Sr25519(user_1_pair.public());
		let user_1 = user_1_signer.clone().into_account();
		let peer_id: PeerId = peer(0);
		let encoded_data = Encode::encode(&peer_id);
		let signature = MultiSignature::Sr25519(user_1_pair.sign(&encoded_data));
		assert_ok!(Network::validate_signature(&encoded_data, &signature, &user_1));

		let mut wrapped_data: Vec<u8> = Vec::new();
		wrapped_data.extend(b"<Bytes>");
		wrapped_data.extend(&encoded_data);
		wrapped_data.extend(b"</Bytes>");

		let signature = MultiSignature::Sr25519(user_1_pair.sign(&wrapped_data));
		assert_ok!(Network::validate_signature(&encoded_data, &signature, &user_1));

    // validate signature is the owner of the peer_id
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let mut total_staked: u128 = 0;

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let _ = Balances::deposit_creating(&user_1, deposit_amount);
    
    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(user_1),
        subnet_id,
        peer(0),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );
	})
}

#[test]
fn test_get_subnet_nodes_included() {
	new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let n_peers: u32 = Network::max_subnet_nodes();
    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Included);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);

    let included = Network::get_subnet_nodes_included(subnet_id);

    log::error!("testing included {:?}", included);
  })
}

#[test]
fn test_propose() {
	new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let n_peers: u32 = Network::max_subnet_nodes();
    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);

    assert_ok!(
      Network::propose(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(1),
        Vec::new()
      ) 
    );
  })
}

#[test]
fn test_propose_not_accountant() {
	new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let n_peers: u32 = Network::max_subnet_nodes() - 1;
    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);

    let _ = Balances::deposit_creating(&account(n_peers+1), deposit_amount);
    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(n_peers+1)),
        subnet_id,
        peer(n_peers+1),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );

    assert_err!(
      Network::propose(
        RuntimeOrigin::signed(account(n_peers+1)),
        subnet_id,
        peer(1),
        Vec::new()
      ),
      Error::<Test>::NodeAccountantEpochNotReached
    );
  })
}


#[test]
fn test_propose_min_subnet_nodes_accountants_error() {
	new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let n_peers: u32 = Network::max_subnet_nodes() - 1;
    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);

    let _ = Balances::deposit_creating(&account(n_peers+1), deposit_amount);
    assert_ok!(
      Network::add_subnet_node(
        RuntimeOrigin::signed(account(n_peers+1)),
        subnet_id,
        peer(n_peers+1),
        // "172.20.54.234".into(),
        // 8888,
        amount,
      ) 
    );

    // Shift node classes to accountant epoch for account(n_peers+1)
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);

    // Add new subnet nodes that aren't accountants yet
    for n in 0..n_peers {
      let _ = Balances::deposit_creating(&account(n), deposit_amount);
      assert_ok!(
        Network::add_subnet_node(
          RuntimeOrigin::signed(account(n)),
          subnet_id,
          peer(n),
          // "172.20.54.234".into(),
          // 8888,
          amount,
        ) 
      );
    }
  
    assert_err!(
      Network::propose(
        RuntimeOrigin::signed(account(n_peers+1)),
        subnet_id,
        peer(1),
        Vec::new()
      ),
      Error::<Test>::SubnetNodesMin
    );
  })
}

#[test]
fn test_propose_peer_has_active_proposal() {
	new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let n_peers: u32 = Network::max_subnet_nodes();
    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);

    assert_ok!(
      Network::propose(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(1),
        Vec::new()
      ) 
    );

    assert_err!(
      Network::propose(
        RuntimeOrigin::signed(account(2)),
        subnet_id,
        peer(1),
        Vec::new()
      ),
      Error::<Test>::NodeHasActiveProposal
    );

    assert_err!(
      Network::propose(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(1),
        Vec::new()
      ),
      Error::<Test>::NodeHasActiveProposal
    );
  })
}

#[test]
fn test_challenge_proposal() {
	new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let n_peers: u32 = Network::max_subnet_nodes();
    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);

    assert_ok!(
      Network::propose(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(1),
        Vec::new()
      ) 
    );

    let proposal_index = ProposalsCount::<Test>::get() - 1;

    assert_ok!(
      Network::challenge_proposal(
        RuntimeOrigin::signed(account(1)),
        subnet_id,
        proposal_index,
        Vec::new()
      ) 
    );
  })
}

#[test]
fn test_challenge_proposal_invalid_index() {
	new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let n_peers: u32 = Network::max_subnet_nodes();
    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);

    assert_ok!(
      Network::propose(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(1),
        Vec::new()
      ) 
    );

    assert_err!(
      Network::challenge_proposal(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        15,
        Vec::new()
      ),
      Error::<Test>::ProposalInvalid
    );
  })
}

#[test]
fn test_challenge_proposal_not_defendant() {
	new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let n_peers: u32 = Network::max_subnet_nodes();
    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);

    assert_ok!(
      Network::propose(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(1),
        Vec::new()
      ) 
    );

    let proposal_index = ProposalsCount::<Test>::get() - 1;

    assert_err!(
      Network::challenge_proposal(
        RuntimeOrigin::signed(account(2)),
        subnet_id,
        proposal_index,
        Vec::new()
      ),
      Error::<Test>::NotDefendant
    );
  })
}

#[test]
fn test_challenge_proposal_challenge_period_passed() {
	new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let n_peers: u32 = Network::max_subnet_nodes();
    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);

    assert_ok!(
      Network::propose(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(1),
        Vec::new()
      ) 
    );

    let proposal_index = ProposalsCount::<Test>::get() - 1;

    let challenge_period = ChallengePeriod::<Test>::get();
    System::set_block_number(System::block_number() + challenge_period + 1);

    assert_err!(
      Network::challenge_proposal(
        RuntimeOrigin::signed(account(1)),
        subnet_id,
        proposal_index,
        Vec::new()
      ),
      Error::<Test>::ProposalChallengePeriodPassed
    );
  })
}

#[test]
fn test_challenge_proposal_already_challenged() {
	new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let n_peers: u32 = Network::max_subnet_nodes();
    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);

    assert_ok!(
      Network::propose(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(1),
        Vec::new()
      ) 
    );

    let proposal_index = ProposalsCount::<Test>::get() - 1;

    assert_ok!(
      Network::challenge_proposal(
        RuntimeOrigin::signed(account(1)),
        subnet_id,
        proposal_index,
        Vec::new()
      ) 
    );

    assert_err!(
      Network::challenge_proposal(
        RuntimeOrigin::signed(account(1)),
        subnet_id,
        proposal_index,
        Vec::new()
      ),
      Error::<Test>::ProposalChallenged
    );

  })
}

#[test]
fn test_proposal_voting() {
	new_test_ext().execute_with(|| {
    let subnet_path: Vec<u8> = "petals-team/StableBeluga2".into();

    build_subnet(subnet_path.clone());

    let subnet_id = SubnetPaths::<Test>::get(subnet_path.clone()).unwrap();

    let n_peers: u32 = Network::max_subnet_nodes();
    let deposit_amount: u128 = 10000000000000000000000;
    let amount: u128 = 1000000000000000000000;
    let amount_staked = build_subnet_nodes(subnet_id.clone(), 0, n_peers, deposit_amount, amount);

    let epoch_length = EpochLength::get();
    let epochs = SubnetNodeClassEpochs::<Test>::get(SubnetNodeClass::Accountant);
    System::set_block_number(System::block_number() + epochs * epoch_length + 1);
    Network::shift_node_classes(System::block_number(), epoch_length);

    assert_ok!(
      Network::propose(
        RuntimeOrigin::signed(account(0)),
        subnet_id,
        peer(1),
        Vec::new()
      ) 
    );

    let proposal_index = ProposalsCount::<Test>::get() - 1;

    assert_ok!(
      Network::challenge_proposal(
        RuntimeOrigin::signed(account(1)),
        subnet_id,
        proposal_index,
        Vec::new()
      ) 
    );

    assert_ok!(
      Network::vote(
        RuntimeOrigin::signed(account(2)),
        subnet_id,
        proposal_index,
        VoteType::Yay
      ) 
    );
  })
}
