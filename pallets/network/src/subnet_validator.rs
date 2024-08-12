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
    epoch: u32,
    mut data: Vec<SubnetNodeData>,
  ) -> DispatchResult {
    // --- Ensure correct epoch 

    // --- Ensure current subnet validator 
    // let validator = SubnetRewardsValidator::<T>::get(subnet_id, epoch);

    // let validator = match SubnetRewardsValidator::<T>::try_get(subnet_id, epoch) {
    //   Ok(validator) => validator,
    //   Err(()) =>
    //     return Err("Unknown SubnetRewardsValidator.".into()),
    // };

    let validator = SubnetRewardsValidator::<T>::get(subnet_id, epoch).ok_or(Error::<T>::InvalidValidator)?;
    
    log::error!("SubnetRewardsValidator account_id {:?}", account_id);
    log::error!("SubnetRewardsValidator validator  {:?}", validator);

    // ensure!(
    //   !validator.is_err(),
    //   Error::<T>::InvalidValidator
    // );

    ensure!(
      account_id == validator,
      Error::<T>::InvalidValidator
    );

    // --- Ensure not submitted already
    ensure!(
      !SubnetRewardsSubmission::<T>::contains_key(subnet_id, epoch),
      Error::<T>::SubnetRewardsAlreadySubmitted
    );

    let total_subnet_nodes: u32 = TotalSubnetNodes::<T>::get(subnet_id.clone());
    // --- Ensure data isn't greater than current registered subnet peers
    ensure!(
      data.len() as u32 <= total_subnet_nodes,
      Error::<T>::InvalidRewardsDataLength
    );

    data.dedup_by(|a, b| a.peer_id == b.peer_id);

    // We don't check data accuracy here because that's the job of attesters
    let mut rewards_sum = 0;
    for d in data.iter() {
      rewards_sum += d.score;
    }

    let min_required_epochs: u64 = MinRequiredNodeConsensusInclusionEpochs::<T>::get();
    let block: u64 = Self::get_current_block_as_u64();
    let epoch_length: u64 = EpochLength::<T>::get();

    // --- Get count of eligible subnet nodes
    let eligible_accounts_count: u32 = Self::get_total_eligible_subnet_nodes_count(
      subnet_id,
      block,
      epoch_length,
      min_required_epochs
    );

    // If data.len() is 0 then the validator is deeming the epoch as invalid

    let rewards_data: RewardsData<T::AccountId> = RewardsData {
      validator: account_id,
      nodes_count: eligible_accounts_count,
      sum: rewards_sum,
      attests: BTreeSet::new(),
      complete: false,
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
    let subnet_node_data = SubnetNodesData::<T>::get(subnet_id.clone(), account_id.clone());
    let min_required_consensus_inclusion_epochs = MinRequiredNodeConsensusInclusionEpochs::<T>::get();

    // --- Ensure epoch eligible for attesting / must be submitable
    ensure!(
      Self::is_epoch_block_eligible(
        block, 
        epoch_length, 
        min_required_consensus_inclusion_epochs, 
        subnet_node_data.initialized
      ),
      Error::<T>::NodeConsensusSubmitEpochNotReached
    );

    // --- Ensure epoch submitted
    let mut submission = match SubnetRewardsSubmission::<T>::try_get(subnet_id.clone(), epoch) {
      Ok(submission) => submission,
      Err(()) =>
        return Err("Unknown SubnetRewardsSubmission.".into()),
    };

    // --- Ensure not attested already
    ensure!(
      submission.attests.insert(account_id.clone()),
      Error::<T>::AlreadyAttested
    );

    SubnetRewardsSubmission::<T>::mutate(
      subnet_id.clone(),
      epoch.clone(),
      |params: &mut RewardsData<T::AccountId>| {
        params.attests = submission.attests;
      }
    );

    Ok(())
  }

  pub fn choose_validators() {

  }

  fn choose_validator(
    small_rng: &mut SmallRng,
    subnet_id: u32,
    block: u64,
    epoch: u32,
    epoch_length: u64,
  ) {
    let min_required_peer_accountant_epochs: u64 = MinRequiredNodeAccountantEpochs::<T>::get();
    let min_subnet_nodes: u32 = MinSubnetNodes::<T>::get();

    let account_ids: Vec<T::AccountId> = Self::get_eligible_subnet_nodes_accounts(
      subnet_id,
      block,
      epoch_length,
      min_required_peer_accountant_epochs
    );

    // --- Ensure min subnet peers that are submittable are at least the minimum required
    // --- If not min subnet peers count then accountant isn't needed
    if (account_ids.len() as u32) < min_subnet_nodes {
      return
    }

    // --- Get eligible validator
    let validator: Option<T::AccountId> = Self::get_random_account(
      small_rng,
      subnet_id,
      account_ids,
      block,
      epoch_length,
      min_required_peer_accountant_epochs,
    );
    
    // --- Insert validator for next epoch
    if let Some(validator) = validator {
      SubnetRewardsValidator::<T>::insert(subnet_id, epoch, validator);
    }
  }

  // Get random account within subnet
  fn get_random_account(
    small_rng: &mut SmallRng,
    subnet_id: u32,
    account_ids: Vec<T::AccountId>,
    block: u64,
    epoch_length: u64,
    min_required_peer_accountant_epochs: u64,
  ) -> Option<T::AccountId> {
    // --- Get accountant
    let accountants_len = account_ids.len();
    if accountants_len == 0 {
      return None;
    }
      
    // --- Get random number within the amount of eligible peers
    let rand_num = small_rng.next_u32();
    let rand_index = rand_num % (account_ids.len() as u32 + 1);

    // --- Choose random accountant from eligible accounts
    let new_accountant: &T::AccountId = &account_ids[rand_index as usize];
        
    Some(new_accountant.clone())
  }
  
  /// Reward the validator of the epoch based on attestations
  fn reward_validator(
    subnet_id: u32, 
    epoch: u32,
  ) {
    if let Ok(submission) = SubnetRewardsSubmission::<T>::try_get(subnet_id.clone(), epoch) {
      // Redundant
      if submission.complete {
        return
      }
      SubnetRewardsSubmission::<T>::mutate(
        subnet_id.clone(),
        epoch.clone(),
        |params: &mut RewardsData<T::AccountId>| {
          params.complete = true;
        }
      );
      let reward = Self::get_validator_reward(
        submission.nodes_count,
        submission.attests.len() as u32
      );
    }
  }

  /// Return the validators reward that submitted data on the previous epoch
  // The attestation percentage must be greater than the MinAttestationPercentage
  pub fn get_validator_reward(
    nodes_count: u32,
    attests: u32
  ) -> u128 {
    let attestation_percentage: u128 = Self::percent_div(attests as u128, nodes_count as u128);
    if MinAttestationPercentage::<T>::get() > attestation_percentage {
      return 0
    }
    Self::percent_mul(BaseReward::<T>::get(), attestation_percentage)
  }

  pub fn slash_validator(subnet_id: u32, validator: T::AccountId) {
    // We never ensure balance is above 0 because any validator chosen must have the target stake
    // balance at a minimum

    // --- Get stake balance
    let account_model_stake: u128 = AccountSubnetStake::<T>::get(validator.clone(), subnet_id);

    // --- Get slash amount up to max slash
    let mut slash_amount: u128 = Self::percent_mul(account_model_stake, SlashPercentage::<T>::get());
    let max_slash: u128 = MaxSlashAmount::<T>::get();
    if slash_amount > max_slash {
      slash_amount = max_slash
    }
    
    Self::decrease_account_stake(
      &validator.clone(),
      subnet_id, 
      slash_amount,
    );
  }
}