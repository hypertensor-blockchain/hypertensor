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

impl<T: Config> Pallet<T> {
  pub fn reward_subnets(block: u64, epoch: u32, epoch_length: u64) {
    let min_required_consensus_inclusion_epochs = MinRequiredNodeConsensusInclusionEpochs::<T>::get();
    // --- Get total per subnet rewards
    // Each subnet nodes rewards are based on the target stake even if previously slashed
    let base_subnet_reward: u128 = BaseSubnetReward::<T>::get();

    for (subnet_id, _) in SubnetsData::<T>::iter() {
      if let Ok(submission) = SubnetRewardsSubmission::<T>::try_get(subnet_id.clone(), epoch) {
        let submission_nodes_count: u128 = submission.nodes_count as u128;
        let submission_attestations: u128 = submission.attests.len() as u128;
        let attestation_percentage: u128 = Self::percent_div(submission_attestations, submission_nodes_count);
        let validator: T::AccountId = submission.validator;
        if MinAttestationPercentage::<T>::get() > attestation_percentage {
          // --- Slash validator
          Self::slash_validator(subnet_id.clone(), validator);

          // --- Attestation not successful, move on to next subnet
          continue
        }

        // --- Reward peers
        let sum: u128 = submission.sum;
        let mut rewarded: BTreeSet<T::AccountId> = BTreeSet::new();
        for data in submission.data.iter() {
          // --- Iterate each subnet account and give rewards if in data and attested
          if let Ok(account_id) = SubnetNodeAccount::<T>::try_get(subnet_id.clone(), data.clone().peer_id) {
            // --- Ensure peer is eligible for rewards by checking if they attested
            // In order to receive rewards, each peer must attest
            // Only submit-eligible nodes can attest the epoch so we don't check for eligibility here
            if !submission.attests.contains(&account_id) {
              continue
            }

            // --- Ensure no duplicates are submitted
            // Data uses dedup_by(|a, b| a.peer_id == b.peer_id) - this is redundant
            if !rewarded.insert(account_id.clone()) {
              continue
            }

            // --- Calculate score percentage of peer versus sum
            let score_percentage: u128 = Self::percent_div(data.clone().score, sum as u128);
            // --- Calculate score percentage of total subnet rewards
            let mut account_rewards: u128 = Self::percent_mul(score_percentage, base_subnet_reward);

            if account_id == validator {
              account_rewards += Self::get_validator_reward(
                submission_nodes_count as u32,
                submission_attestations as u32
              );    
            }

            // --- Skip if no rewards to give
            if account_rewards == 0 {
              continue;
            }

            // --- Increase account stake and emit event
            Self::increase_account_stake(
              &account_id,
              subnet_id.clone(), 
              account_rewards,
            ); 
          }
        }
      } else if let Ok(rewards_validator) = SubnetRewardsValidator::<T>::try_get(subnet_id.clone(), epoch) {
        // If validator didn't submit anything, then slash
        // Even if a subnet is in a broken state, the chosen validator must submit blank data
        Self::slash_validator(subnet_id.clone(), rewards_validator);
      }
    }
  }

  pub fn reward_subnet_nodes(subnet_id: u32) {

  }
}