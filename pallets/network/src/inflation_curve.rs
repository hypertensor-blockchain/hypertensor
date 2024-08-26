// #![feature(isqrt)]

use super::*;
use frame_support::dispatch::Vec;

impl<T: Config> Pallet<T> {
  // pub const SECONDS_PER_YEAR: u64 = 31556926;

  pub fn get_opt_models() -> u32 {
    OptimalSubnets::<T>::get()
  }

  pub fn get_opt_models_percent() -> u128 {
    let opt_models = Self::get_opt_models();
    // Self::percent_div(opt_models as u128, MaxSubnets::<T>::get() as u128)
    Self::PERCENTAGE_FACTOR
  }

  pub fn get_opt_peers() -> u32 {
    // MAX: 4_294_967_295u32
    let opt_peers_per_model = OptimalNodesPerSubnet::<T>::get();
    let total_models = TotalSubnets::<T>::get();
    opt_peers_per_model * total_models
  }

  pub fn get_opt_peers_percent() -> u128 {
    let max_models = MaxSubnets::<T>::get();
    let max_models = MaxSubnetNodes::<T>::get();
    let opt_peers = Self::get_opt_peers();
    Self::percent_div(opt_peers as u128, (max_models * max_models) as u128)
  }

  pub fn get_slope() -> u128 {
    400
  }

  pub fn get_opt_apr(total_stake_balance: u128) -> u128 {
    9
  }

  pub fn get_lower_bound(usage: u128) -> u128 {
    let total_models = TotalSubnets::<T>::get();
    let inflation_lower_bound = InflationLowerBound::<T>::get();
    let epoch_lower_bound = Self::percent_mul(inflation_lower_bound, usage);
    epoch_lower_bound
  }

  pub fn get_upper_bound(usage: u128) -> u128 {
    let total_models = TotalSubnets::<T>::get();
    let inflation_upper_bound = InflationUpperBound::<T>::get();
    let epoch_upper_bound = Self::PERCENTAGE_FACTOR - (Self::PERCENTAGE_FACTOR - inflation_upper_bound) * usage;
    epoch_upper_bound
  }

  // pub fn get_decay(total_models: u32, block: u64, epoch_length: u64) -> u128 {
  //   let max_models = MaxSubnets::<T>::get();
  //   let total_models = TotalSubnets::<T>::get();
  //   let inflation_upper_bound = InflationUpperBound::<T>::get();
  //   let inflation_lower_bound = InflationLowerBound::<T>::get();

  //   // --- Get usage
  //   let usage = Self::percent_div(total_models as u128, max_models as u128);
  //   // --- Get delta
  //   let bound_delta = epoch_upper_bound - epoch_lower_bound;

  //   // --- Get time decay
  //   let time_decay = TimeDecay::<T>::get();

  //   // --- Get last block subnet initialized
  //   let last_block_model_initialized = LastSubnetInitializedBlock::<T>::get();

  //   // --- Get end of time period
  //   let end_of_time_decay = last_block_model_initialized + time_decay;

  //   let mut time_elapsed_as_percentage = Self::PERCENTAGE_FACTOR;

  //   // --- Block should always be greater than the last_block_model_initialized

  //   // --- Get percentage of time elapsed since the last subnet was initialized in 
  //   //     relation to the time decay between subnets initialization
  //   if block < end_of_time_decay {
  //     let time_elasped = block - last_block_model_initialized;
  //     time_elapsed_as_percentage = Self::percent_div(time_elasped, time_decay)
  //   }

  //   $A13*N13*100+L13

  //   time_elapsed_as_percentage * bound_delta + 
    
  // }

  // Get decay of emissions as a variable
  // Include total live subnets, not just subnets that pass consensus to incentivize nodes to remove dead subnets
  pub fn get_decay(block: u64) -> f64 {
    let opt_models = Self::get_opt_models();
    let max_models = MaxSubnets::<T>::get();
    let total_models = TotalSubnets::<T>::get();
    log::error!("total_models {:?}", total_models);

    let usage = Self::percent_div(total_models as u128, max_models as u128);
    log::error!("usage {:?}", usage);

    // --- Get upper bound
    let inflation_upper_bound = InflationUpperBound::<T>::get();
    log::error!("inflation_upper_bound {:?}", inflation_upper_bound);

    let epoch_upper_bound = Self::PERCENTAGE_FACTOR - (Self::PERCENTAGE_FACTOR - inflation_upper_bound) * usage;
    log::error!("epoch_upper_bound {:?}", epoch_upper_bound);

    // --- Get lower bound
    let inflation_lower_bound = InflationLowerBound::<T>::get();
    log::error!("inflation_lower_bound {:?}", inflation_lower_bound);

    let epoch_lower_bound = Self::percent_mul(inflation_lower_bound, usage);
    log::error!("epoch_lower_bound {:?}", epoch_lower_bound);

    // --- Get delta
    let bound_delta = epoch_upper_bound - epoch_lower_bound;
    log::error!("bound_delta {:?}", bound_delta);

    // --- Get time decay
    let time_decay = TimeDecay::<T>::get();
    log::error!("time_decay {:?}", time_decay);

    // --- Get last block subnet initialized
    let last_block_model_initialized = LastSubnetInitializedBlock::<T>::get();
    log::error!("last_block_model_initialized {:?}", last_block_model_initialized);

    // --- Get end of time period
    let end_of_time_decay = last_block_model_initialized + time_decay;
    log::error!("end_of_time_decay {:?}", end_of_time_decay);

    let mut time_elapsed_as_percentage = 0;

    // --- Block should always be greater than the last_block_model_initialized

    // --- Get percentage of time elapsed since the last subnet was initialized in 
    //     relation to the time decay between subnets initialization
    if block < end_of_time_decay {
      let time_elasped = block - last_block_model_initialized;
      log::error!("time_elasped {:?}", time_elasped);
      time_elapsed_as_percentage = Self::PERCENTAGE_FACTOR - Self::percent_div(time_elasped as u128, time_decay as u128);
    }
    log::error!("time_elapsed_as_percentage {:?}", time_elapsed_as_percentage);

    // time_elapsed_as_percentage * bound_delta + epoch_lower_bound
    // Self::percent_mul(time_elapsed_as_percentage, bound_delta) + epoch_lower_bound
    (Self::percent_mul(time_elapsed_as_percentage, bound_delta) + epoch_lower_bound) as f64 / Self::PERCENTAGE_FACTOR as f64
  }

  pub fn get_epoch_rewards(block: u64, total_balance: u128) -> u128 {
    log::error!("block {:?}", block);
    log::error!("total_balance {:?}", total_balance);
    // --- Get APR
    let apr: f64 = Self::get_apr(block, total_balance);
    log::error!("apr {:?}", apr);

    // let apr_u128 = Self::percent_mul(apr as u128, Self::PERCENTAGE_FACTOR);
    // log::error!("apr_u128 {:?}", apr_u128);

    log::error!("apr * total_balance {:?}", apr * total_balance as f64);
    log::error!("apr * total_balance {:?}", (apr * total_balance as f64) as u128);

    // let emissions = Self::gwei_into_eth((apr * total_balance as f64) as u128);
    // log::error!("emissions {:?}", emissions);

    // emissions
    (apr * total_balance as f64) as u128

    // Self::gwei_into_eth((apr as u128 * total_balance) as u128)
  }

  // TODO: Make updateable as storage element
  // const BASE_REWARD_FACTOR: f64 = 1280000.0;
  const BASE_REWARD_FACTOR: f64 = 48.0;

  pub fn get_apr(block: u64, total_balance: u128) -> f64 {
    if total_balance == 0 || block == 0 {
      return 0.0;
    }

    // --- Get seconds per year
    let seconds_per_year: f64 = (T::SecsPerBlock::get() * T::Year::get()) as f64;
    log::error!("seconds_per_year {:?}", seconds_per_year);

    // --- Get blocks per epoch
    let epoch_length: u64 = T::EpochLength::get();
    log::error!("epoch_length {:?}", epoch_length);

    if block < epoch_length {
      return 0.0;
    }

    // --- Get seconds per epoch
    let seconds_per_epoch: f64 = (T::SecsPerBlock::get() * epoch_length) as f64;
    log::error!("seconds_per_epoch {:?}", seconds_per_epoch);

    // --- Get epochs per year
    let epochs_per_year: f64 = seconds_per_year / seconds_per_epoch;
    log::error!("epochs_per_year {:?}", epochs_per_year);

    // --- Get decay
    let decay: f64 = Self::get_decay(block) as f64;
    log::error!("decay {:?}", decay);

    let max_peer = MaxSubnetNodes::<T>::get();
    let min_stake = MinStakeBalance::<T>::get();
    let max_stake: f64 = max_peer as f64 * min_stake as f64;
    let stake_usage: f64 = total_balance as f64 / max_stake;
    let amount_staked_as_gwei_as_f64: f64 = Self::eth_into_gwei(total_balance) as f64;
    log::error!("stake_usage {:?}", stake_usage);

    // --- Calculate APR
    let apr: f64 = libm::exp(
      Self::BASE_REWARD_FACTOR * 31622.0 / 
      libm::pow(amount_staked_as_gwei_as_f64 + amount_staked_as_gwei_as_f64 * stake_usage, 0.5)
    ) - 1.0;

    // let apr = libm::exp(
    //   seconds_per_year as f64 / seconds_per_epoch as f64 * 
    //   Self::BASE_REWARD_FACTOR / 31622.0 / 
    //   libm::pow(Self::eth_into_gwei(total_balance) as f64, 0.5)
    // ) - 1.0;
    log::error!("apr {:?}", apr);

    // --- Adjust for decay
    let decayed_apr: f64 = apr * decay;

    // --- Adjust for epochs
    let epoch_apr: f64 = decayed_apr * 100.0 / epochs_per_year;
    log::error!("epoch_apr {:?}", epoch_apr);

    // --- Return final APR value
    epoch_apr
  }
}