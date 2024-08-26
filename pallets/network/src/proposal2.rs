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
  pub fn do_proposal(
    account_id: T::AccountId, 
    subnet_id: u32,
    peer_id: PeerId,
    data: Vec<u8>,
  ) -> DispatchResult {
    // --- Ensure subnet exists
    ensure!(
      SubnetsData::<T>::contains_key(subnet_id),
      Error::<T>::SubnetNotExist
    );

    // --- Ensure account has peer
    ensure!(
      SubnetNodesData::<T>::contains_key(subnet_id, account_id.clone()),
      Error::<T>::SubnetNodeNotExist
    );
  
    // --- Ensure proposer is accountant - Only this category of nodes can propose and vote on proposals
    let accountant_nodes = SubnetNodesClasses::<T>::get(subnet_id, SubnetNodeClass::Accountant);
    ensure!(
      accountant_nodes.get(&account_id) != None,
      Error::<T>::NodeAccountantEpochNotReached
    );

    // Unique subnet_id -> PeerId
    // Ensure peer ID exists within subnet
    let default_account_id: T::AccountId = T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap();
    let defendant_account_id: (bool, T::AccountId) = match SubnetNodeAccount::<T>::try_get(subnet_id, peer_id.clone()) {
      Ok(_result) => (true, _result),
      Err(()) => (false, default_account_id.clone()),
    };

    ensure!(
      defendant_account_id.0 && defendant_account_id.1 != default_account_id.clone(),
      Error::<T>::PeerIdNotExist
    );

    // --- Ensure the minimum required subnet peers exist
    // --- Only accountants can vote on proposals
    let accountant_nodes = SubnetNodesClasses::<T>::get(subnet_id, SubnetNodeClass::Accountant);
    let accountant_nodes_count = accountant_nodes.len();

    // There must always be the required minimum subnet peers for each vote
    let min_subnet_nodes: u32 = MinSubnetNodes::<T>::get();
    ensure!(
      accountant_nodes_count as u32 >= min_subnet_nodes,
      Error::<T>::SubnetNodesMin
    );

    let block: u64 = Self::get_current_block_as_u64();

    ensure!(
      !Self::account_has_active_proposal2(
        subnet_id, 
        defendant_account_id.clone().1, 
        block,
      ),
      Error::<T>::NotEnoughBalanceToBid
    );

    let proposal_bid_amount: u128 = ProposalBidAmount::<T>::get();
    let proposal_bid_amount_as_balance = Self::u128_to_balance(proposal_bid_amount);

    let can_withdraw: bool = Self::can_remove_balance_from_coldkey_account(
      &account_id,
      proposal_bid_amount_as_balance.unwrap(),
    );

    ensure!(
      can_withdraw,
      Error::<T>::NotEnoughBalanceToBid
    );

    // --- Withdraw bid amount from proposer accounts
    let _ = T::Currency::withdraw(
      &account_id,
      proposal_bid_amount_as_balance.unwrap(),
      WithdrawReasons::except(WithdrawReasons::TRANSFER),
      ExistenceRequirement::KeepAlive,
    );

    let mut yay: BTreeSet<T::AccountId> = BTreeSet::new();
    yay.insert(account_id.clone());

    Proposals::<T>::insert(
      subnet_id,
      0 as u32,
      ProposalParams {
        subnet_id: subnet_id,
        plaintiff: account_id.clone(),
        defendant: defendant_account_id.clone().1,
        plaintiff_bond: proposal_bid_amount,
        defendant_bond: 0,
        eligible_voters: accountant_nodes,
        votes: VoteParams2 {
          yay: yay,
          nay: BTreeSet::new(),
        },
        start_block: block,
        challenge_block: 0, // No challenge block initially
        plaintiff_data: data,
        defendant_data: Vec::new(),
        complete: false,
      }
    );

    Self::deposit_event(
      Event::DishonestSubnetNodeProposed{ 
        subnet_id: subnet_id, 
        account_id: account_id, 
        block: block
      }
    );

    Ok(())
  }

  pub fn do_challenge_proposal(
    account_id: T::AccountId, 
    subnet_id: u32,
    proposal_id: u32,
    data: Vec<u8>,
  ) -> DispatchResult {
    let proposal = match Proposals::<T>::try_get(subnet_id, proposal_id) {
      Ok(proposal) => proposal,
      Err(()) =>
        return Err(Error::<T>::ProposalInvalid.into()),
    };

    // --- Ensure defendant
    ensure!(
      account_id == proposal.defendant,
      Error::<T>::NotDefendant
    );

    // --- Ensure incomplete
    ensure!(
      !proposal.complete,
      Error::<T>::ProposalUnchallenged
    );
    
    let challenge_period = ChallengePeriod::<T>::get();
    let block: u64 = Self::get_current_block_as_u64();

    // --- Ensure challenge period is active
    ensure!(
      block < proposal.start_block + challenge_period,
      Error::<T>::ProposalChallenged
    );

    // --- Ensure unchallenged
    ensure!(
      proposal.challenge_block == 0,
      Error::<T>::ProposalChallenged
    );

    let proposal_bid_amount_as_balance = Self::u128_to_balance(proposal.plaintiff_bond);

    let can_withdraw: bool = Self::can_remove_balance_from_coldkey_account(
      &account_id,
      proposal_bid_amount_as_balance.unwrap(),
    );

    // --- Ensure can bond
    ensure!(
      can_withdraw,
      Error::<T>::NotEnoughBalanceToBid
    );

    // --- Withdraw bid amount from proposer accounts
    let _ = T::Currency::withdraw(
      &account_id,
      proposal_bid_amount_as_balance.unwrap(),
      WithdrawReasons::except(WithdrawReasons::TRANSFER),
      ExistenceRequirement::KeepAlive,
    );

    let mut nay: BTreeSet<T::AccountId> = BTreeSet::new();
    nay.insert(account_id);

    Proposals::<T>::mutate(
      subnet_id,
      0,
      |params: &mut ProposalParams<T::AccountId>| {
        params.votes.nay = nay;
        params.defendant_data = data;
        params.challenge_block = block;
      }
    );

    Ok(())
  }

  pub fn do_vote_proposal(
    account_id: T::AccountId, 
    subnet_id: u32,
    proposal_id: u32,
    vote: VoteType
  ) -> DispatchResult {
    let proposal = match Proposals::<T>::try_get(subnet_id, proposal_id) {
      Ok(proposal) => proposal,
      Err(()) =>
        return Err(Error::<T>::ProposalInvalid.into()),
    };

    // --- Ensure challenged
    ensure!(
      proposal.challenge_block != 0,
      Error::<T>::ProposalUnchallenged
    );

    // --- Ensure incomplete
    ensure!(
      !proposal.complete,
      Error::<T>::ProposalUnchallenged
    );
    
    let voting_period = VotingPeriod::<T>::get();
    let block: u64 = Self::get_current_block_as_u64();

    // --- Ensure voting period is active
    ensure!(
      block < proposal.challenge_block + voting_period,
      Error::<T>::ProposalChallenged
    );

    // --- Ensure is eligible to vote
    ensure!(
      proposal.eligible_voters.get(&account_id).is_some(),
      Error::<T>::ProposalChallenged
    );

    let yays: BTreeSet<T::AccountId> = proposal.votes.yay;
    let nays: BTreeSet<T::AccountId> = proposal.votes.nay;

    // --- Ensure hasn't already voted
    ensure!(
      yays.get(&account_id) == None && nays.get(&account_id) == None,
      Error::<T>::ProposalChallenged
    );

    Proposals::<T>::mutate(
      subnet_id,
      proposal_id,
      |params: &mut ProposalParams<T::AccountId>| {
        if vote == VoteType::Yay {
          params.votes.yay.insert(account_id);
        } else {
          params.votes.nay.insert(account_id);
        };  
      }
    );
    
    Ok(())
  }

  pub fn do_cancel_proposal(
    account_id: T::AccountId, 
    subnet_id: u32,
    proposal_id: u32,
  ) -> DispatchResult {
    let proposal = match Proposals::<T>::try_get(subnet_id, proposal_id) {
      Ok(proposal) => proposal,
      Err(()) =>
        return Err(Error::<T>::ProposalInvalid.into()),
    };

    // --- Ensure plaintiff
    ensure!(
      account_id == proposal.plaintiff,
      Error::<T>::NotPlaintiff
    );
    
    // --- Ensure unchallenged
    ensure!(
      proposal.challenge_block == 0,
      Error::<T>::ProposalChallenged
    );

    Proposals::<T>::mutate(
      subnet_id,
      proposal_id,
      |params: &mut ProposalParams<T::AccountId>| {
        params.complete = true;
      }
    );

    let plaintiff_bond_as_balance = Self::u128_to_balance(proposal.plaintiff_bond);
    // Give plaintiff bond back
    T::Currency::deposit_creating(&proposal.plaintiff, plaintiff_bond_as_balance.unwrap());

    Ok(())
  }

  pub fn do_finanlize_proposal(
    account_id: T::AccountId, 
    subnet_id: u32,
    proposal_id: u32,
  ) -> DispatchResult {
    let proposal = match Proposals::<T>::try_get(subnet_id, proposal_id) {
      Ok(proposal) => proposal,
      Err(()) =>
        return Err(Error::<T>::ProposalInvalid.into()),
    };

    // --- Ensure challenged
    ensure!(
      proposal.challenge_block != 0,
      Error::<T>::ProposalUnchallenged
    );

    // --- Ensure incomplete
    ensure!(
      !proposal.complete,
      Error::<T>::ProposalUnchallenged
    );
    
    let voting_period = VotingPeriod::<T>::get();
    let block: u64 = Self::get_current_block_as_u64();

    // --- Ensure voting period is completed
    ensure!(
      block > proposal.challenge_block + voting_period,
      Error::<T>::ProposalChallenged
    );

    // --- Ensure quorum reached
    let yays_len: u128 = proposal.votes.yay.len() as u128;
    let nays_len: u128 = proposal.votes.nay.len() as u128;
    let voters_len: u128 = proposal.eligible_voters.len() as u128;
    let voting_percentage: u128 = Self::percent_div(yays_len + nays_len, voters_len);

    let yays_percentage: u128 = Self::percent_div(yays_len, voters_len);
    let nays_percentage: u128 = Self::percent_div(nays_len, voters_len);

    let consensus_threshold: u128 = ProposalConsensusThreshold::<T>::get();
    let quorum_reached: bool = voting_percentage >= ProposalQuorum::<T>::get();

    let plaintiff_bond_as_balance = Self::u128_to_balance(proposal.plaintiff_bond);
    let defendant_bond_as_balance = Self::u128_to_balance(proposal.defendant_bond);

    // --- If quorum not reached and both voting options didn't succeed consensus then complete
    if !quorum_reached || 
        (yays_percentage < consensus_threshold && 
        nays_percentage < consensus_threshold && 
        quorum_reached)
      {
      Proposals::<T>::mutate(
        subnet_id,
        proposal_id,
        |params: &mut ProposalParams<T::AccountId>| {
          params.complete = true;
        }
      );

      // Give plaintiff and defendant bonds back
      T::Currency::deposit_creating(&proposal.plaintiff, plaintiff_bond_as_balance.unwrap());
      T::Currency::deposit_creating(&proposal.defendant, defendant_bond_as_balance.unwrap());
      // return 
      return Ok(())
    }

    // --- Mark as complete
    Proposals::<T>::mutate(
      subnet_id,
      proposal_id,
      |params: &mut ProposalParams<T::AccountId>| {
        params.complete = true;
      }
    );

    // --- At this point we know that one of the voting options are in consensus
    if yays_len > nays_len {
      // --- Plaintiff wins
      // --- Remove defendant
      Self::do_remove_subnet_node(block, subnet_id, proposal.defendant);
      T::Currency::deposit_creating(&proposal.plaintiff, plaintiff_bond_as_balance.unwrap());
      // --- Distribute bond to voters in consensus
      Self::distribute_bond(
        proposal.defendant_bond, 
        proposal.votes.yay,
        &proposal.plaintiff
      );
    } else {
      // --- Defendant wins
      T::Currency::deposit_creating(&proposal.defendant, defendant_bond_as_balance.unwrap());
      // --- Distribute bond to voters in consensus
      Self::distribute_bond(
        proposal.plaintiff_bond, 
        proposal.votes.nay,
        &proposal.defendant
      );
    }

    Ok(())
  }

  pub fn distribute_bond(
    bond: u128, 
    voters: BTreeSet<T::AccountId>,
    winner: &T::AccountId
  ) {
    let voters_len = voters.len();
    let distribution_amount = bond.saturating_div(voters_len as u128);
    let distribution_amount_as_balance = Self::u128_to_balance(distribution_amount);
    if !distribution_amount_as_balance.is_some() {
      return
    }

    let mut total_distributed: u128 = 0;
    for voter in voters {
      total_distributed += distribution_amount;
      T::Currency::deposit_creating(&voter, distribution_amount_as_balance.unwrap());
    }

    if total_distributed < bond {
      let remaining_bond = bond - total_distributed;
      let remaining_bid_as_balance = Self::u128_to_balance(remaining_bond);
      if remaining_bid_as_balance.is_some() {
        T::Currency::deposit_creating(winner, remaining_bid_as_balance.unwrap());
      }
    }
  }

  fn account_has_active_proposal2(
    subnet_id: u32, 
    account_id: T::AccountId, 
    block: u64,
  ) -> bool {
    let challenge_period = ChallengePeriod::<T>::get();
    let dishonesty_voting_period = VotingPeriod::<T>::get();

    let mut active_proposal: bool = false;
    for proposal in Proposals::<T>::iter_prefix_values(subnet_id) {
      let defendant: T::AccountId = proposal.defendant;
      if defendant != account_id {
        continue;
      }

      // At this point we have a proposal that matches the defendant

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

      // An account should only have one proposal at a time
      break;
    }

    active_proposal
  }

}