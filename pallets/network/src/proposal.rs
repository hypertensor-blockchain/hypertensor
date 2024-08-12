// Copyright (C) Hypertensor.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use sp_runtime::traits::TrailingZeroInput;

impl<T: Config> Pallet<T> {
  pub fn try_propose_dishonesty(
    proposer: T::AccountId,
    subnet_id: u32,
    peer_id: PeerId,
    proposal_type: PropsType,
    data: Vec<u8>,
    accountant_data_id: Option<u32>,
  ) -> DispatchResult {
    ensure!(
      proposal_type != PropsType::None,
      Error::<T>::PropsTypeInvalid
    );

    ensure!(
      data.len() > 0,
      Error::<T>::DataEmpty
    );

    // --- Ensure subnet exists
    ensure!(
      SubnetsData::<T>::contains_key(subnet_id.clone()),
      Error::<T>::SubnetNotExist
    );

    // --- Ensure account has peer
    ensure!(
      SubnetNodesData::<T>::contains_key(subnet_id.clone(), proposer.clone()),
      Error::<T>::SubnetNodeNotExist
    );
    
    // --- Ensure peer to propose as dishonest exists
    let subnet_node_account: (bool, T::AccountId) = match SubnetNodeAccount::<T>::try_get(subnet_id.clone(), peer_id.clone()) {
      Ok(_result) => (true, _result),
      Err(()) => (false, T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap()),
    };

    ensure!(
      subnet_node_account.clone().0,
      Error::<T>::PeerIdNotExist
    );

    // Get proposal big amount and store in proposal in case this updates
    let proposal_bid_amout: u128 = ProposalBidAmount::<T>::get();
    let proposal_bid_amout_as_balance = Self::u128_to_balance(proposal_bid_amout);

    let can_withdraw: bool = Self::can_remove_balance_from_coldkey_account(
      &proposer,
      proposal_bid_amout_as_balance.unwrap(),
    );

    ensure!(
      can_withdraw,
      Error::<T>::NotEnoughBalanceToBid
    );

    // --- Withdraw bid amount from proposer accounts
    let _ = T::Currency::withdraw(
      &proposer,
      proposal_bid_amout_as_balance.unwrap(),
      WithdrawReasons::except(WithdrawReasons::TRANSFER),
      ExistenceRequirement::KeepAlive,
    );

    // --- Ensure the proposalled peer to be dishonest doesn't already have a proposal with the same type
    let proposal_index = DishonestyProposalsCount::<T>::get();
    
    let block: u64 = Self::get_current_block_as_u64();

    let challenge_period = ChallengePeriod::<T>::get();

    // --- Ensure the proposed dishonest node doesn't already have a proposal within the same
    //     proposal type
    ensure!(
      !Self::account_has_active_proposal(
        subnet_id.clone(), 
        subnet_node_account.clone().1, 
        proposal_type.clone(),
        block,
      ),
      Error::<T>::NotEnoughBalanceToBid
    );

    // If proposal is of a dishonest accountant from data submissions, ensure that the accountant data
    // exists and there is sufficient time to propose that the accountant data is incorrect
    if proposal_type == PropsType::DishonestAccountant {
      // --- Ensure AccountantData exists
      ensure!(
        accountant_data_id != None,
        Error::<T>::InvalidAccountantDataId
      );

      ensure!(
        AccountantData::<T>::contains_key(subnet_id.clone(), accountant_data_id.unwrap()),
        Error::<T>::InvalidAccountantDataId
      );

      // --- Ensure required time period to challenge hasn't passed
      let accountant_data: AccountantDataParams<T::AccountId> = AccountantData::<T>::get(subnet_id.clone(), accountant_data_id.unwrap());
      let accountant_data_block: u64 = accountant_data.block;
      let accountant_data_challenge_period: u64 = AccountantDataChallengePeriod::<T>::get();
      let accountant_data_max_block: u64 = accountant_data_block + accountant_data_challenge_period;

      ensure!(
        block < accountant_data_max_block,
        Error::<T>::InvalidAccountantDataId
      );
    }

    let mut voters: Vec<T::AccountId> = Vec::new();
    voters.push(proposer.clone());

    let min_required_peer_accountant_epochs: u64 = MinRequiredNodeAccountantEpochs::<T>::get();
    let epoch_length: u64 = EpochLength::<T>::get();
    let total_accountants = Self::get_total_accountants(
      subnet_id.clone(),
      block,
      epoch_length,
      min_required_peer_accountant_epochs
    );

    // --- Initiate proposal
    DishonestyProposal::<T>::mutate(
      subnet_id.clone(),
      proposal_index,
      |params: &mut DishonestyProposalParams<T::AccountId>| {
        params.subnet_id = subnet_id.clone();
        params.proposal_type = proposal_type.clone();
        params.proposer = proposer.clone();
        params.total_accountants = total_accountants;
        params.account_id = subnet_node_account.clone().1;
        params.peer_id = peer_id.clone();
        params.bid = proposal_bid_amout;
        params.total_votes = 1;
        params.votes = VotesParams {
          yay: 1,
          nay: 0,
        };
        params.voters = voters.clone();
        params.yay_voters = voters.clone();
        params.nay_voters = Vec::new();
        params.start_block = block;
        params.challenge_block = 0;
        params.data = data;
        if proposal_type == PropsType::DishonestAccountant {
          params.accountant_data_id = Some(accountant_data_id.unwrap());
        } else {
          params.accountant_data_id = None;
        }
      }
    );

    // --- Increase proposal counter
    DishonestyProposalsCount::<T>::put(proposal_index + 1);

    Ok(())
  }

  pub fn try_challenge_dishonesty(
    account_id: T::AccountId, 
    subnet_id: u32,
    proposal_index: u32,
  ) -> DispatchResult {
    ensure!(
      DishonestyProposal::<T>::contains_key(subnet_id, proposal_index),
      Error::<T>::ProposalNotExist
    );

    // --- We don't check if subnet ID or peer ID exists because a proposal
    //     can't exist unless they do

    let proposal = DishonestyProposal::<T>::get(subnet_id, proposal_index);
    let dishonest_account_id: T::AccountId = proposal.account_id;

    // --- Ensure account is the possible challenger
    ensure!(
      account_id == dishonest_account_id,
      Error::<T>::NotChallenger
    );

    // --- Ensure proposal not challenged yet
    ensure!(
      proposal.challenge_block == 0,
      Error::<T>::PropsalAlreadyChallenged
    );

    let proposal_start_block: u64 = proposal.start_block;
    let challenge_period = ChallengePeriod::<T>::get();
    let max_challenge_block = proposal_start_block + challenge_period;
    let block: u64 = Self::get_current_block_as_u64();

    // --- Ensure challenge period hasn't passed yet
    ensure!(
      block <= max_challenge_block,
      Error::<T>::ChallengePeriodPassed
    );

    // Get proposal big amount and store in proposal in case this updates
    let proposal_bid_amout: u128 = proposal.bid;
    let proposal_bid_amout_as_balance = Self::u128_to_balance(proposal_bid_amout);
    
    let can_withdraw: bool = Self::can_remove_balance_from_coldkey_account(
      &account_id,
      proposal_bid_amout_as_balance.unwrap(),
    );

    ensure!(
      can_withdraw,
      Error::<T>::NotEnoughBalanceToBid
    );

    // --- Withdraw bid amount from proposer accounts
    let _ = T::Currency::withdraw(
      &account_id,
      proposal_bid_amout_as_balance.unwrap(),
      WithdrawReasons::except(WithdrawReasons::TRANSFER),
      ExistenceRequirement::KeepAlive,
    );

    // --- Challenge proposal
    DishonestyProposal::<T>::mutate(
      subnet_id.clone(),
      proposal_index,
      |params: &mut DishonestyProposalParams<T::AccountId>| {
        params.total_votes += 1;
        params.challenge_block = block;
        params.votes.nay += 1; // challenger automatically nays proposal
        params.voters.push(account_id.clone());
        params.nay_voters.push(account_id.clone());
      }
    );

    Ok(())
  }

  pub fn try_vote(
    account_id: T::AccountId, 
    subnet_id: u32,
    proposal_index: u32,
    vote: VoteType
  ) -> DispatchResult {
    ensure!(
      DishonestyProposal::<T>::contains_key(subnet_id, proposal_index),
      Error::<T>::ProposalNotExist
    );

    let proposal = DishonestyProposal::<T>::get(subnet_id, proposal_index);

    // --- Ensure dishonest peer still exists 
    ensure!(
      SubnetNodesData::<T>::contains_key(subnet_id.clone(), proposal.account_id),
      Error::<T>::SubnetNodeNotExist
    );

    // --- Ensure voter is accountant
    let account_subnet_node = SubnetNodesData::<T>::get(subnet_id.clone(), account_id.clone());
    let peer_initialized: u64 = account_subnet_node.initialized;
    let block: u64 = Self::get_current_block_as_u64();
    let epoch_length: u64 = EpochLength::<T>::get();
    let min_required_peer_accountant_epochs: u64 = MinRequiredNodeAccountantEpochs::<T>::get();

    ensure!(
      Self::is_epoch_block_eligible(
        block, 
        epoch_length, 
        min_required_peer_accountant_epochs, 
        peer_initialized
      ),
      Error::<T>::NodeAccountantEpochNotReached
    );
    
    let challenge_block: u64 = proposal.challenge_block;

    // --- Ensure proposal has been challenged to initiate voting
    ensure!(
      challenge_block != 0,
      Error::<T>::ProposalNotChallenged
    );

    let voting_period = VotingPeriod::<T>::get();

    // --- Ensure voting period hasn't passed yet
    ensure!(
      block <= challenge_block + voting_period,
      Error::<T>::DishonestyVotingPeriodOver
    );

    let voters: Vec<T::AccountId> = proposal.voters;

    // --- Ensure account hasn't voted yet
    ensure!(
      voters.iter().find(|&x| *x == account_id) == None,
      Error::<T>::DuplicateVote
    );

    // --- Vote on proposal
    DishonestyProposal::<T>::mutate(
      subnet_id.clone(),
      proposal_index,
      |params: &mut DishonestyProposalParams<T::AccountId>| {
        params.total_votes += 1;
        params.voters.push(account_id.clone());
        if vote == VoteType::Yay {
          params.votes.yay += 1;
          params.yay_voters.push(account_id.clone());
        } else {
          params.votes.nay += 1;
          params.nay_voters.push(account_id.clone());
        };
      }
    );

    Ok(())
  }

  pub fn try_finalize_proposal(
    subnet_id: u32,
    proposal_index: u32,
  ) -> DispatchResult {
    ensure!(
      DishonestyProposal::<T>::contains_key(subnet_id, proposal_index),
      Error::<T>::ProposalNotExist
    );

    let proposal = DishonestyProposal::<T>::get(subnet_id, proposal_index);
    let challenge_block: u64 = proposal.challenge_block;
    let challenge_period = ChallengePeriod::<T>::get();
    let max_challenge_block = proposal.start_block + challenge_period;
    let block: u64 = Self::get_current_block_as_u64();

    // Challenge period has passed unchallenged
    if block > max_challenge_block && challenge_block == 0 {
      // Proposal unchalleneged 

      // Return bid back to proposer
      let bid_as_balance = Self::u128_to_balance(proposal.bid);
      T::Currency::deposit_creating(&proposal.proposer, bid_as_balance.unwrap());

      // Remove dishonest peer
      Self::do_remove_subnet_node(block, subnet_id, proposal.account_id);

      return Ok(())
    }

    // --- Ensure proposal has been challenged to initiate voting
    ensure!(
      challenge_block != 0,
      Error::<T>::ProposalNotChallenged
    );

    let voting_period = VotingPeriod::<T>::get();

    // --- Ensure voting period has passed yet
    ensure!(
      block > challenge_block + voting_period,
      Error::<T>::DishonestyVotingPeriodNotOver
    );

    let min_required_peer_accountant_epochs: u64 = MinRequiredNodeAccountantEpochs::<T>::get();
    let epoch_length: u64 = EpochLength::<T>::get();

    let mut total_accountants = Self::get_total_accountants(
      subnet_id.clone(),
      block,
      epoch_length,
      min_required_peer_accountant_epochs
    );

    // We use total_accountants for whichever is greater
    // The time of proposal or the current total accountants
    // 	This prevents manipulation to remove nodes if someone controls multiple
    // 	accountants that can sway vote by removing them during or after voting has begun
    if proposal.total_accountants > total_accountants {
      total_accountants = proposal.total_accountants;
    }

    // --- Check if proposal has reached quorum
    let quorum: u128 = ProposalQuorum::<T>::get();
    let percent_voting: u128 = Self::percent_div(proposal.total_votes as u128, total_accountants as u128);

    ensure!(
      percent_voting > quorum,
      Error::<T>::QuorumNotReached
    );

    let consensus_threshold: u128 = ProposalConsensusThreshold::<T>::get();

    let yay_votes = proposal.votes.yay;
    let yay_votes_percentage: u128 = Self::percent_div(yay_votes as u128, proposal.total_votes as u128);

    let nay_votes = proposal.votes.nay;
    let nay_votes_percentage: u128 = Self::percent_div(nay_votes as u128, proposal.total_votes as u128);

    let bid_as_balance = Self::u128_to_balance(proposal.bid);
    ensure!(
      bid_as_balance.is_some(),
      Error::<T>::CouldNotConvertToBalance
    );

    if yay_votes_percentage > consensus_threshold {
      // Consensus reached
      
      // Remove dishonest peer
      Self::do_remove_subnet_node(block, subnet_id, proposal.account_id);

      // Give proposer bid back
      T::Currency::deposit_creating(&proposal.proposer, bid_as_balance.unwrap());
      
      // Distribute challenger bid to all in consensus
      Self::distributed_proposal_bids(
      	subnet_id.clone(),
        proposal.proposer,
      	proposal.bid,
      	proposal.yay_voters
      );
    } else if nay_votes_percentage > consensus_threshold {
      // Consensus reached

      // We don't assume the proposer is dishonest here so no one is removed
      
      // Give challenger bid back
      T::Currency::deposit_creating(&proposal.account_id, bid_as_balance.unwrap());

      // Distribute proposer bid to all in consensus
      Self::distributed_proposal_bids(
      	subnet_id.clone(),
        proposal.account_id,
      	proposal.bid,
      	proposal.nay_voters
      );
    } else {
      // Give proposer bid back
      T::Currency::deposit_creating(&proposal.proposer, bid_as_balance.unwrap());
  
      // Give challenger bid back
      T::Currency::deposit_creating(&proposal.account_id, bid_as_balance.unwrap());
    }

    Ok(())
  }

  pub fn try_cancel_proposal(
    account_id: T::AccountId, 
    subnet_id: u32,
    proposal_index: u32,
  ) -> DispatchResult {
    ensure!(
      DishonestyProposal::<T>::contains_key(subnet_id, proposal_index),
      Error::<T>::ProposalNotExist
    );

    // Get proposal and remove it from storage
    let proposal = DishonestyProposal::<T>::take(subnet_id, proposal_index);

    // --- Ensure proposal hasn't been challenged yet
    ensure!(
      !proposal.challenge_block == 0,
      Error::<T>::ProposalChallenged
    );

    let proposal_start_block: u64 = proposal.start_block;
    let challenge_period = ChallengePeriod::<T>::get();
    let max_challenge_block = proposal_start_block + challenge_period;
    let block: u64 = Self::get_current_block_as_u64();

    if account_id != proposal.proposer {
      // Only propser can cancel the proposal before being challenged
      // --- Ensure challenge period has passed
      ensure!(
        block > max_challenge_block,
        Error::<T>::ChallengePeriodPassed
      );
    }

    // --- Deposit bid amount from proposer account
    let bid: u128 = proposal.bid;
    let bid_as_balance = Self::u128_to_balance(bid);

    // Give proposer bid back
    T::Currency::deposit_creating(&proposal.proposer, bid_as_balance.unwrap());

    Ok(())
  }

  pub fn account_has_active_proposal(
    subnet_id: u32, 
    account_id: T::AccountId, 
    proposal_type: PropsType,
    block: u64,
  ) -> bool {
    let challenge_period = ChallengePeriod::<T>::get();
    let dishonesty_voting_period = VotingPeriod::<T>::get();

    let mut active_proposal: bool = false;
    for proposal in DishonestyProposal::<T>::iter_prefix_values(subnet_id) {
      let proposal_account_id: T::AccountId = proposal.account_id;
      if proposal_account_id != account_id {
        continue;
      }

      let proposal_proposal_type: PropsType = proposal.proposal_type;
      if proposal_proposal_type != proposal_type {
        continue;
      }

      // At this point we have a proposal that matches the parameters

      let challenge_block: u64 = proposal.challenge_block;
      let proposal_block: u64 = proposal.start_block;

      // Is proposal unchallenged and challenge period has passed?
      if proposal_block + challenge_period > block && challenge_block == 0 {
        continue;
      }

      // Is proposal challenged and voting period has passed?
      if challenge_block + dishonesty_voting_period > block {
        continue;
      }

      active_proposal = true;

      // An account should only have one proposal at a time per proposal type
      break;
    }

    active_proposal
  }

  pub fn distributed_proposal_bids(
    subnet_id: u32, 
    winner: T::AccountId,
    bid: u128, 
    voters: Vec<T::AccountId>
  ) {
    let total_voters = voters.len();
    let distribution_amount = bid.saturating_div(total_voters as u128);
    let distribution_amount_as_balance = Self::u128_to_balance(distribution_amount);

    if !distribution_amount_as_balance.is_some() {
      return
    }

    for voter in voters {
      T::Currency::deposit_creating(&voter, distribution_amount_as_balance.unwrap());
    }

    let total_distribution: u128 = total_voters as u128 * distribution_amount;

    // If bid amount can't be distibuted equally, then give remaining to winner
    if total_distribution < bid {
      let remaining_bid = bid - total_distribution;
      let remaining_bid_as_balance = Self::u128_to_balance(remaining_bid);
      if remaining_bid_as_balance.is_some() {
        T::Currency::deposit_creating(&winner, remaining_bid_as_balance.unwrap());
      }
    }
  }
}