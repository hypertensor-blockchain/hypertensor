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
use frame_support::traits::Randomness;
// use rand::{Rng, SeedableRng};
// use rand::rngs::SmallRng;
// use rand::RngCore;
// use rand::{thread_rng, Rng};
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use rand::RngCore;

impl<T: Config> Pallet<T> {
  pub fn try_submit_accountant_data(
    accountant: T::AccountId,
    model_id: u32,
    data: Vec<AccountantDataPeerParams>,
  ) -> DispatchResult {
    // --- Ensure is epochs accountant
    // let current_accountant: T::AccountId = CurrentAccountant::<T>::get(model_id.clone());

    // New accountants are chosen at the beginning of each epoch, if the previous accountant doesn't submit 
    // data by the end of the epoch, then they will get errors when the new accountants are chosen. New accountants
    // cannot be the last accountants

    // --- Ensure is epochs accountant
    let mut current_accountants: BTreeMap<T::AccountId, bool> = CurrentAccountant2::<T>::get(model_id.clone());
    ensure!(
      current_accountants.contains_key(&accountant.clone()),
      Error::<T>::NotAccountant
    );

    // Check if removed all stake yet
    let has_submitted: bool = match current_accountants.get(&accountant.clone()) {
      Some(submitted) => *submitted,
      None => false,
    };
    ensure!(
      !has_submitted,
      Error::<T>::NotAccountant
    );

    // // --- Ensure didn't already submit
    // ensure!(
    //   accountant == current_accountant,
    //   Error::<T>::NotAccountant
    // );

    let data_len = data.len();
    let total_model_peers: u32 = TotalModelPeers::<T>::get(model_id.clone());

    // --- Ensure length of data does not exceed total model peers of model ID
    ensure!(
      data_len as u32 <= total_model_peers && data_len as u32 > 0,
      Error::<T>::InvalidAccountantData
    );

    // --- Update to data submitted
    current_accountants.insert(accountant.clone(), true);
    CurrentAccountant2::<T>::insert(model_id.clone(), current_accountants);
    
    let accountant_data_index: u32 = AccountantDataCount::<T>::get(model_id.clone());

    let block: u64 = Self::get_current_block_as_u64();
    let epoch: u64 = block / EpochLength::<T>::get();

    AccountantData::<T>::insert(
      model_id.clone(),
      accountant_data_index.clone(),
      AccountantDataParams {
        accountant,
        block,
        epoch,
        data,
      }
    );

    Ok(())
  }

  pub fn check_and_choose_accountant() {
    let block: u64 = Self::get_current_block_as_u64();
    let epoch_length: u64 = EpochLength::<T>::get();
    let min_required_peer_accountant_epochs: u64 = MinRequiredPeerAccountantEpochs::<T>::get();
    let min_model_peers: u32 = MinModelPeers::<T>::get();

    // Predictable rand generator for choosing random accountant 
    let mut small_rng = SmallRng::seed_from_u64(block);

    for (model_id, data) in ModelsData::<T>::iter() {
			let model_activated: bool = match ModelActivated::<T>::try_get(data.path) {
				Ok(is_active) => is_active,
				Err(()) => false,
			};
      if !model_activated {
        Self::clear_accountants(model_id);
        continue;
      }

      // We don't check if model has errors because it is up to the users to remove that model
      // If a model surpasses max errors, not rewards are emitted. Users of this subnet must remove
      // the model from the network.

      // --- Check model peers count
      let model_peers_count = TotalModelPeers::<T>::get(model_id);
      // --- If not min model peers count then accountant isn't needed
      if model_peers_count < min_model_peers {
        Self::clear_accountants(model_id);
        continue;
      }

      // --- Check accountant submitted data
      // let current_accountant = CurrentAccountant::<T>::get(model_id.clone());
      let mut current_accountants: BTreeMap<T::AccountId, bool> = CurrentAccountant2::<T>::get(model_id);

      // --- Give the accountant node a penalty if they didn't submit accountant data
      if !current_accountants.is_empty() {
        for accountant in &current_accountants {
          let has_submitted: bool = match current_accountants.get(&accountant.0) {
            Some(submitted) => *submitted,
            None => false,
          };
    
          if !has_submitted {
            AccountPenaltyCount::<T>::mutate(
              accountant.0, 
              |n: &mut u32| *n += 1
            );
          }  
        }  
      }

      // let accountant_data_index: u32 = AccountantDataCount::<T>::get(model_id);
      // // --- Check accountant data count
      // // If current accountant didn't submit data, increase penalty count
      // // If they did submit data, others can propose the accountant is dishonest
      // // let accountant_data = AccountantData::<T>::get(model_id.clone(), accountant_data_index);
			// match AccountantData::<T>::try_get(model_id.clone(), accountant_data_index) {
			// 	Ok(_) |
			// 	Err(()) => {
      //     AccountPenaltyCount::<T>::mutate(
      //       current_accountant, 
      //       |n: &mut u32| *n += 1
      //     );
      //   },
			// };

      // --- Get random accountant
      let account_ids: Vec<T::AccountId> = Self::get_eligible_model_peers_accounts(
        model_id,
        block,
        epoch_length,
        min_required_peer_accountant_epochs
      );

      // --- If there are no eligible accountants, skip to the next model after clearing
      let accountant: Option<T::AccountId> = Self::get_random_accountant(
        &mut small_rng,
        model_id,
        block,
        epoch_length,
        min_required_peer_accountant_epochs,
        current_accountants.clone(),
      );

      // --- Clear the current accountants for the next epochs accountants
      // This version only uses one accountant for each epoch
      if !current_accountants.is_empty() {
        // --- Clear previous epochs accountants
        current_accountants.clear();
      }

      // --- Insert new accountants only if they exist
      if let Some(accountant) = accountant {
        current_accountants.insert(accountant, false);
        CurrentAccountant2::<T>::insert(model_id, current_accountants);
        
        // --- Increase accountant data count
        AccountantDataCount::<T>::insert(model_id, AccountantDataCount::<T>::get(model_id) + 1);
      }
    }
  }

  fn clear_accountants(model_id: u32) {
    let mut current_accountants: BTreeMap<T::AccountId, bool> = CurrentAccountant2::<T>::get(model_id);
    // --- Remove the current account if exists
    if !current_accountants.is_empty() {
      current_accountants.clear();
      CurrentAccountant2::<T>::insert(model_id, current_accountants);
    }
  }

  // Get random account
  fn get_random_accountant(
    small_rng: &mut SmallRng,
    model_id: u32,
    block: u64,
    epoch_length: u64,
    min_required_peer_accountant_epochs: u64,
    previous_accountants: BTreeMap<T::AccountId, bool>
  ) -> Option<T::AccountId> {
    // --- Get accountant
    let account_ids: Vec<T::AccountId> = Self::get_eligible_model_peers_accounts(
      model_id,
      block,
      epoch_length,
      min_required_peer_accountant_epochs
    );


    let mut is_prev_accountant = true;


    // let new_accountant: &T::AccountId = while is_prev_accountant {
    //   // --- Get random number within the amount of eligible peers
    //   let rand_num = small_rng.next_u32();
    //   let rand_index = rand_num % (account_ids.len() as u32 + 1);

    //   // --- Choose random accountant from eligible accounts
    //   let new_accountant: &T::AccountId = &account_ids[rand_index as usize];
      
    //   if !previous_accountants.contains_key(&new_accountant) {
    //     is_prev_accountant = false;
    //     return new_accountant.clone()
    //   }
    // };
    let accountants_len = account_ids.len();
    if accountants_len == 0 {
      return None;
    }
    
    let mut new_accountant: &T::AccountId = &account_ids[0];

    while is_prev_accountant {
      // --- Get random number within the amount of eligible peers
      let rand_num = small_rng.next_u32();
      let rand_index = rand_num % (account_ids.len() as u32 + 1);

      // --- Choose random accountant from eligible accounts
      let new_accountant: &T::AccountId = &account_ids[rand_index as usize];
      
      if !previous_accountants.contains_key(&new_accountant) {
        is_prev_accountant = false;
      }
    };
    
    Some(new_accountant.clone())
  }

  fn get_round_robin_accountant(
    model_id: u32,
    block: u64,
    epoch_length: u64,
    min_required_peer_accountant_epochs: u64,
  ) -> Option<T::AccountId> {
    // --- Get accountants in model_id list for round robin
    // let mut accountants: Vec<T::AccountId> = Accountants::<T>::get(model_id.clone());
    let mut accountants: Vec<T::AccountId> = Vec::new();

    // --- Get accountant
    let account_ids: Vec<T::AccountId> = Self::get_eligible_model_peers_accounts(
      model_id.clone(),
      block,
      epoch_length,
      min_required_peer_accountant_epochs
    );

    let accountants_len = account_ids.len();
    if accountants_len == 0 {
      return None;
    }

    for account_id in account_ids.into_iter() {
      if !accountants.contains(&account_id) {
        // accountants.insert(account_id);
        accountants.push(account_id);
      }
    }

    // Accountants::<T>::insert(model_id.clone(), accountants);

    let accountants_len = accountants.len();

    // We don't pop accountants because they are removed from the accountants storage element when they are removed
    let mut previous_index = 0;

    // If the previous accountants index less than the length of the accountants, increase by one
    // Otherwise, start at zero
    if previous_index < accountants_len {
      previous_index += 1;      
    }

    let accountant = accountants.get(previous_index);
    Some(accountant.unwrap().clone())
  }

  pub fn clean_accountant_data() {

  }
}