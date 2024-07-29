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
  pub fn set_peer_vote_premium(
    value: u128,
  ) -> DispatchResult {
    ensure!(
      value < 100 && value != PeerVotePremium::<T>::get(), 
      Error::<T>::InvalidPeerVotePremium
    );

    PeerVotePremium::<T>::set(value);

    Self::deposit_event(Event::SetPeerVotePremium(value));

    Ok(())
  }

  pub fn set_quorum(
    value: u128,
  ) -> DispatchResult {
    ensure!(
      value > 0 && value != Quorum::<T>::get(), 
      Error::<T>::InvalidQuorum
    );

    Quorum::<T>::set(value);

    Self::deposit_event(Event::SetQuorum(value));

    Ok(())
  }

  pub fn set_majority(
    value: u128,
  ) -> DispatchResult {
    ensure!(
      value > 50 && value != Majority::<T>::get(), 
      Error::<T>::InvalidQuorum
    );

    Majority::<T>::set(value);

    Self::deposit_event(Event::SetMajority(value));

    Ok(())
  }

}