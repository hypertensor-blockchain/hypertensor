// Copyright (C) 2021 Subspace Labs, Inc.
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

//! Pallet for subnet voting.
//! Sets all parameters required for initializing subnets

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use sp_core::{
  OpaquePeerId as PeerId,
  crypto::KeyTypeId,
  Get
};
use frame_system::{
  pallet_prelude::{OriginFor, BlockNumberFor},
  ensure_signed, ensure_root,
  offchain::{
    AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
    SignedPayload, Signer, SigningTypes, SubmitTransaction,
  },
};
use frame_support::{
  pallet_prelude::DispatchResult,
  ensure,
  dispatch::Vec,
  traits::{Currency, LockableCurrency, ReservableCurrency, WithdrawReasons, LockIdentifier},
};
use sp_runtime::{
  traits::Zero,
  Saturating, Perbill, Percent
};
use pallet_network::{SubnetVote, PreSubnetData, VoteSubnetData};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

mod types;
mod admin;
mod utils;

pub use types::PropIndex;

const MODEL_VOTING_ID: LockIdentifier = *b"modelvot";

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
  use super::*;
  use frame_support::pallet_prelude::*;
  use sp_runtime::traits::TrailingZeroInput;
  // use pallet_conviction_voting::pallet as ConvictionVoting;

  #[pallet::config]
  pub trait Config: frame_system::Config {
  // pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
    /// `rewards` events
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    /// The maximum number of public proposals that can exist at any time.
		#[pallet::constant]
		type MaxActivateProposals: Get<u32>;

    /// The maximum number of public proposals that can exist at any time.
		#[pallet::constant]
		type MaxDeactivateProposals: Get<u32>;

    #[pallet::constant]
		type MaxProposals: Get<u32>;

    /// Minimum amount to stake to create a proposal
    #[pallet::constant]
		type MinProposalStake: Get<u128>;

    #[pallet::constant]
		type VotingPeriod: Get<BlockNumberFor<Self>>;

    #[pallet::constant]
		type EnactmentPeriod: Get<BlockNumberFor<Self>>;

    type SubnetVote: SubnetVote<Self::AccountId>; 

    // type Currency: Currency<Self::AccountId> + LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>> + Send + Sync;
    type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId> + Send + Sync;

    type WeightInfo: WeightInfo;
  }

  	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
    /// Subnet path already exists
		SubnetPathExists,
    /// Subnet proposal invalid - Can't be (Active)
		ProposalInvalid,
    /// Proposal active still
    ProposalActive,
    /// Proposal doesn't exist 
    ProposalNotExist,
    /// Maximum proposals allowed
    MaxActiveProposals,
    /// Maximum activate proposals allowed
    MaxActivateProposals,
    /// Maximum deactivate proposals allowed
    MaxDeactivateProposals,
    /// Proposal voting period closed
    EnactmentPeriodInvalid,
    /// Proposal voting period closed
    VotingPeriodInvalid,
    /// Subnet ID doens't exist
    SubnetIdNotExists,
		/// Minimum required subnet peers not met
		SubnetNodesLengthInvalid,
    /// Minimum required subnet peers not met
		NotEnoughSubnetInitializationBalance,
    /// Minimum required subnet peers stake balance not in wallet
		NotEnoughMinStakeBalance,
    /// Not enough balance to vote
    NotEnoughBalanceToVote,
    /// Could not convert to balance
    CouldNotConvertToBalance,
    /// Proposal type invalid and None
    PropsTypeInvalid,
    /// Vote still active
    VoteActive,
    /// Quorum not reached
    QuorumNotReached,
    /// Executor must be proposer
    NotProposer,
    /// Vote completed alreaady
    VoteComplete,
    /// Enactment period passed
    EnactmentPeriodPassed,
    /// Votes are still open
    VotingOpen,
    /// Vote are not longer open either voting period has passed or proposal no longer active
    VotingNotOpen,
    /// Proposal has concluded
    Concluded,
    /// Vote balance doesn't exist
    VotesBalanceInvalid,
    /// Vote balance is zero
    VoteBalanceZero,
    InvalidQuorum,
    InvalidNodeVotePremium,
    InvalidPeerId,
  }

  /// `pallet-rewards` events
  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    SubnetVoteInInitialized(Vec<u8>, u64),
    SubnetVoteOutInitialized(u32, u64),
    SubnetVoteInSuccess(Vec<u8>, u64),
    SubnetVoteOutSuccess(u32, u64),
    SetNodeVotePremium(u128),
    SetQuorum(u128),
    SetMajority(u128),
  }

	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct SubnetNode<AccountId> {
    pub account_id: AccountId,
		pub peer_id: PeerId,
	}

  #[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct ActivatePropsParams<AccountId> {
    pub path: Vec<u8>,
		pub subnet_nodes: Vec<SubnetNode<AccountId>>,
    pub max_block: u64,
	}

  #[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct PropsParams<AccountId> {
    pub proposer: AccountId,
    pub proposer_stake: u128, // Activate: Non refundable, Deactivate: Refundable
    pub proposal_status: PropsStatus,
    pub proposal_type: PropsType,
    pub path: Vec<u8>,
    pub subnet_data: PreSubnetData,
		pub subnet_nodes: Vec<SubnetNode<AccountId>>,
    pub max_block: u64, // block ending voting
	}

  // #[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	// pub struct DeactivatePropsParams<AccountId> {
  //   pub path: Vec<u8>,
	// 	pub subnet_nodes: Vec<SubnetNode<AccountId>>,
	// }

  #[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct VotesParams {
    pub yay: u128,
		pub nay: u128,
    pub abstain: u128,
	}

  #[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct ActivateVotesParams {
    pub yay: u128,
		pub nay: u128,
	}

  #[pallet::type_value]
	pub fn DefaultAccountId<T: Config>() -> T::AccountId {
		T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap()
	}
	#[pallet::type_value]
	pub fn DefaultSubnetNode<T: Config>() -> SubnetNode<T::AccountId> {
		return SubnetNode {
			account_id: T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap(),
			peer_id: PeerId(Vec::new()),
    };
	}
	#[pallet::type_value]
	pub fn DefaultActivatePropsParams<T: Config>() -> ActivatePropsParams<T::AccountId> {
		return ActivatePropsParams {
			path: Vec::new(),
			subnet_nodes: Vec::new(),
      max_block: 0,
    };
	}
  #[pallet::type_value]
	pub fn DefaultPropsParams<T: Config>() -> PropsParams<T::AccountId> {
		return PropsParams {
      proposer: T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap(),
      proposer_stake: 0,
      proposal_status: PropsStatus::None,
      proposal_type: PropsType::None,
			path: Vec::new(),
      subnet_data: PreSubnetData::default(),
			subnet_nodes: Vec::new(),
      max_block: 0,
    };
	}
  #[pallet::type_value]
	pub fn DefaultVotes<T: Config>() -> VotesParams {
		return VotesParams {
      yay: 0,
      nay: 0,
      abstain: 0,
    }
	}
  // #[pallet::type_value]
	// pub fn DefaultActivateVotes<T: Config>() -> ActivateVotesParams {
	// 	return ActivateVotesParams {
  //     yay: 0,
  //     nay: 0,
  //   }
	// }
  // #[pallet::type_value]
	// pub fn DefaultDeactivatePropsParams<T: Config>() -> DeactivatePropsParams<T::AccountId> {
	// 	return DeactivatePropsParams {
	// 		path: Vec::new(),
	// 		subnet_nodes: Vec::new(),
  //   };
	// }
  #[pallet::type_value]
	pub fn DefaultPropsStatus() -> PropsStatus {
		PropsStatus::None
	}
  #[pallet::type_value]
	pub fn DefaultQuorum() -> u128 {
    // 10,000 * 1e18
		10000000000000000000000

    // 100 * 1e18 for testing
    // 100000000000000000000
	}
  #[pallet::type_value]
	pub fn DefaultMajority() -> u128 {
		66
	}

  #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
  enum VoteOutReason {
    // If subnet peers are performing manipulation for rewards
    SubnetEmissionsManipulation,
    // If the subnet is down
    SubnetDown,
    // If the subnet isn't open-sourced
    SubnetCloseSourced,
    // If subnet is broken
    SubnetBroken,
    // If the subnet doesn't have minimum required peers
    SubnetMinimumNodes,
    // If the subnet is outputting illicit or illegal data
    SubnetIllicit,
    // Other
    Other,
  }

  #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
  pub enum VoteType {
    Yay,
    Nay,
    Abstain,
  }

  #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
  pub enum PropsType {
    None,
    Activate,
    Deactivate,
  }

  impl Default for PropsType {
    fn default() -> Self {
      PropsType::None
    }
  }

  #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
  pub enum PropsStatus {
    // Default status
    None,
    /// Voting in progress or not yet executed
    Active,
    /// Voting succeeded and executed
    Succeeded,
    /// Not enough votes within voting period accomplished
    Defeated,
    /// Proposer cancelled proposal
    Cancelled,
    /// Voting period passed, thus expiring proposal
    Expired,
  }

  impl Default for PropsStatus {
    fn default() -> Self {
      PropsStatus::None
    }
  }

  // #[pallet::storage]
  // pub type ModelTypes<T: Config> =
  //   StorageMap<_, Blake2_128Concat, PropIndex, PropsParams<T::AccountId>, ValueQuery, DefaultPropsParams<T>>;

	// #[pallet::storage]
	// #[pallet::getter(fn activate_props)]
	// pub type ActivateProps<T: Config> =
	// 	StorageMap<_, Blake2_128Concat, PropIndex, ActivatePropsParams<T::AccountId>, ValueQuery, DefaultActivatePropsParams<T>>;

  #[pallet::storage]
  #[pallet::getter(fn props)]
  pub type Proposals<T: Config> =
    StorageMap<_, Blake2_128Concat, PropIndex, PropsParams<T::AccountId>, ValueQuery, DefaultPropsParams<T>>;
  
  // Track active proposals to ensure that we don't increase past the max proposals
  #[pallet::storage]
  #[pallet::getter(fn active_proposals)]
	pub type ActiveProposals<T> = StorageValue<_, u32, ValueQuery>;
  
  #[pallet::storage]
	pub type ActivateProposals<T> = StorageValue<_, u32, ValueQuery>;

  #[pallet::storage]
  pub type DeactivateProposals<T> = StorageValue<_, u32, ValueQuery>;

  #[pallet::storage]
  #[pallet::getter(fn votes)]
  pub type Votes<T: Config> =
    StorageMap<_, Blake2_128Concat, PropIndex, VotesParams, ValueQuery, DefaultVotes<T>>;
  
  #[pallet::storage]
  #[pallet::getter(fn votes_balance)]
  pub type VotesBalance<T: Config> = StorageDoubleMap<
    _,
    Blake2_128Concat,
    PropIndex,
    Identity,
    T::AccountId,
    BalanceOf<T>,
    ValueQuery,
  >;

  // #[pallet::storage]
  // pub type ActivateVotes<T: Config> =
  //   StorageMap<_, Blake2_128Concat, PropIndex, ActivateVotesParams, ValueQuery, DefaultActivateVotes<T>>;
  
  // #[pallet::storage]
	// #[pallet::getter(fn activate_prop_count)]
	// pub type ActivatePropCount<T> = StorageValue<_, PropIndex, ValueQuery>;

  #[pallet::storage]
	#[pallet::getter(fn prop_count)]
	pub type PropCount<T> = StorageValue<_, PropIndex, ValueQuery>;

  // #[pallet::storage]
	// #[pallet::getter(fn deactivate_props)]
	// pub type DeactivateProps<T: Config> =
	// 	StorageMap<_, Blake2_128Concat, PropIndex, ActivatePropsParams<T::AccountId>, ValueQuery, DefaultActivatePropsParams<T>>;

  // #[pallet::storage]
	// #[pallet::getter(fn deactivate_prop_count)]
	// pub type DeactivatePropCount<T> = StorageValue<_, PropIndex, ValueQuery>;

  #[pallet::storage]
	pub type PropsPathStatus<T: Config> =
		StorageMap<_, Blake2_128Concat, Vec<u8>, PropsStatus, ValueQuery, DefaultPropsStatus>;

  #[pallet::storage]
  #[pallet::getter(fn quorum)]
  pub type Quorum<T> = StorageValue<_, u128, ValueQuery, DefaultQuorum>;
  
  #[pallet::storage]
  pub type Majority<T> = StorageValue<_, u128, ValueQuery>;

  #[pallet::storage]
  pub type NodeVotePremium<T> = StorageValue<_, u128, ValueQuery>;

  #[pallet::pallet]
  #[pallet::without_storage_info]
  pub struct Pallet<T>(_);

  #[pallet::call]
  impl<T: Config> Pallet<T> {

    /// Propose a new subnet to be initiated.
		///
		/// May only be call to activate a subnet if 
    ///  - The subnet doesn't already exist within the network pallet
    ///  - The subnet isn't already proposed to be activated via PropsStatus::Active
    ///  - The proposer doesn't have the funds to initiate the subnet
    ///  - The subnet_nodes entered are below or above the min and max requirements
    ///  - The subnet_nodes don't have the minimum required stake balance available
    ///
		/// May only be call to deactivate a subnet if 
    ///  - The subnet already does exist within the network pallet
    ///  - The subnet isn't already proposed to be deactivated via PropsStatus::Active
    ///
    /// The PreSubnetData is used to dictate the subnets rewards and node requirements.
    /// Memory must be accurate to usage of the model for servers/
    /// Higher memory requirements will garner higher rewards but also require increased node requirements.
    /// Lower memory requirements will garner lower rewards but require lesser noes to operate the subnet.
    #[pallet::call_index(0)]
    #[pallet::weight(T::WeightInfo::propose())]
    // #[pallet::weight(0)]
    pub fn propose(
      origin: OriginFor<T>, 
      subnet_data: PreSubnetData,
      mut subnet_nodes: Vec<SubnetNode<T::AccountId>>,
      proposal_type: PropsType
    ) -> DispatchResult {
      let account_id: T::AccountId = ensure_signed(origin)?;

      // --- Ensure proper proposal type
      ensure!(
				proposal_type != PropsType::None,
				Error::<T>::PropsTypeInvalid
			);

      // --- Ensure active proposals count don't exceed max proposals count
      ensure!(
				ActiveProposals::<T>::get() <= T::MaxProposals::get(),
				Error::<T>::MaxActiveProposals
			);

      let proposal_index = PropCount::<T>::get();

      let mut proposer_stake: u128 = 0;
      let proposer_balance = T::Currency::free_balance(&account_id);

      if proposal_type == PropsType::Activate {
        // --- Stake the value of initializing a new subnet
        let subnet_initialization_cost = T::SubnetVote::get_model_initialization_cost();
        proposer_stake = subnet_initialization_cost;
        let subnet_initialization_cost_as_balance = Self::u128_to_balance(subnet_initialization_cost);
    
        ensure!(
          subnet_initialization_cost_as_balance.is_some(),
          Error::<T>::CouldNotConvertToBalance
        );
    
        ensure!(
          proposer_balance >= subnet_initialization_cost_as_balance.unwrap(),
          Error::<T>::NotEnoughSubnetInitializationBalance
        );
    
        // --- Reserve balance to be used once succeeded, otherwise it is freed on defeat
        // The final initialization fee may be more or less than the current initialization cost results
        T::Currency::reserve(
          &account_id,
          subnet_initialization_cost_as_balance.unwrap(),
        );

        // --- Proposal prelims
        Self::try_propose_activate(
          account_id.clone(), 
          subnet_data.clone(), 
          subnet_nodes.clone()
        ).map_err(|e| e)?;
      } else if proposal_type == PropsType::Deactivate {
        // --- Ensure zero subnet peers are submitted on deactivation proposals
        ensure!(
          subnet_nodes.clone().len() == 0,
          Error::<T>::SubnetNodesLengthInvalid
        );

        proposer_stake = T::MinProposalStake::get();
        let proposer_stake_as_balance = Self::u128_to_balance(proposer_stake);

        ensure!(
          proposer_balance >= proposer_stake_as_balance.unwrap(),
          Error::<T>::NotEnoughSubnetInitializationBalance
        );
    
        // --- Reserve balance to be used once succeeded, otherwise it is freed on defeat
        // The final initialization fee may be more or less than the current initialization cost results
        T::Currency::reserve(
          &account_id,
          proposer_stake_as_balance.unwrap(),
        );

        // --- Proposal prelims
        Self::try_propose_deactivate(account_id.clone(), subnet_data.clone().path)
          .map_err(|e| e)?;
      }

      // --- Save proposal
      Proposals::<T>::insert(
        proposal_index,
        PropsParams {
          proposer: account_id.clone(),
          proposer_stake: proposer_stake,
          proposal_status: PropsStatus::Active,
          proposal_type: proposal_type,
          path: subnet_data.clone().path,
          subnet_data: subnet_data.clone(),
          subnet_nodes: subnet_nodes.clone(),
          max_block: Self::convert_block_as_u64(<frame_system::Pallet<T>>::block_number() + T::VotingPeriod::get()),
        },
      );
  
      // --- Set path to current proposal status to active
      PropsPathStatus::<T>::insert(subnet_data.path, PropsStatus::Active);

      // --- Increase proposals count
      PropCount::<T>::put(proposal_index + 1);

      // --- Increase active proposals count
      ActiveProposals::<T>::mutate(|n: &mut u32| *n += 1);

      Ok(())
    }

    /// Vote on a proposal.
		///
		/// May only vote if
    ///  - Voter has enough balance
    ///
    /// Vote is based on balance and balance is staked until execution or defeat.
    #[pallet::call_index(1)]
    // #[pallet::weight(0)]
    #[pallet::weight(T::WeightInfo::cast_vote())]
    pub fn cast_vote(
      origin: OriginFor<T>, 
      proposal_index: PropIndex,
      vote_amount: BalanceOf<T>,
      vote: VoteType,
    ) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

      ensure!(
        Proposals::<T>::contains_key(proposal_index),
        Error::<T>::ProposalInvalid
      );
  
      let proposal = Proposals::<T>::get(proposal_index);

      Self::try_cast_vote(account_id, proposal_index, proposal, vote_amount, vote)
    }

    /// Execute completion of proposal 
    ///
    /// Voting must have completed
    ///
    /// Enactment period must not have completed
    ///
    /// Vote is based on balance and balance is staked until execution or defeat.
    ///
    /// Anyone can call this
    //
    // This cannot fail as long as the proposal ID exists, isn't concluded, and the voting period has completed
    #[pallet::call_index(2)]
    // #[pallet::weight(0)]
    #[pallet::weight(T::WeightInfo::execute())]
    pub fn execute(
      origin: OriginFor<T>, 
      proposal_index: PropIndex,
    ) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

      // --- Ensure proposal exists
      ensure!(
        Proposals::<T>::contains_key(proposal_index),
        Error::<T>::ProposalInvalid
      );
  
      let proposal = Proposals::<T>::get(proposal_index);

      // --- Ensure proposal is active and has not concluded
      ensure!(
        proposal.proposal_status == PropsStatus::Active,
        Error::<T>::Concluded
      );

      ensure!(
        !Self::is_voting_open(proposal.clone()),
        Error::<T>::VotingOpen
      );
  
      // --- Ensure voting has ended
      let max_block = proposal.max_block;
      let block = Self::get_current_block_as_u64();

      ensure!(
        block > max_block,
        Error::<T>::VoteActive
      );

      // --- We made it past the voting period, we cannot fail from here

      // --- If enactment period has passed, expire the proposal
      // Don't revert here to allow expired paths to be reproposed
      if block > max_block + Self::convert_block_as_u64(T::EnactmentPeriod::get())  {
        Self::try_expire(proposal_index, proposal.path.clone())
          .map_err(|e| e)?;
        return Ok(())
      }

      // --- Get status of proposal
      let votes = Votes::<T>::get(proposal_index);

      let quorum_reached = Self::quorum_reached(votes.clone());
      let vote_succeeded = Self::vote_succeeded(votes.clone());

      log::error!("quorum_reached {:?}", quorum_reached);
      log::error!("vote_succeeded {:?}", vote_succeeded);

      let proposer_stake_as_balance = Self::u128_to_balance(proposal.proposer_stake);
      // --- Unreserve here to pay for initialization fee or give back to proposer
      T::Currency::unreserve(
        &proposal.proposer,
        proposer_stake_as_balance.unwrap(),
      );
    
      // --- If quorum and vote YAYS aren greater than vote NAYS, then pass, else, defeat
      if quorum_reached && vote_succeeded {
        // Self::try_succeed(account_id, proposal.proposer, proposal_index, proposal.proposal_type, proposal.subnet_data.clone())
        //   .map_err(|e| e)?;
        Self::try_succeed(account_id, proposal_index, proposal)
          .map_err(|e| e)?;
      } else if quorum_reached && !vote_succeeded {
        Self::try_defeat(proposal_index, proposal.path.clone())
          .map_err(|e| e)?;
      } else {
        Self::try_expire(proposal_index, proposal.path.clone())
          .map_err(|e| e)?;
      }

      Ok(())
    }

    /// Cancel a proposal
    ///
    /// Can only be called by the proposer
    #[pallet::call_index(3)]
    // #[pallet::weight(0)]
    #[pallet::weight(T::WeightInfo::cancel_proposal())]
    pub fn cancel_proposal(
      origin: OriginFor<T>, 
      proposal_index: PropIndex,
    ) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

      // --- Ensure proposal exists
      ensure!(
        Proposals::<T>::contains_key(proposal_index),
        Error::<T>::ProposalInvalid
      );
  
      let proposal = Proposals::<T>::get(proposal_index);
      let proposer = proposal.proposer;
      ensure!(
        proposer == account_id,
        Error::<T>::NotProposer
      );

      ensure!(
        proposal.proposal_status == PropsStatus::Active,
        Error::<T>::Concluded
      );

      // --- Ensure voting hasn't ended 
      // Can't cancel once the proposals voting period has surpassed
      let max_block = proposal.max_block;
      let block = Self::get_current_block_as_u64();

      ensure!(
        block <= max_block,
        Error::<T>::VoteComplete
      );

      Self::try_cancel(proposal_index, proposal.path)
    }

    /// Unreserve vote stake
    ///
    /// Proposal must be not Active
    #[pallet::call_index(4)]
    // #[pallet::weight(0)]
    #[pallet::weight(T::WeightInfo::unreserve())]
    pub fn unreserve(
      origin: OriginFor<T>, 
      proposal_index: PropIndex,
    ) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

      // --- Ensure proposal exists
      ensure!(
        Proposals::<T>::contains_key(proposal_index),
        Error::<T>::ProposalInvalid
      );

      // --- Ensure proposal not active
      let proposal = Proposals::<T>::get(proposal_index);

      // The only way a proposal can not be active or none is if it was executed already
      // Therefor, we do not check the block numbers
      ensure!(
        proposal.proposal_status != PropsStatus::None && proposal.proposal_status != PropsStatus::Active,
        Error::<T>::ProposalInvalid
      );

      ensure!(
        VotesBalance::<T>::contains_key(proposal_index, &account_id),
        Error::<T>::VotesBalanceInvalid
      );

      // --- Get balance and remove from storage
      let balance = VotesBalance::<T>::take(proposal_index, &account_id);

      ensure!(
        Self::balance_to_u128(balance) > 0,
        Error::<T>::VoteBalanceZero
      );

      let reserved = T::Currency::reserved_balance(
        &account_id,
      );

      T::Currency::unreserve(
        &account_id,
        balance,
      );
  
      Ok(())
    }
  }

  #[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
    fn offchain_worker(block_number: BlockNumberFor<T>) {
    }
  }
}

// impl<T: Config + pallet::Config> Pallet<T> {
impl<T: Config> Pallet<T> {
  fn try_propose_activate(account_id: T::AccountId, subnet_data: PreSubnetData, mut subnet_nodes: Vec<SubnetNode<T::AccountId>>) -> DispatchResult {
    // --- Ensure path doesn't already exist in Network or SubnetVoting
    // If it doesn't already exist, then it has either been not proposed or deactivated
    ensure!(
      !T::SubnetVote::get_model_path_exist(subnet_data.clone().path),
      Error::<T>::SubnetPathExists
    );

    // --- Ensure proposal on subnet path not already in progress
    let proposal_status = PropsPathStatus::<T>::get(subnet_data.clone().path);

    // --- Ensure not active
    // A proposal can only be active if the subnet is not already initialized into the blockchain
    ensure!(
      proposal_status != PropsStatus::Active,
      Error::<T>::ProposalInvalid
    );

    ensure!(
      ActivateProposals::<T>::get() < T::MaxActivateProposals::get(),
      Error::<T>::MaxActivateProposals
    );

    // Remove duplicates based on peer_id and account_id
    subnet_nodes.dedup_by(|a, b| a.peer_id == b.peer_id && a.account_id == b.account_id);

    // --- Ensure PeerId's are validated
    for node in subnet_nodes.iter() {
      let valid = Self::validate_peer_id(node.clone().peer_id);
      ensure!(valid, Error::<T>::InvalidPeerId);
    }

    // --- Ensure account has enough balance to pay cost of subnet initialization

    // --- Ensure minimum peers required are already met before going forward
    let subnet_nodes_len: u32 = subnet_nodes.len() as u32;

    // @to-do: Get minimum subnet peers from network pallet
    ensure!(
      subnet_nodes_len >= T::SubnetVote::get_min_subnet_nodes(subnet_data.clone().memory_mb) && 
      subnet_nodes_len <= T::SubnetVote::get_max_subnet_nodes(),
      Error::<T>::SubnetNodesLengthInvalid
    );

    // --- Ensure peers have the minimum required stake balance
    let min_stake: u128 = T::SubnetVote::get_min_stake_balance();
    let min_stake_as_balance = Self::u128_to_balance(min_stake);

    ensure!(
      min_stake_as_balance.is_some(),
      Error::<T>::CouldNotConvertToBalance
    );

    for peer in subnet_nodes.clone() {
      let peer_balance = T::Currency::free_balance(&peer.account_id);

      ensure!(
        peer_balance >= min_stake_as_balance.unwrap(),
        Error::<T>::NotEnoughMinStakeBalance
      );
    }

    ActivateProposals::<T>::mutate(|n: &mut u32| *n += 1);

    // --- Begin to begin voting

    Ok(())
  }

  fn try_propose_deactivate(account_id: T::AccountId, path: Vec<u8>) -> DispatchResult {
    // --- Ensure subnet ID exists to be removed
    let subnet_id = T::SubnetVote::get_model_id_by_path(path.clone());
    ensure!(
      subnet_id != 0,
      Error::<T>::SubnetIdNotExists
    );

    // --- Ensure model has had enough time to initialize
    ensure!(
      T::SubnetVote::is_model_initialized(subnet_id),
      Error::<T>::ProposalInvalid
    );
    
    ensure!(
      DeactivateProposals::<T>::get() < T::MaxDeactivateProposals::get(),
      Error::<T>::MaxDeactivateProposals
    );

    // --- Ensure proposal on subnet path not already in progress
    let proposal_status = PropsPathStatus::<T>::get(path.clone());

    ensure!(
      proposal_status != PropsStatus::Active,
      Error::<T>::ProposalInvalid
    );

    DeactivateProposals::<T>::mutate(|n: &mut u32| *n += 1);
    
    Ok(())
  }

  fn try_cast_vote(
    account_id: T::AccountId, 
    proposal_index: PropIndex, 
    proposal: PropsParams<T::AccountId>,
    vote_amount: BalanceOf<T>,
    vote: VoteType,
  ) -> DispatchResult {
    ensure!(
      Self::is_voting_open(proposal),
      Error::<T>::VotingNotOpen
    );

    // --- Get balance of voter
    let balance = T::Currency::free_balance(&account_id.clone());

    // --- Ensure balance is some
    ensure!(
      Self::balance_to_u128(vote_amount) > 0,
      Error::<T>::VoteBalanceZero
    );

    // --- Ensure enough balance to vote based on vote_amount
    ensure!(
      balance >= vote_amount,
      Error::<T>::NotEnoughBalanceToVote
    );

    // --- Get vote power
    let vote_power: u128 = Self::get_voting_power(account_id.clone(), vote_amount);

    // --- Reserve voting balance of voter
    T::Currency::reserve(
      &account_id,
      vote_amount,
    );

    // --- Increase accounts reserved voting balance in relation to proposal index
    // VotesBalance::<T>::insert(proposal_index.clone(), account_id.clone(), vote_amount);

    VotesBalance::<T>::mutate(proposal_index.clone(), account_id.clone(), |n| *n += vote_amount);
    // VotesBalance::<T>::mutate(proposal_index.clone(), account_id.clone(), |n| n.saturating_add(vote_amount));

    // --- Save vote
    if vote == VoteType::Yay {
      Votes::<T>::mutate(
        proposal_index.clone(),
        |params: &mut VotesParams| {
          params.yay += vote_power;
        }
      );
    } else if vote == VoteType::Nay {
      Votes::<T>::mutate(
        proposal_index.clone(),
        |params: &mut VotesParams| {
          params.nay += vote_power;
        }
      );  
    } else {
      Votes::<T>::mutate(
        proposal_index.clone(),
        |params: &mut VotesParams| {
          params.abstain += vote_power;
        }
      );  
    }

    Ok(())
  }

  fn validate_peers(subnet_nodes: Vec<SubnetNode<T::AccountId>>) -> DispatchResult {
    Ok(())
  }

  fn get_voting_power(account_id: T::AccountId, balance: BalanceOf<T>) -> u128 {
    let is_submittable_subnet_node_account: bool = T::SubnetVote::is_submittable_subnet_node_account(account_id);

    if is_submittable_subnet_node_account {
      let peer_vote_premium = Perbill::from_rational(NodeVotePremium::<T>::get(), 100 as u128);
      let voting_power = balance.saturating_add(peer_vote_premium * balance);
   
      log::error!("balance      {:?}", balance);
      log::error!("voting_power {:?}", voting_power);
      return Self::balance_to_u128(voting_power)
    }

    Self::balance_to_u128(balance)
  }

  fn try_succeed(
    activator: T::AccountId,
    proposal_index: PropIndex, 
    proposal: PropsParams<T::AccountId>, 
  ) -> DispatchResult {
    Proposals::<T>::mutate(
      proposal_index,
      |params: &mut PropsParams<T::AccountId>| {
        params.proposal_status = PropsStatus::Succeeded;
      },
    );

    PropsPathStatus::<T>::insert(proposal.clone().subnet_data.path, PropsStatus::Succeeded);

    ActiveProposals::<T>::mutate(|n: &mut u32| n.saturating_dec());

    if proposal.proposal_type == PropsType::Activate {
      ActivateProposals::<T>::mutate(|n: &mut u32| n.saturating_dec());
      Self::try_activate_model(activator, proposal.clone().proposer, proposal.clone().subnet_data);
      // if proposal.proposer_stake > 0 {
      //   let proposer_stake_as_balance = Self::u128_to_balance(proposal.proposer_stake);
      //   T::Currency::unreserve(
      //     &proposal.proposer,
      //     proposer_stake_as_balance.unwrap(),
      //   );  
      // }
      Proposals::<T>::mutate(
        proposal_index,
        |params: &mut PropsParams<T::AccountId>| {
          params.proposer_stake = 0;
        },
      );

      // // Unreserve the proposal amount to sent to Network pallet
      // if let Ok(()) = Self::try_activate_model(activator, proposal.clone().proposer, proposal.clone().subnet_data) {
      //   // The activation of the subnet slashes the proposers reserves
      // } else {
      //   // if the activation of the subnet fails, unreserve the proposers reserve
      //   if proposal.proposer_stake > 0 {
      //     T::Currency::unreserve(
      //       &proposal.proposer,
      //       proposer_stake_as_balance.unwrap(),
      //     );  
      //   }
      // }
      // // --- Mutate the proposer stake to 0
      // Proposals::<T>::mutate(
      //   proposal_index,
      //   |params: &mut PropsParams<T::AccountId>| {
      //     params.proposer_stake = 0;
      //   },
      // );
      Ok(())
    } else {
      DeactivateProposals::<T>::mutate(|n: &mut u32| n.saturating_dec());
      Self::try_deactivate_model(activator, proposal.clone().proposer, proposal.clone().subnet_data);
      // if proposal.proposer_stake > 0 {
      //   let proposer_stake_as_balance = Self::u128_to_balance(proposal.proposer_stake);
      //   T::Currency::unreserve(
      //     &proposal.proposer,
      //     proposer_stake_as_balance.unwrap(),
      //   );  
      // }
      Proposals::<T>::mutate(
        proposal_index,
        |params: &mut PropsParams<T::AccountId>| {
          params.proposer_stake = 0;
        },
      );
      Ok(())

    }
  }

  fn try_defeat(proposal_index: PropIndex, path: Vec<u8>) -> DispatchResult {
    Proposals::<T>::mutate(
      proposal_index,
      |params: &mut PropsParams<T::AccountId>| {
        params.proposal_status = PropsStatus::Defeated;
      },
    );
  
    PropsPathStatus::<T>::insert(path.clone(), PropsStatus::Defeated);

    ActiveProposals::<T>::mutate(|n: &mut u32| n.saturating_dec());

    Ok(())
  }
  
  fn try_cancel(proposal_index: PropIndex, path: Vec<u8>) -> DispatchResult {
    Proposals::<T>::mutate(
      proposal_index,
      |params: &mut PropsParams<T::AccountId>| {
        params.proposal_status = PropsStatus::Cancelled;
      },
    );

    PropsPathStatus::<T>::insert(path.clone(), PropsStatus::Cancelled);

    ActiveProposals::<T>::mutate(|n: &mut u32| n.saturating_dec());

    Ok(())
  }

  fn try_expire(proposal_index: PropIndex, path: Vec<u8>) -> DispatchResult {
    Proposals::<T>::mutate(
      proposal_index,
      |params: &mut PropsParams<T::AccountId>| {
        params.proposal_status = PropsStatus::Expired;
      },
    );
  
    PropsPathStatus::<T>::insert(path.clone(), PropsStatus::Expired);

    ActiveProposals::<T>::mutate(|n: &mut u32| n.saturating_dec());

    Ok(())
  }

  /// Is voting active and within voting period
  fn is_voting_open(proposal: PropsParams<T::AccountId>) -> bool {
    let block = Self::get_current_block_as_u64();
    let max_block = proposal.max_block;
    
    block <= max_block && proposal.proposal_status == PropsStatus::Active
  }

  fn vote_succeeded(votes: VotesParams) -> bool {
    log::error!("vote_succeeded votes.yay {:?}", votes.yay);
    log::error!("vote_succeeded votes.nay {:?}", votes.nay);

    votes.yay > votes.nay
  }

  fn quorum_reached(votes: VotesParams) -> bool {
    let quorum = Quorum::<T>::get();
    let total_quorum_votes = votes.yay + votes.abstain;
    total_quorum_votes >= quorum
  }

  /// Activate subnet - Someone must add_subnet once activated
  fn try_activate_model(activator: T::AccountId, proposer: T::AccountId, subnet_data: PreSubnetData) -> DispatchResult {
    let vote_subnet_data = VoteSubnetData {
      data: subnet_data.clone(),
      active: true,
    };

    T::SubnetVote::vote_activated(
      activator.clone(),
      subnet_data.clone().path, 
      proposer.clone(),
      vote_subnet_data.clone()
    )
  }

  fn try_deactivate_model(activator: T::AccountId, proposer: T::AccountId, subnet_data: PreSubnetData) -> DispatchResult {
    let vote_subnet_data = VoteSubnetData {
      data: subnet_data.clone(),
      active: false,
    };

    T::SubnetVote::vote_deactivated(
      activator.clone(),
      subnet_data.clone().path, 
      proposer.clone(),
      vote_subnet_data.clone()
    )

    // T::SubnetVote::vote_activated(subnet_data.clone().path, vote_subnet_data)
    // Ok(())
  }
}

impl<T: Config> Pallet<T> {
  fn u128_to_balance(
    input: u128,
  ) -> Option<
    <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance,
  > {
    input.try_into().ok()
  }

  fn balance_to_u128(
    input: BalanceOf<T>,
  ) -> u128 {
    return match input.try_into() {
      Ok(_result) => _result,
      Err(_error) => 0,
    }
  }

  fn get_current_block_as_u64() -> u64 {
    TryInto::try_into(<frame_system::Pallet<T>>::block_number())
      .ok()
      .expect("blockchain will not exceed 2^64 blocks; QED.")
  }

  fn convert_block_as_u64(block: BlockNumberFor<T>) -> u64 {
    TryInto::try_into(block)
      .ok()
      .expect("blockchain will not exceed 2^64 blocks; QED.")
  }
}

// Admin logic
impl<T: Config> AdminInterface for Pallet<T> {
	fn set_peer_vote_premium(value: u128) -> DispatchResult {
		Self::set_peer_vote_premium(value)
	}
	fn set_quorum(value: u128) -> DispatchResult {
		Self::set_quorum(value)
	}
  fn set_majority(value: u128) -> DispatchResult {
		Self::set_majority(value)
	}
}

pub trait AdminInterface {
	fn set_peer_vote_premium(value: u128) -> DispatchResult;
  fn set_quorum(value: u128) -> DispatchResult;
  fn set_majority(value: u128) -> DispatchResult;
}