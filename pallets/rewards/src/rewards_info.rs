use super::*;
use frame_support::pallet_prelude::{Decode, Encode};
use frame_support::storage::IterableStorageMap;
extern crate alloc;
use alloc::vec::Vec;
use codec::Compact;

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct RewardsParams<T: Config> {
  subsidy_halving_interval: Compact<u16>,
}

impl<T: Config> Pallet<T> {
  pub fn get_subsidy_halving_interval(netuid: u16) -> u64 {
    return 0;
  }
}