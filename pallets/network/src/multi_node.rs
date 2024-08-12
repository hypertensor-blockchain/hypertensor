/// Logic for scaling subnet nodes to millions of nodes
/// Uses:
///   1. One subnet peer slot can be opened to multiple nodes that subnets verify
///
/// The `owner` controls the multinode
///   - Who can come in and out of the multinode
///
/// It's up to the nodes in a multi-node network to verify nodes are truthful 
/// Each MultiNode is referenced as an AccountId is responsible for all nodes in the multi-node network
/// The owner of the multi-node network can add or remove nodes

use super::*;
use frame_support::dispatch::Vec;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_io::hashing::blake2_256;
use sp_runtime::traits::TrailingZeroInput;

impl<T: Config> Pallet<T> {
  pub fn add_multi_node_subnet_node(multi_node_id: T::AccountId) -> DispatchResult {
    // --- Ensure owner

    // --- Add subnet peer
    Ok(())
  }

  pub fn remove_multi_node_subnet_node(multi_node_id: T::AccountId) -> DispatchResult {
    // --- Ensure owner

    // --- Remove subnet peer
    Ok(())
  }

  /// Data is used in subnets to verify the node data is truthful
  pub fn add_node(
    multi_node_id: T::AccountId, 
    account_id: T::AccountId,
    data: MultiNodeParams
  ) -> DispatchResult {
    Ok(())
  }

  pub fn remove_node(multi_node_id: T::AccountId, account_id: T::AccountId) -> DispatchResult {
    Ok(())
  }

  pub fn generate_multi_node_account(owner: T::AccountId) -> DispatchResult {
    let multi_node_count = MultiNodeCount::<T>::get();
    MultiNodeCount::<T>::put(multi_node_count + 1);

    let multi_node_account_id: T::AccountId = Self::pure_account(
      &owner,
      multi_node_count,
      None,
    );
    // --- Ensure unique
    log::error!("generate_multi_node_account multi_node_account_id: {:?}", multi_node_account_id);
    
    Ok(())
  }

  pub fn pure_account(
		who: &T::AccountId,
		index: u16,
		maybe_when: Option<(BlockNumberFor<T>, u32)>,
	) -> T::AccountId {
		let (height, ext_index) = maybe_when.unwrap_or_else(|| {
			(
				system::Pallet::<T>::block_number(),
				system::Pallet::<T>::extrinsic_index().unwrap_or_default(),
			)
		});
		let entropy = (b"modlpy/proxy____", who, height, ext_index, 0, index)
			.using_encoded(blake2_256);
		Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
			.expect("infinite length input; no invalid inputs for type; qed")
	}
}