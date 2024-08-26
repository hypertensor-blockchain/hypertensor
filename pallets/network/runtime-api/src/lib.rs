#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::dispatch::Vec;
use pallet_network::{AccountantDataParams, RewardsData};

sp_api::decl_runtime_apis! {
  pub trait NetworkRuntimeApi {
    fn get_subnet_nodes(model_id: u32) -> Vec<u8>;
    fn get_subnet_nodes_included(model_id: u32) -> Vec<u8>;
    fn get_subnet_nodes_submittable(model_id: u32) -> Vec<u8>;
    fn get_subnet_nodes_model_unconfirmed_count(model_id: u32) -> u32;
    fn get_consensus_data(model_id: u32, epoch: u32) -> Vec<u8>;
    fn get_accountant_data(model_id: u32, id: u32) -> Vec<u8>;
  }
}