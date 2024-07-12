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
    pub fn set_vote_model_in(path: Vec<u8>) -> DispatchResult {
        // ensure model doesn't exists by path
        ensure!(!ModelPaths::<T>::contains_key(path.clone()), Error::<T>::ModelNotExist);

        ModelActivated::<T>::insert(path.clone(), true);

        Self::deposit_event(Event::SetVoteModelIn(path));

        Ok(())
    }

    pub fn set_vote_model_out(path: Vec<u8>) -> DispatchResult {
        // ensure model exists by path
        ensure!(ModelPaths::<T>::contains_key(path.clone()), Error::<T>::ModelNotExist);

        ModelActivated::<T>::insert(path.clone(), false);

        Self::deposit_event(Event::SetVoteModelOut(path));

        Ok(())
    }

    pub fn set_max_models(value: u32) -> DispatchResult {
        ensure!(value <= 100, Error::<T>::InvalidMaxModels);

        MaxModels::<T>::set(value);

        Self::deposit_event(Event::SetMaxModels(value));

        Ok(())
    }

    pub fn set_min_model_peers(value: u32) -> DispatchResult {
        let max_model_peers = MaxModelPeers::<T>::get();

        let peer_removal_threshold = PeerRemovalThreshold::<T>::get();
        let min_value = Self::percent_div_round_up(
            1 as u128,
            Self::PERCENTAGE_FACTOR - peer_removal_threshold
        );

        // Ensure over 10
        // Ensure less than MaxModelPeers
        // Ensure divisible by PeerRemovalThreshold
        //  • e.g. if the threshold is .8, we need a minimum of
        ensure!(
            value >= 9 && value <= max_model_peers && value >= (min_value as u32),
            Error::<T>::InvalidMinModelPeers
        );

        MinModelPeers::<T>::set(value);

        Self::deposit_event(Event::SetMinModelPeers(value));

        Ok(())
    }

    pub fn set_max_model_peers(value: u32) -> DispatchResult {
        // Ensure divisible by .01%
        // Ensuring less than or equal to PERCENTAGE_FACTOR is redundant but keep
        // for possible updates in future versions
        // * Remove `value <= Self::PERCENTAGE_FACTOR` if never used in mainnet
        ensure!(
            value <= 1000 && (value as u128) <= Self::PERCENTAGE_FACTOR,
            Error::<T>::InvalidMaxModelPeers
        );

        MaxModelPeers::<T>::set(value);

        Self::deposit_event(Event::SetMaxModelPeers(value));

        Ok(())
    }

    pub fn set_min_stake_balance(value: u128) -> DispatchResult {
        ensure!(value > 0, Error::<T>::InvalidMinStakeBalance);

        MinStakeBalance::<T>::set(value);

        Self::deposit_event(Event::SetMinStakeBalance(value));

        Ok(())
    }

    pub fn set_tx_rate_limit(value: u64) -> DispatchResult {
        TxRateLimit::<T>::set(value);

        Self::deposit_event(Event::SetTxRateLimit(value));

        Ok(())
    }

    pub fn set_max_consensus_epochs_errors(value: u32) -> DispatchResult {
        ensure!(value <= 1000, Error::<T>::InvalidMaxZeroConsensusEpochs);

        MaxModelConsensusEpochsErrors::<T>::set(value);

        Self::deposit_event(Event::SetMaxZeroConsensusEpochs(value));

        Ok(())
    }

    // Set the time required for a model to be in storage before consensus can be formed
    // This allows time for peers to become model peers to the model doesn't increment `no-consensus'`
    pub fn set_min_required_model_consensus_submit_epochs(value: u64) -> DispatchResult {
        // Must be greater than 2 epochs to ensure at least 1 epoch passes
        ensure!(value > 2, Error::<T>::InvalidModelConsensusSubmitEpochs);

        let min_required_peer_consensus_submit_epochs =
            MinRequiredPeerConsensusSubmitEpochs::<T>::get();

        // Must be greater than required submit epochs
        // Peers must have time to become a model peer before submitting consensus
        ensure!(
            value > min_required_peer_consensus_submit_epochs,
            Error::<T>::InvalidModelConsensusSubmitEpochs
        );

        MinRequiredModelConsensusSubmitEpochs::<T>::set(value);

        Self::deposit_event(Event::SetMinRequiredModelConsensusSubmitEpochs(value));

        Ok(())
    }

    pub fn set_min_required_peer_consensus_submit_epochs(value: u64) -> DispatchResult {
        // Must be at least 2 epochs
        // This gives room to be greater than MinRequiredPeerConsensusInclusionEpochs
        ensure!(value > 1, Error::<T>::InvalidPeerConsensusSubmitEpochs);

        let min_required_peer_consensus_inclusion_epochs =
            MinRequiredPeerConsensusInclusionEpochs::<T>::get();

        // Must be greater than required inclusion epochs
        ensure!(
            value > min_required_peer_consensus_inclusion_epochs,
            Error::<T>::InvalidPeerConsensusSubmitEpochs
        );

        MinRequiredPeerConsensusSubmitEpochs::<T>::set(value);

        Self::deposit_event(Event::SetMinRequiredPeerConsensusSubmitEpochs(value));

        Ok(())
    }

    pub fn set_min_required_peer_consensus_inclusion_epochs(value: u64) -> DispatchResult {
        // Must be at least 1 epoch
        ensure!(value > 0, Error::<T>::InvalidPeerConsensusInclusionEpochs);

        let min_required_peer_consensus_submit_epochs =
            MinRequiredPeerConsensusSubmitEpochs::<T>::get();

        // must be less than required submit epochs
        ensure!(
            value < min_required_peer_consensus_submit_epochs,
            Error::<T>::InvalidPeerConsensusInclusionEpochs
        );

        MinRequiredPeerConsensusInclusionEpochs::<T>::set(value);

        Self::deposit_event(Event::SetMinRequiredPeerConsensusEpochs(value));

        Ok(())
    }

    pub fn set_min_required_peer_consensus_dishonesty_epochs(value: u64) -> DispatchResult {
        // Must be at least 1 epoch
        ensure!(value > 1, Error::<T>::InvalidPeerConsensusDishonestyEpochs);

        let min_required_peer_consensus_submit_epochs =
            MinRequiredPeerConsensusSubmitEpochs::<T>::get();

        // must be less than required submit epochs
        ensure!(
            value < min_required_peer_consensus_submit_epochs,
            Error::<T>::InvalidPeerConsensusDishonestyEpochs
        );

        MinRequiredPeerConsensusDishonestyEpochs::<T>::set(value);

        Self::deposit_event(Event::SetMinRequiredPeerConsensusDishonestyEpochs(value));

        Ok(())
    }

    pub fn set_max_outlier_delta_percent(value: u8) -> DispatchResult {
        ensure!(value <= (100 as u8), Error::<T>::InvalidMaxOutlierDeltaPercent);

        MaximumOutlierDeltaPercent::<T>::set(value);

        Self::deposit_event(Event::SetMaximumOutlierDeltaPercent(value));

        Ok(())
    }

    pub fn set_model_peer_consensus_submit_percent_requirement(value: u128) -> DispatchResult {
        // Update MinModelPeers before
        let min_model_peer_consensus_submit_count = Self::percent_mul_round_up(
            MinModelPeers::<T>::get() as u128,
            value
        );

        // Must be less than 100.00% and greater than 51.00%
        // Resulting min model peers submitting consensus requirement must be greater
        // than or equal to four
        ensure!(
            value <= Self::PERCENTAGE_FACTOR &&
                value >= 5100 &&
                min_model_peer_consensus_submit_count >= 4,
            Error::<T>::InvalidModelPeerConsensusSubmitPercentRequirement
        );

        ModelPeerConsensusSubmitPercentRequirement::<T>::set(value);

        Self::deposit_event(Event::SetModelPeerConsensusSubmitPercentRequirement(value));

        Ok(())
    }

    pub fn set_consensus_blocks_interval(value: u64) -> DispatchResult {
        // Ensure a minimum of 1000 blocks per consensus epoch
        ensure!(value >= 1000, Error::<T>::InvalidConsensusBlocksInterval);

        ConsensusBlocksInterval::<T>::set(value);

        Self::deposit_event(Event::SetConsensusBlocksInterval(value));

        Ok(())
    }

    pub fn set_peer_removal_threshold(value: u128) -> DispatchResult {
        let min_model_peers: u32 = MinModelPeers::<T>::get();
        // minimum required value is 1 / min_model_peers
        // e.g. a minimum of 12 model peers will require a minimum value of
        //      8.3%
        let min_value = Self::percent_div(1 as u128, min_model_peers as u128);

        // Ensure between (51.00, 100.00)
        // Ensure divisible by at least one
        //  • This is redundant but we check anyways
        // The minimum peer removal threshold is 30%
        ensure!(
            value <= Self::PERCENTAGE_FACTOR && value >= 5100 && value >= min_value,
            Error::<T>::InvalidPeerRemovalThreshold
        );

        PeerRemovalThreshold::<T>::set(value);

        Self::deposit_event(Event::SetPeerRemovalThreshold(value));

        Ok(())
    }

    pub fn set_max_model_rewards_weight(value: u128) -> DispatchResult {
        // Ensure between (1, 10000)
        ensure!(value <= Self::PERCENTAGE_FACTOR && value > 0, Error::<T>::InvalidPercent);

        MaxModelRewardsWeight::<T>::set(value);

        Self::deposit_event(Event::SetMaxModelRewardsWeight(value));

        Ok(())
    }

    pub fn set_stake_reward_weight(value: u128) -> DispatchResult {
        // Ensure <= PERCENTAGE_FACTOR
        ensure!(value <= Self::PERCENTAGE_FACTOR, Error::<T>::InvalidPercent);

        StakeRewardWeight::<T>::set(value);

        Self::deposit_event(Event::SetStakeRewardWeight(value));

        Ok(())
    }

    pub fn set_model_per_peer_init_cost(value: u128) -> DispatchResult {
        // Ensure > 0
        ensure!(value > 0 && value < 1000, Error::<T>::InvalidModelPerPeerInitCost);

        ModelPerPeerInitCost::<T>::set(value);

        Self::deposit_event(Event::SetModelPerPeerInitCost(value));

        Ok(())
    }

    pub fn set_model_consensus_unconfirmed_threshold(value: u128) -> DispatchResult {
        // Ensure < PERCENTAGE_FACTOR && > 51.00%
        ensure!(
            value < Self::PERCENTAGE_FACTOR && value >= 5100,
            Error::<T>::InvalidModelConsensusUnconfirmedThreshold
        );

        ModelConsensusUnconfirmedThreshold::<T>::set(value);

        Self::deposit_event(Event::SetModelConsensusUnconfirmedThreshold(value));

        Ok(())
    }

    pub fn set_remove_model_peer_epoch_percentage(value: u128) -> DispatchResult {
        // Ensure < PERCENTAGE_FACTOR & > 20%
        ensure!(
            value < Self::PERCENTAGE_FACTOR && value > 2000,
            Error::<T>::InvalidRemoveModelPeerEpochPercentage
        );

        RemoveModelPeerEpochPercentage::<T>::set(value);

        Self::deposit_event(Event::SetRemoveModelPeerEpochPercentage(value));

        Ok(())
    }
}
