use super::*;
use frame_support::dispatch::Vec;

impl<T: Config> Pallet<T> {
  pub fn get_subnet_nodes(
    subnet_id: u32,
  ) -> Vec<SubnetNode<T::AccountId>> {
    if !SubnetsData::<T>::contains_key(subnet_id.clone()) {
      return Vec::new();
    }

    let mut subnet_nodes: Vec<SubnetNode<T::AccountId>> = Vec::new();

    for subnet_node in SubnetNodesData::<T>::iter_prefix_values(subnet_id.clone()) {
      subnet_nodes.push(subnet_node);
    }
    subnet_nodes
  }

  // pub fn get_subnet_nodes_included(
  //   subnet_id: u32,
  // ) -> Vec<SubnetNode<T::AccountId>> {
  //   if !SubnetsData::<T>::contains_key(subnet_id.clone()) {
  //     return Vec::new();
  //   }

  //   let block: u64 = Self::get_current_block_as_u64();
  //   let epoch_length: u64 = T::EpochLength::get();
  //   let min_required_epochs: u64 = MinRequiredNodeConsensusInclusionEpochs::<T>::get();

  //   let mut subnet_nodes: Vec<SubnetNode<T::AccountId>> = Vec::new();

  //   for subnet_node in SubnetNodesData::<T>::iter_prefix_values(subnet_id.clone()) {
  //     let account_id: T::AccountId = subnet_node.clone().account_id;

  //     let account_eligible: bool = Self::is_account_eligible(account_id);

  //     if !account_eligible {
  //       continue
  //     }

  //     let initialized: u64 = subnet_node.clone().initialized;

  //     let do_include: bool = block >= Self::get_eligible_epoch_block(
  //       epoch_length, 
  //       initialized, 
  //       min_required_epochs
  //     );

  //     if !do_include {
  //       continue
  //     }

  //     subnet_nodes.push(subnet_node);
  //   }
  //   subnet_nodes
  // }

  pub fn get_subnet_nodes_included(
    subnet_id: u32,
  ) -> Vec<SubnetNode<T::AccountId>> {
    log::info!("get_subnet_nodes_included");
    log::error!("get_subnet_nodes_included");

    if !SubnetsData::<T>::contains_key(subnet_id.clone()) {
      return Vec::new();
    }

    let mut subnet_nodes: Vec<SubnetNode<T::AccountId>> = Vec::new();

    let node_sets: BTreeMap<T::AccountId, u64> = SubnetNodesClasses::<T>::get(subnet_id.clone(), SubnetNodeClass::Included);

    for subnet_node in SubnetNodesData::<T>::iter_prefix_values(subnet_id.clone()) {
      let account_id: T::AccountId = subnet_node.clone().account_id;
      log::error!("get_subnet_nodes_included account_id {:?}", account_id);

      let account_eligible: bool = Self::is_account_eligible(account_id.clone());
      log::error!("get_subnet_nodes_included account_eligible {:?}", account_eligible);

      if !account_eligible {
        continue
      }

      let is_included = node_sets.get(&account_id);

      if let Some(is_included) = is_included {
        subnet_nodes.push(subnet_node);
      }
    }
    subnet_nodes
  }

  // pub fn get_subnet_nodes_submittable(
  //   subnet_id: u32,
  // ) -> Vec<SubnetNode<T::AccountId>> {
  //   if !SubnetsData::<T>::contains_key(subnet_id.clone()) {
  //     return Vec::new();
  //   }

  //   // let node_sets: BTreeMap<T::AccountId, u64> = SubnetNodesClasses::<T>::get(subnet_id.clone(), SubnetNodeClass::Submittable);

  //   let subnet_nodes: Vec<T::AccountId> = SubnetNodesClasses::<T>::get(subnet_id.clone(), SubnetNodeClass::Submittable).iter()
  //     .map(|x| { 
  //       *x.0
  //      } )
  //     .collect();

  //   subnet_nodes
  // }

  pub fn get_subnet_nodes_submittable(
    subnet_id: u32,
  ) -> Vec<SubnetNode<T::AccountId>> {
    if !SubnetsData::<T>::contains_key(subnet_id.clone()) {
      return Vec::new();
    }

    let mut subnet_nodes: Vec<SubnetNode<T::AccountId>> = Vec::new();

    let node_sets: BTreeMap<T::AccountId, u64> = SubnetNodesClasses::<T>::get(subnet_id.clone(), SubnetNodeClass::Submittable);

    for subnet_node in SubnetNodesData::<T>::iter_prefix_values(subnet_id.clone()) {
      let account_id: T::AccountId = subnet_node.clone().account_id;

      let account_eligible: bool = Self::is_account_eligible(account_id.clone());

      if !account_eligible {
        continue
      }

      let is_submittable = node_sets.get(&account_id);

      if let Some(is_submittable) = is_submittable {
        subnet_nodes.push(subnet_node);
      }
    }
    subnet_nodes
  }

  // pub fn get_subnet_nodes_submittable(
  //   subnet_id: u32,
  // ) -> Vec<SubnetNode<T::AccountId>> {
  //   if !SubnetsData::<T>::contains_key(subnet_id.clone()) {
  //     return Vec::new();
  //   }

  //   let block: u64 = Self::get_current_block_as_u64();
  //   let epoch_length: u64 = T::EpochLength::get();
  //   let min_required_epochs: u64 = MinRequiredNodeConsensusSubmitEpochs::<T>::get();

  //   let mut subnet_nodes: Vec<SubnetNode<T::AccountId>> = Vec::new();

  //   for subnet_node in SubnetNodesData::<T>::iter_prefix_values(subnet_id.clone()) {
  //     let account_id: T::AccountId = subnet_node.clone().account_id;

  //     let account_eligible: bool = Self::is_account_eligible(account_id);

  //     if !account_eligible {
  //       continue
  //     }

  //     let initialized: u64 = subnet_node.clone().initialized;

  //     let do_include: bool = block >= Self::get_eligible_epoch_block(
  //       epoch_length, 
  //       initialized, 
  //       min_required_epochs
  //     );

  //     if !do_include {
  //       continue
  //     }

  //     subnet_nodes.push(subnet_node);
  //   }
  //   subnet_nodes
  // }

  pub fn get_subnet_nodes_model_unconfirmed_count(
    subnet_id: u32,
  ) -> u32 {
    if !SubnetsData::<T>::contains_key(subnet_id.clone()) {
      return 0;
    }

    let unconfirmed_count = SubnetConsensusEpochUnconfirmedCount::<T>::get(subnet_id.clone());

    unconfirmed_count
  }

  // id is consensus ID
  pub fn get_consensus_data(
    subnet_id: u32,
    epoch: u32
  ) -> Option<RewardsData<T::AccountId>> {
    let data = SubnetRewardsSubmission::<T>::get(subnet_id, epoch);
    Some(data?)
  }

  // id is proposal ID
  pub fn get_accountant_data(
    subnet_id: u32,
    id: u32
  ) -> Option<AccountantDataParams<T::AccountId>> {
    let data = AccountantData::<T>::get(subnet_id, id);
    Some(data)
  }

  pub fn get_minimum_subnet_nodes(subnet_id: u32, memory_mb: u128) -> u32 {
    Self::get_min_subnet_nodes(BaseSubnetNodeMemoryMB::<T>::get(), memory_mb)
  }
}