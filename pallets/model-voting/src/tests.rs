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

#![cfg(test)]
use crate::mock::*;
use sp_core::crypto::AccountId32;
use frame_support::{
	assert_noop, assert_ok, assert_err
};
use log::info;
use sp_core::{H256, U256};
use frame_support::traits::Currency;
use sp_core::OpaquePeerId as PeerId;
use crate::{
  Error, ModelPeer, PropsType, ModelVote, VotesBalance, ReservableCurrency, PropCount, VoteType,
  Votes, ActiveProposals, Proposals, PropsStatus, Quorum, PropsPathStatus, BalanceOf
};
type AccountIdOf<Test> = <Test as frame_system::Config>::AccountId;

const DEFAULT_IP: &str = "172.2.54.234";
const DEFAULT_PORT: u16 = 5000;
const DEFAULT_DEPOSIT_AMOUNT: u128 = 10000000000000000000000; // 10,000
const DEFAULT_MODEL_PATH: &str = "hf/llama2";
const DEFAULT_EXISTING_MODEL_PATH: &str = "hf/baluga";
const DEFAUT_VOTE_AMOUNT: u128 = 1000e+18 as u128;

fn account(id: u32) -> AccountIdOf<Test> {
	[id as u8; 32].into()
}

fn peer(id: u32) -> PeerId {
  let peer_id = format!("QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N{id}"); 
	PeerId(peer_id.into())
}

fn default_model_path() -> Vec<u8> {
  DEFAULT_MODEL_PATH.into()
}

fn default_ip() -> Vec<u8> {
  DEFAULT_IP.into()
}

fn build_existing_model(start: u32, end: u32) {
  let model_path: Vec<u8> = DEFAULT_EXISTING_MODEL_PATH.into();
  let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

  assert_ok!(
    Network::vote_model(
      RuntimeOrigin::signed(account(0)), 
      model_path.clone(),
    )
  );

  assert_ok!(
    Network::add_model(
      RuntimeOrigin::signed(account(0)),
      model_path.clone(),
    ) 
  );

  let model_id = pallet_network::ModelPaths::<Test>::get(model_path.clone()).unwrap();
  let min_stake = pallet_network::MinStakeBalance::<Test>::get();

  for n in start..end {
    let _ = Balances::deposit_creating(&account(n), min_stake + 100000);
    assert_ok!(
      Network::add_model_peer(
        RuntimeOrigin::signed(account(n)),
        model_id,
        peer(n),
        "172.20.54.234".into(),
        8888,
        min_stake,
      ) 
    );
  }
}

fn build_model_peers(start: u32, end: u32, deposit_amount: u128) -> Vec<ModelPeer<AccountId>> {
  let mut model_peers: Vec<ModelPeer<<Test as frame_system::Config>::AccountId>> = Vec::new();
  
  for n in start..end {
    let _ = Balances::deposit_creating(&account(n), deposit_amount);
    let model_peer = ModelPeer {
      account_id: account(n),
      peer_id: peer(n),
      ip: default_ip(),
      port: DEFAULT_PORT,
    };
    model_peers.push(model_peer);
  }
  model_peers
}

fn post_success_proposal_activate_ensures(path: Vec<u8>, proposal_index: u32, proposer: u32, proposal_start_block: u64) {
  let proposal = Proposals::<Test>::get(proposal_index);
  assert_eq!(proposal.path, path.clone());
  assert_eq!(proposal.proposal_status, PropsStatus::Active);
  assert_eq!(proposal.proposal_type, PropsType::Activate);
  // assert_eq!(proposal.model_peers, path);
  assert_eq!(proposal.max_block, proposal_start_block + VotingPeriod::get());

  let model_initialization_cost = <pallet_network::Pallet<Test> as ModelVote<Test>>::get_model_initialization_cost();
  assert_eq!(VotesBalance::<Test>::get(0, account(proposer)), model_initialization_cost.clone());

  // let reserved_balance = <pallet_balances::Pallet<Test> as ReservableCurrency<Test>>::reserved_balance(&proposer);
  let reserved_balance = Balances::reserved_balance(&account(proposer));
  assert_eq!(reserved_balance, model_initialization_cost.clone());

  let active_proposals = ActiveProposals::<Test>::get();
  assert_eq!(active_proposals, proposal_index + 1);

  let proposal_path_status = PropsPathStatus::<Test>::get(path.clone());
  assert_eq!(proposal_path_status, PropsStatus::Active);
}

fn post_cast_vote_ensures(proposal_index: u32, voter: u32) {
  assert_err!(
    ModelVoting::unreserve(
      RuntimeOrigin::signed(account(voter)),
      proposal_index, 
    ),
    Error::<Test>::ProposalInvalid
  );
}

fn post_yay_ensures(proposal_index: u32, prev_votes: u128, voter: u32, vote_amount: u128) {
  let reserved_balance = Balances::reserved_balance(&account(voter));
  assert_eq!(reserved_balance, vote_amount);

  let voting_power = ModelVoting::get_voting_power(account(voter), vote_amount);
  assert_eq!(VotesBalance::<Test>::get(proposal_index, account(voter)), voting_power);

  let votes = Votes::<Test>::get(proposal_index);

  assert_eq!(votes.yay, prev_votes + voting_power);
}

fn post_nay_ensures(proposal_index: u32, prev_votes: u128, voter: u32, vote_amount: u128) {
  let reserved_balance = Balances::reserved_balance(&account(voter));
  assert_eq!(reserved_balance, vote_amount);

  let voting_power = ModelVoting::get_voting_power(account(voter), vote_amount);
  assert_eq!(VotesBalance::<Test>::get(proposal_index, account(voter)), voting_power);

  let votes = Votes::<Test>::get(proposal_index);

  assert_eq!(votes.nay, prev_votes + voting_power);
}

fn post_abstain_ensures(proposal_index: u32, prev_votes: u128, voter: u32, vote_amount: u128) {
  let reserved_balance = Balances::reserved_balance(&account(voter));
  assert_eq!(reserved_balance, vote_amount);

  let voting_power = ModelVoting::get_voting_power(account(voter), vote_amount);
  assert_eq!(VotesBalance::<Test>::get(proposal_index, account(voter)), voting_power);

  let votes = Votes::<Test>::get(proposal_index);

  assert_eq!(votes.abstain, prev_votes + voting_power);
}


fn post_activate_execute_succeeded_ensures(proposal_index: u32, path: Vec<u8>) {
  let is_active = pallet_network::ModelActivated::<Test>::get(path.clone());
  assert_eq!(is_active, Some(true));

  let proposal = Proposals::<Test>::get(proposal_index);
  assert_eq!(proposal.proposal_status, PropsStatus::Succeeded);

  let active_proposals = ActiveProposals::<Test>::get();
  assert_eq!(active_proposals, proposal_index);

  post_proposal_concluded(proposal_index, path.clone());

  let proposal_path_status = PropsPathStatus::<Test>::get(path.clone());
  assert_eq!(proposal_path_status, PropsStatus::Succeeded);
}

fn post_deactivate_succeeded_execute_ensures(proposal_index: u32, path: Vec<u8>) {
  let is_active = pallet_network::ModelActivated::<Test>::get(path.clone());
  assert_eq!(is_active, Some(false));

  let proposal = Proposals::<Test>::get(proposal_index);
  assert_eq!(proposal.proposal_status, PropsStatus::Succeeded);

  let active_proposals = ActiveProposals::<Test>::get();
  assert_eq!(active_proposals, proposal_index);

  post_proposal_concluded(proposal_index, path.clone());

  let proposal_path_status = PropsPathStatus::<Test>::get(path.clone());
  assert_eq!(proposal_path_status, PropsStatus::Succeeded);
}


fn post_activate_cancel_ensures(proposal_index: u32, path: Vec<u8>) {
  let is_active = pallet_network::ModelActivated::<Test>::get(path.clone());
  assert_eq!(is_active, None);

  let proposal = Proposals::<Test>::get(proposal_index);
  assert_eq!(proposal.proposal_status, PropsStatus::Cancelled);

  let proposal_path_status = PropsPathStatus::<Test>::get(path.clone());
  assert_eq!(proposal.proposal_status, PropsStatus::Cancelled);

  post_proposal_concluded(proposal_index, path.clone());
}

fn post_success_proposal_deactivate_ensures(path: Vec<u8>, proposal_index: u32, proposer: u32, proposal_start_block: u64) {
  let proposal = Proposals::<Test>::get(proposal_index);
  assert_eq!(proposal.path, path.clone());
  assert_eq!(proposal.proposal_status, PropsStatus::Active);
  assert_eq!(proposal.proposal_type, PropsType::Deactivate);
  assert_eq!(proposal.max_block, proposal_start_block + VotingPeriod::get());

  let active_proposals = ActiveProposals::<Test>::get();
  assert_eq!(active_proposals, proposal_index + 1);

  let proposal_path_status = PropsPathStatus::<Test>::get(path.clone());
  assert_eq!(proposal_path_status, PropsStatus::Active);
}

fn post_proposal_concluded(proposal_index: u32, path: Vec<u8>) {
  let active_proposals = ActiveProposals::<Test>::get();
  assert_eq!(active_proposals, proposal_index);

  // --- Ensure cannot call twice
  assert_err!(
    ModelVoting::execute(
      RuntimeOrigin::signed(account(0)),
      proposal_index,
    ),
    Error::<Test>::Concluded
  );

  // --- Ensure cannot cast vote
  assert_err!(
    ModelVoting::cast_vote(
      RuntimeOrigin::signed(account(255)),
      proposal_index,
      1000,
      VoteType::Yay,
    ),
    Error::<Test>::VotingNotOpen
  );

  // --- Ensure cannot cancel proposal
  assert_err!(
    ModelVoting::cancel_proposal(
      RuntimeOrigin::signed(account(0)),
      proposal_index,
    ),
    Error::<Test>::Concluded
  );

  let proposal_path_status = PropsPathStatus::<Test>::get(path.clone());
  assert_ne!(proposal_path_status, PropsStatus::Active);
  assert_ne!(proposal_path_status, PropsStatus::None);
}

fn post_proposal_conclusion_unreserves(proposal_index: u32, start: u32, end: u32, vote_amount: u128) {
  for n in start..end {
    let beginning_balance = Balances::free_balance(&account(n));
    let votes_balance = VotesBalance::<Test>::get(proposal_index, account(n));

    assert_ok!(
      ModelVoting::unreserve(
        RuntimeOrigin::signed(account(n)),
        proposal_index, 
      )
    );

    let balance = Balances::free_balance(&account(n));
    assert_eq!(balance, beginning_balance + votes_balance);

    let votes_balance = VotesBalance::<Test>::get(proposal_index, account(n));
    assert_eq!(votes_balance, 0);
  }
}

fn build_propose_activate(path: Vec<u8>, start: u32, end: u32, deposit_amount: u128) -> u32 {
  let model_peers = build_model_peers(start, end, deposit_amount);

  assert_ok!(
    ModelVoting::propose(
      RuntimeOrigin::signed(account(0)),
      default_model_path(), 
      model_peers,
      PropsType::Activate,
    )
  );
  0
}

fn build_propose_deactivate(path: Vec<u8>, start: u32, end: u32, deposit_amount: u128) -> u32 {
  let min_model_peers = pallet_network::MinModelPeers::<Test>::get();
  build_existing_model(0, min_model_peers);

  let model_peers = build_model_peers(start, end, deposit_amount);

  assert_ok!(
    ModelVoting::propose(
      RuntimeOrigin::signed(account(0)),
      DEFAULT_EXISTING_MODEL_PATH.into(), 
      Vec::new(),
      PropsType::Deactivate,
    )
  );
  0
}

fn make_model_peer_included() {
  let consensus_blocks_interval = pallet_network::ConsensusBlocksInterval::<Test>::get();
  let min_required_consensus_inclusion_epochs = pallet_network::MinRequiredPeerConsensusInclusionEpochs::<Test>::get();
  System::set_block_number(System::block_number() + consensus_blocks_interval * min_required_consensus_inclusion_epochs + 1000);
}

#[test]
fn test_propose_activate() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();

    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();
    let min_stake = pallet_network::MinStakeBalance::<Test>::get();
    let model_initialization_cost = <pallet_network::Pallet<Test> as ModelVote<Test>>::get_model_initialization_cost();
    let _ = Balances::deposit_creating(&account(0), model_initialization_cost);

    let model_peers = build_model_peers(0, min_model_peers, min_stake);

    assert_ok!(
      ModelVoting::propose(
        RuntimeOrigin::signed(account(0)),
        default_model_path(), 
        model_peers,
        PropsType::Activate,
      )
    );

    post_success_proposal_activate_ensures(default_model_path(), prop_count, 0, System::block_number());
  })
}

#[test]
fn test_propose_activate_model_path_exists_err() {
  new_test_ext().execute_with(|| {

    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    build_existing_model(0, min_model_peers);
    let model_data = pallet_network::ModelsData::<Test>::get(1);
    let model_path: Vec<u8> = model_data.unwrap().path;

    let min_stake = pallet_network::MinStakeBalance::<Test>::get();
    let model_initialization_cost = <pallet_network::Pallet<Test> as ModelVote<Test>>::get_model_initialization_cost();
    let _ = Balances::deposit_creating(&account(0), model_initialization_cost);

    assert_err!(
      ModelVoting::propose(
        RuntimeOrigin::signed(account(0)),
        model_path,
        Vec::new(),
        PropsType::Activate,
      ),
      Error::<Test>::ModelPathExists
    );
  })
}

#[test]
fn test_propose_activate_already_active_err() {
  new_test_ext().execute_with(|| {
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();
    let min_stake = pallet_network::MinStakeBalance::<Test>::get();
    let model_initialization_cost = <pallet_network::Pallet<Test> as ModelVote<Test>>::get_model_initialization_cost();
    let _ = Balances::deposit_creating(&account(0), model_initialization_cost);

    let model_peers = build_model_peers(0, min_model_peers, min_stake);

    assert_ok!(
      ModelVoting::propose(
        RuntimeOrigin::signed(account(0)),
        default_model_path(), 
        model_peers.clone(),
        PropsType::Activate,
      )
    );

    let _ = Balances::deposit_creating(&account(0), model_initialization_cost);
    let model_peers = build_model_peers(0, min_model_peers, min_stake);

    assert_err!(
      ModelVoting::propose(
        RuntimeOrigin::signed(account(0)),
        default_model_path(),
        model_peers.clone(),
        PropsType::Activate,
      ),
      Error::<Test>::ProposalInvalid
    );
  })
}

#[test]
fn test_propose_activate_peers_min_length_err() {
  new_test_ext().execute_with(|| {
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();
    let min_stake = pallet_network::MinStakeBalance::<Test>::get();
    let model_initialization_cost = <pallet_network::Pallet<Test> as ModelVote<Test>>::get_model_initialization_cost();
    let _ = Balances::deposit_creating(&account(0), model_initialization_cost);

    assert_err!(
      ModelVoting::propose(
        RuntimeOrigin::signed(account(0)),
        default_model_path(),
        Vec::new(),
        PropsType::Activate,
      ),
      Error::<Test>::ModelPeersLengthInvalid
    );
  })
}

#[test]
fn test_propose_activate_peers_balance_err() {
  new_test_ext().execute_with(|| {
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();
    let min_stake = pallet_network::MinStakeBalance::<Test>::get();
    let model_initialization_cost = <pallet_network::Pallet<Test> as ModelVote<Test>>::get_model_initialization_cost();
    let _ = Balances::deposit_creating(&account(0), model_initialization_cost);

    let model_peers = build_model_peers(0, min_model_peers, min_stake-10000);

    assert_err!(
      ModelVoting::propose(
        RuntimeOrigin::signed(account(0)),
        default_model_path(),
        model_peers,
        PropsType::Activate,
      ),
      Error::<Test>::NotEnoughMinStakeBalance
    );
  })
}

#[test]
fn test_propose_activate_model_init_balance_err() {
  new_test_ext().execute_with(|| {
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();
    let offset = 1;
    let min_stake = pallet_network::MinStakeBalance::<Test>::get();

    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();
    // add model to ensure initialization cost is over zero
    build_existing_model(offset, min_model_peers + offset);

    make_model_peer_included();

    let _ = Balances::deposit_creating(&account(0), 0);

    let model_peers = build_model_peers(offset, min_model_peers + offset, min_stake);

    assert_err!(
      ModelVoting::propose(
        RuntimeOrigin::signed(account(0)),
        default_model_path(), 
        model_peers,
        PropsType::Activate,
      ),
      Error::<Test>::NotEnoughModelInitializationBalance
    );
  })
}

#[test]
fn test_cast_vote_activate_yay() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);

    let _ = Balances::deposit_creating(&account(0), DEFAUT_VOTE_AMOUNT);

    let votes = Votes::<Test>::get(prop_count);

    assert_ok!(
      ModelVoting::cast_vote(
        RuntimeOrigin::signed(account(0)),
        prop_count,
        DEFAUT_VOTE_AMOUNT,
        VoteType::Yay,
      )
    );

    post_yay_ensures(prop_count, votes.yay, 0, DEFAUT_VOTE_AMOUNT);
    post_cast_vote_ensures(prop_count, 0);
  })
}

#[test]
fn test_cast_vote_activate_yay_props_exists_err() {
  new_test_ext().execute_with(|| {
    assert_err!(
      ModelVoting::cast_vote(
        RuntimeOrigin::signed(account(0)),
        0,
        DEFAUT_VOTE_AMOUNT,
        VoteType::Yay,
      ),
      Error::<Test>::ProposalInvalid
    );
  })
}

#[test]
fn test_cast_vote_activate_yay_voting_not_open_err() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);

    let voting_period = VotingPeriod::get();
    
    let _ = Balances::deposit_creating(&account(0), DEFAUT_VOTE_AMOUNT);

    System::set_block_number(System::block_number() + voting_period + 1);

    assert_err!(
      ModelVoting::cast_vote(
        RuntimeOrigin::signed(account(0)),
        proposal_index,
        DEFAUT_VOTE_AMOUNT,
        VoteType::Yay,
      ),
      Error::<Test>::VotingNotOpen
    );

  })
}

#[test]
fn test_cast_vote_activate_yay_not_enough_balance_err() {
  new_test_ext().execute_with(|| {
    let offset = 1;
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0+offset, min_model_peers+offset, DEFAULT_DEPOSIT_AMOUNT);
    
    let _ = Balances::deposit_creating(&account(0), DEFAUT_VOTE_AMOUNT-1000);

    assert_err!(
      ModelVoting::cast_vote(
        RuntimeOrigin::signed(account(0)),
        proposal_index,
        DEFAUT_VOTE_AMOUNT,
        VoteType::Yay,
      ),
      Error::<Test>::NotEnoughBalanceToVote
    );
  })
}

#[test]
fn test_cast_vote_activate_nay() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);

    let _ = Balances::deposit_creating(&account(0), DEFAUT_VOTE_AMOUNT);

    let votes = Votes::<Test>::get(prop_count);

    assert_ok!(
      ModelVoting::cast_vote(
        RuntimeOrigin::signed(account(0)),
        prop_count,
        DEFAUT_VOTE_AMOUNT,
        VoteType::Nay,
      )
    );

    post_nay_ensures(prop_count, votes.nay, 0, DEFAUT_VOTE_AMOUNT);
    post_cast_vote_ensures(prop_count, 0);
  })
}

#[test]
fn test_cast_vote_activate_nay_props_exists_err() {
  new_test_ext().execute_with(|| {
    assert_err!(
      ModelVoting::cast_vote(
        RuntimeOrigin::signed(account(0)),
        0,
        DEFAUT_VOTE_AMOUNT,
        VoteType::Nay,
      ),
      Error::<Test>::ProposalInvalid
    );
  })
}

#[test]
fn test_cast_vote_activate_nay_voting_not_open_err() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);

    let voting_period = VotingPeriod::get();
    
    let _ = Balances::deposit_creating(&account(0), DEFAUT_VOTE_AMOUNT);

    System::set_block_number(System::block_number() + voting_period + 1);

    assert_err!(
      ModelVoting::cast_vote(
        RuntimeOrigin::signed(account(0)),
        proposal_index,
        DEFAUT_VOTE_AMOUNT,
        VoteType::Nay,
      ),
      Error::<Test>::VotingNotOpen
    );

  })
}

#[test]
fn test_cast_vote_activate_nay_not_enough_balance_err() {
  new_test_ext().execute_with(|| {
    let offset = 1;
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0+offset, min_model_peers+offset, DEFAULT_DEPOSIT_AMOUNT);
    
    let _ = Balances::deposit_creating(&account(0), DEFAUT_VOTE_AMOUNT-1000);

    assert_err!(
      ModelVoting::cast_vote(
        RuntimeOrigin::signed(account(0)),
        proposal_index,
        DEFAUT_VOTE_AMOUNT,
        VoteType::Nay,
      ),
      Error::<Test>::NotEnoughBalanceToVote
    );
  })
}

#[test]
fn test_cast_vote_activate_abstain() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);

    let _ = Balances::deposit_creating(&account(0), DEFAUT_VOTE_AMOUNT);

    let votes = Votes::<Test>::get(prop_count);

    assert_ok!(
      ModelVoting::cast_vote(
        RuntimeOrigin::signed(account(0)),
        prop_count,
        DEFAUT_VOTE_AMOUNT,
        VoteType::Abstain,
      )
    );

    post_abstain_ensures(prop_count, votes.abstain, 0, DEFAUT_VOTE_AMOUNT);
    post_cast_vote_ensures(prop_count, 0);
  })
}

#[test]
fn test_cast_vote_activate_abstain_props_exists_err() {
  new_test_ext().execute_with(|| {
    assert_err!(
      ModelVoting::cast_vote(
        RuntimeOrigin::signed(account(0)),
        0,
        DEFAUT_VOTE_AMOUNT,
        VoteType::Abstain,
      ),
      Error::<Test>::ProposalInvalid
    );
  })
}

#[test]
fn test_cast_vote_activate_abstain_voting_not_open_err() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);

    let voting_period = VotingPeriod::get();
    
    let _ = Balances::deposit_creating(&account(0), DEFAUT_VOTE_AMOUNT);

    System::set_block_number(System::block_number() + voting_period + 1);

    assert_err!(
      ModelVoting::cast_vote(
        RuntimeOrigin::signed(account(0)),
        proposal_index,
        DEFAUT_VOTE_AMOUNT,
        VoteType::Abstain,
      ),
      Error::<Test>::VotingNotOpen
    );
  })
}

#[test]
fn test_cast_vote_activate_abstain_not_enough_balance_err() {
  new_test_ext().execute_with(|| {
    let offset = 1;
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0+offset, min_model_peers+offset, DEFAULT_DEPOSIT_AMOUNT);
    
    let _ = Balances::deposit_creating(&account(0), DEFAUT_VOTE_AMOUNT-1000);

    assert_err!(
      ModelVoting::cast_vote(
        RuntimeOrigin::signed(account(0)),
        proposal_index,
        DEFAUT_VOTE_AMOUNT,
        VoteType::Abstain,
      ),
      Error::<Test>::NotEnoughBalanceToVote
    );
  })
}

#[test]
fn test_execute_activate_succeeded() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);
    
    for n in 0..min_model_peers {
      let _ = Balances::deposit_creating(&account(n), DEFAUT_VOTE_AMOUNT);
  
      assert_ok!(
        ModelVoting::cast_vote(
          RuntimeOrigin::signed(account(n)),
          proposal_index,
          DEFAUT_VOTE_AMOUNT,
          VoteType::Yay,
        )
      );
      post_cast_vote_ensures(proposal_index, n);
    }

    let voting_period = VotingPeriod::get();
    System::set_block_number(System::block_number() + voting_period + 1);

    assert_ok!(
      ModelVoting::execute(
        RuntimeOrigin::signed(account(0)),
        proposal_index,
      )
    );

    post_activate_execute_succeeded_ensures(proposal_index, DEFAULT_MODEL_PATH.into());

    post_proposal_conclusion_unreserves(proposal_index, 0, min_model_peers, DEFAUT_VOTE_AMOUNT);
  })
}

#[test]
fn test_execute_voting_period_err() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);
    
    for n in 0..min_model_peers {
      let _ = Balances::deposit_creating(&account(n), DEFAUT_VOTE_AMOUNT);
  
      assert_ok!(
        ModelVoting::cast_vote(
          RuntimeOrigin::signed(account(n)),
          proposal_index,
          DEFAUT_VOTE_AMOUNT,
          VoteType::Yay,
        )
      );
      post_cast_vote_ensures(proposal_index, n);
    }

    assert_err!(
      ModelVoting::execute(
        RuntimeOrigin::signed(account(0)),
        proposal_index,
      ),
      Error::<Test>::VotingOpen,
    );
  })
}

#[test]
fn test_execute_enactment_period_err() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);
    
    for n in 0..min_model_peers {
      let _ = Balances::deposit_creating(&account(n), DEFAUT_VOTE_AMOUNT);
  
      assert_ok!(
        ModelVoting::cast_vote(
          RuntimeOrigin::signed(account(n)),
          proposal_index,
          DEFAUT_VOTE_AMOUNT,
          VoteType::Yay,
        )
      );
      post_cast_vote_ensures(proposal_index, n);
    }

    let voting_period = VotingPeriod::get();
    let enactment_period = EnactmentPeriod::get();

    System::set_block_number(System::block_number() + voting_period + enactment_period + 1);

    assert_err!(
      ModelVoting::execute(
        RuntimeOrigin::signed(account(0)),
        proposal_index,
      ),
      Error::<Test>::EnactmentPeriodPassed,
    );
  })
}

#[test]
fn test_execute_quorum_not_reached_err() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);
    
    for n in 0..min_model_peers {
      let _ = Balances::deposit_creating(&account(n), DEFAUT_VOTE_AMOUNT);
  
      assert_ok!(
        ModelVoting::cast_vote(
          RuntimeOrigin::signed(account(n)),
          proposal_index,
          DEFAUT_VOTE_AMOUNT,
          VoteType::Yay,
        )
      );
      post_cast_vote_ensures(proposal_index, n);
    }

    let voting_period = VotingPeriod::get();
    let enactment_period = EnactmentPeriod::get();

    System::set_block_number(System::block_number() + voting_period + enactment_period + 1);

    assert_err!(
      ModelVoting::execute(
        RuntimeOrigin::signed(account(0)),
        proposal_index,
      ),
      Error::<Test>::EnactmentPeriodPassed
    );
  })
}

#[test]
fn test_execute_defeated() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let mut min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    // if min_model_peers < 12 {
    //   min_model_peers = 12
    // }

    // Get more nay voters than yay voters
    let yay_voters = min_model_peers / 4;

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);
    
    let vote_amount: u128 = 100000e+18 as u128;

    let quorum = Quorum::<Test>::get();

    let mut total_vote_amount = 0;

    for n in 0..yay_voters {
      let _ = Balances::deposit_creating(&account(n), vote_amount);
      total_vote_amount += vote_amount;
      assert_ok!(
        ModelVoting::cast_vote(
          RuntimeOrigin::signed(account(n)),
          proposal_index,
          vote_amount,
          VoteType::Yay,
        )
      );
      post_cast_vote_ensures(proposal_index, n);
    }

    for n in yay_voters..min_model_peers {
      let _ = Balances::deposit_creating(&account(n), vote_amount);
      assert_ok!(
        ModelVoting::cast_vote(
          RuntimeOrigin::signed(account(n)),
          proposal_index,
          vote_amount,
          VoteType::Nay,
        )
      );
      post_cast_vote_ensures(proposal_index, n);
    }

    assert!(total_vote_amount >= quorum, "Total votes must be greater than quorum");

    let voting_period = VotingPeriod::get();
    let enactment_period = EnactmentPeriod::get();

    System::set_block_number(System::block_number() + voting_period + 1);

    assert_ok!(
      ModelVoting::execute(
        RuntimeOrigin::signed(account(0)),
        proposal_index,
      )
    );

    let proposal = Proposals::<Test>::get(proposal_index);
    assert_eq!(proposal.proposal_status, PropsStatus::Defeated);

    post_proposal_conclusion_unreserves(proposal_index, 0, min_model_peers, vote_amount);
  })
}

#[test]
fn test_execute_cancelled() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);
    
    for n in 0..min_model_peers {
      let _ = Balances::deposit_creating(&account(n), DEFAUT_VOTE_AMOUNT);
  
      assert_ok!(
        ModelVoting::cast_vote(
          RuntimeOrigin::signed(account(n)),
          proposal_index,
          DEFAUT_VOTE_AMOUNT,
          VoteType::Yay,
        )
      );
      post_cast_vote_ensures(proposal_index, n);
    }

    assert_ok!(
      ModelVoting::cancel_proposal(
        RuntimeOrigin::signed(account(0)),
        proposal_index,
      )
    );

    post_activate_cancel_ensures(prop_count, DEFAULT_MODEL_PATH.into());
  })
}

#[test]
fn test_execute_cancelled_not_proposer_err() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);
    
    for n in 0..min_model_peers {
      let _ = Balances::deposit_creating(&account(n), DEFAUT_VOTE_AMOUNT);
  
      assert_ok!(
        ModelVoting::cast_vote(
          RuntimeOrigin::signed(account(n)),
          proposal_index,
          DEFAUT_VOTE_AMOUNT,
          VoteType::Yay,
        )
      );
      post_cast_vote_ensures(proposal_index, n);
    }

    assert_err!(
      ModelVoting::cancel_proposal(
        RuntimeOrigin::signed(account(1)),
        proposal_index,
      ),
      Error::<Test>::NotProposer
    );
  })
}

#[test]
fn test_execute_cancelled_proposal_index_err() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);
    
    for n in 0..min_model_peers {
      let _ = Balances::deposit_creating(&account(n), DEFAUT_VOTE_AMOUNT);
  
      assert_ok!(
        ModelVoting::cast_vote(
          RuntimeOrigin::signed(account(n)),
          proposal_index,
          DEFAUT_VOTE_AMOUNT,
          VoteType::Yay,
        )
      );
      post_cast_vote_ensures(proposal_index, n);
    }

    assert_err!(
      ModelVoting::cancel_proposal(
        RuntimeOrigin::signed(account(1)),
        proposal_index + 1,
      ),
      Error::<Test>::ProposalInvalid
    );
  })
}

#[test]
fn test_execute_cancelled_vote_completed_err() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);
    
    for n in 0..min_model_peers {
      let _ = Balances::deposit_creating(&account(n), DEFAUT_VOTE_AMOUNT);
  
      assert_ok!(
        ModelVoting::cast_vote(
          RuntimeOrigin::signed(account(n)),
          proposal_index,
          DEFAUT_VOTE_AMOUNT,
          VoteType::Yay,
        )
      );
      post_cast_vote_ensures(proposal_index, n);
    }

    let voting_period = VotingPeriod::get();

    System::set_block_number(System::block_number() + voting_period + 1);

    assert_err!(
      ModelVoting::cancel_proposal(
        RuntimeOrigin::signed(account(0)),
        proposal_index,
      ),
      Error::<Test>::VoteComplete
    );
  })
}

#[test]
fn test_propose_deactivate() {
  new_test_ext().execute_with(|| {
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();
    build_existing_model(0, min_model_peers);
    let prop_count = PropCount::<Test>::get();

    assert_ok!(
      ModelVoting::propose(
        RuntimeOrigin::signed(account(0)),
        DEFAULT_EXISTING_MODEL_PATH.into(), 
        Vec::new(),
        PropsType::Deactivate,
      )
    );

    post_success_proposal_deactivate_ensures(DEFAULT_EXISTING_MODEL_PATH.into(), prop_count, 0, System::block_number());
  })
}

#[test]
fn test_propose_deactivate_peers_min_length_err() {
  new_test_ext().execute_with(|| {
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();
    build_existing_model(0, min_model_peers);
    let min_stake = pallet_network::MinStakeBalance::<Test>::get();
    let model_initialization_cost = <pallet_network::Pallet<Test> as ModelVote<Test>>::get_model_initialization_cost();
    let _ = Balances::deposit_creating(&account(0), model_initialization_cost);

    let model_peers = build_model_peers(0, min_model_peers, min_stake);

    assert_err!(
      ModelVoting::propose(
        RuntimeOrigin::signed(account(0)),
        DEFAULT_EXISTING_MODEL_PATH.into(), 
        model_peers,
        PropsType::Deactivate,
      ),
      Error::<Test>::ModelPeersLengthInvalid
    );
  })
}

#[test]
fn test_propose_deactivate_model_id_exist_err() {
  new_test_ext().execute_with(|| {
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    assert_err!(
      ModelVoting::propose(
        RuntimeOrigin::signed(account(0)),
        DEFAULT_EXISTING_MODEL_PATH.into(), 
        Vec::new(),
        PropsType::Deactivate,
      ),
      Error::<Test>::ModelIdNotExists
    );
  })
}

#[test]
fn test_propose_deactivate_already_active_err() {
  new_test_ext().execute_with(|| {
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();
    build_existing_model(0, min_model_peers);

    assert_ok!(
      ModelVoting::propose(
        RuntimeOrigin::signed(account(0)),
        DEFAULT_EXISTING_MODEL_PATH.into(), 
        Vec::new(),
        PropsType::Deactivate,
      )
    );

    assert_err!(
      ModelVoting::propose(
        RuntimeOrigin::signed(account(0)),
        DEFAULT_EXISTING_MODEL_PATH.into(), 
        Vec::new(),
        PropsType::Deactivate,
      ),
      Error::<Test>::ProposalInvalid
    );
  })
}

#[test]
fn test_execute_deactivate_succeeded() {
  new_test_ext().execute_with(|| {
    let prop_count = PropCount::<Test>::get();
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();

    let proposal_index = build_propose_deactivate(DEFAULT_EXISTING_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);

    for n in 0..min_model_peers {
      let _ = Balances::deposit_creating(&account(n), DEFAUT_VOTE_AMOUNT);
  
      assert_ok!(
        ModelVoting::cast_vote(
          RuntimeOrigin::signed(account(n)),
          proposal_index,
          DEFAUT_VOTE_AMOUNT,
          VoteType::Yay,
        )
      );
      post_cast_vote_ensures(proposal_index, n);
    }

    let voting_period = VotingPeriod::get();
    System::set_block_number(System::block_number() + voting_period + 1);

    assert_ok!(
      ModelVoting::execute(
        RuntimeOrigin::signed(account(0)),
        proposal_index,
      )
    );

    post_deactivate_succeeded_execute_ensures(proposal_index, DEFAULT_EXISTING_MODEL_PATH.into());

    post_proposal_conclusion_unreserves(proposal_index, 0, min_model_peers, DEFAUT_VOTE_AMOUNT);
  })
}

#[test]
fn test_propose_activate_expired() {
  new_test_ext().execute_with(|| {
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();
    build_existing_model(0, min_model_peers);
    let prop_count = PropCount::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);

    let voting_period = VotingPeriod::get();
    System::set_block_number(System::block_number() + voting_period + 1);

    assert_ok!(
      ModelVoting::execute(
        RuntimeOrigin::signed(account(0)),
        proposal_index,
      )
    );

    let proposal = Proposals::<Test>::get(proposal_index);
    assert_eq!(proposal.proposal_status, PropsStatus::Expired);
  })
}

#[test]
fn test_balance_on_multiple_votes() {
  new_test_ext().execute_with(|| {
    let min_model_peers = pallet_network::MinModelPeers::<Test>::get();
    build_existing_model(0, min_model_peers);
    let prop_count = PropCount::<Test>::get();

    let proposal_index = build_propose_activate(DEFAULT_MODEL_PATH.into(), 0, min_model_peers, DEFAULT_DEPOSIT_AMOUNT);

    for n in 0..min_model_peers {
      let _ = Balances::deposit_creating(&account(n), DEFAUT_VOTE_AMOUNT);
  
      assert_ok!(
        ModelVoting::cast_vote(
          RuntimeOrigin::signed(account(n)),
          proposal_index,
          DEFAUT_VOTE_AMOUNT,
          VoteType::Yay,
        )
      );
    }

    for n in 0..min_model_peers {
      let votes_balance = VotesBalance::<Test>::get(proposal_index, account(n));
      assert_eq!(votes_balance, DEFAUT_VOTE_AMOUNT);

      let reserve_balance: BalanceOf<Test> = <pallet_balances::Pallet<Test> as ReservableCurrency<AccountId>>::reserved_balance(&account(n));
      assert_eq!(reserve_balance, DEFAUT_VOTE_AMOUNT);
    }

    for n in 0..min_model_peers {
      let _ = Balances::deposit_creating(&account(n), DEFAUT_VOTE_AMOUNT);
  
      assert_ok!(
        ModelVoting::cast_vote(
          RuntimeOrigin::signed(account(n)),
          proposal_index,
          DEFAUT_VOTE_AMOUNT,
          VoteType::Nay,
        )
      );
    }

    for n in 0..min_model_peers {
      let votes_balance = VotesBalance::<Test>::get(proposal_index, account(n));
      assert_eq!(votes_balance, DEFAUT_VOTE_AMOUNT*2);

      let reserve_balance: BalanceOf<Test> = <pallet_balances::Pallet<Test> as ReservableCurrency<AccountId>>::reserved_balance(&account(n));
      assert_eq!(reserve_balance, DEFAUT_VOTE_AMOUNT*2);
    }

    for n in 0..min_model_peers {
      let _ = Balances::deposit_creating(&account(n), DEFAUT_VOTE_AMOUNT);
  
      assert_ok!(
        ModelVoting::cast_vote(
          RuntimeOrigin::signed(account(n)),
          proposal_index,
          DEFAUT_VOTE_AMOUNT,
          VoteType::Abstain
        )
      );
    }

    for n in 0..min_model_peers {
      let votes_balance = VotesBalance::<Test>::get(proposal_index, account(n));
      assert_eq!(votes_balance, DEFAUT_VOTE_AMOUNT*3);

      let reserve_balance: BalanceOf<Test> = <pallet_balances::Pallet<Test> as ReservableCurrency<AccountId>>::reserved_balance(&account(n));
      assert_eq!(reserve_balance, DEFAUT_VOTE_AMOUNT*3);
    }
  })
}