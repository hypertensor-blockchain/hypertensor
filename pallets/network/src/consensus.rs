use super::*;
use frame_support::dispatch::Vec;
use sp_runtime::Saturating;
use sp_std::collections::btree_map::BTreeMap;

impl<T: Config> Pallet<T> {
  pub fn form_peer_consensus(block: u64) {
    let maximum_outlier_delta_percent: u8 = MaximumOutlierDeltaPercent::<T>::get();
    let subnet_node_consensus_submit_percent_requirement: u128 = SubnetNodeConsensusSubmitPercentRequirement::<T>::get();
    let max_model_consensus_epoch_errors: u32 = MaxSubnetConsensusEpochsErrors::<T>::get();
    let epoch_length: u64 = EpochLength::<T>::get();
    let min_required_model_consensus_submit_epochs = MinRequiredSubnetConsensusSubmitEpochs::<T>::get();
    let model_consensus_unconfirmed_threshold = SubnetConsensusUnconfirmedThreshold::<T>::get();
    let subnet_node_seq_consensus_sent_threshold = SubnetNodeConsecutiveConsensusSentThreshold::<T>::get();
    let min_required_peer_consensus_submit_epochs: u64 = MinRequiredNodeConsensusSubmitEpochs::<T>::get();
    let model_consecutive_epochs_threshold = SubnetConsecutiveEpochsThreshold::<T>::get();
    let peer_against_consensus_removal_threshold = NodeAgainstConsensusRemovalThreshold::<T>::get();
    let max_model_consensus_unconfirmed_seq_epochs = MaxSubnetConsensusUnconfirmedConsecutiveEpochs::<T>::get();
    let peer_removal_threshold = NodeRemovalThreshold::<T>::get();

    // Iter each subnet and check consensus data
    // if any existing subnets have no submissions
    for (subnet_id, data) in SubnetsData::<T>::iter() {
      let model_initialized = data.initialized;

      // If subnet can't yet form consensus, continue
      //
      // We use this here instead of when initializing the subnet or peer in order to keep the required time
      // universal in the case subnets or peers are added before an update to the EpochLength
      //
      // This also should give time for peer to come in
      // Subnets should already be hosted before being voted in, therefor by the time a subnet can enter into
      // consensus, peers should already be initialized
      //
      // While this is already checked when submitting consensus data, we recheck again instead of checking
      // the count of SubnetTotalConsensusSubmits to ensure if a mode can have data submitted that data was 
      // submitted.
      //
      // e.g. Can't form consensus if the following parameters
      //			• subnet initialized		0
      //			• epoch_length 			  20
      //			• epochs							10
      //			• current block 			200
      //	eligible block is 200
      // 	can't submit on 200, 201 based on is_in_consensus_steps()
      //	can submit between 202-219
      //	200 is less than or equal to 200, don't form consensus and continue
      // 	Note: Consensus submissions happen after the eligible block so if it equals
      //				the eligible block we must wait 1 more epoch for the data to be submitted
      //
      // e.g. Can form consensus if the following parameters
      //			• subnet initialized		0
      //			• epoch_length 			  20
      //			• epochs							10
      //			• current block 			220
      //	eligible block is 200
      // 	can't submit on 200, 201 based on is_in_consensus_steps()
      //	can submit between 202-219
      //	220 is not less than or equal to 200, form consensus
      //
      if block <= Self::get_eligible_epoch_block(
        epoch_length, 
        model_initialized, 
        min_required_model_consensus_submit_epochs
      ) {
        log::info!("Subnet ID {:?} not eligible for consensus yet", subnet_id.clone());
        continue
      }

      // * 1. Ensure subnet consensus errors aren't at the max
      //      • If this is the case, it should be called to be removed by an account
      // * 2. Check enough peers submitted
      // * 3. Check if peers unconfirmed the consensus data before finally forming consensus
      //
      //   If the previous steps are successful
      //
      // * 4. Begin calculating consensus data

      //
      // Get the reoccuring count of how many epochs a subnet hasn't been able to form consensus properly
      // This happens when not enough peers have submitted data during a confirmed epoch
      // Or there have been too many sequential errors on the subnet using `unconfirm` consensus data
      //
      // Simply continue here instead of removing subnet
      // This subnet should be removed by an account
      // Anyone can manually remove subnet if certain parameters are met
      let model_consensus_epoch_errors: u32 = SubnetConsensusEpochsErrors::<T>::get(subnet_id.clone());
      if model_consensus_epoch_errors > max_model_consensus_epoch_errors {
        log::info!("Subnet ID {:?} surpassed maximum subnet consensus epoch errors", subnet_id.clone());
        Self::reset_model_consensus_data_and_results(subnet_id.clone());
        continue
      }

      // *
      // At this point, a subnet should have consensus data and we check if its eligible
      //
     
      // Possibly include setting SubnetConsensusEpochsErrors back to zero if consensus was submitted
      // on a subnet. This would only have a subnet removed if zero consensus epochs were consecutive
       
      // How many peers submitted consensus or unconfirmed consensus on this subnet
      let subnet_node_submits = SubnetTotalConsensusSubmits::<T>::get(subnet_id.clone());

      // Count of eligible to have submitted consensus data subnet peers on current epochs data
      let total_subnet_nodes: u32 = Self::get_prev_epoch_total_eligible_subnet_nodes_count(
        subnet_id.clone(),
        block,
        epoch_length,
        min_required_peer_consensus_submit_epochs
      );

      // Ensure enough peers submitted consensus or unconfirmed as a sum
      //
      // If not enough peers submitted consensus or unconfirmed we can assume there is an issue with the subnet
      // or the subnet didn't initialize enough peers to generate rewards
      // Therefor we continue instead of increasing each accounts penalties count
      if Self::percent_div(subnet_node_submits as u128, total_subnet_nodes as u128) < subnet_node_consensus_submit_percent_requirement {
        // If enough SubnetConsensusEpochsErrors increment, the subnet can be removed
        log::info!("Not enough submissions on subnet ID {:?}", subnet_id.clone());
        SubnetConsensusEpochsErrors::<T>::mutate(subnet_id.clone(), |n: &mut u32| *n += 1);
        SubnetConsecutiveSuccessfulEpochs::<T>::insert(subnet_id.clone(), 0);
        Self::reset_model_consensus_data_and_results(subnet_id.clone());

        // if model_consensus_epoch_errors >= 0 && 0 % 0 == 0 {

        // }
        continue
      }

      //
      // Unconfirm check
      //
      // We check for unconfirm first before submit as a backstop for subnets being down
      //
      // Get legitimacy of consensus data
      //  1. See if peers agree the consensus data is legitimate
      //     • Previously submitted peers can unconfirm the consensus data
      //     • This should happen is a subnet goes into an error state before all peers have submitted consensus data

      // Check the status of the subnet and if any peers submitted `unconfirm` status for this epoch
      // If enough peers submitted `unconfirm` status for the subnet on this epoch, skip consensus
      // Get count of how many subnet peers submitted `unconfirm` status of the subnet
      //
      // Ensure enough peers have confirmed the consensus data is legitimate
      // This does not get counted towards SubnetConsensusEpochsErrors if under the max sequential unconfirmed epochs
      let unconfirmed_count: u32 = SubnetConsensusEpochUnconfirmedCount::<T>::get(subnet_id.clone());
      let model_consensus_unconfirmed_seq_epochs_count = SubnetConsensusUnconfirmedConsecutiveEpochsCount::<T>::get(subnet_id.clone());
      if Self::percent_div(unconfirmed_count as u128, subnet_node_submits as u128) >= model_consensus_unconfirmed_threshold {
        log::info!("Subnet ID {:?} consensus data unconfirmed by {:?} subnet peers", subnet_id.clone(), unconfirmed_count);
        // Increase the count of unconfirmed epochs in a row
        // This resets every successful epoch
        SubnetConsensusUnconfirmedConsecutiveEpochsCount::<T>::mutate(subnet_id.clone(), |n: &mut u32| *n += 1);

        // Increase subnet consensus epochs errors if unconfirmed data too many epochs in a row
        if model_consensus_unconfirmed_seq_epochs_count + 1 > max_model_consensus_unconfirmed_seq_epochs {
          SubnetConsensusEpochsErrors::<T>::mutate(subnet_id.clone(), |n: &mut u32| *n += 1);
        }

        SubnetConsecutiveSuccessfulEpochs::<T>::insert(subnet_id.clone(), 0);
        Self::reset_model_consensus_data_and_results(subnet_id.clone());
        continue
      } else if model_consensus_unconfirmed_seq_epochs_count > 0 {
        // Reset sequence
        // We have now confirmed peers are submitting data successfully, whether data or unconfirming
        // Begin forming consensus...
        SubnetConsensusUnconfirmedConsecutiveEpochsCount::<T>::remove(subnet_id.clone());
      }

      //
      // Submit check
      //
      // At this point enough peer submitted either data or an unconfirm
      // but not enough to unconfirm
      // We now check if we can form consensus based on the submitted consensus data using the percent requirement
      let subnet_node_submit_submissions = SubnetConsensusEpochSubmitCount::<T>::get(subnet_id.clone());

      // Ensure enough peers submitted consensus data
      // If not enough peers submitted consensus we can assume there is an issue with the subnet
      // or the subnet didn't initialize enough peers to generate rewards
      // Therefor we continue instead of increasing each accounts penalties count
      if Self::percent_div(subnet_node_submit_submissions as u128, total_subnet_nodes as u128) < subnet_node_consensus_submit_percent_requirement {
        log::info!("Not enough consensus submissions for subnet ID {:?}", subnet_id.clone());
        SubnetConsecutiveSuccessfulEpochs::<T>::insert(subnet_id.clone(), 0);
        // If enough SubnetConsensusEpochsErrors increment, the subnet can be removed
        SubnetConsensusEpochsErrors::<T>::mutate(subnet_id.clone(), |n: &mut u32| *n += 1);

        Self::reset_model_consensus_data_and_results(subnet_id.clone());
        continue
      }

      // All checks are complete
      //
      // Check if we can less_one subnet errors based on consecutive successful epochs
      let model_consecutive_successful_epochs = SubnetConsecutiveSuccessfulEpochs::<T>::get(subnet_id.clone());
      if model_consecutive_successful_epochs >= model_consecutive_epochs_threshold && model_consecutive_successful_epochs % model_consecutive_epochs_threshold == 0 {
        log::info!("Subnet ID {:?} epoch successful, removing an error", subnet_id.clone());
        SubnetConsensusEpochsErrors::<T>::mutate(subnet_id.clone(), |n: &mut u32| n.saturating_dec());
      }

      // Begin forming consensus..
      //	
      // Iter each subnet peer's data included in consensus
      // Calculate and form consensus of all submitted data per account/peer

      let mut consensus_peer_count: u32 = 0;
      let mut against_consensus_peer_count: BTreeMap<T::AccountId, u32> = BTreeMap::new();

      for peer_consensus_result in SubnetNodeConsensusResults::<T>::iter_prefix_values(subnet_id.clone()) {
        let consensus_result_account_id: T::AccountId = peer_consensus_result.account_id;

        // Check subnet peer exists
        // Subnet peer may have exited the blockchain during the epoch
        //
        // Consensus data is initially checked that a subnet peer is able to be included in consensus
        // therefor we only check if they continue to exist, not if they should be in the data itself
        let subnet_node_exists: bool = SubnetNodesData::<T>::contains_key(subnet_id.clone(), consensus_result_account_id.clone());
        // If so, skip their consensus and remove their consensus data
        // This may be redundant with:
        //  • `can_remove_or_update_subnet_node()` function
        //  • `do_include` variable in `submit_consensus_data()`
        // but we check regardless
        if !subnet_node_exists {
          SubnetNodeConsensusResults::<T>::remove(subnet_id.clone(), consensus_result_account_id.clone());
          continue
        }

        let consensus_result_unsuccessful: u32 = peer_consensus_result.unsuccessful;
        let consensus_result_total_submits: u32 = peer_consensus_result.total_submits;

        // Percent of peers in consensus that a peer is no longer hosting subnets
        // This is count of peers that left the peer absent from consensus divided by the total consensus submissions
        let removal_consensus_percentage: u128 = Self::percent_div(
          consensus_result_unsuccessful as u128, 
          consensus_result_total_submits as u128
        );

        // In the following logic blocks we must either remove the peer from SubnetNodeConsensusResults
        // or generate a score for the peer for emissions logic to operate successfully

        // If a peer is deemed out of consensus through the NodeRemovalThreshold this takes 
        // care of the following =>
        // 		1. Removes peers that are deemed out of consensus by other peers that didn't submit them in
        //			 their consensus submission.
        // 		
        // 		2. Removing any peers that are potentially brute forcing peer storage but aren't actually hosting subnets.
        //			 peers are required to be included in consensus data after `x` epochs of being stored onchain before
        //       they can submit consensus data themselves.
        if removal_consensus_percentage > peer_removal_threshold {
          // Subnet peer is out of consensus
          //  1. Remove subnet peer
          //  2. Increment AccountPenaltyCount of accounts against this consensus
          let consensus_result_peer_id: PeerId = peer_consensus_result.peer_id;

          // Remove subnet peer storage and consensus data
          log::info!(
            "AccountId {:?} out of consensus through {:?}% in subnet ID {:?}", 
            consensus_result_account_id, 
            removal_consensus_percentage, 
            subnet_id
          );
          Self::do_remove_subnet_node(block, subnet_id.clone(), consensus_result_account_id.clone());

          let consensus_result_successful_consensus: Vec<T::AccountId> = peer_consensus_result.successful_consensus;

          // Increment penalties on peers who were against this consensus
          // These are peers that submitted consensus data
          // These peers submitted a success on the peer is hosting subnets
          // Although consensus deems they are not
          for dishonest_account_id in consensus_result_successful_consensus.iter() {
            if let Some(count) = against_consensus_peer_count.get_mut(&dishonest_account_id.clone()) {
              *count += 1;
            } else {
              against_consensus_peer_count.insert(dishonest_account_id.clone(), 1);
            }
            AccountPenaltyCount::<T>::mutate(dishonest_account_id.clone(), |n: &mut u32| *n += 1);
          }	
        } else {
          // Subnet peer is in consensus
          //  1. If subnet peer submitted consensus data then get score
          //     Else: remove from SubnetNodeConsensusResults so they don't receive rewards
          //  2. Increment AccountPenaltyCount of accounts against this consensus

          // The following logic is for 
          // 1. Nodes that are in consensus are being deemed hosting subnets
          //	
          // logic =>
          // 		1. We check if they submitted consensus or unconfirmed
          //
          // 		if the peer didn't submit consensus this means
          // 				1. Nodes that didn't submit consensus data but can are removed
          //				2. Nodes that don't meet MinRequiredNodeConsensusSubmitEpochs
          // 		For `2`, even if a peer can't submit data we need to know if they are hosting subnets before they are
          // 		eligible for rewards and submitting consensus data, and so they don't hold a place in storage while 
          //		other peers are trying to become peers. In either case we remove them from SubnetNodeConsensusResults 
          //		for this current epoch.
          //
          // 		2. We then get the average score for the peer based on the interquantile algorithm from all submitted scores
          //			 and update their `score` - this is used later when generating emissions
          //
          let peer_submitted: bool = NodeConsensusEpochSubmitted::<T>::get(subnet_id.clone(), consensus_result_account_id.clone());
          
          let peer_unconfirmed: bool = NodeConsensusEpochUnconfirmed::<T>::get(subnet_id.clone(), consensus_result_account_id.clone());

          // Nodes that are able to be included in consensus data but can't submit data will be removed in `else`
          // If the peer submitted data, we create their score.
          if peer_submitted || peer_unconfirmed {
            // Get all scores submitted on peer
            let consensus_result_scores: Vec<u128> = peer_consensus_result.scores;

            // Calculate peers score average
            let average_score: u128 = Self::get_average_score(consensus_result_scores);

            // Set SubnetNodeConsensusResults peers score as average score
            SubnetNodeConsensusResults::<T>::mutate(
              subnet_id.clone(),
              consensus_result_account_id.clone(),
              |params: &mut SubnetNodeConsensusResultsParams<T::AccountId>| {
                params.score = average_score;
              }
            );
          } else {
            // If peer didn't submit remove them from SubnetNodeConsensusResults
            // This includes peers that didn't submit or are ineligible to submit
            // We previously checked these peers to be in consensus already so removing them won't
            // impact them being a peer. They will simply not receive rewards if removed at this point
            SubnetNodeConsensusResults::<T>::remove(subnet_id.clone(), consensus_result_account_id.clone());
          }

          let consensus_result_unsuccessful_consensus: Vec<T::AccountId> = peer_consensus_result.unsuccessful_consensus;

          // Increment penalties on peers who were against consensus
          // These are peers that submitted consensus data
          // These peers left the `in-consensus peer` absent from their consensus submit
          for dishonest_account_id in consensus_result_unsuccessful_consensus.iter() {
            if let Some(count) = against_consensus_peer_count.get_mut(&dishonest_account_id.clone()) {
              *count += 1;
            } else {
              against_consensus_peer_count.insert(dishonest_account_id.clone(), 1);
            }
            AccountPenaltyCount::<T>::mutate(dishonest_account_id, |n: &mut u32| *n += 1);
          }
        }

        consensus_peer_count += 1;
      }

      // Calculate scores
      //
      // Go back over consensus data to update score
      //
      // Scores are now generated by this point
      // Go over scores and ensure submitters aren't being dishonest based on max delta
      // Penalize outliers that submitted scores with deltas greater than required vs. average score
      for peer_consensus_result in SubnetNodeConsensusResults::<T>::iter_prefix_values(subnet_id.clone()) {
        // We don't check `subnet_node_exists` again, it would have been removed in the previous code block

        let consensus_result_successful_consensus: Vec<T::AccountId> = peer_consensus_result.successful_consensus;
        let consensus_result_scores: Vec<u128> = peer_consensus_result.scores;
        let avg_score: u128 = peer_consensus_result.score;
        
        let delta: u128 = (avg_score as f64 * maximum_outlier_delta_percent as f64 / 100.0) as u128;

        let max_required_score: u128 = avg_score + delta;
        let min_required_score: u128 = avg_score - delta;

        let mut score_index: usize = 0;
        for score in consensus_result_scores.iter() {
          // if peer submitted score outside allowed delta
          if *score <= min_required_score || *score >= max_required_score {
            let account_id: &T::AccountId = &consensus_result_successful_consensus[score_index];

            if let Some(count) = against_consensus_peer_count.get_mut(&account_id.clone()) {
              *count += 1;
            } else {
              against_consensus_peer_count.insert(account_id.clone(), 1);
            }
            AccountPenaltyCount::<T>::mutate(account_id, |n: &mut u32| *n += 1);
          }
          score_index += 1;
        }
      }

      // Ensure submittable subnet peers submitted
      //
      // At this point, we have ran consensus data and any `unconfirm`'s were unsuccessful
      //
      // Any subnet peers out of consensus are now removed
      //
      // Iterate through all subnet peers
      // Check for peers that could have submitted consensus data but did not
      // If peer didn't submit consensus and is eligible to, increase consensus penalty
      // Instead of checking SubnetNodeConsensusResults we check NodeConsensusEpochSubmitted because
      // it hasn't been impacted during forming consensus
      for subnet_node in SubnetNodesData::<T>::iter_prefix_values(subnet_id.clone()) {
        let peer_initialized: u64 = subnet_node.initialized;

        // Ensure account could have submitted consensus data on the allotted submission blocks
        // If not, increase AccountPenaltyCount
        //
        // e.g. Couldn't submit consensus data if the following parameters
        //			• peer initialized		0
        //			• epoch_length 						20
        //			• epochs							10
        //			• current block 			200
        //	• eligible block is 200
        // 	• can't submit on 200, 201 based on is_in_consensus_steps()
        //	• can submit between 202-219
        //	• 200 is not greater than 200, couldn't submit data
        //
        // e.g. Could have submitted consensus data if the following parameters
        //			• peer initialized		0
        //			• epoch_length 						20
        //			• epochs							10
        //			• current block 			220
        //	• eligible block is 200
        // 	• can't submit on 200, 201 based on is_in_consensus_steps()
        //	• can submit between 202-219
        //	• 220 is greater than 200, could have submitted data
        //
        let can_submit: bool = block > Self::get_eligible_epoch_block(
          epoch_length, 
          peer_initialized, 
          min_required_peer_consensus_submit_epochs
        );

        if !can_submit {
          continue
        }

        let account_id: T::AccountId = subnet_node.account_id;

        // If subnet peer has been against consensus and breaches threshold
        // Then remove that subnet peer
        //
        // Consensus expects all peers to be in full consensus and if not an `unconfirm` should
        // be passed.
        //
        // e.g. If subnet peer has been against consensus on this epoch 30% of the results
        //      and the threshold is 25%, then remove that peer
        if let Some(count) = against_consensus_peer_count.get_mut(&account_id.clone()) {
          let against_percent: u128 = Self::percent_div(*count as u128, consensus_peer_count as u128);

          if against_percent >= peer_against_consensus_removal_threshold {
            // Remove subnet peer storage and consensus data
            log::info!(
              "AccountId {:?} against consensus at {:?}% in subnet ID {:?}", 
              account_id, 
              against_percent,
              subnet_id
            );
            Self::do_remove_subnet_node(block, subnet_id.clone(), account_id.clone());
          }
        }

        // Check if peer has submitted data
        let peer_submitted: bool = NodeConsensusEpochSubmitted::<T>::get(subnet_id.clone(), account_id.clone());

        // Check if peer unconfirmed
        let peer_unconfirmed: bool = NodeConsensusEpochUnconfirmed::<T>::get(subnet_id.clone(), account_id.clone());
        
        // If peer didn't submit any form of consensus and can submit, increase penalty count
        if !peer_submitted && !peer_unconfirmed && can_submit {
          // In case of subnet state issues where a peer couldn't generate legitimate consensus data
          // Check how many times they missed consensus consecutively
          // We allow peers to miss consensus up to 

          SubnetNodeConsecutiveConsensusSent::<T>::insert(subnet_id.clone(), account_id.clone(), 0);
          SubnetNodeConsecutiveConsensusNotSent::<T>::mutate(subnet_id.clone(), account_id.clone(), |n: &mut u32| *n += 1);

          // We do not implement MaxSubnetNodeConsecutiveConsensusNotSent here
          // If a peer doesn't submit any consensus, they always increment up one AccountPenaltyCount

          AccountPenaltyCount::<T>::mutate(account_id.clone(), |n: &mut u32| *n += 1);
        } else {

          // Increase consensus sent sequence count
          SubnetNodeConsecutiveConsensusSent::<T>::mutate(subnet_id.clone(), account_id.clone(), |n: &mut u32| *n += 1);
          
          // Get consensus sent sequence count
          let subnet_node_seq_consensus_sent = SubnetNodeConsecutiveConsensusSent::<T>::get(
            subnet_id.clone(), 
            account_id.clone()
          );

          // For every time a subnet peer sequentially and successfully submits consensus
          // and their sequence count is greater than the threshold we increment their 
          // AccountPenaltyCount down by one
          //
          // This is because it's possible for there to be issues for the p2p subnet hosting network
          // e.g. If a peer wasn't being dishonest but somehow their submitted data was much earlier
          //      or later than others and they were against the consensus formed, we allow them to increment
          //      their accounts penalty count by 1 for every x consecutive epochs submitted and in-consensus
          //
          // e.g. If a subnet peer successfully sent 100 epochs in a row and the threshold is 100
          //      we then less one their AccountPenaltyCount
          //
          //      This will happen every 100 epochs if the threshold is 100
          //
          if subnet_node_seq_consensus_sent >= subnet_node_seq_consensus_sent_threshold && subnet_node_seq_consensus_sent % subnet_node_seq_consensus_sent_threshold == 0 {
            // Less one AccountPenaltyCount
            AccountPenaltyCount::<T>::mutate(account_id.clone(), |n: &mut u32| n.saturating_dec());
          }

          // Get how many epochs in a row subnet peer has missed consensus submissions
          let subnet_node_seq_consensus_not_sent = SubnetNodeConsecutiveConsensusNotSent::<T>::get(
            subnet_id.clone(), 
            account_id.clone()
          );

          // Since the subnet peer sent in consensus successfully
          // then reset sequential missed consensus count to zero
          if subnet_node_seq_consensus_not_sent > 0 {
            SubnetNodeConsecutiveConsensusNotSent::<T>::insert(subnet_id.clone(), account_id.clone(), 0);
          }
        }

        // Reset NodeConsensusEpochSubmitted on all peers
        NodeConsensusEpochSubmitted::<T>::insert(subnet_id.clone(), account_id.clone(), false);

        // Enact AccountPenaltyCount mechanism
        // Ensure subnet peer is still eligible
        // • is_account_eligible checks if they are over the max penalties count
        if !Self::is_account_eligible(account_id.clone()) {
          log::info!("AccountId {:?} ineligible in subnet ID {:?}", account_id, subnet_id);
          // If False, remove all of accounts subnet peers across all subnets
          Self::do_remove_account_subnet_nodes(block, account_id);
        }
      }

      //
      // At this point all consensus data is calculated
      //

      // Reset count of consensus submits
      // SubnetTotalConsensusSubmits::<T>::insert(subnet_id.clone(), 0);

      // Reset next epoch data not needed for emissions
      Self::reset_model_consensus_data(subnet_id.clone());

      // Get the total subnet peers after consensus to ensure subnet is still able to emit rewards
      // If so, we add to SubnetsInConsensus
      // This is queried when generating rewards and reset each epoch
      // let total_subnet_nodes: u32 = TotalSubnetNodes::<T>::get(subnet_id.clone());
      // let min_subnet_nodes: u32 = MinSubnetNodes::<T>::get();
      // if total_subnet_nodes >= min_subnet_nodes {
      //   // This gets reset in `emission`
      //   SubnetsInConsensus::<T>::append(subnet_id.clone());
      // }

      // If we get to this point, the subnet is in consensus
      // It's possible there aren't enough subnet peers to meet the minimum requirements
      // If this is the case, it will be dealt with next epoch
      SubnetsInConsensus::<T>::append(subnet_id.clone());
      SubnetConsecutiveSuccessfulEpochs::<T>::mutate(subnet_id.clone(), |n: &mut u32| *n += 1);
    }
  }

  pub fn reset_model_consensus_data_and_results(subnet_id: u32) {
    let _ = SubnetNodeConsensusResults::<T>::clear_prefix(subnet_id.clone(), u32::MAX, None);
    let _ = NodeConsensusEpochSubmitted::<T>::clear_prefix(subnet_id.clone(), u32::MAX, None);
    let _ = NodeConsensusEpochUnconfirmed::<T>::clear_prefix(subnet_id.clone(), u32::MAX, None);
    let _ = SubnetTotalConsensusSubmits::<T>::remove(subnet_id.clone());
    let _ = SubnetConsensusEpochUnconfirmedCount::<T>::remove(subnet_id.clone());
    let _ = SubnetConsensusEpochSubmitCount::<T>::remove(subnet_id.clone());
  }

  pub fn reset_model_consensus_data(subnet_id: u32) {
    let _ = NodeConsensusEpochSubmitted::<T>::clear_prefix(subnet_id.clone(), u32::MAX, None);
    let _ = NodeConsensusEpochUnconfirmed::<T>::clear_prefix(subnet_id.clone(), u32::MAX, None);
    let _ = SubnetTotalConsensusSubmits::<T>::remove(subnet_id.clone());
    let _ = SubnetConsensusEpochUnconfirmedCount::<T>::remove(subnet_id.clone());
    let _ = SubnetConsensusEpochSubmitCount::<T>::remove(subnet_id.clone());
  }

  // pub fn get_accountants(subnet_id: u32) -> Vec<PeerId> {
  //   // This is a placeholder function for retrieving accountants for a subnet
  //   // You would need to implement this based on your specific use case
  //   // For example, you might store accountant IDs in a separate storage map
  //   // and retrieve them using `AccountId` as the key
  //   //
  //   // For now, we just return a hardcoded list of accountant IDs
  //   vec![PeerId::from([1, 2, 3, 4, 5]), PeerId::from([6, 7, 8, 9, 10])]
  // }
}