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
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use rand::RngCore;

impl<T: Config> Pallet<T> {
  /// Submit subnet scores per subnet node
  /// Validator of the epoch receives rewards when attestation passes consensus
  pub fn do_validate(
    subnet_id: u32, 
    account_id: T::AccountId,
    block: u64, 
    epoch_length: u64,
    epoch: u32,
    mut data: Vec<SubnetNodeData>,
  ) -> DispatchResult {
    // TODO: Track how many nodes leave AFTER the validator submits their consensus data
    // This allows us to measure the delta between attestation percentage versus validator data
    // e.g. If there are 1000 validators and 100 leave on the block following, we know the max
    // attestation percentage will only be 90%. We can alter the attestation percentage up to 100% based on
    // the amount of nodes that left during the epoch following the validators entry.
    //
    // Each attestor will be able to track this to get an accurate measurement of the validators
    // consensus data before attesting.
    // e.g. If 1000 validators and 10 leaves, the attestors measurement of the validators consensus data will be
    // accurate up to a maximum of 99.0%, we can calculate the delta between consensus datas accuracy at the current
    // space in time, versus the accuracy including the validators that left afterwards. If 1 left, we can increase the
    // attestation percentage up to 100% based on the amount of nodes that left during the epoch following the validators entry.
    // We can also track not only the count, but who left for the greatest accuracy

    // --- Ensure current subnet validator 
    let validator = SubnetRewardsValidator::<T>::get(subnet_id, epoch).ok_or(Error::<T>::InvalidValidator)?;
    
    ensure!(
      account_id == validator,
      Error::<T>::InvalidValidator
    );

    // --- Ensure not submitted already
    ensure!(
      !SubnetRewardsSubmission::<T>::contains_key(subnet_id, epoch),
      Error::<T>::SubnetRewardsAlreadySubmitted
    );

    // --- Get count of eligible nodes that can be submitted for consensus rewards
    // This is the maximum amount of nodes that can be entered
    let included_nodes_count = SubnetNodesClasses::<T>::get(subnet_id, SubnetNodeClass::Included).len();
    // let accountant_nodes_count = SubnetNodesClasses::<T>::get(subnet_id, SubnetNodeClass::Accountant).len();

    // --- Ensure data isn't greater than current registered subnet peers
    ensure!(
      data.len() as u32 <= included_nodes_count as u32,
      Error::<T>::InvalidRewardsDataLength
    );

    // Remove duplicates based on peer_id
    data.dedup_by(|a, b| a.peer_id == b.peer_id);

    // --- Sum of all entries scores
    // Each score is then used against the sum(scores) for emissions
    // We don't check data accuracy here because that's the job of attesters
    let mut scores_sum = 0;
    for d in data.iter() {
      scores_sum += d.score;
    }

    let submittable_nodes_count = SubnetNodesClasses::<T>::get(subnet_id, SubnetNodeClass::Submittable).len();

    // If data.len() is 0 then the validator is deeming the epoch as invalid

    // --- Validator auto-attests the epoch
    let mut attests: BTreeSet<T::AccountId> = BTreeSet::new();
    attests.insert(account_id.clone());

    let rewards_data: RewardsData<T::AccountId> = RewardsData {
      validator: account_id,
      nodes_count: submittable_nodes_count as u32,
      sum: scores_sum,
      attests: attests,
      data: data
    };

    SubnetRewardsSubmission::<T>::insert(subnet_id, epoch, rewards_data);
  
    Ok(())
  }

    /// Attest validator subnet rewards data
  // Nodes must attest data to receive rewards
  pub fn do_attest(
    subnet_id: u32, 
    account_id: T::AccountId,
    block: u64, 
    epoch_length: u64,
    epoch: u32,
  ) -> DispatchResult {
    let submittable_nodes = SubnetNodesClasses::<T>::get(subnet_id, SubnetNodeClass::Submittable);
    // --- Ensure epoch eligible for attesting - must be submittable
    ensure!(
      submittable_nodes.get(&account_id) != None,
      Error::<T>::NodeConsensusSubmitEpochNotReached
    );

    SubnetRewardsSubmission::<T>::try_mutate_exists(
      subnet_id,
      epoch.clone(),
      |maybe_params| -> DispatchResult {
        let params = maybe_params.as_mut().ok_or(Error::<T>::InvalidSubnetRewardsSubmission)?;
        let mut attests = &mut params.attests;
        attests.insert(account_id.clone());

        params.attests = attests.clone();
        Ok(())
      }
    )?;

    Ok(())
  }

  // /// Attest validator subnet rewards data
  // // Nodes must attest data to receive rewards
  // pub fn do_attest(
  //   subnet_id: u32, 
  //   account_id: T::AccountId,
  //   block: u64, 
  //   epoch_length: u64,
  //   epoch: u32,
  // ) -> DispatchResult {
  //   let submittable_nodes = SubnetNodesClasses::<T>::get(subnet_id, SubnetNodeClass::Submittable);
  //   // --- Ensure epoch eligible for attesting - must be submittable
  //   ensure!(
  //     submittable_nodes.get(&account_id) != None,
  //     Error::<T>::NodeConsensusSubmitEpochNotReached
  //   );

  //   // // --- Ensure epoch submitted
  //   // let mut submission = match SubnetRewardsSubmission::<T>::try_get(subnet_id, epoch) {
  //   //   Ok(submission) => submission,
  //   //   Err(()) =>
  //   //     return Err(Error::<T>::InvalidSubnetRewardsSubmission.into()),
  //   // };

  //   // // --- Ensure not attested already
  //   // ensure!(
  //   //   submission.attests.insert(account_id.clone()),
  //   //   Error::<T>::AlreadyAttested
  //   // );

    
  //   // SubnetRewardsSubmission::<T>::try_mutate_exists(
  //   //   subnet_id,
  //   //   epoch.clone(),
  //   //   |params: &mut RewardsData<T::AccountId>| {
  //   //     params.attests = submission.attests;
  //   //   }
  //   // );

  //   // SubnetRewardsSubmission::<T>::try_mutate_exists(
  //   //   subnet_id,
  //   //   epoch.clone(),
  //   //   |params: &mut RewardsData<T::AccountId>| {
  //   //     params.attests = submission.attests;
  //   //   }
  //   // )

  //   SubnetRewardsSubmission::<T>::try_mutate_exists(
  //     subnet_id,
  //     epoch.clone(),
  //     |maybe_params| -> DispatchResult {
  //       let params = maybe_params.as_mut().ok_or(Error::<T>::InvalidSubnetRewardsSubmission)?;

  //       let mut attests = &mut params.attests;
  //       attests.insert(account_id.clone());

  //       params.attests = attests.clone();
  //       Ok(())
  //     }
  //   )?;

  //   Ok(())
  // }

  pub fn choose_validator(
    block: u64,
    subnet_id: u32,
    min_subnet_nodes: u32,
    epoch: u32,
  ) {
    let node_sets: BTreeMap<T::AccountId, u64> = SubnetNodesClasses::<T>::get(subnet_id, SubnetNodeClass::Submittable);

    // --- Ensure min subnet peers that are submittable are at least the minimum required
    // --- Consensus cannot begin until this minimum is reached
    // --- If not min subnet peers count then accountant isn't needed
    if (node_sets.len() as u32) < min_subnet_nodes {
      return
    }

    let account_ids: Vec<T::AccountId> = node_sets.iter()
      .map(|x| x.0.clone())
      .collect();

    // --- Get eligible validator
    let validator: Option<T::AccountId> = Self::get_random_account(
      block,
      account_ids,
    );
    
    // --- Insert validator for next epoch
    if let Some(validator) = validator {
      SubnetRewardsValidator::<T>::insert(subnet_id, epoch, validator);
    }
  }

  // pub fn choose_validator_v1(
  //   small_rng: &mut SmallRng,
  //   subnet_id: u32,
  //   min_subnet_nodes: u32,
  //   epoch: u32,
  // ) {
  //   let node_sets: BTreeMap<T::AccountId, u64> = SubnetNodesClasses::<T>::get(subnet_id, SubnetNodeClass::Submittable);

  //   // --- Ensure min subnet peers that are submittable are at least the minimum required
  //   // --- Consensus cannot begin until this minimum is reached
  //   // --- If not min subnet peers count then accountant isn't needed
  //   if (node_sets.len() as u32) < min_subnet_nodes {
  //     return
  //   }

  //   let account_ids: Vec<T::AccountId> = node_sets.iter()
  //     .map(|x| x.0.clone())
  //     .collect();

  //   // --- Get eligible validator
  //   let validator: Option<T::AccountId> = Self::get_random_account(
  //     small_rng,
  //     account_ids,
  //   );
    
  //   // --- Insert validator for next epoch
  //   if let Some(validator) = validator {
  //     SubnetRewardsValidator::<T>::insert(subnet_id, epoch, validator);
  //   }
  // }

  // Get random account within subnet
  fn get_random_account(
    block: u64,
    account_ids: Vec<T::AccountId>,
  ) -> Option<T::AccountId> {
    // --- Get accountant
    let accounts_len = account_ids.len();
    if accounts_len == 0 {
      return None;
    }
      
    // --- Get random number within the amount of eligible peers
    // let rand_num = small_rng.next_u32();
    // let rand_index = rand_num % (account_ids.len() as u32);
    log::error!("get_random_account accounts_len {:?}", accounts_len);
    log::error!("get_random_account block {:?}", block);

    let rand_index = Self::get_random_number((accounts_len - 1) as u32, block as u32);
    log::error!("get_random_account rand_index {:?}", rand_index);
    log::info!("get_random_account rand_index {:?}", rand_index);

    // --- Choose random accountant from eligible accounts
    let new_account: &T::AccountId = &account_ids[rand_index as usize];
        
    Some(new_account.clone())
  }
  
  // Get random account within subnet
  fn get_random_account_v1(
    small_rng: &mut SmallRng,
    account_ids: Vec<T::AccountId>,
  ) -> Option<T::AccountId> {
    // --- Get accountant
    let accounts_len = account_ids.len();
    if accounts_len == 0 {
      return None;
    }
      
    // --- Get random number within the amount of eligible peers
    let rand_num = small_rng.next_u32();
    let rand_index = rand_num % (account_ids.len() as u32);
    log::error!("get_random_account_v1 rand_index {:?}", rand_index);
    log::info!("get_random_account_v1 rand_index {:?}", rand_index);

    // --- Choose random accountant from eligible accounts
    let new_account: &T::AccountId = &account_ids[rand_index as usize];
        
    Some(new_account.clone())
  }

  /// Return the validators reward that submitted data on the previous epoch
  // The attestation percentage must be greater than the MinAttestationPercentage
  pub fn get_validator_reward(
    attestation_percentage: u128,
  ) -> u128 {
    if MinAttestationPercentage::<T>::get() > attestation_percentage {
      return 0
    }
    Self::percent_mul(BaseReward::<T>::get(), attestation_percentage)
  }

  pub fn slash_validator(subnet_id: u32, validator: T::AccountId, attestation_percentage: u128) {
    // We never ensure balance is above 0 because any validator chosen must have the target stake
    // balance at a minimum

    // --- Get stake balance
    // This could be greater than the target stake balance
    let account_model_stake: u128 = AccountSubnetStake::<T>::get(validator.clone(), subnet_id);

    // --- Get slash amount up to max slash
    //
    let mut slash_amount: u128 = Self::percent_mul(account_model_stake, SlashPercentage::<T>::get());
    // --- Update slash amount up to attestation percent
    slash_amount = Self::percent_mul(slash_amount, Self::PERCENTAGE_FACTOR - attestation_percentage);
    // --- Update slash amount up to max slash
    let max_slash: u128 = MaxSlashAmount::<T>::get();
    if slash_amount > max_slash {
      slash_amount = max_slash
    }
    
    // --- Decrease account stake
    Self::decrease_account_stake(
      &validator.clone(),
      subnet_id, 
      slash_amount,
    );

    // --- Increase validator penalty count
    AccountPenaltyCount::<T>::mutate(validator, |n: &mut u32| *n += 1);
  }
}