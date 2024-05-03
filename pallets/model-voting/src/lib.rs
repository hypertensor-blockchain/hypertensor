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

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
  use super::*;
  use frame_support::pallet_prelude::*;
  use pallet_network::ModelVote;

  #[pallet::config]
  pub trait Config: frame_system::Config {
    /// `rewards` events
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    type ModelVote: ModelVote;

    // type WeightInfo: WeightInfo;
  }

  	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Minimum required model peers not met
		ModelPeersMin,
  }
  /// Pallet rewards for issuing rewards to block producers.
  #[pallet::pallet]
  pub struct Pallet<T>(_);

  /// `pallet-rewards` events
  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    ModelVoteInInitialized(Vec<u8>, u64),
    ModelVoteOutInitialized(u32, u64),
    ModelVoteInSuccess(Vec<u8>, u64),
    ModelVoteOutSuccess(u32, u64),
  }

	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct ModelPeer<AccountId> {
		pub peer_id: PeerId,
		pub ip: Vec<u8>,
		pub port: u16,
	}

  enum VoteOutReason {
    // If model peers are performing manipulation for rewards
    ModelEmissionsManipulation,
    // If the model is down
    ModelDown,
    // If the model isn't open-sourced
    ModelCloseSourced,
    // If model is broken
    ModelBroken,
    // If the model doesn't have minimum required peers
    ModelMinimumPeers,
    // If the model is outputting illicit and illegal data
    ModelIllisit,
    // Other
    Other,
  }


  #[pallet::call]
  impl<T: Config> Pallet<T> {
    #[pallet::call_index(0)]
    #[pallet::weight(0)]
    pub fn propose_model_vote_in(origin: OriginFor<T>, path: Vec<u8>, model_peers: Vec<ModelPeer>) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

      // Check path doesn't already exist in Network or ModelVoting

      // ...

      // Ensure account has enough balance to pay cost of model initialization

      // ...

      // Ensure account is already an existing peer and account eligible

      // Ensure minimum peers required are already met before going forward
			ensure!(
				model_peers.len() >= 10,
				Error::<T>::ModelPeersMin
			);

      // ...

      // Vote power based on a time and balance weighted mechanism

      // ...

      // Queue each model peer to be called in an offchain-worker using their PeerId, IP, and Port to ensure model is running
      // Must be called multiple times with a 100% success rate over the course of the minimum required voting time span

      // ...
    }

    #[pallet::call_index(1)]
    #[pallet::weight(0)]
    pub fn propose_model_vote_out(
      origin: OriginFor<T>, 
      model_id: u32,
      reason: VoteOutReason,
      explanation: Vec<u8>,
    ) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;
    }

    #[pallet::call_index(2)]
    #[pallet::weight(0)]
    pub fn vote(
      origin: OriginFor<T>, 
      proposal_index: u32,
      vote: BalanceOf<T>,
    ) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;
    }

    #[pallet::call_index(3)]
    #[pallet::weight(0)]
    pub fn activate_model(
      origin: OriginFor<T>, 
      path: Vec<u8>,
      reason: VoteOutReason,
      explanation: Vec<u8>,
    ) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

      // Ensure model vote in passed
    }

    #[pallet::call_index(4)]
    #[pallet::weight(0)]
    pub fn deactivate_model(
      origin: OriginFor<T>, 
      model_id: u32,
    ) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

      // Ensure model vote out passed
    }
  }
}