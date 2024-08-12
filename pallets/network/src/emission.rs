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
use system::Config;
use frame_support::dispatch::Vec;
use num_traits::float::FloatCore;
use sp_runtime::{PerThing, Percent};
use sp_runtime::Saturating;

impl<T: Config + pallet::Config> Pallet<T> {
	pub fn generate_emissions() {
		let mut total_stake: u128 = 0;
		let max_stake_balance: u128 = MaxStakeBalance::<T>::get();

		// *** 1. Get all subnets in-consensus and remove it afterwards using `take()`
		let model_ids: Vec<u32> = SubnetsInConsensus::<T>::take();

		// If there are no subnets in-consensus, return
		if model_ids.len() == 0 {
			return
		}

		// Subnet ID => Total Subnet Stake
		// Used to get subnet weights in `get_model_emissions_weights()`
		let mut models_data: BTreeMap<u32, u128> = BTreeMap::new();

		// *** 2. Get total stake sum of live subnets subnet peers in-consensus
		// We iter with model_ids over `iter_values()` in order to initialize a BTreeMap with subnet_id as k1
		for subnet_id in model_ids.iter() {
			let mut total_model_stake: u128 = 0;

			let total_model_stake: u128 = SubnetNodeConsensusResults::<T>::iter_prefix_values(subnet_id.clone())
				.map(|x| {
					let account_model_stake: u128 = AccountSubnetStake::<T>::get(x.account_id, subnet_id.clone());
					// Only get up to max stake balance
					if account_model_stake > max_stake_balance {
						total_model_stake += max_stake_balance;
						max_stake_balance
					} else {
						total_model_stake += account_model_stake;
						account_model_stake
					}
				})
				.sum();

			total_stake.saturating_accrue(total_model_stake);

			models_data.insert(subnet_id.clone(), total_model_stake);
		}

		// *** 3. If there is no total stake balance or subnets in-consensus
		// Then return
		if total_stake == 0 {
			return
		}

		// *** 4. Get total rewards in vault
		let total_vault_balance: u128 = StakeVaultBalance::<T>::get();

		if total_vault_balance == 0 {
			return
		}

		// If SubnetNodeConsensusResults has no values it will be returned during `if total_stake == 0` above
		let consensus_len = SubnetNodeConsensusResults::<T>::iter().count();

		// if consensus_len == 0 {
		// 	return
		// }

		// ** @to-do: Add `compute_rewards()`
		// let total_rewards = Self::compute_rewards(consensus_len as u128, total_stake, total_vault_balance);


		// *** 5. Ensure divisible by percentage factor
		// Node can have a minimum of 0.01% of rewards on both score and stake balance
		// We ensure this is divisible by how many peers there are
		// This isn't perfect but it's a quick way to ensure rewards are distributed properly
		// without requiring to check values after rewards are distributed
		// consensus_len / total_vault_balance > 0.01 { return }
		// consensus_len * 100.00 > total_vault_balance { return }
		if (consensus_len as u128).saturating_mul(Self::PERCENTAGE_FACTOR) > total_vault_balance {
			return
		}

		// *** 6. Weight of rewards towards stake balance
		let stake_reward_weight: u128 = StakeRewardWeight::<T>::get();

		// *** 7. Weight of rewards towards score sum
		let score_reward_weight = Self::PERCENTAGE_FACTOR.saturating_sub(stake_reward_weight);

		// *** 8. Get subnet weights based on excess distribution algorithm
		let models_data: Vec<(u32, u128)> = Self::get_model_emissions_weights(models_data, total_stake);

		// *** 9. If there are no subnet weights, don't run emissions
		if models_data.len() == 0 {
			return
		} 
		// else {
			// Ensure subnet weights sum isn't above PERCENTAGE_FACTOR
		// }

		// -- Track emissions rewarded
		let mut total_emissions_on_epoch: u128 = 0;		

		// *** 10. Iter each subnet that clear minimum weight and distribute rewards to subnet validators
		for subnet in models_data.iter() {
			let subnet_id: u32 = subnet.0;
			let model_weight: u128 = subnet.1;

			// Redundant
			if model_weight == 0 {
				let _ = SubnetNodeConsensusResults::<T>::clear_prefix(subnet_id.clone(), u32::MAX, None);
				continue
			}

			// *** 11. Get all 
			//			a. Accounts submitted, in-consensus, stake balances, and scores
			//			b. The sum of in-consensus subnet stake balances and scorse
			// 
			// Cannot use drain_prefix with mapping so we clear after
			//
			let mut total_model_stake_sum: u128 = 0;
			let mut scores_sum: u128 = 0;
			let accounts: Vec<(T::AccountId, u128, u128)> = SubnetNodeConsensusResults::<T>::iter_prefix_values(subnet_id.clone())
				.map(|x| {
					let mut account_model_stake_balance: u128 = match AccountSubnetStake::<T>::try_get(&x.account_id, subnet_id.clone()) {
						Ok(balance) => balance,
						Err(()) => 0,
					};
					if account_model_stake_balance > max_stake_balance {
						account_model_stake_balance = max_stake_balance;
					}

					total_model_stake_sum.saturating_accrue(account_model_stake_balance);
					scores_sum.saturating_accrue(x.score);

					(x.account_id, account_model_stake_balance, x.score)
				})
				.collect();

			// *** 12. Accounts in-consensus must meet minumum required threshold percent during form_peer_consensus()
			// if not, account.len() will be zero
			if accounts.len() == 0 {
				// We don't clear_prefix here because it is already at zero
				continue
			}

			// *** 13. Reset storage for next epoch
			// to-do: check if all cleared
			let _ = SubnetNodeConsensusResults::<T>::clear_prefix(subnet_id.clone(), u32::MAX, None);

			// *** 14. Rewards to distribute to subnet peers
			let model_emissions: u128 = Self::percent_mul(total_vault_balance, model_weight);

			// *** 15. Return if subnet weight is zero 
			if model_emissions == 0 {
				continue
			}

			// *** 16. Return if either are zero 
			// Both variables are required to generate emissions
			if total_model_stake_sum == 0 || scores_sum == 0 {
				continue
			}
			
			// *** 17. Iter each account in-consensus
			for (account_id, stake_balance, score) in accounts.iter() {
				// *** 18. If balance is zero, continue
				// Redundant
				if *stake_balance == 0 {
					continue
				}
	
				// *** 19. Percent of stake peer has in subnet stake
				// If under 0.01% it will return zero
				// This is checked later in `account_avg_weight`
				let account_stake_percentage: u128 = Self::percent_div(
					*stake_balance, 
					total_model_stake_sum
				);

				// *** 20. Percent of score peer has in scores sum
				// If under 0.01% it will return zero
				// This is checked later in `account_avg_weight`
				let account_score_percentage: u128 = Self::percent_div(*score, scores_sum);
		
				// *** 21. Calculate weights together
				// This increases the odds of receiving rewards vs. doing them separately if the sum or weight is low
				let account_avg_weight_1: u128 = Self::percent_mul(stake_reward_weight, account_stake_percentage);
				let account_avg_weight_2: u128 = Self::percent_mul(score_reward_weight, account_score_percentage);
				let account_avg_weight: u128 = account_avg_weight_1 + account_avg_weight_2;

				// *** 22. Continue if weight zero
				// This previous calculations will round to 0 if weight is under 0.01%
				if account_avg_weight == 0 {
					continue
				}
	
				// *** 22. Get accounts total emissions on this subnet
				let account_total_emissions: u128 = Self::percent_mul(model_emissions, account_avg_weight);
				
				// Redundant
				if account_total_emissions == 0 {
					continue
				}

				// *** 23. Increase accounts staking balances
				// Increase account subnet stake
				// Increase account total stake
				// Increase subnet stake
				// Increase total stake
				// note: there is no rate limiter on this function
				Self::increase_account_stake(
					&account_id,
					subnet_id.clone(), 
					account_total_emissions,
				);

				total_emissions_on_epoch.saturating_accrue(account_total_emissions);
			}
		}

		// Decrease stake vault balance
		StakeVaultBalance::<T>::set(total_vault_balance.saturating_sub(total_emissions_on_epoch));
		log::info!("Generated emissions for a total of {:?}", total_emissions_on_epoch);
  }

	pub fn generate_emissionsf(block: u64) {
		let mut total_stake: u128 = 0;

		// *** 1. Get all subnets in-consensus and remove it afterwards using `take()`
		let model_ids: Vec<u32> = SubnetsInConsensus::<T>::take();

		// If there are no subnets in-consensus, return
		if model_ids.len() == 0 {
			return
		}

		let max_stake_balance: u128 = MaxStakeBalance::<T>::get();
		let stake_reward_weight: u128 = StakeRewardWeight::<T>::get();
		let score_reward_weight = Self::PERCENTAGE_FACTOR.saturating_sub(stake_reward_weight);

		for subnet_id in model_ids.iter() {
			let mut total_model_stake_sum: u128 = 0;
			let mut scores_sum: u128 = 0;
			let accounts: Vec<(T::AccountId, u128, u128)> = SubnetNodeConsensusResults::<T>::iter_prefix_values(subnet_id.clone())
				.map(|x| {
					let mut account_model_stake_balance: u128 = match AccountSubnetStake::<T>::try_get(&x.account_id, subnet_id.clone()) {
						Ok(balance) => balance,
						Err(()) => 0,
					};
					if account_model_stake_balance > max_stake_balance {
						account_model_stake_balance = max_stake_balance;
					}

					total_model_stake_sum.saturating_accrue(account_model_stake_balance);
					scores_sum.saturating_accrue(x.score);

					(x.account_id, account_model_stake_balance, x.score)
				})
				.collect();

			if accounts.len() == 0 {
				// We don't clear_prefix here because it is already at zero
				continue
			}
			
			// *** Reset storage for next epoch
			let _ = SubnetNodeConsensusResults::<T>::clear_prefix(subnet_id.clone(), u32::MAX, None);

			// *** Rewards to distribute to subnet peers based on stake to the specific subnet
			let epoch_rewards: u128 = Self::get_epoch_rewards(block, total_model_stake_sum);

			// *** 17. Iter each account in-consensus
			for (account_id, stake_balance, score) in accounts.iter() {
				// *** 18. If balance is zero, continue
				// Redundant
				if *stake_balance == 0 {
					continue
				}

				let account_stake_percentage: u128 = Self::percent_div(
					*stake_balance, 
					total_model_stake_sum
				);

				let account_score_percentage: u128 = Self::percent_div(*score, scores_sum);

				// *** 21. Calculate weights together
				// This increases the odds of receiving rewards vs. doing them separately if the sum or weight is low
				let account_avg_weight_1: u128 = Self::percent_mul(stake_reward_weight, account_stake_percentage);
				let account_avg_weight_2: u128 = Self::percent_mul(score_reward_weight, account_score_percentage);
				let account_avg_weight: u128 = account_avg_weight_1 + account_avg_weight_2;

				// *** 22. Continue if weight zero
				// This previous calculations will round to 0 if weight is under 0.01%
				if account_avg_weight == 0 {
					continue
				}
	
				// *** 22. Get accounts total emissions on this subnet
				let account_total_emissions: u128 = Self::percent_mul(epoch_rewards, account_avg_weight);
				
				// Redundant
				if account_total_emissions == 0 {
					continue
				}

				// *** 23. Increase accounts staking balances
				// Increase account subnet stake
				// Increase account total stake
				// Increase subnet stake
				// Increase total stake
				// note: there is no rate limiter on this function
				Self::increase_account_stake(
					&account_id,
					subnet_id.clone(), 
					account_total_emissions,
				);
			}
		}
	}

	// Excess Weight Distribution
	//
	// Weights are as `model_stake_balance / total_stake_balance`
	//
	// No 1 subnet can have over MaxSubnetRewardsWeight e.g. 50% of total rewards
	// If one does, we balance and distribute the excess in proportion to the other subnets
	//
	// Ensures subnet weights don't surpass the max weight based on MaxSubnetRewardsWeight
	// Any excess of weights from subnets is distributed over other subnets weights based 
	// on the total sum of underweight subnet weights.
	//
	// Returns subnet_id and subnet weight - weight will be rounded down
	//
	// The weights are used to determine how much of the stake vault rewards are to 
	// be distributed to each subnet
	//
	/// `models_data` is Subnet ID => total subnet stake balance
	/// `total_stake` is the total amount staked of live subnets
	fn get_model_emissions_weights(models_data: BTreeMap<u32, u128>, total_stake: u128) -> Vec<(u32, u128)> {
		// push eligible data into models_data
		let mut model_weights_data: Vec<(u32, u128)> = Vec::new();

		// We first get weights as u128 in order to sort percentages
		for (subnet_id, total_model_stake) in models_data.iter() {
			// Subnet must have a minimum of 0.01% staked versus the total staked to be included
			// All percentages are rounded down when they are odd numbers
			let model_stake_percentage = Self::percent_div(*total_model_stake, total_stake);

			// Subnet peers must collectly keep a minimum required stake percentage of 0.01%
			if model_stake_percentage == 0 {
				SubnetConsensusEpochsErrors::<T>::mutate(subnet_id.clone(), |n: &mut u32| *n += 1);
			}

			if model_stake_percentage != 0 {
				model_weights_data.push((*subnet_id, model_stake_percentage))
			}
		}
		
		let model_weights_data_len = model_weights_data.len();

		// If there is no subnet weights data
		// Return empty Vec
		if model_weights_data_len == 0 {
			return Vec::<(u32, u128)>::new()
		}

		// Sort in descending order
		model_weights_data.sort_by(|a, b| {
			b.1.cmp(&a.1)
    });
		
		// If there is 1 subnet do not run any computations and return now
		if model_weights_data_len <= 1 {
			return model_weights_data;
		}

		// Get total weight of subnets
		// This doesn't need to be 100.0, can be above or below 100.0 to perform calculations
		//
		// It is likely the initial_weights_sum will be under 100.0 if numbers aren't directly divisible by 100.0
		//
		// For In is generally faster than model_weights_data.iter().map(|x| x.1).sum()
		let mut initial_weights_sum: u128 = 0;
		for data in model_weights_data.iter() {
			initial_weights_sum += data.1;
		}

		let mut target_weight: u128 = MaxSubnetRewardsWeight::<T>::get();

		// Make sure math is possible
		// if not update the target_weight
		//
		// Ensure excess weight can be distributed while remaining the sum and not going overweight
		//
		// e.g. If target weight is 10% and there are 2 subnets
		//			• The minimum weight would be 50%
		//			• Assuming [50,50]
		//				• If kept 10%, the sum will not equal the initial_weights_sum
		//					• The excess would be 80 on index 0, and index 0 and 1 would decrease to 10. Thus summing to 20
		// e.g. If target weight is 10% and there are 3 subnets
		//			• The minimum weight would be 33.33%
		//			• Assuming [40,40,20]
		//				• If kept 10%, the sum will not equal the initial_weights_sum
		//					• The excess would collectively be 60 on index 0 and 1, and index 2 would increase to 10 and index 0 and 1 
		//						would decrease to 10. Thus summing to 30
		let min_weight: u128 = Self::PERCENTAGE_FACTOR / model_weights_data_len as u128;
		if target_weight < min_weight {
			target_weight = min_weight;
		}

		// The target number the model_stake_percentage cannot be greater than
		let target_num: u128 = Self::percent_mul(initial_weights_sum, target_weight);
		
		// If a subnet has over max weight
    // distribute that to the other subnets
    // based on their proportion of remaining weight
		let mut excess = 0;
    for data in model_weights_data.iter() {
			let weight: u128 = data.1;
			if weight > target_num {
				excess += weight - target_num;
			}
		}

		// if zero excess, return model_weights_data now
		if excess == 0 {
			return model_weights_data;
		}

		let mut weights_sum = initial_weights_sum;
		for data in model_weights_data.iter_mut() {
			let weight: u128 = data.1;
			if weight > target_weight {
				data.1 = target_weight;
			} else {
				// max amount this number can be allotted
				let max_allot = target_weight - weight;
				let percent_of_sum = Self::percent_div(weight, weights_sum);
				let possible_allot = Self::percent_mul(excess, percent_of_sum);
				if max_allot > possible_allot {
					data.1 += possible_allot;
					excess -= possible_allot;
				} else {
					data.1 += max_allot;
					excess -= max_allot;
				}
			}
			weights_sum -= weight;
		}

		model_weights_data
	}
}

// @to-do: compute the rewards from the stake rewards vault to give on epoch based on ideal stake
//				 based on total subnet, peers, and eligible stake balance
pub fn compute_rewards(total_subnet_nodes: u128, total_stake: u128, total_vault_balance: u128) -> u128 {
	total_vault_balance
}