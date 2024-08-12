// // Copyright (C) Hypertensor.
// // SPDX-License-Identifier: Apache-2.0

// // Licensed under the Apache License, Version 2.0 (the "License");
// // you may not use this file except in compliance with the License.
// // You may obtain a copy of the License at
// //
// // 	http://www.apache.org/licenses/LICENSE-2.0
// //
// // Unless required by applicable law or agreed to in writing, software
// // distributed under the License is distributed on an "AS IS" BASIS,
// // WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// // See the License for the specific language governing permissions and
// // limitations under the License.

// use super::*;
// use sp_runtime::traits::TrailingZeroInput;

// impl<T: Config> Pallet<T> {
//   pub fn try_propose_replace_subnet_node_with_queued(
//     subnet_id: u32,
//     peer_ids: Vec<PeerId>,
//     queued_peer_ids: Vec<PeerId>
//   ) -> DispatchResult {
//     Ok(())
//   }

//   pub fn try_replace_subnet_node_with_queued(subnet_id: u32) {
//     let peer_replacement_params: Vec<NodeReplaceConsensusParams> = match SubnetNodeQueueReplacementConsensus::<T>::try_get(subnet_id.clone()) {
//       Ok(params) => params,
//       Err(()) => Vec::new(),
//     };

//     if peer_replacement_params.len() == 0 {
//       return
//     }

//     let epoch_length: u64 = EpochLength::<T>::get();
//     let block: u64 = Self::get_current_block_as_u64();
//     let min_required_peer_consensus_submit_epochs: u64 = MinRequiredNodeConsensusSubmitEpochs::<T>::get();

//     let total_submittable_subnet_nodes: u32 = Self::get_total_eligible_subnet_nodes_count(
//       subnet_id.clone(),
//       block,
//       epoch_length,
//       min_required_peer_consensus_submit_epochs
//     );

//     let required_votes: u32 = Self::percent_mul(total_submittable_subnet_nodes as u128, NodeRemovalThreshold::<T>::get()) as u32;

//     for replacement in peer_replacement_params.iter() {
//       if replacement.votes > required_votes {
//         // let peer_id = peer_ids.get(replacement.index as usize).cloned().unwrap();
//         // let queued_peer_id = queued_peer_ids.get(replacement.index as usize).cloned().unwrap();

//         // let new_peer_id = Self::try_replace_subnet_node(subnet_id.clone(), peer_id, queued_peer_id)?;

//         // if let Err(e) = Self::deposit_event(Event::NodeReplaced(
//         //   subnet_id,
//         //   peer_id,
//         //   queued_peer_id,
//         //   new_peer_id,
//         // )) {
//         //   e.report();
//         // }

//         // break;
//       }
//     }

//   }
// }
