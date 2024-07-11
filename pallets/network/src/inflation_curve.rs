use super::*;
use crate::{ MILLISECS_PER_BLOCK, DAYS, YEAR };
use frame_support::dispatch::Vec;

impl<T: Config> Pallet<T> {
    pub fn get_opt_models() -> u32 {
        OptimalModels::<T>::get()
    }

    pub fn get_opt_models_percent() -> u128 {
        let opt_models = Self::get_opt_models();
        // Self::percent_div(opt_models as u128, MaxModels::<T>::get() as u128)
        Self::PERCENTAGE_FACTOR
    }

    pub fn get_opt_peers() -> u32 {
        // MAX: 4_294_967_295u32
        let opt_peers_per_model = OptimalPeersPerModel::<T>::get();
        let total_models = TotalModels::<T>::get();
        opt_peers_per_model * total_models
    }

    pub fn get_opt_peers_percent() -> u128 {
        let max_models = MaxModels::<T>::get();
        let max_models = MaxModelPeers::<T>::get();
        let opt_peers = Self::get_opt_peers();
        Self::percent_div(opt_peers as u128, (max_models * max_models) as u128)
    }

    pub fn get_slope() -> u128 {
        400
    }

    pub fn get_opt_apr(total_stake_balance: u128) -> u128 {
        9
    }

    pub fn get_apr() -> u128 {}

    pub fn get_lower_bound(usage: u128) -> u128 {
        let total_models = TotalModels::<T>::get();
        let main_lower_bound = InflationLowerBound::<T>::get();
        let epoch_lower_bound = Self::percent_mul(main_lower_bound, usage);
        epoch_lower_bound
    }

    pub fn get_upper_bound(usage: u128) -> u128 {
        let total_models = TotalModels::<T>::get();
        let main_upper_bound = InflationUpperBound::<T>::get();
        let epoch_upper_bound =
            Self::PERCENTAGE_FACTOR - (Self::PERCENTAGE_FACTOR - main_upper_bound) * usage;
        epoch_upper_bound
    }

    // Get decay of emissions as a variable
    pub fn get_decay(total_models: u32, block: u64, interval: u64) -> u128 {
        let opt_models = Self::get_opt_models();
        let max_models = MaxModels::<T>::get();
        let total_models = TotalModels::<T>::get();
        let usage = Self::percent_div(total_models as u128, max_models as u128);

        // --- Get upper bound
        let total_models = TotalModels::<T>::get();
        let main_upper_bound = InflationUpperBound::<T>::get();
        let epoch_upper_bound =
            Self::PERCENTAGE_FACTOR - (Self::PERCENTAGE_FACTOR - main_upper_bound) * usage;

        // -- Get lower bound
        let total_models = TotalModels::<T>::get();
        let main_lower_bound = InflationLowerBound::<T>::get();
        let epoch_lower_bound = Self::percent_mul(main_lower_bound, usage);

        // -- Get delta
        let bound_delta = epoch_upper_bound - epoch_lower_bound;

        // -- Get time decay
        let time_decay = TimeDecay::<T>::get();

        // -- Get last block model initialized
        let last_block_model_initialized = LastModelInitializationBlock::<T>::get();

        // -- Get end of time period
        let end_of_time_decay = last_block_model_initialized + time_decay;

        let mut time_elapsed_as_percentage = Self::PERCENTAGE_FACTOR;

        // --- Block should always be greater than the last_block_model_initialized

        // --- Get percentage of time elapsed since the last model was initialized in
        //     relation to the time decay between models initialization
        if block < end_of_time_decay {
            let time_elasped = block - last_block_model_initialized;
            time_elapsed_as_percentage = Self::percent_div(time_elasped, time_decay);
        }

        (time_elapsed_as_percentage * bound_delta) / Self::PERCENTAGE_FACTOR + epoch_lower_bound
    }

    pub fn get_epochs_emissions(peers_count: u32, total_stake_balance: u128) -> u128 {
        // --- Get APR
        let apr = Self::get_apr();

        // --- Calculate APR as an epoch to get total emissions
        let milliseconds_per_block: u64 = MILLISECS_PER_BLOCK;
        let consensus_blocks_interval: u64 = ConsensusBlocksInterval::<T>::get();
        let x = consensus_blocks_interval / YEAR;
        9
    }
}
