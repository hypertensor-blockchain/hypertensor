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

//! Pallet for issuing rewards to block producers.

#![cfg_attr(not(feature = "std"), no_std)]

// pub mod weights;

pub use pallet::*;
use sp_core::OpaquePeerId as PeerId;
use frame_system::{
  pallet_prelude::{OriginFor, BlockNumberFor},
  ensure_signed, ensure_root,
  WeightInfo,
  offchain::{
    AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
    SignedPayload, Signer, SigningTypes, SubmitTransaction,
  },
};
use frame_support::{
  weights::Weight,
  pallet_prelude::DispatchResult,
  ensure,
  dispatch::Vec,
  traits::{Currency, LockableCurrency, WithdrawReasons, LockIdentifier},
};
use sp_runtime::Percent;
use sp_runtime::{Saturating, Perbill};
use sp_core::Get;
use sp_core::crypto::KeyTypeId;
use pallet_network::SubnetVote;

mod types;
// mod utils;

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

    #[pallet::constant]
		type VotingPeriod: Get<BlockNumberFor<Self>>;

    type SubnetVote: SubnetVote<Self::AccountId>; 

    type Currency: Currency<Self::AccountId> + LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>> + Send + Sync;

    type WeightInfo: WeightInfo;
  }

  	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
    /// Subnet path already exists
		SubnetPathExists,
    /// Subnet proposal invalid - Can't be (ActivateVoting DectivateVoting Activated)
		ProposalInvalid,
    /// Proposal doesn't exist 
    ProposalNotExist,
    /// Proposal voting period closed
    EnactmentPeriodInvalid,
    /// Proposal voting period closed
    VotingPeriodInvalid,
    /// Subnet ID doens't exist
    SubnetIdNotExists,
		/// Minimum required model peers not met
		SubnetNodesLengthInvalid,
    /// Minimum required model peers not met
		NotEnoughSubnetInitializationBalance,
    /// Minimum required model peers stake balance not in wallet
		NotEnoughMinStakeBalance,
    /// Not enough balance to vote
    NotEnoughBalanceToVote,
    /// Could not convert to balance
    CouldNotConvertToBalance,
    /// Proposal type invalid and None
    PropsTypeInvalid,
  }

  /// `pallet-rewards` events
  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    SubnetVoteInInitialized(Vec<u8>, u64),
    SubnetVoteOutInitialized(u32, u64),
    SubnetVoteInSuccess(Vec<u8>, u64),
    SubnetVoteOutSuccess(u32, u64),
  }

	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct SubnetNode<AccountId> {
    pub account_id: AccountId,
		pub peer_id: PeerId,
		pub ip: Vec<u8>,
		pub port: u16,
	}

  #[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct ActivatePropsParams<AccountId> {
    pub path: Vec<u8>,
		pub subnet_nodes: Vec<SubnetNode<AccountId>>,
    pub max_block: u64,
	}

  #[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct PropsParams<AccountId> {
    pub proposal_status: PropsStatus,
    pub proposal_type: PropsType,
    pub path: Vec<u8>,
		pub subnet_nodes: Vec<SubnetNode<AccountId>>,
    pub max_block: u64,
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
      ip: Vec::new(),
      port: 0,
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
      proposal_status: PropsStatus::None,
      proposal_type: PropsType::None,
			path: Vec::new(),
			subnet_nodes: Vec::new(),
      max_block: 0,
    };
	}
  #[pallet::type_value]
	pub fn DefaultVotes<T: Config>() -> VotesParams {
		return VotesParams {
      yay: 0,
      nay: 0,
    }
	}
  #[pallet::type_value]
	pub fn DefaultActivateVotes<T: Config>() -> ActivateVotesParams {
		return ActivateVotesParams {
      yay: 0,
      nay: 0,
    }
	}
  // #[pallet::type_value]
	// pub fn DefaultDeactivatePropsParams<T: Config>() -> DeactivatePropsParams<T::AccountId> {
	// 	return DeactivatePropsParams {
	// 		path: Vec::new(),
	// 		subnet_nodes: Vec::new(),
  //   };
	// }
  #[pallet::type_value]
	pub fn DefaultPropsStatus<T: Config>() -> PropsStatus {
		PropsStatus::None
	}

  #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
  enum VoteOutReason {
    // If model peers are performing manipulation for rewards
    SubnetEmissionsManipulation,
    // If the model is down
    SubnetDown,
    // If the model isn't open-sourced
    SubnetCloseSourced,
    // If model is broken
    SubnetBroken,
    // If the model doesn't have minimum required peers
    SubnetMinimumNodes,
    // If the model is outputting illicit or illegal data
    SubnetIllicit,
    // Other
    Other,
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
    // Activation voting
    ActivateVoting,
    // Deactivation voting
    DectivateVoting,
    // Proposal activated
    Activated,
    // Proposal deactivated
    Deactivated,

    Active,

    Succeeded,

    Defeated,
  }

  impl Default for PropsStatus {
    fn default() -> Self {
      PropsStatus::None
    }
  }

	#[pallet::storage]
	#[pallet::getter(fn activate_props)]
	pub type ActivateProps<T: Config> =
		StorageMap<_, Blake2_128Concat, PropIndex, ActivatePropsParams<T::AccountId>, ValueQuery, DefaultActivatePropsParams<T>>;

  #[pallet::storage]
  #[pallet::getter(fn props)]
  pub type Proposals<T: Config> =
    StorageMap<_, Blake2_128Concat, PropIndex, PropsParams<T::AccountId>, ValueQuery, DefaultPropsParams<T>>;
  
  #[pallet::storage]
  #[pallet::getter(fn active_proposals)]
	pub type ActiveProposals<T> = StorageValue<_, u32, ValueQuery>;
  
  #[pallet::storage]
  #[pallet::getter(fn votes)]
  pub type Votes<T: Config> =
    StorageMap<_, Blake2_128Concat, PropIndex, VotesParams, ValueQuery, DefaultVotes<T>>;
  
  #[pallet::storage]
  pub type ActivateVotes<T: Config> =
    StorageMap<_, Blake2_128Concat, PropIndex, ActivateVotesParams, ValueQuery, DefaultActivateVotes<T>>;
  
  #[pallet::storage]
	#[pallet::getter(fn activate_prop_count)]
	pub type ActivatePropCount<T> = StorageValue<_, PropIndex, ValueQuery>;

  #[pallet::storage]
	#[pallet::getter(fn prop_count)]
	pub type PropCount<T> = StorageValue<_, PropIndex, ValueQuery>;

  #[pallet::storage]
	#[pallet::getter(fn deactivate_props)]
	pub type DeactivateProps<T: Config> =
		StorageMap<_, Blake2_128Concat, PropIndex, ActivatePropsParams<T::AccountId>, ValueQuery, DefaultActivatePropsParams<T>>;

  #[pallet::storage]
	#[pallet::getter(fn deactivate_prop_count)]
	pub type DeactivatePropCount<T> = StorageValue<_, PropIndex, ValueQuery>;

  #[pallet::storage]
	pub type PropsPathStatus<T: Config> =
		StorageMap<_, Blake2_128Concat, Vec<u8>, PropsStatus, ValueQuery, DefaultPropsStatus<T>>;

  #[pallet::storage]
  #[pallet::getter(fn quorum)]
  pub type Quorum<T> = StorageValue<_, u128, ValueQuery>;
  
  #[pallet::storage]
  pub type NodeVotePremium<T> = StorageValue<_, u128, ValueQuery>;

  #[pallet::pallet]
  #[pallet::without_storage_info]
  pub struct Pallet<T>(_);

  #[pallet::call]
  impl<T: Config> Pallet<T> {
    #[pallet::call_index(0)]
    #[pallet::weight(0)]
    pub fn propose(
      origin: OriginFor<T>, 
      path: Vec<u8>, 
      subnet_nodes: Vec<SubnetNode<T::AccountId>>,
      proposal_type: PropsType
    ) -> DispatchResult {
      let account_id: T::AccountId = ensure_signed(origin)?;

      ensure!(
				proposal_type != PropsType::None,
				Error::<T>::PropsTypeInvalid
			);

      if proposal_type == PropsType::Activate {
        Self::try_propose_activate(account_id.clone(), path.clone(), subnet_nodes.clone());
      } else if proposal_type == PropsType::Deactivate {
        ensure!(
          subnet_nodes.clone().len() == 0,
          Error::<T>::SubnetNodesLengthInvalid
        );
        Self::try_propose_deactivate(account_id.clone(), path.clone());
      }

      let proposal_index = PropCount::<T>::get();

      Proposals::<T>::insert(
        proposal_index,
        PropsParams {
          proposal_status: PropsStatus::Active,
          proposal_type: proposal_type,
          path: path.clone(),
          subnet_nodes: subnet_nodes.clone(),
          max_block: Self::convert_block_as_u64(<frame_system::Pallet<T>>::block_number() + T::VotingPeriod::get()),
        },
      );
  
      PropsPathStatus::<T>::insert(path.clone(), PropsStatus::Active);

      PropCount::<T>::put(proposal_index + 1);

      ActiveProposals::<T>::mutate(|n: &mut u32| *n += 1);

      Ok(())
    }

    #[pallet::call_index(1)]
    #[pallet::weight(0)]
    pub fn propose_activate(origin: OriginFor<T>, path: Vec<u8>, subnet_nodes: Vec<SubnetNode<T::AccountId>>) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

      // // Check path doesn't already exist in Network or SubnetVoting
      // // If it doesn't already exist, then it has either been not proposed or deactivated
			// ensure!(
			// 	!T::SubnetVote::get_model_path_exist(path.clone()),
			// 	Error::<T>::SubnetPathExists
			// );

      // // Ensure can propose new model path
      // let proposal_status = PropsPathStatus::<T>::get(path.clone());

      // ensure!(
			// 	proposal_status != PropsStatus::ActivateVoting ||
      //   proposal_status != PropsStatus::DectivateVoting ||
      //   proposal_status != PropsStatus::Activated,
			// 	Error::<T>::ProposalInvalid
			// );

      // // ...

      // // Ensure account has enough balance to pay cost of model initialization
      // let model_initialization_cost = T::SubnetVote::get_model_initialization_cost();
      // let model_initialization_cost_as_balance = Self::u128_to_balance(model_initialization_cost);

      // ensure!(
      //   model_initialization_cost_as_balance.is_some(),
      //   Error::<T>::CouldNotConvertToBalance
      // );
  
      // let initializer_balance = T::Currency::free_balance(&account_id);
      // ensure!(
			// 	model_initialization_cost_as_balance.unwrap() >= initializer_balance,
			// 	Error::<T>::NotEnoughSubnetInitializationBalance
			// );

      // // Lock balance
      // // The final initialization fee may be more or less than the current initialization cost results
      // T::Currency::set_lock(
      //   MODEL_VOTING_ID,
      //   &account_id,
      //   model_initialization_cost_as_balance.unwrap(),
      //   WithdrawReasons::RESERVE
      // );
    
      // // ...

      // // Ensure account is already an existing peer and account eligible

      // // Ensure minimum peers required are already met before going forward
      // // @to-do: Get minimum model peers from network pallet
			// ensure!(
			// 	subnet_nodes.len() as u32 >= T::SubnetVote::get_min_subnet_nodes() && 
      //   subnet_nodes.len() as u32 <= T::SubnetVote::get_max_subnet_nodes(),
			// 	Error::<T>::SubnetNodesLengthInvalid
			// );

      // // Ensure peers have the minimum required stake balance
      // let min_stake: u128 = T::SubnetVote::get_min_stake_balance();
      // let min_stake_as_balance = Self::u128_to_balance(min_stake);

      // ensure!(
      //   min_stake_as_balance.is_some(),
      //   Error::<T>::CouldNotConvertToBalance
      // );

      // for peer in subnet_nodes.clone() {
      //   let peer_balance = T::Currency::free_balance(&peer.account_id);

      //   ensure!(
      //     peer_balance >= min_stake_as_balance.unwrap(),
      //     Error::<T>::NotEnoughMinStakeBalance
      //   );
      // }

      // // Insert proposal
      // let activate_proposal_index = ActivatePropCount::<T>::get();

      // ActivateProps::<T>::insert(
      //   activate_proposal_index,
      //   ActivatePropsParams {
      //     path: path.clone(),
      //     subnet_nodes: subnet_nodes.clone(),
      //     max_block: Self::convert_block_as_u64(<frame_system::Pallet<T>>::block_number() + T::VotingPeriod::get()),
      //   },
      // );

      // PropsPathStatus::<T>::insert(path.clone(), PropsStatus::ActivateVoting);

      // ActivatePropCount::<T>::put(activate_proposal_index + 1);

      // ...

      // Vote power based on a time and balance weighted mechanism
    
      // ...

      // Queue each model peer to be called in an offchain-worker using their PeerId, IP, and Port to ensure model is running
      // Must be called multiple times with a 100% success rate over the course of the minimum required voting time span

      // ...
      Ok(())
    }

    #[pallet::call_index(2)]
    #[pallet::weight(0)]
    pub fn propose_deactivate(
      origin: OriginFor<T>, 
      model_id: u32,
      reason: VoteOutReason,
      explanation: Vec<u8>,
    ) -> DispatchResult {
			// let account_id: T::AccountId = ensure_signed(origin)?;

      // // Ensure path exists in Network
			// ensure!(
			// 	T::SubnetVote::get_model_id_exist(model_id.clone()),
			// 	Error::<T>::SubnetIdNotExists
			// );

      // let model_data = T::SubnetVote::get_model_data(model_id.clone());
      // let path = model_data.path;

      // // Ensure can propose new model path
      // let proposal_status = PropsPathStatus::<T>::get(path.clone());

      // ensure!(
      //   proposal_status != PropsStatus::ActivateVoting ||
      //   proposal_status != PropsStatus::DectivateVoting ||
      //   proposal_status != PropsStatus::Deactivated,
      //   Error::<T>::ProposalInvalid
      // );
      
      Ok(())
    }

    #[pallet::call_index(3)]
    #[pallet::weight(0)]
    pub fn vote_activation_yes(
      origin: OriginFor<T>, 
      proposal_index: PropIndex,
      vote_amount: BalanceOf<T>,
    ) -> DispatchResult {
			// let account_id: T::AccountId = ensure_signed(origin.clone())?;

      // ensure!(
			// 	ActivateProps::<T>::contains_key(proposal_index.clone()),
			// 	Error::<T>::ProposalNotExist
			// );

      // let activation_proposal = ActivateProps::<T>::get(proposal_index.clone());
      // let path = activation_proposal.path;

      // let proposal_status = PropsPathStatus::<T>::get(path.clone());

      // ensure!(
      //   proposal_status == PropsStatus::ActivateVoting,
      //   Error::<T>::ProposalInvalid
      // );

      // let block = Self::get_current_block_as_u64();
      // let max_block = activation_proposal.max_block;

      // ensure!(
			// 	block < max_block,
			// 	Error::<T>::VotingPeriodInvalid
			// );

      // let balance = T::Currency::free_balance(&account_id.clone());

      // ensure!(
			// 	balance >= vote_amount,
			// 	Error::<T>::NotEnoughBalanceToVote
			// );

      // let vote_power: u128 = Self::get_voting_power(account_id.clone(), vote_amount);

      // T::Currency::set_lock(
      //   MODEL_VOTING_ID,
      //   &account_id,
      //   vote_amount,
      //   WithdrawReasons::RESERVE
      // );

      // ActivateVotes::<T>::mutate(
      //   proposal_index.clone(),
      //   |params: &mut ActivateVotesParams| {
      //     params.yay += vote_power;
      //   }
      // );

      Ok(())
    }

    #[pallet::call_index(4)]
    #[pallet::weight(0)]
    pub fn vote_activation_no(
      origin: OriginFor<T>, 
      proposal_index: PropIndex,
      vote_amount: BalanceOf<T>,
    ) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;
      Ok(())
    }

    #[pallet::call_index(5)]
    #[pallet::weight(0)]
    pub fn cast_vote(
      origin: OriginFor<T>, 
      proposal_index: PropIndex,
      vote_amount: BalanceOf<T>,
    ) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

      ensure!(
        Proposals::<T>::contains_key(proposal_index),
        Error::<T>::ProposalInvalid
      );
  
      let proposal = Proposals::<T>::get(proposal_index);
  
      Self::try_cast_vote(account_id, proposal_index, proposal, vote_amount);
  
      Ok(())
    }

    #[pallet::call_index(6)]
    #[pallet::weight(0)]
    pub fn activate_model(
      origin: OriginFor<T>, 
      proposal_index: PropIndex,
    ) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

      // Ensure model vote in passed

      Ok(())
    }

    #[pallet::call_index(7)]
    #[pallet::weight(0)]
    pub fn deactivate_model(
      origin: OriginFor<T>, 
      proposal_index: PropIndex,
    ) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

      // Ensure model vote out passed
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
  // fn try_propose_activate(account_id: T::AccountId, path: Vec<u8>, subnet_nodes: Vec<SubnetNode<T::AccountId>>) -> DispatchResult {
  //   // Check path doesn't already exist in Network or SubnetVoting
  //   // If it doesn't already exist, then it has either been not proposed or deactivated
  //   ensure!(
  //     !T::SubnetVote::get_model_path_exist(path.clone()),
  //     Error::<T>::SubnetPathExists
  //   );

  //   // Ensure can propose new model path
  //   let proposal_status = PropsPathStatus::<T>::get(path.clone());

  //   ensure!(
  //     proposal_status != PropsStatus::ActivateVoting ||
  //     proposal_status != PropsStatus::Active ||
  //     proposal_status != PropsStatus::DectivateVoting ||
  //     proposal_status != PropsStatus::Activated,
  //     Error::<T>::ProposalInvalid
  //   );

  //   // // ...

  //   // Ensure account has enough balance to pay cost of model initialization
  //   let model_initialization_cost = T::SubnetVote::get_model_initialization_cost();
  //   let model_initialization_cost_as_balance = Self::u128_to_balance(model_initialization_cost);

  //   ensure!(
  //     model_initialization_cost_as_balance.is_some(),
  //     Error::<T>::CouldNotConvertToBalance
  //   );

  //   let initializer_balance = T::Currency::free_balance(&account_id);
  //   ensure!(
  //     model_initialization_cost_as_balance.unwrap() >= initializer_balance,
  //     Error::<T>::NotEnoughSubnetInitializationBalance
  //   );

  //   // Lock balance
  //   // The final initialization fee may be more or less than the current initialization cost results
  //   T::Currency::set_lock(
  //     MODEL_VOTING_ID,
  //     &account_id,
  //     model_initialization_cost_as_balance.unwrap(),
  //     WithdrawReasons::RESERVE
  //   );
  
  //   // ...

  //   // Ensure account is already an existing peer and account eligible

  //   // Ensure minimum peers required are already met before going forward
  //   // @to-do: Get minimum model peers from network pallet
  //   ensure!(
  //     subnet_nodes.len() as u32 >= T::SubnetVote::get_min_subnet_nodes() && 
  //     subnet_nodes.len() as u32 <= T::SubnetVote::get_max_subnet_nodes(),
  //     Error::<T>::SubnetNodesLengthInvalid
  //   );

  //   // Ensure peers have the minimum required stake balance
  //   let min_stake: u128 = T::SubnetVote::get_min_stake_balance();
  //   let min_stake_as_balance = Self::u128_to_balance(min_stake);

  //   ensure!(
  //     min_stake_as_balance.is_some(),
  //     Error::<T>::CouldNotConvertToBalance
  //   );

  //   for peer in subnet_nodes.clone() {
  //     let peer_balance = T::Currency::free_balance(&peer.account_id);

  //     ensure!(
  //       peer_balance >= min_stake_as_balance.unwrap(),
  //       Error::<T>::NotEnoughMinStakeBalance
  //     );
  //   }

  //   // Insert proposal
  //   let activate_proposal_index = ActivatePropCount::<T>::get();


  //   ActivateProps::<T>::insert(
  //     activate_proposal_index,
  //     ActivatePropsParams {
  //       path: path.clone(),
  //       subnet_nodes: subnet_nodes.clone(),
  //       max_block: Self::convert_block_as_u64(<frame_system::Pallet<T>>::block_number() + T::VotingPeriod::get()),
  //     },
  //   );

  //   PropsPathStatus::<T>::insert(path.clone(), PropsStatus::ActivateVoting);

  //   ActivatePropCount::<T>::put(activate_proposal_index + 1);

  //   Ok(())
  // }

  fn try_propose_activate(account_id: T::AccountId, path: Vec<u8>, subnet_nodes: Vec<SubnetNode<T::AccountId>>) -> DispatchResult {
    // --- Ensure path doesn't already exist in Network or SubnetVoting
    // If it doesn't already exist, then it has either been not proposed or deactivated
    ensure!(
      !T::SubnetVote::get_model_path_exist(path.clone()),
      Error::<T>::SubnetPathExists
    );

    // --- Ensure proposal on model path not already in progress
    let proposal_status = PropsPathStatus::<T>::get(path.clone());

    ensure!(
      proposal_status != PropsStatus::Active,
      Error::<T>::ProposalInvalid
    );

    // --- Ensure account has enough balance to pay cost of model initialization
    let model_initialization_cost = T::SubnetVote::get_model_initialization_cost();
    let model_initialization_cost_as_balance = Self::u128_to_balance(model_initialization_cost);

    ensure!(
      model_initialization_cost_as_balance.is_some(),
      Error::<T>::CouldNotConvertToBalance
    );

    let initializer_balance = T::Currency::free_balance(&account_id);
    ensure!(
      model_initialization_cost_as_balance.unwrap() >= initializer_balance,
      Error::<T>::NotEnoughSubnetInitializationBalance
    );

    // --- Lock balance to be used once succeeded, otherwise it is freed on defeat
    // The final initialization fee may be more or less than the current initialization cost results
    T::Currency::set_lock(
      MODEL_VOTING_ID,
      &account_id,
      model_initialization_cost_as_balance.unwrap(),
      WithdrawReasons::RESERVE
    );
  
    // --- Ensure minimum peers required are already met before going forward
    // @to-do: Get minimum model peers from network pallet
    ensure!(
      subnet_nodes.len() as u32 >= T::SubnetVote::get_min_subnet_nodes() && 
      subnet_nodes.len() as u32 <= T::SubnetVote::get_max_subnet_nodes(),
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

    // --- Begin to begin voting

    Ok(())
  }

  fn try_propose_deactivate(account_id: T::AccountId, path: Vec<u8>) -> DispatchResult {
    // --- Ensure model ID exists to be removed
    ensure!(
      T::SubnetVote::get_model_id_by_path(path.clone()) != 0,
      Error::<T>::SubnetIdNotExists
    );

    // --- Ensure proposal on model path not already in progress
    let proposal_status = PropsPathStatus::<T>::get(path.clone());

    ensure!(
      proposal_status != PropsStatus::Active,
      Error::<T>::ProposalInvalid
    );
    
    Ok(())
  }

  fn try_cast_vote(
    account_id: T::AccountId, 
    proposal_index: PropIndex, 
    proposal: PropsParams<T::AccountId>,
    vote_amount: BalanceOf<T>
  ) -> DispatchResult {
    let proposal_status = proposal.proposal_status;

    ensure!(
      proposal_status == PropsStatus::Active,
      Error::<T>::ProposalInvalid
    );

    let block = Self::get_current_block_as_u64();
    let max_block = proposal.max_block;

    ensure!(
      block < max_block,
      Error::<T>::VotingPeriodInvalid
    );

    let balance = T::Currency::free_balance(&account_id.clone());

    ensure!(
      balance >= vote_amount,
      Error::<T>::NotEnoughBalanceToVote
    );

    let vote_power: u128 = Self::get_voting_power(account_id.clone(), vote_amount);

    T::Currency::set_lock(
      MODEL_VOTING_ID,
      &account_id,
      vote_amount,
      WithdrawReasons::RESERVE
    );

    Votes::<T>::mutate(
      proposal_index.clone(),
      |params: &mut VotesParams| {
        params.yay += vote_power;
      }
    );

    Ok(())
  }

  fn get_voting_power(account_id: T::AccountId, balance: BalanceOf<T>) -> u128 {
    // let is_submittable_subnet_node_account: bool = T::SubnetVote::is_submittable_subnet_node_account(account_id);

    // if is_submittable_subnet_node_account {
    //   let peer_vote_premium = Perbill::from_rational(NodeVotePremium::<T>::get(), 100 as u128);
    //   let voting_power = balance.saturating_add(peer_vote_premium * balance);
    //   return Self::balance_to_u128(voting_power)
    // }

    // Self::balance_to_u128(balance)
    0
  }
}

impl<T: Config> Pallet<T> {
  pub fn u128_to_balance(
    input: u128,
  ) -> Option<
    <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance,
  > {
    input.try_into().ok()
  }

  pub fn balance_to_u128(
    input: BalanceOf<T>,
  ) -> u128 {
    // input.saturated_into::<u128>()
    // input as u128
    // input.try_into().ok().expect("REASON")

    return match input.try_into() {
      Ok(_result) => _result,
      Err(_error) => 0,
    }
  }

  pub fn get_current_block_as_u64() -> u64 {
    TryInto::try_into(<frame_system::Pallet<T>>::block_number())
      .ok()
      .expect("blockchain will not exceed 2^64 blocks; QED.")
  }

  pub fn convert_block_as_u64(block: BlockNumberFor<T>) -> u64 {
    TryInto::try_into(block)
      .ok()
      .expect("blockchain will not exceed 2^64 blocks; QED.")
  }
}