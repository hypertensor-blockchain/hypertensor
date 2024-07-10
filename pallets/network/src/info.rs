use super::*;
use frame_support::dispatch::Vec;

impl<T: Config> Pallet<T> {
  pub fn get_model_peers(
    model_id: u32,
  ) -> Vec<ModelPeer<T::AccountId>> {
    if !ModelsData::<T>::contains_key(model_id.clone()) {
      return Vec::new();
    }

    let mut model_peers: Vec<ModelPeer<T::AccountId>> = Vec::new();

    for model_peer in ModelPeersData::<T>::iter_prefix_values(model_id.clone()) {
      model_peers.push(model_peer);
    }
    model_peers
  }

  pub fn get_model_peers_included(
    model_id: u32,
  ) -> Vec<ModelPeer<T::AccountId>> {
    if !ModelsData::<T>::contains_key(model_id.clone()) {
      return Vec::new();
    }

    let block: u64 = Self::get_current_block_as_u64();
    let interval: u64 = ConsensusBlocksInterval::<T>::get();
    let min_required_epochs: u64 = MinRequiredPeerConsensusInclusionEpochs::<T>::get();

    let mut model_peers: Vec<ModelPeer<T::AccountId>> = Vec::new();

    for model_peer in ModelPeersData::<T>::iter_prefix_values(model_id.clone()) {
      let account_id: T::AccountId = model_peer.clone().account_id;

      let account_eligible: bool = Self::is_account_eligible(account_id);

      if !account_eligible {
        continue
      }

      let initialized: u64 = model_peer.clone().initialized;

      let do_include: bool = block >= Self::get_eligible_epoch_block(
        interval, 
        initialized, 
        min_required_epochs
      );

      if !do_include {
        continue
      }

      model_peers.push(model_peer);
    }
    model_peers
  }

  pub fn get_model_peers_submittable(
    model_id: u32,
  ) -> Vec<ModelPeer<T::AccountId>> {
    if !ModelsData::<T>::contains_key(model_id.clone()) {
      return Vec::new();
    }

    let block: u64 = Self::get_current_block_as_u64();
    let interval: u64 = ConsensusBlocksInterval::<T>::get();
    let min_required_epochs: u64 = MinRequiredPeerConsensusSubmitEpochs::<T>::get();

    let mut model_peers: Vec<ModelPeer<T::AccountId>> = Vec::new();

    for model_peer in ModelPeersData::<T>::iter_prefix_values(model_id.clone()) {
      let account_id: T::AccountId = model_peer.clone().account_id;

      let account_eligible: bool = Self::is_account_eligible(account_id);

      if !account_eligible {
        continue
      }

      let initialized: u64 = model_peer.clone().initialized;

      let do_include: bool = block >= Self::get_eligible_epoch_block(
        interval, 
        initialized, 
        min_required_epochs
      );

      if !do_include {
        continue
      }

      model_peers.push(model_peer);
    }
    model_peers
  }

  pub fn get_model_peers_model_unconfirmed_count(
    model_id: u32,
  ) -> u32 {
    if !ModelsData::<T>::contains_key(model_id.clone()) {
      return 0;
    }

    let unconfirmed_count = ModelConsensusEpochUnconfirmedCount::<T>::get(model_id.clone());

    unconfirmed_count
  }
}