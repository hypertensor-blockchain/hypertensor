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
use frame_support::{
	assert_noop, assert_ok, assert_err
};
use log::info;
use sp_core::{H256, U256};
use frame_support::traits::Currency;

type AccountIdOf<Test> = <Test as frame_system::Config>::AccountId;

fn account(id: u8) -> AccountIdOf<Test> {
	[id; 32].into()
}

#[test]
fn test_set_vote_model_in() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team-3/StableBeluga2".into();

    let value = pallet_network::ModelVoteIn::<Test>::get(model_path.clone());
    assert_eq!(value, None);
    let value = pallet_network::ModelVoteOut::<Test>::get(model_path.clone());
    assert_eq!(value, None);

    assert_ok!(
      Admin::set_vote_model_in(
        RuntimeOrigin::root(),
        model_path.clone(),
      )
    );

    let value1 = pallet_network::ModelVoteIn::<Test>::get(model_path.clone());
    assert_eq!(value1, Some(true));
    let value = pallet_network::ModelVoteOut::<Test>::get(model_path.clone());
    assert_eq!(value, Some(false));
  })
}

fn test_set_vote_model_out() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "petals-team-3/StableBeluga2".into();

    let value = pallet_network::ModelVoteIn::<Test>::get(model_path.clone());
    assert_eq!(value, None);
    let value = pallet_network::ModelVoteOut::<Test>::get(model_path.clone());
    assert_eq!(value, None);

    assert_ok!(
      Admin::set_vote_model_in(
        RuntimeOrigin::root(),
        model_path.clone(),
      )
    );

    let value = pallet_network::ModelVoteIn::<Test>::get(model_path.clone());
    assert_eq!(value, Some(true));
    let value = pallet_network::ModelVoteOut::<Test>::get(model_path.clone());
    assert_eq!(value, Some(false));

    assert_err!(
      Admin::set_vote_model_out(
        RuntimeOrigin::root(),
        model_path.clone(),
      ),
      pallet_network::Error::<Test>::ModelNotExist
    );

    assert_ok!(
      Network::add_model(
        RuntimeOrigin::signed(account(0)),
        model_path.clone(),
      ) 
    );

    assert_ok!(
      Admin::set_vote_model_out(
        RuntimeOrigin::root(),
        model_path.clone(),
      )
    );

    let value = pallet_network::ModelVoteIn::<Test>::get(model_path.clone());
    assert_eq!(value, Some(false));
    let value = pallet_network::ModelVoteOut::<Test>::get(model_path.clone());
    assert_eq!(value, Some(true));
  })
}

#[test]
fn test_set_max_models() {
  new_test_ext().execute_with(|| {
    assert_ok!(
      Admin::set_max_models(
        RuntimeOrigin::root(),
        11,
      )
    );

    let value = Network::max_models();
    assert_eq!(value, 11);

    assert_ok!(
      Admin::set_max_models(
        RuntimeOrigin::root(),
        12,
      )
    );

    let value = Network::max_models();
    assert_eq!(value, 12);
  })
}

#[test]
fn test_set_min_model_peers() {
  new_test_ext().execute_with(|| {
    assert_err!(
      Admin::set_min_model_peers(
        RuntimeOrigin::root(),
        0,
      ),
      pallet_network::Error::<Test>::InvalidMinModelPeers
    );

    assert_ok!(
      Admin::set_min_model_peers(
        RuntimeOrigin::root(),
        11,
      )
    );

    let value = Network::min_model_peers();
    assert_eq!(value, 11);

    assert_ok!(
      Admin::set_min_model_peers(
        RuntimeOrigin::root(),
        12,
      )
    );

    let value = Network::min_model_peers();
    assert_eq!(value, 12);
  })
}

#[test]
fn test_set_max_model_peers() {
  new_test_ext().execute_with(|| {
    assert_err!(
      Admin::set_max_model_peers(
        RuntimeOrigin::root(),
        201,
      ),
      pallet_network::Error::<Test>::InvalidMaxModelPeers
    );

    assert_ok!(
      Admin::set_max_model_peers(
        RuntimeOrigin::root(),
        11,
      )
    );

    let value = Network::max_model_peers();
    assert_eq!(value, 11);

    assert_ok!(
      Admin::set_max_model_peers(
        RuntimeOrigin::root(),
        12,
      )
    );

    let value = Network::max_model_peers();
    assert_eq!(value, 12);
  })
}

#[test]
fn test_set_min_stake_balance() {
  new_test_ext().execute_with(|| {
    assert_err!(
      Admin::set_min_stake_balance(
        RuntimeOrigin::root(),
        0,
      ),
      pallet_network::Error::<Test>::InvalidMinStakeBalance
    );

    assert_ok!(
      Admin::set_min_stake_balance(
        RuntimeOrigin::root(),
        11,
      )
    );

    let value = pallet_network::MinStakeBalance::<Test>::get();
    assert_eq!(value, 11);

    assert_ok!(
      Admin::set_min_stake_balance(
        RuntimeOrigin::root(),
        12,
      )
    );

    let value = pallet_network::MinStakeBalance::<Test>::get();
    assert_eq!(value, 12);
  })
}

#[test]
fn test_set_tx_rate_limit() {
  new_test_ext().execute_with(|| {
    assert_ok!(
      Admin::set_tx_rate_limit(
        RuntimeOrigin::root(),
        999,
      )
    );

    let value = pallet_network::TxRateLimit::<Test>::get();
    assert_eq!(value, 999);
  })
}

#[test]
fn test_set_max_consensus_epochs_errors() {
  new_test_ext().execute_with(|| {
    assert_err!(
      Admin::set_max_consensus_epochs_errors(
        RuntimeOrigin::root(),
        1001,
      ),
      pallet_network::Error::<Test>::InvalidMaxZeroConsensusEpochs
    );

    assert_ok!(
      Admin::set_max_consensus_epochs_errors(
        RuntimeOrigin::root(),
        999,
      )
    );

    let value = pallet_network::MaxModelConsensusEpochsErrors::<Test>::get();
    assert_eq!(value, 999);
  })
}

#[test]
fn test_set_min_required_model_consensus_submit_epochs() {
  new_test_ext().execute_with(|| {
    assert_ok!(
      Admin::set_min_required_model_consensus_submit_epochs(
        RuntimeOrigin::root(),
        999,
      )
    );

    let value = pallet_network::MinRequiredModelConsensusSubmitEpochs::<Test>::get();
    assert_eq!(value, 999);
  })
}

#[test]
fn test_set_min_required_peer_consensus_submit_epochs() {
  new_test_ext().execute_with(|| {

    let value = pallet_network::MinRequiredPeerConsensusInclusionEpochs::<Test>::get();

    assert_err!(
      Admin::set_min_required_peer_consensus_submit_epochs(
        RuntimeOrigin::root(),
        value - 1,
      ),
      pallet_network::Error::<Test>::InvalidPeerConsensusInclusionEpochs
    );

    assert_ok!(
      Admin::set_min_required_peer_consensus_submit_epochs(
        RuntimeOrigin::root(),
        999,
      )
    );

    let value = pallet_network::MinRequiredPeerConsensusSubmitEpochs::<Test>::get();
    assert_eq!(value, 999);
  })
}

#[test]
fn test_set_min_required_peer_consensus_epochs() {
  new_test_ext().execute_with(|| {
    let submit_epochs = pallet_network::MinRequiredPeerConsensusSubmitEpochs::<Test>::get();
    assert_err!(
      Admin::set_min_required_peer_consensus_inclusion_epochs(
        RuntimeOrigin::root(),
        submit_epochs + 1,
      ),
      pallet_network::Error::<Test>::InvalidPeerConsensusSubmitEpochs
    );

    assert_ok!(
      Admin::set_min_required_peer_consensus_inclusion_epochs(
        RuntimeOrigin::root(),
        submit_epochs - 1,
      )
    );

    let value = pallet_network::MinRequiredPeerConsensusInclusionEpochs::<Test>::get();
    assert_eq!(value, submit_epochs - 1);
  })
}

#[test]
fn test_set_max_outlier_delta_percent() {
  new_test_ext().execute_with(|| {
    assert_err!(
      Admin::set_max_outlier_delta_percent(
        RuntimeOrigin::root(),
        101,
      ),
      pallet_network::Error::<Test>::InvalidMaxOutlierDeltaPercent
    );

    assert_ok!(
      Admin::set_max_outlier_delta_percent(
        RuntimeOrigin::root(),
        99,
      )
    );

    let value = pallet_network::MaximumOutlierDeltaPercent::<Test>::get();
    assert_eq!(value, 99);
  })
}

#[test]
fn test_set_model_peer_consensus_submit_percent_requirement() {
  new_test_ext().execute_with(|| {
    assert_err!(
      Admin::set_model_peer_consensus_submit_percent_requirement(
        RuntimeOrigin::root(),
        101,
      ),
      pallet_network::Error::<Test>::InvalidPercent
    );

    assert_ok!(
      Admin::set_model_peer_consensus_submit_percent_requirement(
        RuntimeOrigin::root(),
        99,
      )
    );

    let value = pallet_network::ModelPeerConsensusSubmitPercentRequirement::<Test>::get();
    assert_eq!(value, 99);
  })
}

#[test]
fn test_set_consensus_blocks_interval() {
  new_test_ext().execute_with(|| {
    assert_err!(
      Admin::set_consensus_blocks_interval(
        RuntimeOrigin::root(),
        1,
      ),
      pallet_network::Error::<Test>::InvalidConsensusBlocksInterval
    );

    assert_ok!(
      Admin::set_consensus_blocks_interval(
        RuntimeOrigin::root(),
        1000,
      )
    );

    let value = pallet_network::ConsensusBlocksInterval::<Test>::get();
    assert_eq!(value, 1000);
  })
}

#[test]
fn test_set_peer_removal_threshold() {
  new_test_ext().execute_with(|| {
    assert_err!(
      Admin::set_peer_removal_threshold(
        RuntimeOrigin::root(),
        101,
      ),
      pallet_network::Error::<Test>::InvalidPercent
    );

    assert_ok!(
      Admin::set_peer_removal_threshold(
        RuntimeOrigin::root(),
        99,
      )
    );

    let value = pallet_network::PeerRemovalThreshold::<Test>::get();
    assert_eq!(value, 99);
  })
}

#[test]
fn test_set_max_model_rewards_weight() {
  new_test_ext().execute_with(|| {
    assert_err!(
      Admin::set_max_model_rewards_weight(
        RuntimeOrigin::root(),
        10001,
      ),
      pallet_network::Error::<Test>::InvalidPercent
    );

    assert_err!(
      Admin::set_max_model_rewards_weight(
        RuntimeOrigin::root(),
        0,
      ),
      pallet_network::Error::<Test>::InvalidPercent
    );

    let max_model_rewards_weight = 9999;

    assert_ok!(
      Admin::set_max_model_rewards_weight(
        RuntimeOrigin::root(),
        max_model_rewards_weight,
      )
    );

    let value = pallet_network::MaxModelRewardsWeight::<Test>::get();
    assert_eq!(value, max_model_rewards_weight);
  })
}

#[test]
fn test_set_stake_reward_weight() {
  new_test_ext().execute_with(|| {
    assert_err!(
      Admin::set_stake_reward_weight(
        RuntimeOrigin::root(),
        10001,
      ),
      pallet_network::Error::<Test>::InvalidPercent
    );

    let stake_reward_weight = 9999;

    assert_ok!(
      Admin::set_stake_reward_weight(
        RuntimeOrigin::root(),
        stake_reward_weight,
      )
    );

    let value = pallet_network::StakeRewardWeight::<Test>::get();
    assert_eq!(value, stake_reward_weight);
  })
}