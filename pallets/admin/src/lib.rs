// Copyright (C) 2021 Subspace Labs, Inc.
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

//! Pallet for issuing rewards to block producers.

#![cfg_attr(not(feature = "std"), no_std)]

// pub mod weights;

pub use pallet::*;
use frame_system::{
  pallet_prelude::OriginFor,
  ensure_signed, ensure_root
};
use frame_support::{
  weights::Weight,
  pallet_prelude::DispatchResult,
  ensure,
  dispatch::Vec
};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
  use super::*;
  use frame_support::pallet_prelude::*;
  use pallet_network::AdminInterface as NetworkAdminInterface;
  use pallet_model_voting::AdminInterface as SubnetVotingAdminInterface;

  #[pallet::config]
  pub trait Config: frame_system::Config {
    /// `rewards` events
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    type NetworkAdminInterface: NetworkAdminInterface;

    type SubnetVotingAdminInterface: SubnetVotingAdminInterface;

    // type WeightInfo: WeightInfo;
  }

  /// Pallet rewards for issuing rewards to block producers.
  #[pallet::pallet]
  pub struct Pallet<T>(_);

  /// `pallet-rewards` events
  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    SetVoteSubnetIn(Vec<u8>),
    SetVoteSubnetOut(Vec<u8>),
    SetMaxSubnets(u32),
    SetMinSubnetNodes(u32),
    SetMaxSubnetNodes(u32),
    SetMinStakeBalance(u128),
    SetTxRateLimit(u64),
    SetMaxZeroConsensusEpochs(u32),
    SetMinRequiredSubnetConsensusSubmitEpochs(u64),
    SetMinRequiredNodeConsensusSubmitEpochs(u64),
    SetMinRequiredNodeConsensusEpochs(u64),
    SetMaximumOutlierDeltaPercent(u8),
    SetSubnetNodeConsensusSubmitPercentRequirement(u128),
    SetConsensusBlocksInterval(u64),
    SetNodeRemovalThreshold(u8),
    SetMaxSubnetRewardsWeight(u16),
  }

  //
  // All conditional logic takes place in the callee pallets themselves
  //
  #[pallet::call]
  impl<T: Config> Pallet<T> {
    #[pallet::call_index(0)]
    #[pallet::weight(0)]
    pub fn set_vote_model_in(origin: OriginFor<T>, value: Vec<u8>) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_vote_model_in(value)
    }

    #[pallet::call_index(1)]
    #[pallet::weight(0)]
    pub fn set_vote_model_out(origin: OriginFor<T>, value: Vec<u8>) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_vote_model_out(value)
    }

    #[pallet::call_index(2)]
    #[pallet::weight(0)]
    pub fn set_max_models(origin: OriginFor<T>, value: u32) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_max_models(value)
    }

    #[pallet::call_index(3)]
    #[pallet::weight(0)]
    pub fn set_min_subnet_nodes(origin: OriginFor<T>, value: u32) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_min_subnet_nodes(value)
    }

    #[pallet::call_index(4)]
    #[pallet::weight(0)]
    pub fn set_max_subnet_nodes(origin: OriginFor<T>, value: u32) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_max_subnet_nodes(value)
    }

    #[pallet::call_index(5)]
    #[pallet::weight(0)]
    pub fn set_min_stake_balance(origin: OriginFor<T>, value: u128) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_min_stake_balance(value)
    }

    #[pallet::call_index(6)]
    #[pallet::weight(0)]
    pub fn set_tx_rate_limit(origin: OriginFor<T>, value: u64) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_tx_rate_limit(value)
    }

    #[pallet::call_index(7)]
    #[pallet::weight(0)]
    pub fn set_max_consensus_epochs_errors(origin: OriginFor<T>, value: u32) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_max_consensus_epochs_errors(value)
    }

    #[pallet::call_index(8)]
    #[pallet::weight(0)]
    pub fn set_min_required_model_consensus_submit_epochs(origin: OriginFor<T>, value: u64) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_min_required_model_consensus_submit_epochs(value)
    }

    #[pallet::call_index(9)]
    #[pallet::weight(0)]
    pub fn set_min_required_peer_consensus_submit_epochs(origin: OriginFor<T>, value: u64) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_min_required_peer_consensus_submit_epochs(value)
    }

    #[pallet::call_index(10)]
    #[pallet::weight(0)]
    pub fn set_min_required_peer_consensus_inclusion_epochs(origin: OriginFor<T>, value: u64) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_min_required_peer_consensus_inclusion_epochs(value)
    }

    #[pallet::call_index(11)]
    #[pallet::weight(0)]
    pub fn set_min_required_peer_consensus_dishonesty_epochs(origin: OriginFor<T>, value: u64) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_min_required_peer_consensus_dishonesty_epochs(value)
    }

    #[pallet::call_index(12)]
    #[pallet::weight(0)]
    pub fn set_max_outlier_delta_percent(origin: OriginFor<T>, value: u8) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_max_outlier_delta_percent(value)
    }

    #[pallet::call_index(13)]
    #[pallet::weight(0)]
    pub fn set_subnet_node_consensus_submit_percent_requirement(origin: OriginFor<T>, value: u128) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_subnet_node_consensus_submit_percent_requirement(value)
    }

    #[pallet::call_index(14)]
    #[pallet::weight(0)]
    pub fn set_consensus_blocks_interval(origin: OriginFor<T>, value: u64) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_consensus_blocks_interval(value)
    }

    #[pallet::call_index(15)]
    #[pallet::weight(0)]
    pub fn set_peer_removal_threshold(origin: OriginFor<T>, value: u128) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_peer_removal_threshold(value)
    }

    #[pallet::call_index(16)]
    #[pallet::weight(0)]
    pub fn set_max_model_rewards_weight(origin: OriginFor<T>, value: u128) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_max_model_rewards_weight(value)
    }
    
    #[pallet::call_index(17)]
    #[pallet::weight(0)]
    pub fn set_stake_reward_weight(origin: OriginFor<T>, value: u128) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_stake_reward_weight(value)
    }

    #[pallet::call_index(18)]
    #[pallet::weight(0)]
    pub fn set_model_per_peer_init_cost(origin: OriginFor<T>, value: u128) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_model_per_peer_init_cost(value)
    }

    #[pallet::call_index(19)]
    #[pallet::weight(0)]
    pub fn set_model_consensus_unconfirmed_threshold(origin: OriginFor<T>, value: u128) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_model_consensus_unconfirmed_threshold(value)
    }

    #[pallet::call_index(20)]
    #[pallet::weight(0)]
    pub fn set_remove_subnet_node_epoch_percentage(origin: OriginFor<T>, value: u128) -> DispatchResult {
      ensure_root(origin)?;
      T::NetworkAdminInterface::set_remove_subnet_node_epoch_percentage(value)
    }

    #[pallet::call_index(21)]
    #[pallet::weight(0)]
    pub fn set_peer_vote_premium(origin: OriginFor<T>, value: u128) -> DispatchResult {
      ensure_root(origin)?;
      T::SubnetVotingAdminInterface::set_peer_vote_premium(value)
    }

    #[pallet::call_index(22)]
    #[pallet::weight(0)]
    pub fn set_quorum(origin: OriginFor<T>, value: u128) -> DispatchResult {
      ensure_root(origin)?;
      T::SubnetVotingAdminInterface::set_quorum(value)
    }
  }
}