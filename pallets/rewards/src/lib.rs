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
// use frame_support::weights::Weight;
// use frame_support::pallet_prelude::DispatchResult;
use frame_system::{
  pallet_prelude::{BlockNumberFor, OriginFor},
  ensure_signed, ensure_root
};
use frame_support::{
  traits::{Currency, FindAuthor, Get},
  weights::Weight,
  pallet_prelude::DispatchResult,
  sp_runtime::SaturatedConversion,
  ensure,
  sp_runtime::Perbill
};
// use frame_support::sp_runtime::SaturatedConversion;
// use frame_support::ensure;
// use frame_support::sp_runtime::Perbill;

// use crate::BLOCKS_PER_HALVING;
// use frame_support::sp_runtime::BLOCKS_PER_HALVING;
// use frame_system::pallet_prelude::BLOCKS_PER_HALVING;
// use node_template_runtime::BLOCKS_PER_HALVING;
// pub trait WeightInfo {
//   fn on_initialize() -> Weight;
// }

#[frame_support::pallet]
pub mod pallet {
  use super::*;
  use frame_support::pallet_prelude::*;
  use frame_support::traits::Currency;
  use frame_support::traits::FindAuthor;
  use pallet_network::IncreaseStakeVault;

  pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

  #[pallet::config]
  pub trait Config: frame_system::Config {
    /// `rewards` events
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    type Currency: Currency<Self::AccountId>;

    type IncreaseStakeVault: IncreaseStakeVault;

    // /// Fixed reward for block producer.
    // #[pallet::constant]
    // type BlockReward: Get<BalanceOf<Self>>;

    #[pallet::constant]
    type HalvingInterval: Get<u32>;

    #[pallet::constant]
    type InitialBlockSubsidy: Get<u128>;

    type FindAuthor: FindAuthor<Self::AccountId>;

    // type WeightInfo: WeightInfo;
  }

  #[pallet::type_value]
	pub fn DefaultValidatorRewardPercent<T: Config>() -> u32 {
		30
	}

  #[pallet::storage] // stores percent rewards to validator, rest go to peers pallet
  #[pallet::getter(fn validator_reward_percent)]
	pub type ValidatorRewardPercent<T> = StorageValue<_, u32, ValueQuery, DefaultValidatorRewardPercent<T>>;

  /// Pallet rewards for issuing rewards to block producers.
  #[pallet::pallet]
	#[pallet::without_storage_info] /// for testing purpses remove in production
  pub struct Pallet<T>(_);

  /// `pallet-rewards` events
  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    /// Issued reward for the block author.
    BlockReward {
        block_author: T::AccountId,
        validator_reward: BalanceOf<T>,
        model_peers_reward: BalanceOf<T>,
    },

    SetValidatorRewardPercent(u32),
  }

  #[pallet::error]
	pub enum Error<T> {
    ValidatorPercentTooHigh,
    ValidatorPercentTooLow,
  }

  #[pallet::hooks]
  impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
    fn on_initialize(now: BlockNumberFor<T>) -> Weight {
      Self::do_initialize(now);
      // T::WeightInfo::on_initialize()
      // T::WeightInfo::zero()
      T::DbWeight::get().reads(1)
    }
  }

  #[pallet::call]
  impl<T: Config> Pallet<T> {
    #[pallet::call_index(1)]
    #[pallet::weight(0)]
    pub fn set_validator_reward_percent(origin: OriginFor<T>, percent: u32) -> DispatchResult {
      ensure_root(origin)?;

      ensure!(
        percent <= 100 as u32,
        Error::<T>::ValidatorPercentTooHigh
      );

      <ValidatorRewardPercent<T>>::set(percent);

      Self::deposit_event(Event::SetValidatorRewardPercent(percent));

      Ok(())
    }
  }
}

impl<T: Config> Pallet<T> {

  fn do_initialize(_n: BlockNumberFor<T>) {
    use pallet_network::IncreaseStakeVault;

    let block_author = T::FindAuthor::find_author(
      frame_system::Pallet::<T>::digest()
        .logs
        .iter()
        .filter_map(|d| d.as_pre_runtime()),
      )
      .expect("Block author must always be present; QED");

    let subsidy: BalanceOf<T> = Self::get_block_subsidy(_n);
    
    let validator_percent = Perbill::from_rational(ValidatorRewardPercent::<T>::get(), 100 as u32);
    let validator_reward = validator_percent * subsidy;

    let model_peers_reward = subsidy - validator_reward;

    T::IncreaseStakeVault::increase_stake_vault(model_peers_reward.saturated_into::<u128>());

    T::Currency::deposit_creating(&block_author, validator_reward);

    Self::deposit_event(Event::BlockReward {
     block_author,
     validator_reward,
     model_peers_reward
    });
  }

  fn get_block_subsidy(block_number: BlockNumberFor<T>) -> BalanceOf<T> {
    let halving_interval: u32 = T::HalvingInterval::get();

    let block_num_as_u64: u64 = TryInto::try_into(block_number)
      .ok()
      .expect("fn get_block_subsidy block_num_as_u64 Err.");

    let halvings: u64 = block_num_as_u64 / halving_interval as u64;

    if halvings >= 64 {
      return (0 as u128).saturated_into::<BalanceOf<T>>();
    }

    let mut initial_block_subsidy: u128 = T::InitialBlockSubsidy::get();
    initial_block_subsidy >>= halvings;

    let block_subsidy: BalanceOf<T> = initial_block_subsidy.saturated_into::<BalanceOf<T>>();

    block_subsidy
  }
}