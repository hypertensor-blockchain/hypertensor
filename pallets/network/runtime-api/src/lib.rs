#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::dispatch::Vec;

sp_api::decl_runtime_apis! {
  pub trait NetworkRuntimeApi {
    fn get_model_peers(model_id: u32) -> Vec<u8>;
    fn get_model_peers_included(model_id: u32) -> Vec<u8>;
    fn get_model_peers_submittable(model_id: u32) -> Vec<u8>;
    fn get_model_peers_model_unconfirmed_count(model_id: u32) -> u32;    
  }
}