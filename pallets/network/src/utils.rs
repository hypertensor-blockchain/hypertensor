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
use frame_system::pallet_prelude::BlockNumberFor;
use num_traits::float::FloatCore; // is used in floor() ceil()
use frame_support::dispatch::Vec;
use scale_info::prelude::vec;
use no_std_net::{IpAddr, Ipv4Addr, Ipv6Addr};

impl<T: Config> Pallet<T> {
  /// The block steps in between epochs
  // • 1 is for forming consensus with `form_consensus()`
  // • 2 is for generating emissions with `generate_emissions()`
  pub const CONSENSUS_STEPS: u64 = 2;

  pub fn get_tx_rate_limit() -> u64 {
    TxRateLimit::<T>::get()
  }

  pub fn set_last_tx_block(key: &T::AccountId, block: u64) {
    LastTxBlock::<T>::insert(key, block)
  }

  pub fn get_last_tx_block(key: &T::AccountId) -> u64 {
    LastTxBlock::<T>::get(key)
  }

  pub fn exceeds_tx_rate_limit(prev_tx_block: u64, current_block: u64) -> bool {
    let rate_limit: u64 = Self::get_tx_rate_limit();
    if rate_limit == 0 || prev_tx_block == 0 {
      return false;
    }

    return current_block - prev_tx_block <= rate_limit;
  }

  pub fn get_current_block_as_u64() -> u64 {
    TryInto::try_into(<frame_system::Pallet<T>>::block_number())
      .ok()
      .expect("blockchain will not exceed 2^64 blocks; QED.")
  }

  pub fn convert_block_as_u64(block: BlockNumberFor<T>) -> u64 {
    TryInto::try_into(block)
      .ok()
      .expect("blockchain will not exceed 2^64 blocks; QED.")
  }

  // pub fn get_average_score(values: Vec<u128>) -> u128 {
  //   let outliers: Vec<u128> = Self::filter_outliers(values.clone());
  //   let sum: u128 = outliers.iter().sum();
  //   // If sum == 0, get average of values instead
  //   if sum == 0 {
  //     let sum: u128 = values.iter().sum();
  //     let count: u128 = values.len().try_into().unwrap();
  //     if count == 0 {
  //       return 0
  //     } else {
  //       return sum / count
  //     }
  //   }
  //   let count: u128 = outliers.len().try_into().unwrap();
  //   sum / count
  // }

  pub fn get_average_score(values: Vec<u128>) -> u128 {
    let filtered_values: Vec<u128> = Self::filter_outliers(values);
    let average = Self::get_average(filtered_values);
    average
  }

  // https://stackoverflow.com/a/56883420
  // interquartile filter
  fn filter_outliers(values: Vec<u128>) -> Vec<u128> {
    if values.len() == 4 {
      let mut final_values: Vec<u128> = Vec::new();
      let mut values: Vec<u128> = values;
      values.sort();
      // Only push 2 middle values
      // This is a rare scenario but in order to remove any outliers too high or too low
      // we get only the 2 middle values
      final_values.push(values[1]);
      final_values.push(values[2]);
      return final_values
    } else if values.len() < 4 {
      return values
    }

    let mut final_values: Vec<u128> = Vec::new();
    let mut values: Vec<u128> = values;
    values.sort();

    let q1 = Self::get_quantile(values.clone(), 48.0);
    let q3 = Self::get_quantile(values.clone(), 52.0);

    let iqr = q3 - q1;
    let max_value = q3 + iqr * 1.5;
    let min_value = q1 - iqr * 1.5;

    let values_iter: scale_info::prelude::vec::IntoIter<u128> = values.into_iter();

    for value in values_iter {
      // push middle of curve values only
      if value as f64 >= min_value && value as f64 <= max_value {
        final_values.push(value);
      }
    }

    return final_values
  }

  fn get_quantile(array: Vec<u128>, quantile: f64) -> f64 {
    // Get the index the quantile is at.
    let index = quantile / 100.0 * (array.len() as f64 - 1.0);

    // Check if it has decimal places.
    if index as f64 % 1.0 == 0.0 {
      return array[index as usize] as f64;
    } else {
      // Get the lower index.
      let lower_index = index.floor() as f64;
      // Get the remaining.
      let remainder = index - lower_index;
      // Add the remaining to the lowerindex value.
      return array[lower_index as usize] as f64 + remainder * (array[lower_index as usize + 1] as f64 - array[lower_index as usize] as f64) as f64;
    }
  }

  fn get_average(array: Vec<u128>) -> u128 {
    let mut sum = 0;

    for value in array.iter() {
      sum += *value;
    }

    // Returning the average of the numbers
    return sum / array.len() as u128;
  }

  // Validates IP Address
  pub fn validate_ip_address(ip: Vec<u8>) -> bool {
    let ip_as_string = String::from_utf8(ip.clone()).unwrap();

    // If is in IP format
    let is_ip_address: bool = match ip_as_string.parse::<IpAddr>() {
      Ok(_) => true,
      Err(_) => false
    };

    if !is_ip_address {
      return false
    }

    // Unwrap safely
    let ip_as_string_parsed: IpAddr = ip_as_string.parse::<IpAddr>().unwrap();

    // May be redundant but checked
    let is_unspecified: bool = ip_as_string_parsed.is_unspecified();

    // If localhost IP address
    let is_loopback: bool = ip_as_string_parsed.is_loopback();

    if is_unspecified || is_loopback {
      return false
    }

    let is_ipv4: bool = ip_as_string_parsed.is_ipv4();

    let is_ipv6: bool = ip_as_string_parsed.is_ipv6();

    // Ensure ipv4 or ipv6
    if !is_ipv4 && !is_ipv6 {
      return false
    }
    
    // All checks have passed return true
		true
	}

  // Validates Port
  pub fn validate_port(port: u16) -> bool {
		if port.clamp(0, u16::MAX) <= 0 {
			return false;
		}

		true
	}

  // Loosely validates Node ID
  pub fn validate_peer_id(peer_id: PeerId) -> bool {
    let mut valid = false;

    let peer_id_0 = peer_id.0;

    let len = peer_id_0.len();

    // PeerId must be equal to or greater than 32 chars
    // PeerId must be equal to or less than 128 chars
    if len < 32 || len > 128 {
      return false
    };

    let first_char = peer_id_0[0];

    let second_char = peer_id_0[1];

    if first_char == 49 {
      // Node ID (ed25519, using the "identity" multihash) encoded as a raw base58btc multihash
      valid = len <= 128;
    } else if first_char == 81 && second_char == 109 {
      // Node ID (sha256) encoded as a raw base58btc multihash
      valid = len <= 128;
    } else if first_char == 102 || first_char == 98 || first_char == 122 || first_char == 109 {
      // Node ID (sha256) encoded as a CID
      valid = len <= 128;
    }
    
    valid
  }

  pub fn get_percentage_as_f64(x: f64, y: f64) -> f64 {
    // Convert to percentage f64.
    if x == 0.0 || y == 0.0 {
      return 0.0
    }

    let result = (x * 100.0) / y;
    result
  }

  pub fn get_min_subnet_nodes(base_node_memory: u128, memory_mb: u128) -> u32 {
    // --- Get min nodes based on default memory settings
    let real_min_subnet_nodes: u128 = memory_mb / base_node_memory;
    let mut min_subnet_nodes: u32 = MinSubnetNodes::<T>::get();
    if real_min_subnet_nodes as u32 > min_subnet_nodes {
      min_subnet_nodes = real_min_subnet_nodes as u32;
    }
    min_subnet_nodes
  }

  pub fn get_target_subnet_nodes(base_node_memory: u128, min_subnet_nodes: u32) -> u32 {
    Self::percent_mul(
      min_subnet_nodes.into(), 
      TargetSubnetNodesMultiplier::<T>::get()
    ) as u32 + min_subnet_nodes
  }



  // pub fn get_percentage_as_u128(x: u128, y: u128) -> u128 {
  //   // Convert to percentage u128.
  //   if x == 0 || y == 0 {
  //     return 0
  //   }

  //   let result = (x * Self::PERCENTAGE_FACTOR) / y;
  //   result
  // }

  // get eligible blocks for consensus submissions and inclusion on subnets and peers
  pub fn get_eligible_epoch_block(epoch_length: u64, initialized: u64, epochs: u64) -> u64 {
    let eligible_block: u64 = initialized - (initialized % epoch_length) + epoch_length * epochs;
    eligible_block
  }

  pub fn get_model_initialization_cost(block: u64) -> u128 {
    let mut subnet_nodes_included_count: u128 = 0;
    let epoch_length: u64 = T::EpochLength::get();
    let min_required_consensus_inclusion_epochs = MinRequiredNodeConsensusInclusionEpochs::<T>::get();

    for subnet_node in SubnetNodesData::<T>::iter_values() {
      let is_included: bool = block >= Self::get_eligible_epoch_block(
        epoch_length, 
        subnet_node.initialized, 
        min_required_consensus_inclusion_epochs
      );

      if is_included {
        subnet_nodes_included_count += 1;
      }
    }

    let init_cost = SubnetPerNodeInitCost::<T>::get();
    subnet_nodes_included_count * init_cost
  }

  // Returns true if consensus block steps are being performed
  // Such as 1. forming consensus 2. generating emissions
  // Current block must be greater than the epoch_length
  pub fn is_in_consensus_steps(block: u64, epoch_length: u64) -> bool {
    block % epoch_length == 0 || (block - 1) % epoch_length == 0
  }
 
  // If can submit consensus
  //  • Must not be in consensus steps, that is forming consensus or generating emissions
  //  • Must not be in can remove peer range
  // This allows subnet peers to query consensus data before submitting based on
  // currently stored and live peers
  //
  // e.g. If a subnet peer is removed in the middle of consensus submissions, subnet peers can
  //      manipulate the storage to have other peers forced out of consensus
  pub fn can_submit_consensus(block: u64, epoch_length: u64) -> bool {
    let in_consensus_steps: bool = Self::is_in_consensus_steps(block, epoch_length);
    let can_remove_or_update_subnet_node: bool = Self::can_remove_or_update_subnet_node(block, epoch_length);
    !in_consensus_steps && !can_remove_or_update_subnet_node
  }
  
  // If a subnet or subnet peer is able to be included or submit consensus
  //
  // This checks if the block is equal to or greater than therefor shouldn't 
  // be used while checking if a subnet or subnet peer was able to accept or be 
  // included in consensus during the forming of consensus since it checks for
  // the previous epochs eligibility
  pub fn is_epoch_block_eligible(
    block: u64, 
    epoch_length: u64, 
    epochs: u64, 
    initialized: u64
  ) -> bool {
    block >= Self::get_eligible_epoch_block(
      epoch_length, 
      initialized, 
      epochs
    )
  }

  // @to-do
  pub fn can_model_accept_consensus_submissions(
    subnet_id: u32,
    block: u64, 
    epoch_length: u64
  ) -> bool {
    true
  }

  // Can a subnet peel be updated or removed
  //
  // Subnet peers can update/remove at the beginning of each epoch
  // based on the RemoveSubnetNodeEpochPercentage as a percentage
  // of the epochs blocks span
  //
  // This is to avoid any storage changes that can impact consensus or emissions
  pub fn can_remove_or_update_subnet_node(block: u64, epoch_length: u64) -> bool {
    let in_consensus_steps: bool = Self::is_in_consensus_steps(block, epoch_length);

    // Get percentage of beginning of epoch can remove peer
    let remove_peer_block_percentage_of_epoch = RemoveSubnetNodeEpochPercentage::<T>::get();

    // Get blocks span following consensus steps subnet peers can exit
    let block_span_can_remove_peer = Self::percent_mul(
      epoch_length as u128,
      remove_peer_block_percentage_of_epoch
    );

    //
    // The result is the span minus the consensus steps
    //
    // e.g. • If we are on block 10000
    //      • The consensus steps are 2 blocks (forming consensus + generating emissions)
    //      • If the epoch_length is 100 blocks per epoch
    //      • If the percentage is 10.0% of the epoch resulting in 8 blocks (10 blocks - 2 steps)
    //          • 10000-10001 will be the consensus steps
    //          • 10002-10010 will be the subnet removal span
    //          • 10002 will be the start block
    //          • 10010 will be the end block
    //          • Subnet peers can exit/update between block 10002 and 10010
    //          • Thus subnet peers can submit consensus between 10011 - 10100

    // start the block after consensus steps
    let start_block = Self::CONSENSUS_STEPS + (block - (block % epoch_length));
    // end the block up to percentage of epoch blocks
    let end_block = block_span_can_remove_peer as u64 + (block - (block % epoch_length));

    // Start block must be greater than or equal to current block
    // End block must be less than or equal to current block
    start_block <= block && block <= end_block && !in_consensus_steps
  }

  // Get subnet peer is eligible to be a subnet peer
  // Checks if account penalties do not surpass the max allowed penalties
  pub fn is_account_eligible(account_id: T::AccountId) -> bool {
    let max_account_penalty_count = MaxAccountPenaltyCount::<T>::get();
    let account_penalty_count = AccountPenaltyCount::<T>::get(account_id);
    account_penalty_count <= max_account_penalty_count
  }


  // Remove all account's subnet peers across all of their subnets
  pub fn do_remove_account_subnet_nodes(block: u64, account_id: T::AccountId) {
    let model_ids: Vec<u32> = AccountSubnets::<T>::get(account_id.clone());
    for subnet_id in model_ids.iter() {
      Self::do_remove_subnet_node(block, *subnet_id, account_id.clone());
    }
  }

  /// Remove subnet peer from subnet
  // to-do: Add slashing to subnet peers stake balance
  // note: We don't reset AccountPenaltyCount
  pub fn do_remove_subnet_node(block: u64, subnet_id: u32, account_id: T::AccountId) {
    // Take and remove SubnetNodesData account_id as key
    // `take()` returns and removes data
    if let Ok(subnet_node) = SubnetNodesData::<T>::try_get(subnet_id, account_id.clone()) {
      let peer_id = subnet_node.peer_id;

      SubnetNodesData::<T>::remove(subnet_id, account_id.clone());

      // Remove SubnetNodeAccount peer_id as key
      SubnetNodeAccount::<T>::remove(subnet_id, peer_id.clone());

      // Update SubnetAccount to reflect removal block instead of initialized block
      // Node will be able to unstake after required epochs have passed
      let mut model_accounts: BTreeMap<T::AccountId, u64> = SubnetAccount::<T>::get(subnet_id);
      model_accounts.insert(account_id.clone(), block);
      SubnetAccount::<T>::insert(subnet_id, model_accounts);

      // Update total subnet peers by substracting 1
      TotalSubnetNodes::<T>::mutate(subnet_id, |n: &mut u32| *n -= 1);

      // Remove subnet_id from AccountSubnets
      let mut account_model_ids: Vec<u32> = AccountSubnets::<T>::get(account_id.clone());
      account_model_ids.retain(|&x| x != subnet_id);
      // Insert retained subnet_id's
      AccountSubnets::<T>::insert(account_id.clone(), account_model_ids);

      // Remove from classifications
      for class_id in SubnetNodeClass::iter() {
        let mut node_sets: BTreeMap<T::AccountId, u64> = SubnetNodesClasses::<T>::get(subnet_id, class_id);
        node_sets.retain(|k, _| *k != account_id.clone());
        SubnetNodesClasses::<T>::insert(subnet_id, class_id, node_sets);
      }

      log::info!("Removed subnet peer AccountId {:?} from subnet ID {:?}", account_id.clone(), subnet_id);

      Self::deposit_event(
        Event::SubnetNodeRemoved { 
          subnet_id: subnet_id, 
          account_id: account_id.clone(), 
          peer_id: peer_id,
          block: block
        }
      );
    }
  }

  // pub fn do_add_subnet_node(
  //   block: u64, 
  //   subnet_id: u32, 
  //   account_id: T::AccountId,
  //   peer_id: PeerId,
  //   ip: Vec<u8>,
  //   port: u16,
  // ) {
  //   let subnet_node: SubnetNode<T::AccountId> = SubnetNode {
  //     account_id: account_id.clone(),
  //     peer_id: peer_id.clone(),
  //     ip: ip.clone(),
  //     port: port.clone(),
  //     initialized: block,
  //   };

  //   // Insert SubnetNodesData with account_id as key
  //   SubnetNodesData::<T>::insert(subnet_id, account_id.clone(), subnet_node);

  //   // Insert subnet peer account to keep peer_ids unique within subnets
  //   SubnetNodeAccount::<T>::insert(subnet_id, peer_id.clone(), account_id.clone());

  //   // Update to current block
  //   model_accounts.insert(account_id.clone(), block);
  //   SubnetAccount::<T>::insert(subnet_id, model_accounts);

  //   // Add subnet_id to account
  //   // Account can only have a subnet peer per subnet so we don't check if it exists
  //   AccountSubnets::<T>::append(account_id.clone(), subnet_id);

  //   // Increase total subnet peers
  //   TotalSubnetNodes::<T>::mutate(subnet_id, |n: &mut u32| *n += 1);
  // }

  pub fn get_account_slash_percentage(account_id: T::AccountId) -> u128 {
    let model_ids: Vec<u32> = AccountSubnets::<T>::get(account_id.clone());
    0
  }

  // subnet_id: UID of subnet
  // block: current block
  // epoch_length: the number of blocks per epoch
  // epochs: required number of epochs
  pub fn get_total_eligible_subnet_nodes_count(
    subnet_id: u32,
    block: u64,
    epoch_length: u64,
    epochs: u64
  ) -> u32 {
    // Count of eligible to submit consensus data subnet peers
    let mut total_eligible_subnet_nodes = 0;
    
    // increment total_eligible_subnet_nodes with subnet peers that are eligible to submit consensus data
    for subnet_node in SubnetNodesData::<T>::iter_prefix_values(subnet_id) {
      let initialized: u64 = subnet_node.initialized;
      if Self::is_epoch_block_eligible(
        block, 
        epoch_length, 
        epochs, 
        initialized
      ) {
        total_eligible_subnet_nodes += 1;
      }
    }

    total_eligible_subnet_nodes
  }

  // Gets the count of all eligible to submit subnet peers on the previous epoch to account for the current block steps
  // subnet_id: UID of subnet
  // block: current block
  // epoch_length: the number of blocks per epoch
  // epochs: required number of epochs
  pub fn get_prev_epoch_total_eligible_subnet_nodes_count(
    subnet_id: u32,
    block: u64,
    epoch_length: u64,
    epochs: u64
  ) -> u32 {
    // Count of eligible to submit consensus data subnet peers
    let mut total_eligible_subnet_nodes = 0;
    
    // increment total_eligible_subnet_nodes with subnet peers that are eligible to submit consensus data
    for subnet_node in SubnetNodesData::<T>::iter_prefix_values(subnet_id) {
      let initialized: u64 = subnet_node.initialized;
      if block > Self::get_eligible_epoch_block(
        epoch_length, 
        initialized, 
        epochs
      ) {
        total_eligible_subnet_nodes += 1;
      }
    }

    total_eligible_subnet_nodes
  }

  // subnet_id: UID of subnet
  // block: current block
  // epoch_length: the number of blocks per epoch
  // epochs: required number of epochs
  pub fn get_eligible_subnet_nodes_accounts(
    subnet_id: u32,
    block: u64,
    epoch_length: u64,
    epochs: u64
  ) -> Vec<T::AccountId> {
    // Count of eligible to submit consensus data subnet peers
    let mut account_ids: Vec<T::AccountId> = Vec::new();
    
    // increment total_eligible_subnet_nodes with subnet peers that are eligible to submit consensus data
    for subnet_node in SubnetNodesData::<T>::iter_prefix_values(subnet_id) {
      let initialized: u64 = subnet_node.initialized;
      if Self::is_epoch_block_eligible(
        block, 
        epoch_length, 
        epochs, 
        initialized
      ) {
        account_ids.push(subnet_node.account_id)
      }
    }

    account_ids
  }

  // subnet_id: UID of subnet
  // block: current block
  // epoch_length: the number of blocks per epoch
  // epochs: required number of epochs
  pub fn get_total_accountants(
    subnet_id: u32,
    block: u64,
    epoch_length: u64,
    epochs: u64
  ) -> u32 {
    // Count of eligible to submit consensus data subnet peers
    let mut total_submit_eligible_subnet_nodes = 0;
    
    // increment total_submit_eligible_subnet_nodes with subnet peers that are eligible to submit consensus data
    for subnet_node in SubnetNodesData::<T>::iter_prefix_values(subnet_id) {
      let initialized: u64 = subnet_node.initialized;
      if Self::is_epoch_block_eligible(
        block, 
        epoch_length, 
        epochs, 
        initialized
      ) {
        total_submit_eligible_subnet_nodes += 1;
      }
    }

    total_submit_eligible_subnet_nodes
  }

  // pub fn is_accountant(
  //   subnet_id: u32, 
  //   account_id: u32
  // ) -> bool {
  //   let account_subnet_node = SubnetNodesData::<T>::get(subnet_id, account_id.clone());
  //   let submitter_peer_initialized: u64 = account_subnet_node.initialized;
  //   let block: u64 = Self::get_current_block_as_u64();
  //   let epoch_length: u64 = T::EpochLength::get();
  //   let min_required_peer_accountant_epochs: u64 = MinRequiredNodeAccountantEpochs::<T>::get();

  //   return Self::is_epoch_block_eligible(
  //     block, 
  //     epoch_length, 
  //     min_required_peer_accountant_epochs, 
  //     submitter_peer_initialized
  //   )
  // }

  /// Check if subnet ID exists and account has a subnet peer within subnet ID exists
  pub fn is_model_and_subnet_node(subnet_id: u32, account_id: T::AccountId) -> bool {
    if !SubnetsData::<T>::contains_key(subnet_id) {
      return false
    }

    if !SubnetNodesData::<T>::contains_key(subnet_id, account_id) {
      return false
    }

    return true
  }

  /// Check if subnet ID exists, account has a subnet peer within subnet ID exists, and peer ID exists within subnet
  // Note: We aren't checking if the account is the subnet peer, just if they exist at all
  pub fn is_model_and_subnet_node_and_subnet_node_account(
    subnet_id: u32, 
    peer_id: PeerId, 
    account_id: T::AccountId
  ) -> bool {
    if !Self::is_model_and_subnet_node(subnet_id, account_id) {
      return false
    }

    let subnet_node_account_exists: bool = match SubnetNodeAccount::<T>::try_get(subnet_id, peer_id) {
      Ok(_result) => true,
      Err(()) => false,
    };

    return subnet_node_account_exists
  }

  pub fn transition_idle_to_included() {

  }

  pub fn transition_included_to_submittable() {

  }

  pub fn transition_submittable_to_accountant() {

  }

  /// Shift up subnet nodes to new classifications
  // This is used to know the len() of each class of subnet nodes instead of iterating through each time
  pub fn shift_node_classes(block: u64, epoch_length: u64) {
    for (subnet_id, _) in SubnetsData::<T>::iter() {
      let class_ids = SubnetNodeClass::iter();
      let last_class_id = class_ids.clone().last().unwrap();
      log::error!("last_class_id {:?}", last_class_id);

      for mut class_id in class_ids {
        // Can't increase user class after last so skip
        if class_id == last_class_id {
          continue;
        }

        log::error!("current class_id {:?}", class_id);

        // If there are none in the class, then skip
        // let node_sets: BTreeMap<T::AccountId, u64> = SubnetNodesClasses::<T>::get(
        //   subnet_id, 
        //   class_id.clone()
        // )
        // .ok_or(continue)
        // .unwrap();
        let node_sets: BTreeMap<T::AccountId, u64> = SubnetNodesClasses::<T>::get(
          subnet_id, 
          class_id.clone()
        );

        // If initialized but empty, then skip
        if node_sets.is_empty() {
          continue;
        }
        
        // --- Get next class to shift into
        let class_index = class_id.index();
        log::error!("class_index {:?}", last_class_id);

        // --- Safe unwrap from `continue` from last
        let next_class_id: SubnetNodeClass = SubnetNodeClass::from_repr(class_index + 1).unwrap();
        log::error!("next_class_id {:?}", next_class_id);

        // --- Copy the node sets for mutation
        let mut node_sets_copy: BTreeMap<T::AccountId, u64> = node_sets.clone();
        
        // --- Get next node sets for mutation or initialize new BTreeMap
        let mut next_node_sets: BTreeMap<T::AccountId, u64> = match SubnetNodesClasses::<T>::try_get(subnet_id, next_class_id) {
          Ok(next_node_sets) => next_node_sets,
          Err(_) => BTreeMap::new(),
        };
    
        // --- Get epochs required to be in class from the initialization block
        let epochs = SubnetNodeClassEpochs::<T>::get(class_id.clone());
        log::error!("epochs {:?}", epochs);

        for node_set in node_sets.iter() {
          if let Ok(subnet_node_data) = SubnetNodesData::<T>::try_get(subnet_id, node_set.0) {
            let initialized: u64 = subnet_node_data.initialized;
            if Self::is_epoch_block_eligible(
              block, 
              epoch_length, 
              epochs, 
              initialized
            ) {
              log::error!("node_set insert");
              // --- Insert to the next classification, will only insert if doesn't already exist
              next_node_sets.insert(node_set.0.clone(), *node_set.1);
            }  
          } else {
            log::error!("node_set node exists remove");
            // Remove the account from classification if they don't exist anymore
            node_sets_copy.remove(node_set.clone().0);
          }
        }
        // --- Update classifications
        SubnetNodesClasses::<T>::insert(subnet_id, class_id, node_sets_copy);
        SubnetNodesClasses::<T>::insert(subnet_id, next_class_id, next_node_sets);
      }
    }
  }

  pub fn do_choose_validator_and_accountants(epoch: u32) {
    for (subnet_id, _) in SubnetsData::<T>::iter() {

    }
  }

  pub fn eth_into_gwei(
    input: u128,
  ) -> u128 {
    input / 1000000000
  }

  pub fn gwei_into_eth(
    input: u128,
  ) -> u128 {
    input * 1000000000
  }
}