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
use frame_support::dispatch::Vec;
use no_std_net::IpAddr;

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
        TryInto::try_into(block).ok().expect("blockchain will not exceed 2^64 blocks; QED.")
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
        let average: u128 = Self::get_average(filtered_values);
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
            return final_values;
        } else if values.len() < 4 {
            return values;
        }

        let mut final_values: Vec<u128> = Vec::new();
        let mut values: Vec<u128> = values;
        values.sort();

        let q1: f64 = Self::get_quantile(values.clone(), 48.0);
        let q3: f64 = Self::get_quantile(values.clone(), 52.0);

        let iqr: f64 = q3 - q1;
        let max_value: f64 = q3 + iqr * 1.5;
        let min_value: f64 = q1 - iqr * 1.5;

        let values_iter: scale_info::prelude::vec::IntoIter<u128> = values.into_iter();

        for value in values_iter {
            // push middle of curve values only
            if (value as f64) >= min_value && (value as f64) <= max_value {
                final_values.push(value);
            }
        }

        return final_values;
    }

    fn get_quantile(array: Vec<u128>, quantile: f64) -> f64 {
        // Get the index the quantile is at.
        let index: f64 = (quantile / 100.0) * ((array.len() as f64) - 1.0);

        // Check if it has decimal places.
        if (index as f64) % 1.0 == 0.0 {
            return array[index as usize] as f64;
        } else {
            // Get the lower index.
            let lower_index: f64 = libm::floor(index);
            // Get the remaining.
            let remainder: f64 = index - lower_index;
            // Add the remaining to the lowerindex value.
            return (array[lower_index as usize] as f64) +
                remainder *
                    (
                        ((array[(lower_index as usize) + 1] as f64) -
                            (array[lower_index as usize] as f64)) as f64
                    );
        }
    }

    fn get_average(array: Vec<u128>) -> u128 {
        let sum: u128 = array.iter().sum();
        let average: u128 = sum / (array.len() as u128);
        return average;
    }

    // Validates IP Address
    pub fn validate_ip_address(ip: Vec<u8>) -> bool {
        let ip_as_string: String = String::from_utf8(ip.clone()).unwrap();

        // If is in IP format
        let is_ip_address: bool = match ip_as_string.parse::<IpAddr>() {
            Ok(_) => true,
            Err(_) => false,
        };

        if !is_ip_address {
            return false;
        }

        // Unwrap safely
        let ip_as_string_parsed: IpAddr = ip_as_string.parse::<IpAddr>().unwrap();

        // May be redundant but checked
        let is_unspecified: bool = ip_as_string_parsed.is_unspecified();

        // If localhost IP address
        let is_loopback: bool = ip_as_string_parsed.is_loopback();

        if is_unspecified || is_loopback {
            return false;
        }

        let is_ipv4: bool = ip_as_string_parsed.is_ipv4();

        let is_ipv6: bool = ip_as_string_parsed.is_ipv6();

        // Ensure ipv4 or ipv6
        if !is_ipv4 && !is_ipv6 {
            return false;
        }

        // All checks have passed return true+
        true
    }

    // Validates Port
    pub fn validate_port(port: u16) -> bool {
        if port.clamp(0, u16::MAX) <= 0 {
            return false;
        }

        true
    }

    // Loosely validates Peer ID
    pub fn validate_peer_id(peer_id: PeerId) -> bool {
        let mut valid: bool = false;

        let peer_id_0: Vec<u8> = peer_id.0;

        let len: usize = peer_id_0.len();

        // PeerId must be equal to or greater than 32 chars
        // PeerId must be equal to or less than 128 chars
        if len < 32 || len > 128 {
            return false;
        }

        let first_char = peer_id_0[0];

        let second_char = peer_id_0[1];

        if first_char == 49 {
            // Peer ID (ed25519, using the "identity" multihash) encoded as a raw base58btc multihash
            valid = len <= 128;
        } else if first_char == 81 && second_char == 109 {
            // Peer ID (sha256) encoded as a raw base58btc multihash
            valid = len <= 128;
        } else if first_char == 102 || first_char == 98 || first_char == 122 || first_char == 109 {
            // Peer ID (sha256) encoded as a CID
            valid = len <= 128;
        }

        valid
    }

    pub fn get_percentage_as_f64(x: f64, y: f64) -> f64 {
        // Convert to percentage f64.
        if x == 0.0 || y == 0.0 {
            return 0.0;
        }

        let result: f64 = (x * 100.0) / y;
        result
    }

    // pub fn get_percentage_as_u128(x: u128, y: u128) -> u128 {
    //   // Convert to percentage u128.
    //   if x == 0 || y == 0 {
    //     return 0
    //   }

    //   let result = (x * Self::PERCENTAGE_FACTOR) / y;
    //   result
    // }

    // get eligible blocks for consensus submissions and inclusion on models and peers
    pub fn get_eligible_epoch_block(epochs_interval: u64, initialized: u64, epochs: u64) -> u64 {
        let eligible_block: u64 =
            initialized - (initialized % epochs_interval) + epochs_interval * epochs;
        eligible_block
    }

    /*
    Get the cost of initializing a model
    The cost is the number of model peers that are included in consensus
    multiplied by the cost of initializing a model peer
  */
    pub fn get_model_initialization_cost(block: u64) -> u128 {
        let mut model_peers_included_count: u128 = 0;
        let interval: u64 = ConsensusBlocksInterval::<T>::get();
        let min_required_consensus_inclusion_epochs: u64 =
            MinRequiredPeerConsensusInclusionEpochs::<T>::get();

        for model_peer in ModelPeersData::<T>::iter_values() {
            let is_included: bool =
                block >=
                Self::get_eligible_epoch_block(
                    interval,
                    model_peer.initialized,
                    min_required_consensus_inclusion_epochs
                );

            if is_included {
                model_peers_included_count += 1;
            }
        }

        let init_cost: u128 = ModelPerPeerInitCost::<T>::get();
        model_peers_included_count * init_cost
    }

    // Returns true if consensus block steps are being performed
    // Such as 1. forming consensus 2. generating emissions
    // Current block must be greater than the interval
    pub fn is_in_consensus_steps(block: u64, interval: u64) -> bool {
        block % interval == 0 || (block - 1) % interval == 0
    }

    // If can submit consensus
    //  • Must not be in consensus steps, that is forming consensus or generating emissions
    //  • Must not be in can remove peer range
    // This allows model peers to query consensus data before submitting based on
    // currently stored and live peers
    //
    // e.g. If a model peer is removed in the middle of consensus submissions, model peers can
    //      manipulate the storage to have other peers forced out of consensus
    pub fn can_submit_consensus(block: u64, interval: u64) -> bool {
        let in_consensus_steps: bool = Self::is_in_consensus_steps(block, interval);
        let can_remove_or_update_model_peer: bool = Self::can_remove_or_update_model_peer(
            block,
            interval
        );
        !in_consensus_steps && !can_remove_or_update_model_peer
    }

    // If a model or model peer is able to be included or submit consensus
    //
    // This checks if the block is equal to or greater than therefor shouldn't
    // be used while checking if a model or model peer was able to accept or be
    // included in consensus during the forming of consensus since it checks for
    // the previous epochs eligibility
    pub fn is_epoch_block_eligible(
        block: u64,
        interval: u64,
        epochs: u64,
        initialized: u64
    ) -> bool {
        block >= Self::get_eligible_epoch_block(interval, initialized, epochs)
    }

    // Can a model accept consensus submissions from model peers
    //
    //  • must not be in consensus steps
    //  • must not in model peer removal blocks span
    //  • must meet the minimum required epochs based on the model initialization block
    //  • must meet the minimum required model peers based on the MinModelPeers
    pub fn can_model_accept_consensus_submissions(
        model_id: u32,
        block: u64,
        interval: u64
    ) -> bool {
        // 1. The blockchain must not be in its consensus steps or model peer removal blocks span.
        //    During these block steps, no changes can be made to storage that impact consensus or emissions.
        let can_submit_consensus: bool = Self::can_submit_consensus(block, interval);
        if !can_submit_consensus {
            return false;
        }

        // 2. The minimum required model peers must be initialized before the MinRequiredModelConsensusSubmitEpochs based on the MinModelPeers.
        let min_model_peers: u32 = MinModelPeers::<T>::get();
        let total_model_peers: u32 = TotalModelPeers::<T>::get(model_id);
        if total_model_peers < min_model_peers {
            return false;
        }
        true
    }

    // Can a model peer be updated or removed
    //
    // Model peers can update/remove at the beginning of each epoch
    // based on the RemoveModelPeerEpochPercentage as a percentage
    // of the epochs blocks span
    //
    // This is to avoid any storage changes that can impact consensus or emissions
    pub fn can_remove_or_update_model_peer(block: u64, interval: u64) -> bool {
        let in_consensus_steps: bool = Self::is_in_consensus_steps(block, interval);

        // Get percentage of beginning of epoch can remove peer
        let remove_peer_block_percentage_of_epoch: u128 =
            RemoveModelPeerEpochPercentage::<T>::get();

        // Get blocks span following consensus steps model peers can exit
        let block_span_can_remove_peer: u128 = Self::percent_mul(
            interval as u128,
            remove_peer_block_percentage_of_epoch
        );

        //
        // The result is the span minus the consensus steps
        //
        // e.g. • If we are on block 10000
        //      • The consensus steps are 2 blocks (forming consensus + generating emissions)
        //      • If the interval is 100 blocks per epoch
        //      • If the percentage is 10.0% of the epoch resulting in 8 blocks (10 blocks - 2 steps)
        //          • 10000-10001 will be the consensus steps
        //          • 10002-10010 will be the model removal span
        //          • 10002 will be the start block
        //          • 10010 will be the end block
        //          • Model peers can exit/update between block 10002 and 10010
        //          • Thus model peers can submit consensus between 10011 - 10100

        // start the block after consensus steps
        let start_block: u64 = Self::CONSENSUS_STEPS + (block - (block % interval));
        // end the block up to percentage of epoch blocks
        let end_block: u64 = (block_span_can_remove_peer as u64) + (block - (block % interval));

        // Start block must be greater than or equal to current block
        // End block must be less than or equal to current block
        start_block <= block && block <= end_block && !in_consensus_steps
    }

    // Get model peer is eligible to be a model peer
    // Checks if account penalties do not surpass the max allowed penalties
    pub fn is_account_eligible(account_id: T::AccountId) -> bool {
        let max_account_penalty_count: u32 = MaxAccountPenaltyCount::<T>::get();
        let account_penalty_count: u32 = AccountPenaltyCount::<T>::get(account_id);
        account_penalty_count <= max_account_penalty_count
    }

    // Remove all account's model peers across all of their models
    pub fn do_remove_account_model_peers(block: u64, account_id: T::AccountId) {
        // Get all model_ids for account_id
        let model_ids: Vec<u32> = AccountModels::<T>::get(account_id.clone());
        // Iterate through model_ids and remove model peers
        for model_id in model_ids.iter() {
            Self::do_remove_model_peer(block, *model_id, account_id.clone());
        }
    }

    /// Remove model peer from model
    // to-do: Add slashing to model peers stake balance
    // note: We don't reset AccountPenaltyCount
    pub fn do_remove_model_peer(block: u64, model_id: u32, account_id: T::AccountId) {
        // Take and remove ModelPeersData account_id as key
        // `take()` returns and removes data
        let model_peer: ModelPeer<T::AccountId> = ModelPeersData::<T>::take(
            model_id.clone(),
            account_id.clone()
        );

        // Remove ModelPeerAccount peer_id as key
        ModelPeerAccount::<T>::remove(model_id.clone(), model_peer.clone().peer_id);

        // Update ModelAccount to reflect removal block instead of initialized block
        // Peer will be able to unstake after required epochs have passed
        let mut model_accounts: BTreeMap<T::AccountId, u64> = ModelAccount::<T>::get(
            model_id.clone()
        );
        model_accounts.insert(account_id.clone(), block);
        ModelAccount::<T>::insert(model_id.clone(), model_accounts);

        // Update total model peers by substracting 1
        TotalModelPeers::<T>::mutate(model_id.clone(), |n: &mut u32| {
            *n -= 1;
        });

        // Remove model peer consensus results
        ModelPeerConsensusResults::<T>::remove(model_id.clone(), account_id.clone());

        ModelPeerConsecutiveConsensusSent::<T>::remove(model_id.clone(), account_id.clone());
        ModelPeerConsecutiveConsensusNotSent::<T>::remove(model_id.clone(), account_id.clone());

        PeerConsensusEpochSubmitted::<T>::remove(model_id.clone(), account_id.clone());
        PeerConsensusEpochUnconfirmed::<T>::remove(model_id.clone(), account_id.clone());

        // Remove model_id from AccountModels
        let mut account_model_ids: Vec<u32> = AccountModels::<T>::get(account_id.clone());
        account_model_ids.retain(|&x| x != model_id);

        // Insert retained model_id's
        AccountModels::<T>::insert(account_id.clone(), account_model_ids);

        log::info!(
            "Removed model peer AccountId {:?} from model ID {:?}",
            account_id.clone(),
            model_id.clone()
        );

        Self::deposit_event(Event::ModelPeerRemoved {
            model_id: model_id.clone(),
            account_id: account_id.clone(),
            peer_id: model_peer.clone().peer_id,
            block: block,
        });
    }

    // TODO: ?
    pub fn get_account_slash_percentage(account_id: T::AccountId) -> u128 {
        let _model_ids: Vec<u32> = AccountModels::<T>::get(account_id.clone());
        0
    }

    // Get the total number of model peers that are eligible to submit consensus data
    pub fn get_total_submittable_model_peers(
        model_id: u32,
        block: u64,
        consensus_blocks_interval: u64,
        min_required_peer_consensus_submit_epochs: u64
    ) -> u32 {
        // Count of eligible to submit consensus data model peers
        let mut total_submit_eligible_model_peers = 0;

        // increment total_submit_eligible_model_peers with model peers that are eligible to submit consensus data
        for model_peer in ModelPeersData::<T>::iter_prefix_values(model_id.clone()) {
            let initialized: u64 = model_peer.initialized;
            if
                Self::is_epoch_block_eligible(
                    block,
                    consensus_blocks_interval,
                    min_required_peer_consensus_submit_epochs,
                    initialized
                )
            {
                total_submit_eligible_model_peers += 1;
            }
        }

        total_submit_eligible_model_peers
    }

    // Gets the count of all eligible-to-submit model peers on the previous epoch to account for the current block steps
    pub fn get_prev_epoch_total_submittable_model_peers(
        model_id: u32,
        block: u64,
        consensus_blocks_interval: u64,
        min_required_peer_consensus_submit_epochs: u64
    ) -> u32 {
        // Count of eligible to submit consensus data model peers
        let mut total_submit_eligible_model_peers: u32 = 0;

        // increment total_submit_eligible_model_peers with model peers that are eligible to submit consensus data
        for model_peer in ModelPeersData::<T>::iter_prefix_values(model_id.clone()) {
            let initialized: u64 = model_peer.initialized;
            if
                block >
                Self::get_eligible_epoch_block(
                    consensus_blocks_interval,
                    initialized,
                    min_required_peer_consensus_submit_epochs
                )
            {
                total_submit_eligible_model_peers += 1;
            }
        }

        total_submit_eligible_model_peers
    }

    pub fn get_total_dishonesty_voting_model_peers(
        model_id: u32,
        block: u64,
        consensus_blocks_interval: u64,
        min_required_peer_consensus_dishonesty_epochs: u64
    ) -> u32 {
        // Count of eligible to submit consensus data model peers
        let mut total_submit_eligible_model_peers = 0;

        // increment total_submit_eligible_model_peers with model peers that are eligible to submit consensus data
        for model_peer in ModelPeersData::<T>::iter_prefix_values(model_id.clone()) {
            let initialized: u64 = model_peer.initialized;
            if
                Self::is_epoch_block_eligible(
                    block,
                    consensus_blocks_interval,
                    min_required_peer_consensus_dishonesty_epochs,
                    initialized
                )
            {
                total_submit_eligible_model_peers += 1;
            }
        }

        total_submit_eligible_model_peers
    }
}
