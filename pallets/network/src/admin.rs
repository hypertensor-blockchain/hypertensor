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
use frame_support::dispatch::Vec;

impl<T: Config> Pallet<T> {
  pub fn set_vote_model_in(path: Vec<u8>) -> DispatchResult {
    // ensure subnet doesn't exists by path
    ensure!(
      !SubnetPaths::<T>::contains_key(path.clone()),
      Error::<T>::SubnetNotExist
    );
    
		SubnetActivated::<T>::insert(path.clone(), true);

    Self::deposit_event(Event::SetVoteSubnetIn(path));

    Ok(())
  }

  pub fn set_vote_model_out(path: Vec<u8>) -> DispatchResult {
    // ensure subnet exists by path
    ensure!(
      SubnetPaths::<T>::contains_key(path.clone()),
      Error::<T>::SubnetNotExist
    );

		SubnetActivated::<T>::insert(path.clone(), false);

    Self::deposit_event(Event::SetVoteSubnetOut(path));

    Ok(())
  }

  pub fn set_max_models(value: u32) -> DispatchResult {
    ensure!(
      value <= 100,
      Error::<T>::InvalidMaxSubnets
    );

    MaxSubnets::<T>::set(value);

    Self::deposit_event(Event::SetMaxSubnets(value));

    Ok(())
  }

  pub fn set_min_subnet_nodes(value: u32) -> DispatchResult {
    let max_subnet_nodes = MaxSubnetNodes::<T>::get();

    let peer_removal_threshold = NodeRemovalThreshold::<T>::get();
    let min_value = Self::percent_div_round_up(1 as u128, Self::PERCENTAGE_FACTOR - peer_removal_threshold);

    // Ensure over 10
    // Ensure less than MaxSubnetNodes
    // Ensure divisible by NodeRemovalThreshold
    //  • e.g. if the threshold is .8, we need a minimum of
    ensure!(
      value >= 9 && value <= max_subnet_nodes && value >= min_value as u32,
      Error::<T>::InvalidMinSubnetNodes
    );

    MinSubnetNodes::<T>::set(value);

    Self::deposit_event(Event::SetMinSubnetNodes(value));

    Ok(())
  }

  pub fn set_max_subnet_nodes(value: u32) -> DispatchResult {
    // Ensure divisible by .01%
    // Ensuring less than or equal to PERCENTAGE_FACTOR is redundant but keep
    // for possible updates in future versions
    // * Remove `value <= Self::PERCENTAGE_FACTOR` if never used in mainnet
    ensure!(
      value <= 1000 && value as u128 <= Self::PERCENTAGE_FACTOR,
      Error::<T>::InvalidMaxSubnetNodes
    );

    MaxSubnetNodes::<T>::set(value);

    Self::deposit_event(Event::SetMaxSubnetNodes(value));

    Ok(())
  }

  pub fn set_min_stake_balance(value: u128) -> DispatchResult {
    ensure!(
      value > 0,
      Error::<T>::InvalidMinStakeBalance
    );

    MinStakeBalance::<T>::set(value);

    Self::deposit_event(Event::SetMinStakeBalance(value));

    Ok(())
  }

  pub fn set_tx_rate_limit(value: u64) -> DispatchResult {
    TxRateLimit::<T>::set(value);

    Self::deposit_event(Event::SetTxRateLimit(value));

    Ok(())
  }

  pub fn set_max_consensus_epochs_errors(value: u32) -> DispatchResult {
    ensure!(
      value <= 1000,
      Error::<T>::InvalidMaxZeroConsensusEpochs
    );

    MaxSubnetConsensusEpochsErrors::<T>::set(value);

    Self::deposit_event(Event::SetMaxZeroConsensusEpochs(value));

    Ok(())
  }

  // Set the time required for a subnet to be in storage before consensus can be formed
  // This allows time for peers to become subnet peers to the subnet doesn't increment `no-consensus'`
  pub fn set_min_required_model_consensus_submit_epochs(value: u64) -> DispatchResult {
    // Must be greater than 2 epochs to ensure at least 1 epoch passes
    ensure!(
      value > 2,
      Error::<T>::InvalidSubnetConsensusSubmitEpochs
    );

    let min_required_peer_consensus_submit_epochs = MinRequiredNodeConsensusSubmitEpochs::<T>::get();

    // Must be greater than required submit epochs
    // Nodes must have time to become a subnet peer before submitting consensus
    ensure!(
      value > min_required_peer_consensus_submit_epochs,
      Error::<T>::InvalidSubnetConsensusSubmitEpochs
    );

    MinRequiredSubnetConsensusSubmitEpochs::<T>::set(value);

    Self::deposit_event(Event::SetMinRequiredSubnetConsensusSubmitEpochs(value));

    Ok(())
  }

  pub fn set_min_required_peer_consensus_submit_epochs(value: u64) -> DispatchResult {
    // Must be at least 2 epochs
    // This gives room to be greater than MinRequiredNodeConsensusInclusionEpochs
    ensure!(
      value > 1,
      Error::<T>::InvalidNodeConsensusSubmitEpochs
    );

    let min_required_peer_consensus_inclusion_epochs = MinRequiredNodeConsensusInclusionEpochs::<T>::get();

    // Must be greater than required inclusion epochs
    ensure!(
      value > min_required_peer_consensus_inclusion_epochs,
      Error::<T>::InvalidNodeConsensusSubmitEpochs
    );

    MinRequiredNodeConsensusSubmitEpochs::<T>::set(value);

    Self::deposit_event(Event::SetMinRequiredNodeConsensusSubmitEpochs(value));

    Ok(())
  }
  
  pub fn set_min_required_peer_consensus_inclusion_epochs(value: u64) -> DispatchResult {
    // Must be at least 1 epoch
    ensure!(
      value > 0,
      Error::<T>::InvalidNodeConsensusInclusionEpochs
    );

    let min_required_peer_consensus_submit_epochs = MinRequiredNodeConsensusSubmitEpochs::<T>::get();

    // must be less than required submit epochs
    ensure!(
      value < min_required_peer_consensus_submit_epochs,
      Error::<T>::InvalidNodeConsensusInclusionEpochs
    );

    MinRequiredNodeConsensusInclusionEpochs::<T>::set(value);

    Self::deposit_event(Event::SetMinRequiredNodeConsensusEpochs(value));

    Ok(())
  }

  pub fn set_min_required_peer_consensus_dishonesty_epochs(value: u64) -> DispatchResult {
    // Must be at least 1 epoch
    ensure!(
      value > 1,
      Error::<T>::InvalidNodeConsensusDishonestyEpochs
    );

    let min_required_peer_consensus_submit_epochs = MinRequiredNodeConsensusSubmitEpochs::<T>::get();

    // must be less than required submit epochs
    ensure!(
      value < min_required_peer_consensus_submit_epochs,
      Error::<T>::InvalidNodeConsensusDishonestyEpochs
    );

    MinRequiredNodeAccountantEpochs::<T>::set(value);

    Self::deposit_event(Event::SetMinRequiredNodeAccountantEpochs(value));

    Ok(())
  }

  pub fn set_max_outlier_delta_percent(value: u8) -> DispatchResult {
    ensure!(
      value <= 100 as u8,
      Error::<T>::InvalidMaxOutlierDeltaPercent
    );

    MaximumOutlierDeltaPercent::<T>::set(value);

    Self::deposit_event(Event::SetMaximumOutlierDeltaPercent(value));

    Ok(())
  }

  pub fn set_subnet_node_consensus_submit_percent_requirement(value: u128) -> DispatchResult {
    // Update MinSubnetNodes before
    let min_subnet_node_consensus_submit_count = Self::percent_mul_round_up(MinSubnetNodes::<T>::get() as u128, value);

    // Must be less than 100.00% and greater than 51.00%
    // Resulting min subnet peers submitting consensus requirement must be greater
    // than or equal to four
    ensure!(
      value <= Self::PERCENTAGE_FACTOR && 
      value >= 5100 && 
      min_subnet_node_consensus_submit_count >= 4,
      Error::<T>::InvalidSubnetNodeConsensusSubmitPercentRequirement
    );

    SubnetNodeConsensusSubmitPercentRequirement::<T>::set(value);

    Self::deposit_event(Event::SetSubnetNodeConsensusSubmitPercentRequirement(value));

    Ok(())
  }

  pub fn set_consensus_blocks_interval(value: u64) -> DispatchResult {
    // Ensure a minimum of 1000 blocks per consensus epoch
    ensure!(
      value >= 1000,
      Error::<T>::InvalidEpochLengthsInterval
    );

    // EpochLength::<T>::set(value);

    Self::deposit_event(Event::SetEpochLengthsInterval(value));

    Ok(())
  }

  pub fn set_peer_removal_threshold(value: u128) -> DispatchResult {
    let min_subnet_nodes: u32 = MinSubnetNodes::<T>::get();
    // minimum required value is 1 / min_subnet_nodes
    // e.g. a minimum of 12 subnet peers will require a minimum value of
    //      8.3%
    let min_value = Self::percent_div(1 as u128, min_subnet_nodes as u128);

    // Ensure between (51.00, 100.00)
    // Ensure divisible by at least one
    //  • This is redundant but we check anyways
    // The minimum peer removal threshold is 30%
    ensure!(
      value <= Self::PERCENTAGE_FACTOR && value >= 5100 && value >= min_value,
      Error::<T>::InvalidNodeRemovalThreshold
    );

    NodeRemovalThreshold::<T>::set(value);

    Self::deposit_event(Event::SetNodeRemovalThreshold(value));

    Ok(())
  }

  pub fn set_max_model_rewards_weight(value: u128) -> DispatchResult {
    // Ensure between (1, 10000)
    ensure!(
      value <= Self::PERCENTAGE_FACTOR && value > 0,
      Error::<T>::InvalidPercent
    );

    MaxSubnetRewardsWeight::<T>::set(value);

    Self::deposit_event(Event::SetMaxSubnetRewardsWeight(value));

    Ok(())
  }

  pub fn set_stake_reward_weight(value: u128) -> DispatchResult {
    // Ensure <= PERCENTAGE_FACTOR
    ensure!(
      value <= Self::PERCENTAGE_FACTOR,
      Error::<T>::InvalidPercent
    );

    StakeRewardWeight::<T>::set(value);

    Self::deposit_event(Event::SetStakeRewardWeight(value));

    Ok(())
  }

  pub fn set_model_per_peer_init_cost(value: u128) -> DispatchResult {
    // Ensure > 0
    ensure!(
      value > 0 && value < 1000,
      Error::<T>::InvalidSubnetPerNodeInitCost
    );
    
    SubnetPerNodeInitCost::<T>::set(value);

    Self::deposit_event(Event::SetSubnetPerNodeInitCost(value));

    Ok(())
  }

  pub fn set_model_consensus_unconfirmed_threshold(value: u128) -> DispatchResult {
    // Ensure < PERCENTAGE_FACTOR && > 51.00%
    ensure!(
      value < Self::PERCENTAGE_FACTOR && value >= 5100,
      Error::<T>::InvalidSubnetConsensusUnconfirmedThreshold
    );
    
    SubnetConsensusUnconfirmedThreshold::<T>::set(value);

    Self::deposit_event(Event::SetSubnetConsensusUnconfirmedThreshold(value));

    Ok(())
  }

  pub fn set_remove_subnet_node_epoch_percentage(value: u128) -> DispatchResult {
    // Ensure < PERCENTAGE_FACTOR & > 20%
    ensure!(
      value < Self::PERCENTAGE_FACTOR && value > 2000,
      Error::<T>::InvalidRemoveSubnetNodeEpochPercentage
    );
    
    RemoveSubnetNodeEpochPercentage::<T>::set(value);

    Self::deposit_event(Event::SetRemoveSubnetNodeEpochPercentage(value));

    Ok(())
  }
}