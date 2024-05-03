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

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use codec::{Decode, Encode};
use frame_system::{self as system, ensure_signed};
use frame_support::{
	dispatch::{DispatchResult, Vec},
	ensure,
	traits::{tokens::WithdrawReasons, Get, Currency, ExistenceRequirement},
};
use sp_runtime::RuntimeDebug;
use scale_info::prelude::string::String;
use sp_core::OpaquePeerId as PeerId;
use sp_std::collections::{btree_map::BTreeMap, btree_set::BTreeSet};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

mod consensus;
mod utils;
mod math;
mod admin;
mod staking;
mod emission;
mod info;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*};
	use frame_system::pallet_prelude::*;
	use serde::{Deserialize, Serialize};
	use sp_std::{prelude::*, str};
	use sp_runtime::traits::TrailingZeroInput;

	/// This pallet's configuration trait
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

    /// The overarching event type.
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    type Currency: Currency<Self::AccountId> + Send + Sync;

    #[pallet::constant]
    type StringLimit: Get<u32>;

    #[pallet::constant] // Initial transaction rate limit.
    type InitialTxRateLimit: Get<u64>;
	}

	/// Events for the pallet.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// Models
		ModelAdded { account: T::AccountId, model_id: u32, model_path: Vec<u8>, block: u64 },
		ModelRemoved { account: T::AccountId, model_id: u32, model_path: Vec<u8>, reason: Vec<u8>, block: u64 },

		// Model Peers
		ModelPeerAdded { model_id: u32, account_id: T::AccountId, peer_id: PeerId, block: u64 },
		ModelPeerUpdated { model_id: u32, account_id: T::AccountId, peer_id: PeerId, block: u64 },
		ModelPeerRemoved { model_id: u32, account_id: T::AccountId, peer_id: PeerId, block: u64 },

		// Consensus
		ConsensusDataSubmitted(u32, T::AccountId, Vec<ModelPeerData>),
		ConsensusDataUnconfirmed(u32, T::AccountId),

		// Emissions
		EmissionsGenerated(u32, u128),

		// Stake
		StakeAdded(u32, T::AccountId, u128),
		StakeRemoved(u32, T::AccountId, u128),

		// Admin 
		SetVoteModelIn(Vec<u8>),
    SetVoteModelOut(Vec<u8>),
    SetMaxModels(u32),
    SetMinModelPeers(u32),
    SetMaxModelPeers(u32),
    SetMinStakeBalance(u128),
    SetTxRateLimit(u64),
    SetMaxZeroConsensusEpochs(u32),
    SetMinRequiredModelConsensusSubmitEpochs(u64),
    SetMinRequiredPeerConsensusSubmitEpochs(u64),
    SetMinRequiredPeerConsensusEpochs(u64),
    SetMaximumOutlierDeltaPercent(u8),
    SetModelPeerConsensusSubmitPercentRequirement(u128),
    SetConsensusBlocksInterval(u64),
    SetPeerRemovalThreshold(u128),
    SetMaxModelRewardsWeight(u128),
		SetStakeRewardWeight(u128),
		SetModelPerPeerInitCost(u128),
		SetModelConsensusUnconfirmedThreshold(u128),
		SetRemoveModelPeerEpochPercentage(u128),		
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Errors should have helpful documentation associated with them.
		PeerConsensusSubmitEpochNotReached,
		/// Maximum models reached
		MaxModels,
		/// Account has model peer under model already
		ModelPeerExist,
		/// Peer ID already in use
		PeerIdExist,
		/// Model peer doesn't exist
		ModelPeerNotExist,
		/// Model already exists
		ModelExist,
		/// Model doesn't exist
		ModelNotExist,
		/// Minimum required model peers not reached
		ModelPeersMin,
		/// Maximum allowed model peers reached
		ModelPeersMax,
		/// Model has not been voted in
		ModelNotVotedIn,
		/// Model not validated to be removed
		ModelCantBeRemoved,
		/// Account is eligible
		AccountEligible,
		/// Account is ineligible
		AccountIneligible,
		// invalid submit consensus block
		/// Cannot submit consensus during invalid blocks
		InvalidSubmitConsensusBlock,
		/// Cannot remove model peer during invalid blocks
		InvalidRemoveOrUpdateModelPeerBlock,
		/// Transaction rate limiter exceeded
		TxRateLimitExceeded,
		/// PeerId format invalid
		InvalidPeerId,
		/// IP address format invalid
		InvalidIpAddress,
		/// Port invalid
		InvalidPort,
		// Admin
		/// Consensus block interval invalid, must reach minimum
		InvalidConsensusBlocksInterval,
		/// Invalid maximimum models, must not exceed maximum allowable
		InvalidMaxModels,
		/// Invalid min model peers, must not be less than minimum allowable
		InvalidMinModelPeers,
		/// Invalid maximimum model peers, must not exceed maximimum allowable
		InvalidMaxModelPeers,
		/// Invalid minimum stake balance, must be greater than or equal to minimim required stake balance
		InvalidMinStakeBalance,
		/// Invalid percent number, must be in 1e4 format. Used for elements that only require correct format
		InvalidPercent,
		/// Invalid model peer consensus submit percent requirement
		InvalidModelPeerConsensusSubmitPercentRequirement,
		/// Invalid percent number based on MinModelPeers as `min_value = 1 / MinModelPeers`
		// This ensures it's possible to form consensus to remove peers
		InvalidPeerRemovalThreshold,
		/// Invalid maximimum zero consensus epochs, must not exceed maximum allowable
		InvalidMaxZeroConsensusEpochs,
		/// Invalid model consensus `submit` epochs, must be greater than 2 and greater than MinRequiredPeerConsensusSubmitEpochs
		InvalidModelConsensusSubmitEpochs,
		/// Invalid peer consensus `inclusion` epochs, must be greater than 0 and less than MinRequiredPeerConsensusSubmitEpochs
		InvalidPeerConsensusInclusionEpochs,
		/// Invalid peer consensus `submit` epochs, must be greater than 1 and greater than MinRequiredPeerConsensusInclusionEpochs
		InvalidPeerConsensusSubmitEpochs,
		/// Invalid max outlier delta percentage, must be in format convertible to f64
		InvalidMaxOutlierDeltaPercent,
		/// Invalid model per peer init cost, must be greater than 0 and less than 1000
		InvalidModelPerPeerInitCost,
		/// Invalid model consensus uncunfirmed threshold, must be in 1e4 format
		InvalidModelConsensusUnconfirmedThreshold,
		/// Invalid remove model peer epoch percentage, must be in 1e4 format and greater than 20.00
		InvalidRemoveModelPeerEpochPercentage,
		// staking
		/// u128 -> BalanceOf conversion error
		CouldNotConvertToBalance,
		/// Not enough balance on Account to stake and keep alive
		NotEnoughBalanceToStake,
		/// Required unstake epochs not met based on MinRequiredUnstakeEpochs
		RequiredUnstakeEpochsNotMet,
		/// Amount will kill account
		BalanceWithdrawalError,
		/// Not enough stake to withdraw
		NotEnoughStaketoWithdraw,
		MaxStakeReached,
		// if min stake not met on both stake and unstake
		MinStakeNotReached,
		// consensus
		ModelInitializeRequirement,
		ConsensusDataInvalidLen,
		/// Invalid consensus score, must be in 1e4 format and greater than 0
		InvalidScore,
		/// Consensus data already submitted
		ConsensusDataAlreadySubmitted,
		/// Consensus data already unconfirmed
		ConsensusDataAlreadyUnconfirmed,

		/// Math multiplication overflow
		MathMultiplicationOverflow,
	}
	
	// Used for decoding API data - not in use in v1.0
	#[derive(Deserialize, Serialize)]
	struct SerdePeerId {
		peer_uid: String,
	}

	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct ModelData {
		pub id: u32,
		pub path: Vec<u8>,
		pub initialized: u64,
	}

	// The submit consensus data format
	// Scoring is calculated off-chain between model peers hosting AI models together
	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct ModelPeerData {
		pub peer_id: PeerId,
		pub score: u128,
	}
	// #[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	// pub struct ModelPeerData<AccountId> {
	// 	pub account_id: AccountId,
	// 	pub peer_id: PeerId,
	// 	pub score: u64,
	// }

	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct ModelPeer<AccountId> {
		pub account_id: AccountId,
		pub peer_id: PeerId,
		pub ip: Vec<u8>,
		pub port: u16,
		pub initialized: u64,
	}

	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct ModelPeerConsensusResultsParams<AccountId> {
		pub account_id: AccountId,
		pub peer_id: PeerId,
		pub scores: Vec<u128>,
		pub score: u128, // average score equated when forming consensus from data
		pub successful: u32,
		pub successful_consensus: Vec<AccountId>, // peer data that gave a success on peer (peer is included)
		pub unsuccessful: u32,
		pub unsuccessful_consensus: Vec<AccountId>, // peer data that gave unsuccess on peer (peer not included)
		pub total_submits: u32,
	}

	// #[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub enum ConsensusType {
		Null,
    Submit,
    Unconfirm,
	}

	// Parameters for each model peers consensus data
	// It will store the most recent block they submitted and the type of submit
	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct ConsensusSubmissionDataParams<ConsensusType> {
		block: u64,
		consensus_type: ConsensusType,
	}

	// types
	#[pallet::type_value]
	pub fn DefaultAccountId<T: Config>() -> T::AccountId {
		T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap()
	}
	#[pallet::type_value]
	pub fn DefaultPeerRemovalThreshold<T: Config>() -> u128 {
		7500
	}
	#[pallet::type_value]
	pub fn DefaultPeerAgainstConsensusRemovalThreshold<T: Config>() -> u128 {
		2500
	}
	#[pallet::type_value]
	pub fn DefaultAccountTake<T: Config>() -> u128 {
		0
	}
	// The consensus data format
	//
	// `account_id`
	// 	• The AccountId of the model peer
	// `peer_id`
	// 	• The PeerId of the model peer
	// `scores`
	// 	• The scores of each model peer submitting data on the model peer
	// `score`
	// 	• The final score calculated from all `scores`
	// `successful`
	// 	• The count of model peers that submitted data on the model peer
	// `successful_consensus`
	// 	• Array of each model peer that submitted data on the model peer
	// `unsuccessful`
	// 	• The count of model peers that didn't submit data on the model peer
	// `unsuccessful_consensus`
	// 	• Array of each model peer that didn't submit data on the model peer
	// `total_submits`
	// 	• Count of all submits
	#[pallet::type_value]
	pub fn DefaultModelPeerConsensusResults<T: Config>() -> ModelPeerConsensusResultsParams<T::AccountId> {
		return ModelPeerConsensusResultsParams {
			account_id: T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap(),
			peer_id: PeerId(Vec::new()),
			scores: Vec::new(),
			score: 0,
			successful: 0,
			successful_consensus: Vec::new(),
			unsuccessful: 0,
			unsuccessful_consensus: Vec::new(),
			total_submits: 0,
		};
	}
	// #[pallet::type_value]
	// pub fn DefaultModelData<T: Config>() -> ModelData {
	// 	return ModelData {
	// 		id: 0,
	// 		path: Vec::new(),
	// 		initialized: 0,
	// 	};
	// }
	#[pallet::type_value]
	pub fn DefaultModelPeer<T: Config>() -> ModelPeer<T::AccountId> {
		return ModelPeer {
			account_id: T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap(),
			peer_id: PeerId(Vec::new()),
			ip: Vec::new(),
			port: 0,
			initialized: 0,
		};
	}
	// #[pallet::type_value]
	// pub fn DefaultConsensusType<T: Config>() -> ConsensusType {
	// 	return ConsensusType::Null
	// }
	#[pallet::type_value]
	pub fn DefaultConsensusSubmissionData<T: Config>() -> ConsensusSubmissionDataParams<ConsensusType> {
		return ConsensusSubmissionDataParams {
			block: 0,
			consensus_type: ConsensusType::Null,
		};
	}
	/// Must be greater than MinRequiredPeerConsensusSubmitEpochs
	#[pallet::type_value]
	pub fn DefaultMinRequiredModelConsensusSubmitEpochs<T: Config>() -> u64 {
		6
	}
	/// Must be less than MinRequiredModelConsensusSubmitEpochs
	/// Must be greater than MinRequiredPeerConsensusInclusionEpochs
	#[pallet::type_value]
	pub fn DefaultMinRequiredPeerConsensusSubmitEpochs<T: Config>() -> u64 {
		3
	}
	/// Must be less than MinRequiredPeerConsensusSubmitEpochs
	#[pallet::type_value]
	pub fn DefaultMinRequiredPeerConsensusInclusionEpochs<T: Config>() -> u64 {
		2
	}
	#[pallet::type_value]
	pub fn DefaultConsensusBlocksInterval<T: Config>() -> u64 {
		20
	}
	#[pallet::type_value]
	pub fn DefaultModelPeersInitializationEpochs<T: Config>() -> u64 {
		100
	}
	#[pallet::type_value]
	pub fn DefaultRemoveModelPeerEpochPercentage<T: Config>() -> u128 {
		2000
	}
	#[pallet::type_value]
	pub fn DefaultMinRequiredUnstakeEpochs<T: Config>() -> u64 {
		10
	}
	#[pallet::type_value]
	pub fn DefaultMinModelPeers<T: Config>() -> u32 {
		12
	}
	#[pallet::type_value]
	pub fn DefaultMaxModelPeers<T: Config>() -> u32 {
		255
	}
	#[pallet::type_value]
	pub fn DefaultMaxModels<T: Config>() -> u32 {
		10
	}
	#[pallet::type_value]
	pub fn DefaultModelPerPeerInitCost<T: Config>() -> u128 {
		28e+18 as u128
	}
	#[pallet::type_value]
	pub fn DefaultTxRateLimit<T: Config>() -> u64 {
		T::InitialTxRateLimit::get()
	}
	#[pallet::type_value]
	pub fn DefaultLastTxBlock<T: Config>() -> u64 {
		0
	}
	#[pallet::type_value]
	pub fn DefaultConsensusTxRateLimit<T: Config>() -> u64 {
		0
	}
	#[pallet::type_value]
	pub fn DefaultLastConsensusTxBlock<T: Config>() -> u64 {
		0
	}
	#[pallet::type_value]
	pub fn DefaultAccountPenaltyCount<T: Config>() -> u32 {
		0
	}
	#[pallet::type_value]
	pub fn DefaultMaxStakeBalance<T: Config>() -> u128 {
		1000000000000000000000000
	}
	#[pallet::type_value]
	pub fn DefaultMinStakeBalance<T: Config>() -> u128 {
		1000e+18 as u128
	}
	#[pallet::type_value]
	pub fn DefaultStakeRewardWeight<T: Config>() -> u128 {
		4000
	}
	#[pallet::type_value]
	pub fn DefaultMaxModelRewardsWeight<T: Config>() -> u128 {
		4000
	}	
	#[pallet::type_value]
	pub fn DefaultModelPeerConsensusPercentRequirement<T: Config>() -> u8 {
		75
	}
	#[pallet::type_value]
	pub fn DefaultModelPeerConsensusSubmitPercentRequirement<T: Config>() -> u128 {
		5100
	}
	#[pallet::type_value]
	pub fn DefaultMaximumOutlierDeltaPercent<T: Config>() -> u8 {
		1
	}
	#[pallet::type_value]
	pub fn DefaultMaxAccountPenaltyCount<T: Config>() -> u32 {
		12
	}
	#[pallet::type_value]
	pub fn DefaultMaxModelPeerConsecutiveConsensusNotSent<T: Config>() -> u32 {
		2
	}
	#[pallet::type_value]
	pub fn DefaultModelConsensusUnconfirmedThreshold<T: Config>() -> u128 {
		5100
	}
	#[pallet::type_value]
	pub fn DefaultMaxModelConsensusUnconfirmedConsecutiveEpochs<T: Config>() -> u32 {
		2
	}
	#[pallet::type_value]
	pub fn DefaultPeerConsensusEpochSubmitted<T: Config>() -> bool {
		false
	}
	#[pallet::type_value]
	pub fn DefaultPeerConsensusEpochUnconfirmed<T: Config>() -> bool {
		false
	}
	#[pallet::type_value]
	pub fn DefaultMaxZeroConsensusEpochs<T: Config>() -> u32 {
		100
	}
	#[pallet::type_value]
	pub fn DefaultMaxModelConsensusEpochsErrors<T: Config>() -> u32 {
		100
	}
	#[pallet::type_value]
	pub fn DefaultMaxModelResponseErrors<T: Config>() -> u32 {
		100
	}
	#[pallet::type_value]
	pub fn DefaultModelsInConsensus<T: Config>() -> Vec<u32> {
		Vec::new()
	}
	#[pallet::type_value]
	pub fn DefaultModelConsecutiveEpochsThreshold<T: Config>() -> u32 {
		100
	}
	#[pallet::type_value]
	pub fn DefaultModelPeerConsecutiveConsensusSent<T: Config>() -> u32 {
		0
	}
	#[pallet::type_value]
	pub fn DefaultModelPeerConsecutiveConsensusNotSent<T: Config>() -> u32 {
		2
	}
	
	#[pallet::storage] // model_path => boolean
	pub type ModelActivated<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, bool>;

	#[pallet::storage]
	#[pallet::getter(fn max_models)] // max models at any given time
	pub type MaxModels<T> = StorageValue<_, u32, ValueQuery, DefaultMaxModels<T>>;

	// Ensures no duplicate model paths within the network at one time
	// If a model path is voted out, it can be voted up later on and any
	// stakes attached to the model_id won't impact the re-initialization
	// of the model path.
	#[pallet::storage]
	#[pallet::getter(fn models_v3)] // model_path --> model_id
	pub type ModelPaths<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, u32>;

	// Stores by a unique id
	#[pallet::storage] // model_id => data struct
	pub type ModelsData<T: Config> = StorageMap<_, Blake2_128Concat, u32, ModelData>;

	// Cost to initialize a new model based on count of current model peers
	// See `get_model_initialization_cost()`
	#[pallet::storage]
	pub type ModelPerPeerInitCost<T> = StorageValue<_, u128, ValueQuery, DefaultModelPerPeerInitCost<T>>;
	
	// Percentage of the beginning of an epoch for model peer to exit blockchain storage
	// At the beginning of each epoch, model peers can exit the blockchain, but only within this time frame
	// represented as a percentage of the epoch
	#[pallet::storage]
	pub type RemoveModelPeerEpochPercentage<T> = StorageValue<_, u128, ValueQuery, DefaultRemoveModelPeerEpochPercentage<T>>;

	#[pallet::storage]
	#[pallet::getter(fn total_models)]
	pub type TotalModels<T> = StorageValue<_, u32, ValueQuery>;

	// Amount of epochs for model peers to attach to a model
	// If MinModelPeers is not reached by this time anyone can remove the model
	#[pallet::storage]
	pub type ModelPeersInitializationEpochs<T> = StorageValue<_, u64, ValueQuery, DefaultModelPeersInitializationEpochs<T>>;

	// Minimum amount of peers in a model
	// required for model activity
	#[pallet::storage]
	#[pallet::getter(fn min_model_peers)]
	pub type MinModelPeers<T> = StorageValue<_, u32, ValueQuery, DefaultMinModelPeers<T>>;

	// Maximim peers in a model at any given time
	#[pallet::storage]
	#[pallet::getter(fn max_model_peers)]
	pub type MaxModelPeers<T> = StorageValue<_, u32, ValueQuery, DefaultMaxModelPeers<T>>;

	// Data per model peer
	#[pallet::storage] // model_id --> account_id --> data
	#[pallet::getter(fn model_peers)]
	pub type ModelPeersData<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		ModelPeer<T::AccountId>,
		ValueQuery,
		DefaultModelPeer<T>,
	>;

	// Tracks each model an account is a model peer on
	// This is used as a helper when removing accounts from all models they are peers on
	#[pallet::storage] // account_id --> [model_ids]
	pub type AccountModels<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Vec<u32>, ValueQuery>;

	// Total count of model peers within a model
	#[pallet::storage] // model_uid --> peer_data
	#[pallet::getter(fn total_model_peers)]
	pub type TotalModelPeers<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, u32, ValueQuery>;

	// Used for unique peer_ids
	#[pallet::storage] // model_id --> account_id --> peer_id
	#[pallet::getter(fn model_peer_account)]
	pub type ModelPeerAccount<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		PeerId,
		T::AccountId,
		ValueQuery,
		DefaultAccountId<T>,
	>;

	// Used primarily for staking as a fail safe if models or peers get removed due to being out of consensus
	// Unlike ModelPeersData this never deletes until staking is 0
	// u64 is either the initialized block or the removal block
	//		Updates to block of add or remove peer, whichever is latest
	//		This works with MinRequiredUnstakeEpochs
	#[pallet::storage] // model_id --> (account_id, (initialized or removal block))
	pub type ModelAccount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		BTreeMap<T::AccountId, u64>,
		ValueQuery,
	>;

	// Amount of epochs for removed models peers required to unstake
	#[pallet::storage]
	pub type MinRequiredUnstakeEpochs<T> = StorageValue<_, u64, ValueQuery, DefaultMinRequiredUnstakeEpochs<T>>;

	// Total stake sum of all peers in all models
	#[pallet::storage] // ( total_stake )
	#[pallet::getter(fn total_stake)]
	pub type TotalStake<T: Config> = StorageValue<_, u128, ValueQuery>;

	// Total stake sum of all peers in specified model
	#[pallet::storage] // model_uid --> peer_data
	#[pallet::getter(fn total_model_stake)]
	pub type TotalModelStake<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, u128, ValueQuery>;

	// An accounts stake per model
	#[pallet::storage] // account--> model_id --> u128
	#[pallet::getter(fn account_model_stake)]
	pub type AccountModelStake<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Identity,
		u32,
		u128,
		ValueQuery,
		DefaultAccountTake<T>,
	>;

	// An accounts stake across all models
	#[pallet::storage] // account_id --> all models balance
	#[pallet::getter(fn total_account_stake)]
	pub type TotalAccountStake<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u128, ValueQuery>;

	// Maximum stake balance per model
	// Only checked on `do_add_stake` and `generate_emissions`
	// A model staker can have greater than the max stake balance although any rewards
	// they would receive based on their stake balance will only account up to the max stake balance allowed
	#[pallet::storage]
	pub type MaxStakeBalance<T: Config> = StorageValue<_, u128, ValueQuery, DefaultMaxStakeBalance<T>>;

	// Minimum required model peer stake balance per model
	#[pallet::storage]
	pub type MinStakeBalance<T: Config> = StorageValue<_, u128, ValueQuery, DefaultMinStakeBalance<T>>;
		
	// Rate limit
	#[pallet::storage] // ( tx_rate_limit )
	pub type TxRateLimit<T> = StorageValue<_, u64, ValueQuery, DefaultTxRateLimit<T>>;

	// Last transaction on rate limited functions
	#[pallet::storage] // key --> last_block
	pub type LastTxBlock<T: Config> =
		StorageMap<_, Identity, T::AccountId, u64, ValueQuery, DefaultLastTxBlock<T>>;

	// *NOT IMPLEMENTED
	#[pallet::storage] // ( tx_rate_limit )
	pub type ConsensusTxRateLimit<T> = StorageValue<_, u64, ValueQuery, DefaultConsensusTxRateLimit<T>>;

	// *NOT IMPLEMENTED
	#[pallet::storage] // key --> last_block
	pub type LastConsensusTxBlock<T: Config> =
		StorageMap<_, Identity, T::AccountId, u64, ValueQuery, DefaultLastConsensusTxBlock<T>>;
	
	// Current consensus epoch
	// *NOT IMPLEMENTED
	#[pallet::storage]
	#[pallet::getter(fn peer_consensus_epoch)]
	pub type ModelPeerConsensusEpoch<T> = StorageValue<_, u64, ValueQuery>;

	// Vector of model_ids stored during `form_consensus()` then used to generate rewards in `generate_emissions()`
	// This is by default an empty vector and resets back to an empty vector each time `generate_emissions()` is called
	#[pallet::storage]
	pub type ModelsInConsensus<T> = StorageValue<_, Vec<u32>, ValueQuery, DefaultModelsInConsensus<T>>;

	// total consensus submits and unconfirms on epoch, reset each epoch
	#[pallet::storage]
	#[pallet::getter(fn model_total_consensus_submits)] 
	pub type ModelTotalConsensusSubmits<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;

	// The threshold of epochs for a models consensus successes to reach to increment error count down
	#[pallet::storage]
	pub type ModelConsecutiveEpochsThreshold<T> = StorageValue<_, u32, ValueQuery, DefaultModelConsecutiveEpochsThreshold<T>>;

	// The total count of successful epochs in a row
	#[pallet::storage]
	pub type ModelConsecutiveSuccessfulEpochs<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;

	// Max epochs where consensus isn't formed before model being removed
	#[pallet::storage]
	pub type MaxModelConsensusEpochsErrors<T> = StorageValue<_, u32, ValueQuery, DefaultMaxModelConsensusEpochsErrors<T>>;
	
	// Count of epochs a model has consensus errors
	// This can incrase on the following issues:
	//	1. Not enough submit-able peers submitted consensus data.
	//	2. The model doesn't reach the required 0.01% stake balance towards the model versus all other live models.
	//	3. The model consensus submission data is `unconfirmed` sequentially too many times based on
	//				MaxModelConsensusUnconfirmedConsecutiveEpochs
	//				ModelConsensusUnconfirmedConsecutiveEpochsCount
	//
	#[pallet::storage] // model_id => count
	pub type ModelConsensusEpochsErrors<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;

	// The Max errors from expected return values when calling a model
	// *NOT IMPLEMENTED YET
	#[pallet::storage]
	pub type MaxModelResponseErrors<T> = StorageValue<_, u32, ValueQuery, DefaultMaxZeroConsensusEpochs<T>>;

	// Tracks errors from expected return values when calling a model
	// Stored count of model response errors
	// Ran through offchain worker
	// Stored by validator
	// *NOT IMPLEMENTED YET
	#[pallet::storage] // model_id --> errors count
	pub type ModelResponseErrors<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;
	
	// The max errors from expected return values when calling a model through a model peer's server
	// *NOT IMPLEMENTED YET
	#[pallet::storage]
	pub type MaxAccountResponseErrors<T> = StorageValue<_, u32, ValueQuery, DefaultMaxZeroConsensusEpochs<T>>;

	// Tracks errors from expected return values when calling a model through a model peer's server
	// Stored count of account model response errors
	// Ran through offchain worker
	// Stored by validator
	// *NOT IMPLEMENTED YET
	#[pallet::storage] // model_id --> errors count
	pub type AccountResponseErrors<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		u32,
		ValueQuery,
	>;

	// If model peer sent consensus data during epoch
	#[pallet::storage] // model_id --> account -> boolean
	pub type PeerConsensusEpochSubmitted<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		bool,
		ValueQuery,
		DefaultPeerConsensusEpochSubmitted<T>,
	>;

	// If model model peer unconfirmed consensus data during epoch
	//
	// There should be no incentive to unconfirm consensus data outside of models out of a healthy state
	// If unconfirmed, model peer receives no rewards
	#[pallet::storage]
	pub type PeerConsensusEpochUnconfirmed<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		bool,
		ValueQuery,
		DefaultPeerConsensusEpochUnconfirmed<T>,
	>;

	// Works alongside ModelConsensusUnconfirmedThreshold
	// Count of model peers that confirm consensus should be formed
	#[pallet::storage]
	pub type ModelConsensusEpochSubmitCount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;
		
	// Percentage (100.00 as 10000) of model peers submitting consensus to deem a model in an error or alike state
	// If enough error submissions come in then consensus is skipped for the model
	// This is an important feature in case a model is unhealthy nearing the end of the epoch
	// This avoids peers that submit consensus data later in the epoch that cannot query accurate model peer scores from
	// the decentralized machine-learning model hosting network from submitting illegitimate consensus data
	#[pallet::storage]
	pub type ModelConsensusUnconfirmedThreshold<T> = StorageValue<_, u128, ValueQuery, DefaultModelConsensusUnconfirmedThreshold<T>>;

	// Works alongside ModelConsensusUnconfirmedThreshold
	// Count of model peers that confirm consensus should not be formed
	#[pallet::storage]
	pub type ModelConsensusEpochUnconfirmedCount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;

	// The max epochs to a model can sequentially be unconfirmed before incrementing ModelConsensusEpochsErrors
	// Increments ModelConsensusEpochsErrors is ModelConsensusUnconfirmedConsecutiveEpochsCount > MaxModelConsensusUnconfirmedConsecutiveEpochs
	#[pallet::storage]
	pub type MaxModelConsensusUnconfirmedConsecutiveEpochs<T> = StorageValue<_, u32, ValueQuery, DefaultMaxModelConsensusUnconfirmedConsecutiveEpochs<T>>;

	// The sequential count of epochs a model has unconfirmed its consensus data
	// This resets on a successful consensus to zero
	#[pallet::storage]
	pub type ModelConsensusUnconfirmedConsecutiveEpochsCount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;

	// The maximum amount of times in a row a model peer can miss consensus before incrementing AccountPenaltyCount
	#[pallet::storage]
	pub type MaxModelPeerConsecutiveConsensusNotSent<T> = StorageValue<
		_, 
		u32, 
		ValueQuery, 
		DefaultMaxModelPeerConsecutiveConsensusNotSent<T>
	>;
	
	// Count of how many times in a row a model peer missed consensus
	#[pallet::storage] // account_id --> u32
	pub type ModelPeerConsecutiveConsensusNotSent<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		u32,
		ValueQuery,
		DefaultModelPeerConsecutiveConsensusNotSent<T>,
	>;

	// Count of how many times in a row a model peer missed consensus
	// *NOT IN USE
	#[pallet::storage] // account_id --> u32
	pub type LatestModelPeerConsensusSubmissionData<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		ConsensusSubmissionDataParams<ConsensusType>,
		ValueQuery,
		DefaultConsensusSubmissionData<T>,
	>;
	
	// The maximum amount of times in a row a model peer can miss consensus before incrementing AccountPenaltyCount
	#[pallet::storage]
	pub type ModelPeerConsecutiveConsensusSentThreshold<T> = StorageValue<
		_, 
		u32, 
		ValueQuery, 
		DefaultMaxModelPeerConsecutiveConsensusNotSent<T>
	>;
	
	// Count of how many times in a row a model peer successfully submitted consensus
	// When submitted enough times in a row, a model peer can have their penalties incremented down
	// based on ModelPeerConsecutiveConsensusSentThreshold
	#[pallet::storage]
	pub type ModelPeerConsecutiveConsensusSent<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		u32,
		ValueQuery,
		DefaultModelPeerConsecutiveConsensusSent<T>,
	>;
	
	// Epochs required from model initialization to accept consensus submissions
	// Epochs required based on ConsensusBlocksInterval
	// Each epoch is ConsensusBlocksInterval
	// Min required epochs for a model to be in storage for based on initialized
	#[pallet::storage]
	#[pallet::getter(fn min_required_model_consensus_submit_epochs)]
	pub type MinRequiredModelConsensusSubmitEpochs<T> = StorageValue<_, u64, ValueQuery, DefaultMinRequiredModelConsensusSubmitEpochs<T>>;

	// Epochs required from peer initialization to submit consensus
	// Epochs required based on ConsensusBlocksInterval
	// Each epoch is ConsensusBlocksInterval
	// This must always be at least 1 epoch
	// Must always be greater than MinRequiredPeerConsensusInclusionEpochs
	//
	// This is used to ensure peers aren't misusing add_model_peer() function
	// Combined with MinRequiredPeerConsensusInclusionEpochs peers are required to be
	// in consensus before they can submit any data.
	// Rewards are emitted if required epochs are reached, submitted, and is in consensus
	// Peer won't receive rewards if they don't meet this requirement
	#[pallet::storage]
	#[pallet::getter(fn min_required_peer_consensus_submit_epochs)]
	pub type MinRequiredPeerConsensusSubmitEpochs<T> = StorageValue<_, u64, ValueQuery, DefaultMinRequiredPeerConsensusSubmitEpochs<T>>;

	// Epochs required to be included in consensus
	// Epochs required based on ConsensusBlocksInterval
	// Each epoch is ConsensusBlocksInterval
	// This must always be at least 1 epoch
	// Must always be less than MinRequiredPeerConsensusSubmitEpochs
	//
	// This is used to ensure peers aren't misusing add_model_peer() function
	// If a peer is not hosting a model theoretically consensus submitters will
	// have them removed before they are able to submit consensus data.
	#[pallet::storage]
	#[pallet::getter(fn min_required_consensus_epochs)]
	pub type MinRequiredPeerConsensusInclusionEpochs<T> = StorageValue<_, u64, ValueQuery, DefaultMinRequiredPeerConsensusInclusionEpochs<T>>;

	// Consensus data submitted and filtered per epoch
	#[pallet::storage] // model => enum => consensus results
	#[pallet::getter(fn model_peer_consensus_results)]
	pub type ModelPeerConsensusResults<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		ModelPeerConsensusResultsParams<T::AccountId>,
		ValueQuery,
		DefaultModelPeerConsensusResults<T>,
	>;

	// Maximum delta a score can be from the average without incurring penalties
	#[pallet::storage]
	pub type MaximumOutlierDeltaPercent<T> = StorageValue<_, u8, ValueQuery, DefaultMaximumOutlierDeltaPercent<T>>;

	// Maximum model peer penalty count
	#[pallet::storage]
	pub type MaxAccountPenaltyCount<T> = StorageValue<_, u32, ValueQuery, DefaultMaxAccountPenaltyCount<T>>;

	// Count of times a peer is against consensus
	// This includes:
	// 1. being against other peers that conclude another peer is out of consensus
	// 2. being against other peers that conclude another peer is in consensus
	// 3. score delta is too high on consensus data submission
	// 4. not submitting consensus data
	#[pallet::storage] // account_id --> u32
	#[pallet::getter(fn model_peer_penalty_count)]
	pub type AccountPenaltyCount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		u32,
		ValueQuery,
		DefaultAccountPenaltyCount<T>
	>;

	// Required percentage of peers submitting consensus in relation to submittable peers to form consensus and generate rewards
	#[pallet::storage]
	#[pallet::getter(fn model_peer_consensus_submit_percent_requirement)]
	pub type ModelPeerConsensusSubmitPercentRequirement<T: Config> = 
		StorageValue<_, u128, ValueQuery, DefaultModelPeerConsensusSubmitPercentRequirement<T>>;
		

	// Blocks per consensus form
	#[pallet::storage]
	pub type ConsensusBlocksInterval<T: Config> = StorageValue<_, u64, ValueQuery, DefaultConsensusBlocksInterval<T>>;

	// Consensus threshold percentage for peer to be removed
	// If a peer is not sent in by enough of other peers based on PeerRemovalThreshold
	// They will be removed as a peer and will not longer generate incentives
	#[pallet::storage]
	pub type PeerRemovalThreshold<T: Config> = StorageValue<_, u128, ValueQuery, DefaultPeerRemovalThreshold<T>>;

	// Threshold percentage for peer to be removed
	// If a peer is against consensus in relation to the count of all consensus submissions
	// They will be removed as a peer and will not longer generate incentives
	// e.g. If a peer is against consensus passed the threshold on one epoch, they will gain
	//			AccountPenaltyCount's and also be removed as a model peer
	#[pallet::storage]
	pub type PeerAgainstConsensusRemovalThreshold<T: Config> = StorageValue<_, u128, ValueQuery, DefaultPeerAgainstConsensusRemovalThreshold<T>>;

	// // The max amount of times a peer can reach the PeerRemovalThreshold before being removed
	// // It's possible for a peer to not be "ONLINE" while they are online because their peer
	// #[pallet::storage]
	// pub type MaxPeerOutOfConsensusCount<T> = StorageValue<_, u32, ValueQuery>;

	// #[pallet::storage] // account_id --> u32
	// pub type PeerOutOfConsensusCount<T: Config> = StorageMap<
	// 	_,
	// 	Blake2_128Concat,
	// 	T::AccountId,
	// 	u32,
	// 	ValueQuery,
	// >;

	#[pallet::storage] // stores epoch balance of rewards from block rewards to be distributed to peers/stakers
	#[pallet::getter(fn stake_vault_balance)]
	pub type StakeVaultBalance<T> = StorageValue<_, u128, ValueQuery>;

	// Format is 1e4 as 100.00% = 10000
	#[pallet::storage] // peer staking weight towards rewards vs. score
	pub type StakeRewardWeight<T> = StorageValue<_, u128, ValueQuery, DefaultStakeRewardWeight<T>>;

	// Format is 1e4 as 100.00% = 10000
	#[pallet::storage] // maximum percentage of rewards a model can have per epoch
	pub type MaxModelRewardsWeight<T> = StorageValue<_, u128, ValueQuery, DefaultMaxModelRewardsWeight<T>>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit consensus data per epoch
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::submit_consensus_data())]
		pub fn submit_consensus_data(
			origin: OriginFor<T>,
			model_id: u32,
			error: bool,
			consensus_data: Vec<ModelPeerData>,
		) -> DispatchResultWithPostInfo {
			let account_id: T::AccountId = ensure_signed(origin)?;

			let consensus_blocks_interval: u64 = ConsensusBlocksInterval::<T>::get();
			let block: u64 = Self::get_current_block_as_u64();

			// Ensure account is eligible
			ensure!(
				Self::is_account_eligible(account_id.clone()),
				Error::<T>::AccountIneligible
			);
					
			// Ensure 
			// • Consensus isn't being formed 
			// • Emissions aren't being generated
			// • Model peers can't be removed
			// Submitting consensus data during consensus steps will interfere
			// with the storage elements required to run consensus steps
			ensure!(
				Self::can_submit_consensus(block, consensus_blocks_interval),
				Error::<T>::InvalidSubmitConsensusBlock
			);

			// Ensure model exists
			ensure!(
				ModelsData::<T>::contains_key(model_id.clone()),
				Error::<T>::ModelNotExist
			);
			
			// Ensure model peer exists
			ensure!(
				ModelPeersData::<T>::contains_key(model_id.clone(), account_id.clone()),
				Error::<T>::ModelPeerNotExist
			);

			let total_model_peers: u32 = TotalModelPeers::<T>::get(model_id.clone());
			let consensus_data_len = consensus_data.len();

			// Confirm data
			// Loosely confirm submitted vector isn't greater than total_model_peers or data is None
			// Peer submitter data length must be less than or equal to 
			// the total count of peers within model and greater than zero
			ensure!(
				consensus_data_len <= total_model_peers as usize && consensus_data_len > 0,
				Error::<T>::ConsensusDataInvalidLen
			);

			let peer_consensus_epoch_submitted = PeerConsensusEpochSubmitted::<T>::get(model_id.clone(), account_id.clone());
			// Ensure peer hasn't sent consensus data already
			ensure!(
				!peer_consensus_epoch_submitted,
				Error::<T>::ConsensusDataAlreadySubmitted
			);

			// safe unwrap
			let model = ModelsData::<T>::get(model_id.clone()).unwrap();
			let model_initialized: u64 = model.initialized;

			let min_required_model_consensus_submit_epochs = MinRequiredModelConsensusSubmitEpochs::<T>::get();

			// Ensure model has passed required epochs to accept submissions and generate consensus and rewards
			// We get the eligible start block
			//
			// We use this here instead of when initializing the model or peer in order to keep the required time
			// universal in the case models or peers are added before an update to the ConsensusBlocksInterval
			//
			// e.g. Can't submit consensus if the following parameters
			//			• model initialized		0
			//			• interval 						20
			//			• epochs							10
			//			• current block 			199
			//	eligible block is 200
			// 	can't submit on 200, 201 based on is_in_consensus_steps()
			//	can submit between 202-219
			//	199 is not greater than or equal to 200, revert
			//
			// e.g. Can submit consensus if the following parameters
			//			• model initialized		0
			//			• interval 						20
			//			• epochs							10
			//			• current block 			205
			//	eligible block is 200
			// 	can't submit on 200, 201 based on is_in_consensus_steps()
			//	can submit between 202-219
			//	205 is not greater than or equal to 200, allow consensus data submission
			//
			ensure!(
				block >= Self::get_eligible_epoch_block(
					consensus_blocks_interval, 
					model_initialized, 
					min_required_model_consensus_submit_epochs
				),
				Error::<T>::ModelInitializeRequirement
			);

			let account_model_peer = ModelPeersData::<T>::get(model_id.clone(), account_id.clone());

			// Require submitter meets MinRequiredPeerConsensusSubmitEpochs
			// Peer must be initialized for minimum required blocks to submit data
			let submitter_peer_initialized: u64 = account_model_peer.initialized;

			let min_required_peer_consensus_submit_epochs: u64 = MinRequiredPeerConsensusSubmitEpochs::<T>::get();

			// We get the eligible start block
			//
			// We use this here instead of when initializing the model or peer in order to keep the required time
			// universal in the case models or peers are added before an update to the ConsensusBlocksInterval
			ensure!(
				Self::is_epoch_block_eligible(
					block, 
					consensus_blocks_interval, 
					min_required_peer_consensus_submit_epochs, 
					submitter_peer_initialized
				),
				Error::<T>::PeerConsensusSubmitEpochNotReached
			);

			// Count of eligible to submit consensus data model peers
			let total_submit_eligible_model_peers: u32 = Self::get_total_submittable_model_peers(
				model_id.clone(),
				block,
				consensus_blocks_interval,
				min_required_peer_consensus_submit_epochs
			);

			// By the time a model reaches its minimum required epochs to accept submissions, its required
			// there are enough peers initialized to submit consensus data.
			//
			// Models must be initialized with the minimum amount of peers dedicated to submitting consensus
			//
			// Ensure model has minimum required peers to submit consensus data to form consensus
			// Rewards are only given to theoretically active models
			let min_model_peers: u32 = MinModelPeers::<T>::get();
			ensure!(
				total_submit_eligible_model_peers >= min_model_peers,
				Error::<T>::ModelPeersMin
			);

			// Peers in data must meet minimum required blocks from initialization to be
			// included in consensus so they don't miss a consensus epoch if they register
			// late in the epoch.
			// This requirement of blocks gives time for other peers
			// who can submit consensus data to confirm if they are not hosting models
			// before these peers can have a chance to submit consensus data. This prevents
			// the opporunity to manipulate the consensus mechanism
			let min_required_consensus_inclusion_epochs = MinRequiredPeerConsensusInclusionEpochs::<T>::get();

			// Iter each peer to check if exist against submitted data and store results
			for model_peer in ModelPeersData::<T>::iter_prefix_values(model_id.clone()) {
				let blockchain_peer_initialized: u64 = model_peer.initialized;

				// Do not include peers that have not yet met min_required_consensus_inclusion_epochs
				// We get the eligible start block
				//
				// We use this here instead of when initializing the model or peer in order to keep the required time
				// universal in the case models or peers are added before an update to the ConsensusBlocksInterval
				let do_include: bool = Self::is_epoch_block_eligible(
					block, 
					consensus_blocks_interval, 
					min_required_consensus_inclusion_epochs, 
					blockchain_peer_initialized
				);

				if !do_include {
					continue
				}

				let blockchain_account_id: T::AccountId = model_peer.account_id;

				// Ensure account is eligible
				// This is an unlikely scenario
				let account_eligible: bool = Self::is_account_eligible(blockchain_account_id.clone());

				if !account_eligible {
					continue
				}

				// Get data of model peer that consensus data is being submitted on
				let blockchain_peer_id: PeerId = model_peer.peer_id;

				let mut contains = false;
				for data in consensus_data.iter() {
					let data_peer_id: &PeerId = &data.peer_id;

					if blockchain_peer_id.clone() == data_peer_id.clone() {
						contains = true;
						let data_score: u128 = data.score;
						// The score is a simple number designed to adapt to upgrades as
						// p2p model hosting technology progresses over time
						// to allow each model category to have it's own scoring mechanism between peers
						// alongside the progression of the hypertensor blockchain
						
            // Hardcode max score as 1e4 (100.00)
            // Hardcode min score as 1   (0.01)
						// • This requires a model peer to be hosting a minimum of .01% of the sum of total scores
						// 100.00     = 10000 or 1e4 (100/100 * 1e4)
						// 10.00      = 1000  or 1e3 (10/100 * 1e4)
            // 1.00       = 100   or 1e2 (1/100 * 1e4)
            // 0.1        = 10    or 1e1 (.1/100 * 1e4)
            // 0.01       = 1     or 1 	 (.01/100 * 1e4)
						ensure!(
							data_score <= Self::PERCENTAGE_FACTOR && data_score > 0,
							Error::<T>::InvalidScore
						);

						// We push scores only to successful peer inclusions
						// This matches scores up with successful_consensus accountId array
						ModelPeerConsensusResults::<T>::mutate(
							model_id.clone(),
							blockchain_account_id.clone(),
							|params: &mut ModelPeerConsensusResultsParams<T::AccountId>| {
								params.account_id = blockchain_account_id.clone();
								params.peer_id = blockchain_peer_id.clone();
								params.scores.push(data_score);
								params.successful += 1;
								params.successful_consensus.push(account_id.clone());
								params.total_submits += 1;
							}
						);	
						break
					}
				}

				if !contains {
					// Mutate results based on peers submitter didn't submit
					// this is used to gather data on which peers data may
					// or may not be in consensus to create consensus ratings
					// based on peers submission data
					ModelPeerConsensusResults::<T>::mutate(
						model_id.clone(),
						blockchain_account_id.clone(),
						|params: &mut ModelPeerConsensusResultsParams<T::AccountId>| {
							params.account_id = blockchain_account_id.clone();
							params.peer_id = blockchain_peer_id.clone();
							params.unsuccessful += 1;
							params.unsuccessful_consensus.push(account_id.clone());
							params.total_submits += 1;
						}
					);
				}
			}

			// Set to peer has submitted data
			// This is reset each epoch
			PeerConsensusEpochSubmitted::<T>::insert(model_id.clone(), account_id.clone(), true);

			let peer_consensus_epoch_unconfirmed = PeerConsensusEpochUnconfirmed::<T>::get(model_id.clone(), account_id.clone());

			// Increment submissions if hasn't already unconfirmed
			// Each submission or unconfirm count as a submit of consensus
			// unconfirming or submitting can only be done one time per model peer
			// This is reset each epoch
			//
			if !peer_consensus_epoch_unconfirmed {
				ModelTotalConsensusSubmits::<T>::mutate(model_id.clone(), |n: &mut u32| *n += 1);
			}

			ModelConsensusEpochSubmitCount::<T>::mutate(model_id.clone(), |n: &mut u32| *n += 1);

			// LatestModelPeerConsensusSubmissionData::<T>::insert(
			// 	model_id.clone(), 
			// 	account_id.clone(), 
			// 	ConsensusSubmissionDataParams {
			// 		block: block,
			// 		consensus_type: ConsensusType::Submit
			// 	}
			// );

			Self::deposit_event(Event::ConsensusDataSubmitted(
				model_id.clone(), 
				account_id.clone(),
				consensus_data
			));

			Ok(Pays::No.into())
		}

		/// Unconfirm this epoch and or confirm data as illegitimate
		/// Send in if there issues with the data such as the model not being in an error state
		// This gives a chance to rectify consensus and model hosting issues
		//
		// This can be called one time and can be called after or before `submit_consensus_data()` and vice versa
		// If the ModelConsensusUnconfirmedThreshold is not reached, forming consensus will continue
		//
		// This step is not required by model peers
		// e.g. If model peers submitted consensus data before a model going down before all others have submitted
		//			their consensus data, they can then confirm the data is corrupted and avoid a corrupted consensus all together
		//
		// This can only be done sequentially up to MaxModelConsensusUnconfirmedConsecutiveEpochs
		// It is up to model peers to fix the issues so they don't breach MaxModelConsensusUnconfirmedConsecutiveEpochs
		// If ModelConsensusUnconfirmedConsecutiveEpochsCount breaches its max, it will increase the ModelConsensusEpochsErrors
		//
		// If the ModelConsensusUnconfirmedThreshold is reached consensus will be skipped on the model
		#[pallet::call_index(1)]
		#[pallet::weight({0})]
		pub fn unconfirm_consensus_data(
			origin: OriginFor<T>,
			model_id: u32,
		) -> DispatchResultWithPostInfo {
			let account_id: T::AccountId = ensure_signed(origin)?;
			
			let consensus_blocks_interval: u64 = ConsensusBlocksInterval::<T>::get();
			let block: u64 = Self::get_current_block_as_u64();

			// Ensure 
			// • Consensus isn't being formed 
			// • Emissions aren't being generated
			// • Model peers can't be removed
			ensure!(
				Self::can_submit_consensus(block, consensus_blocks_interval),
				Error::<T>::InvalidSubmitConsensusBlock
			);

			// Ensure model exists
			ensure!(
				ModelsData::<T>::contains_key(model_id.clone()),
				Error::<T>::ModelNotExist
			);
			
			// Ensure model peer exists
			ensure!(
				ModelPeersData::<T>::contains_key(model_id.clone(), account_id.clone()),
				Error::<T>::ModelPeerNotExist
			);
			
			let account_model_peer = ModelPeersData::<T>::get(model_id.clone(), account_id.clone());

			// Require submitter meets MinRequiredPeerConsensusSubmitEpochs
			// Peer must be initialized for minimum required blocks to submit data
			let submitter_peer_initialized: u64 = account_model_peer.initialized;

			let min_required_peer_consensus_submit_epochs: u64 = MinRequiredPeerConsensusSubmitEpochs::<T>::get();

			// We get the eligible start block
			//
			// We use this here instead of when initializing the model or peer in order to keep the required time
			// universal in the case models or peers are added before an update to the ConsensusBlocksInterval
			// ensure!(
			// 	block >= Self::get_eligible_epoch_block(
			// 		consensus_blocks_interval, 
			// 		submitter_peer_initialized, 
			// 		min_required_peer_consensus_submit_epochs
			// 	),
			// 	Error::<T>::PeerConsensusSubmitEpochNotReached
			// );

			ensure!(
				Self::is_epoch_block_eligible(
					block, 
					consensus_blocks_interval, 
					min_required_peer_consensus_submit_epochs, 
					submitter_peer_initialized
				),
				Error::<T>::PeerConsensusSubmitEpochNotReached
			);

			// Peer can unconfirm data if:
			//	1. Has sent in consensus data already as `error: false`
			//	2. Has NOT already sent in `error: true` when calling `submit_consensus_data()`
			//
			let peer_consensus_epoch_confirm = PeerConsensusEpochUnconfirmed::<T>::get(model_id.clone(), account_id.clone());
			// Ensure peer hasn't sent consensus data unconfirm already
			ensure!(
				!peer_consensus_epoch_confirm,
				Error::<T>::ConsensusDataAlreadyUnconfirmed
			);

			// Set to peer has submitted confirmation data
			// This is reset each epoch
			PeerConsensusEpochUnconfirmed::<T>::insert(model_id.clone(), account_id.clone(), true);

			let peer_consensus_epoch_submitted = PeerConsensusEpochSubmitted::<T>::get(model_id.clone(), account_id.clone());

			// Increase consensus submits if hasn't already submitted consensus data
			// Each submission or unconfirm count as a submit of consensus
			// unconfirming or submitting can only be done one time per model peer
			// This is used against ModelConsensusEpochUnconfirmedCount to decide if the ModelConsensusUnconfirmedThreshold
			// is reached while forming consensus
			if !peer_consensus_epoch_submitted {
				ModelTotalConsensusSubmits::<T>::mutate(model_id.clone(), |n: &mut u32| *n += 1);
			}

			// Increase model consensus unconfirmed count
			ModelConsensusEpochUnconfirmedCount::<T>::mutate(model_id.clone(), |n: &mut u32| *n += 1);

			// LatestModelPeerConsensusSubmissionData::<T>::insert(
			// 	model_id.clone(), 
			// 	account_id.clone(), 
			// 	ConsensusSubmissionDataParams {
			// 		block: block,
			// 		consensus_type: ConsensusType::Unconfirm
			// 	}
			// );

			Self::deposit_event(Event::ConsensusDataUnconfirmed(model_id.clone(), account_id.clone()));

			Ok(Pays::No.into())
		}

		/// Add model
		/// Model must be activated through voting mechanism
		/// A fee is required to initialize the model that goes to current model validators
		// TESTNET V1
		// • No democracy with focus on peer consensus only
		// • Specific models are automatically initialized as `voted` in
		// TESTNET V2
		// • Will implement a democratic voting process required to add models based on
		//	 balance and time weighted voting that impacts ModelActivated
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::add_model())]
		// #[pallet::weight({0})]
		pub fn add_model(
			origin: OriginFor<T>, 
			model_path: Vec<u8>,
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

			// Ensure path is unique
			ensure!(
				!ModelPaths::<T>::contains_key(model_path.clone()),
				Error::<T>::ModelExist
			);

			let activated: bool = match ModelActivated::<T>::try_get(model_path.clone()) {
				Ok(is_active) => is_active,
				Err(()) => false,
			};

			// Ensure model voted in
			ensure!(
				activated,
				Error::<T>::ModelNotVotedIn
			);

			// Ensure max models not reached
			// Get total live models
			let total_models: u32 = (ModelsData::<T>::iter().count() + 1).try_into().unwrap();
			let max_models: u32 = MaxModels::<T>::get();
			ensure!(
				total_models <= max_models,
				Error::<T>::MaxModels
			);

			let block: u64 = Self::get_current_block_as_u64();
			let model_fee: u128 = Self::get_model_initialization_cost(block);

			if model_fee > 0 {
				let model_fee_as_balance = Self::u128_to_balance(model_fee);
				// Add non refundable fee
				// Fails on negative balance

				// *
				// Get account_id of the model vote initializer
				//

				let can_withdraw: bool = Self::can_remove_balance_from_coldkey_account(
					&account_id,
					model_fee_as_balance.unwrap(),
				);

				if !can_withdraw {
					// If fee cannot be covered then return
					return Ok(())
				}

				let _ = T::Currency::withdraw(
					&account_id,
					model_fee_as_balance.unwrap(),
					WithdrawReasons::except(WithdrawReasons::TRANSFER),
					ExistenceRequirement::KeepAlive,
				);

				// Send portion to stake rewards vault
				// Send portion to treasury

				// increase stake balance with model initialization cost
				StakeVaultBalance::<T>::mutate(|n: &mut u128| *n += model_fee);
			}

			// Get total models ever
			let model_len: u32 = TotalModels::<T>::get();
			let model_id = model_len + 1;
			
			let model_data = ModelData {
				id: model_id.clone(),
				path: model_path.clone(),
				initialized: block,
			};

			// Store unique path
			ModelPaths::<T>::insert(model_path.clone(), model_id.clone());
			// Store model data
			ModelsData::<T>::insert(model_id.clone(), model_data.clone());
			// Increase total models. This is used for unique Model IDs
			TotalModels::<T>::mutate(|n: &mut u32| *n += 1);

			Self::deposit_event(Event::ModelAdded { 
				account: account_id, 
				model_id: model_id.clone(), 
				model_path: model_path.clone(),
				block: block
			});

			Ok(())
		}

		/// Remove a model if the model has been voted out
		/// This can be done by anyone as long as the required conditions pass
		#[pallet::call_index(3)]
		#[pallet::weight({0})]
		pub fn remove_model(
			origin: OriginFor<T>, 
			model_id: u32,
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

			ensure!(
				ModelsData::<T>::contains_key(model_id.clone()),
				Error::<T>::ModelNotExist
			);

			let model = ModelsData::<T>::get(model_id.clone()).unwrap();
			let model_path: Vec<u8> = model.path;
			let model_initialized: u64 = model.initialized;

			// ----
			// Models can be removed by
			// 		1. Model can be voted off
			//		2. Model can reach max zero consensus count
			//		3. Model can be offline too many times
			//		4. Model has min peers after initialization period
			// ----

			let mut reason_for_removal: Vec<u8> = Vec::new();

			// 1.
			// Check model voted out
			let activated: bool = match ModelActivated::<T>::try_get(model_path.clone()) {
				Ok(is_active) => is_active,
				Err(()) => false,
			};

			// Push into reason
			if !activated {
				reason_for_removal.push(1)
			}

			// 2.
			// Model can reach max zero consensus count
			let zero_consensus_epochs: u32 = ModelConsensusEpochsErrors::<T>::get(model_id.clone());
			let max_zero_consensus_epochs: u32 = MaxModelConsensusEpochsErrors::<T>::get();
			let too_many_max_consensus_epochs: bool = zero_consensus_epochs > max_zero_consensus_epochs;

			// Push into reason
			if too_many_max_consensus_epochs {
				reason_for_removal.push(2)
			}

			// 3.
			// Check if model is offline too many times
			let is_offline: bool = false;

			// Push into reason
			if is_offline {
				reason_for_removal.push(3)
			}

			// 4.
			// Check if model has min amount of peers
			// If min peers are not met and initialization epochs has surpassed
			// then model can be removed
			let total_model_peers: u32 = TotalModelPeers::<T>::get(model_id.clone());
			let min_model_peers: u32 = MinModelPeers::<T>::get();
			let block: u64 = Self::get_current_block_as_u64();
			let mut has_min_peers: bool = true;
			if total_model_peers < min_model_peers {
				let consensus_blocks_interval: u64 = ConsensusBlocksInterval::<T>::get();
				let model_peers_initialization_epochs: u64 = ModelPeersInitializationEpochs::<T>::get();
				// Ensure initialization epochs have passed
				// If not return false
				let initialized: bool = block < Self::get_eligible_epoch_block(
					consensus_blocks_interval, 
					model_initialized, 
					model_peers_initialization_epochs
				);
				// Push into reason
				if !initialized {
					reason_for_removal.push(4)
				}	
			}

			// Must have at least one of the possible reasons to be removed
			ensure!(
				!activated || too_many_max_consensus_epochs || is_offline || !has_min_peers,
				Error::<T>::ModelCantBeRemoved
			);

			// Remove unique path
			ModelPaths::<T>::remove(model_path.clone());
			// Remove model data
			ModelsData::<T>::remove(model_id.clone());

			// We don't subtract TotalModels since it's used for ids

			// Remove all peers data
			let _ = ModelPeersData::<T>::clear_prefix(model_id.clone(), u32::MAX, None);
			let _ = TotalModelPeers::<T>::remove(model_id.clone());
			let _ = ModelPeerAccount::<T>::clear_prefix(model_id.clone(), u32::MAX, None);

			// Remove all model consensus data
			Self::reset_model_consensus_data_and_results(model_id.clone());
			let _ = ModelConsensusEpochsErrors::<T>::remove(model_id.clone());

			Self::deposit_event(Event::ModelRemoved { 
				account: account_id, 
				model_id: model_id.clone(), 
				model_path: model_path.clone(),
				reason: reason_for_removal,
				block: block
			});

			Ok(())
		}

		/// Add a model peer that is currently hosting an AI model
		/// A minimum stake balance is required
		// Before adding model peer you must become a peer hosting the model of choice
		// This fn will claim your peer_id and associate it with your account as peer_id => account_id
		// If this reverts due to `ModelPeerExist` you must remove the peer node and try again with a new peer_id
		// It's possible someone can claim the peer_id before you do
		// due to the requirement of staking this is an unlikely scenario.
		// Once you claim the peer_id, no one else can claim it.
		// After RequiredModelPeerEpochs pass and the peer is in consensus, rewards will be emitted to the account
		#[pallet::call_index(4)]
		// #[pallet::weight(T::WeightInfo::add_model_peer())]
		#[pallet::weight({0})]
		pub fn add_model_peer(
			origin: OriginFor<T>, 
			model_id: u32, 
			peer_id: PeerId, 
			ip: Vec<u8>,
			port: u16,
			stake_to_be_added: u128
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin.clone())?;

			let consensus_blocks_interval: u64 = ConsensusBlocksInterval::<T>::get();
			let block: u64 = Self::get_current_block_as_u64();

			// Not required since emissions are calculated for in-consensus model peers only
			// // Ensure consensus isn't being formed or emissions are being generated
			// ensure!(
			// 	!Self::is_in_consensus_steps(block, consensus_blocks_interval),
			// 	Error::<T>::InvalidSubmitConsensusBlock
			// );

			ensure!(
				ModelsData::<T>::contains_key(model_id.clone()),
				Error::<T>::ModelNotExist
			);

			// Ensure account is eligible
			ensure!(
				Self::is_account_eligible(account_id.clone()),
				Error::<T>::AccountIneligible
			);
			
			// Ensure max peers isn't surpassed
			let total_model_peers: u32 = TotalModelPeers::<T>::get(model_id.clone()) + 1;
			let max_model_peers: u32 = MaxModelPeers::<T>::get();
			ensure!(
				total_model_peers <= max_model_peers,
				Error::<T>::ModelPeersMax
			);

			// Unique model_id -> AccountId
			// Ensure account doesn't already have a peer within model
			ensure!(
				!ModelPeersData::<T>::contains_key(model_id.clone(), account_id.clone()),
				Error::<T>::ModelPeerExist
			);

			// Unique model_id -> PeerId
			// Ensure peer ID doesn't already exist within model regardless of account_id
			let peer_exists: bool = match ModelPeerAccount::<T>::try_get(model_id.clone(), peer_id.clone()) {
				Ok(_) => true,
				Err(()) => false,
			};

			ensure!(
				!peer_exists,
				Error::<T>::PeerIdExist
			);

			// Validate peer_id
			ensure!(
				Self::validate_peer_id(peer_id.clone()),
				Error::<T>::InvalidPeerId
			);

			// Validate IP Address
			ensure!(
				Self::validate_ip_address(ip.clone()),
				Error::<T>::InvalidIpAddress
			);

			// Validate port
			ensure!(Self::validate_port(port.clone()), Error::<T>::InvalidPort);

			// ====================
			// Initiate stake logic
			// ====================
			match Self::do_add_stake(
				origin.clone(), 
				model_id.clone(),
				account_id.clone(),
				stake_to_be_added,
			) {
				Ok(stake) => (),
				Err(err) => {
					ensure!(false, err);
				}
			}

			// ====================
			// Insert peer into storage
			// ====================
			let model_peer: ModelPeer<T::AccountId> = ModelPeer {
				account_id: account_id.clone(),
				peer_id: peer_id.clone(),
				ip: ip.clone(),
				port: port.clone(),
				initialized: block,
			};
			// Insert ModelPeersData with account_id as key
			ModelPeersData::<T>::insert(model_id.clone(), account_id.clone(), model_peer);

			// Insert model peer account to keep peer_ids unique within models
			ModelPeerAccount::<T>::insert(model_id.clone(), peer_id.clone(), account_id.clone());

			// Insert unstaking reinforcements
			// This data is specifically used for allowing unstaking after being removed
			// ModelAccount is not removed from storage until the peer has unstaked their entire stake balance
			// This stores the block they are initialized at
			// If removed, the initialized block will be replace with the removal block
			let mut model_accounts: BTreeMap<T::AccountId, u64> = ModelAccount::<T>::get(model_id.clone());
			// let model_account: Option<&u64> = model_accounts.get(&account_id.clone());
			let block_initialized_or_removed: u64 = match model_accounts.get(&account_id.clone()) {
				Some(block_initialized_or_removed) => *block_initialized_or_removed,
				None => 0,
			};

			// If previously removed or removed themselves
			// Ensure they have either unstaked or have waited enough epochs to unstake
			// to readd themselves as a model peer
			if block_initialized_or_removed != 0 {
				let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<T>::get();
				// Ensure min required epochs have surpassed to unstake
				// Based on either initialized block or removal block
				ensure!(
					block >= Self::get_eligible_epoch_block(
						consensus_blocks_interval, 
						block_initialized_or_removed, 
						min_required_unstake_epochs
					),
					Error::<T>::RequiredUnstakeEpochsNotMet
				);	
			}

			// Update to current block
			model_accounts.insert(account_id.clone(), block);
			ModelAccount::<T>::insert(model_id.clone(), model_accounts);

			// Add model_id to account
			// Account can only have a model peer per model so we don't check if it exists
			AccountModels::<T>::append(account_id.clone(), model_id.clone());

			// Increase total model peers
			TotalModelPeers::<T>::mutate(model_id.clone(), |n: &mut u32| *n += 1);

			Self::deposit_event(
				Event::ModelPeerAdded { 
					model_id: model_id.clone(), 
					account_id: account_id.clone(), 
					peer_id: peer_id.clone(),
					block: block
				}
			);

			Ok(())
		}

		/// Update a model peer
		#[pallet::call_index(5)]
		#[pallet::weight({0})]
		pub fn update_model_peer(
			origin: OriginFor<T>, 
			model_id: u32, 
			peer_id: PeerId,
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin.clone())?;

			// --- Ensure model exists
			ensure!(
				ModelsData::<T>::contains_key(model_id.clone()),
				Error::<T>::ModelNotExist
			);

			ensure!(
				ModelPeersData::<T>::contains_key(model_id.clone(), account_id.clone()),
				Error::<T>::ModelPeerNotExist
			);
			
			// Unique model_id -> PeerId
			// Ensure peer ID doesn't already exist within model regardless of account_id
			let peer_exists: bool = match ModelPeerAccount::<T>::try_get(model_id.clone(), peer_id.clone()) {
				Ok(_) => true,
				Err(()) => false,
			};

			ensure!(
				!peer_exists,
				Error::<T>::PeerIdExist
			);

			// Validate peer_id
			ensure!(
				Self::validate_peer_id(peer_id.clone()),
				Error::<T>::InvalidPeerId
			);
				
			let model_peer = ModelPeersData::<T>::get(model_id.clone(), account_id.clone());

			let block: u64 = Self::get_current_block_as_u64();
			let consensus_blocks_interval: u64 = ConsensusBlocksInterval::<T>::get();

			// Ensure updates are happening before consensus submissions
			ensure!(
				Self::can_remove_or_update_model_peer(block, consensus_blocks_interval),
				Error::<T>::InvalidRemoveOrUpdateModelPeerBlock
			);

			let min_required_peer_consensus_submit_epochs = MinRequiredPeerConsensusSubmitEpochs::<T>::get();

			// Check if model peer is eligible for consensus submission
			//
			// Updating a peer_id can only be done if the model peer can submit consensus data
			// Otherwise they must remove their peer and start a new one
			//
			// This is a backup incase models go down and model hosters all need to spin up
			// new nodes under new peer_id's
			ensure!(
				Self::is_epoch_block_eligible(
					block, 
					consensus_blocks_interval, 
					min_required_peer_consensus_submit_epochs, 
					model_peer.initialized
				),
				Error::<T>::PeerConsensusSubmitEpochNotReached
			);

			// ====================
			// Mutate peer_id into storage
			// ====================
			ModelPeersData::<T>::mutate(
				model_id.clone(),
				account_id.clone(),
				|params: &mut ModelPeer<T::AccountId>| {
					params.peer_id = peer_id.clone();
				}
			);

			// Update unique model peer_id
			ModelPeerAccount::<T>::insert(model_id.clone(), peer_id.clone(), account_id.clone());

			
			Self::deposit_event(
				Event::ModelPeerUpdated { 
					model_id: model_id.clone(), 
					account_id: account_id.clone(), 
					peer_id: peer_id.clone(),
					block: block
				}
			);

			Ok(())
		}

		/// Remove your model peer
		/// Unstaking must be done seperately
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::remove_model_peer())]
		// #[pallet::weight({0})]
		pub fn remove_model_peer(
			origin: OriginFor<T>, 
			model_id: u32, 
			// model_path: Vec<u8>, 
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin.clone())?;

			let consensus_blocks_interval: u64 = ConsensusBlocksInterval::<T>::get();
			let block: u64 = Self::get_current_block_as_u64();

			// ensure!(
			// 	!Self::is_in_consensus_steps(block, consensus_blocks_interval),
			// 	Error::<T>::InvalidSubmitConsensusBlock
			// );

			// --- Ensure model exists
			ensure!(
				ModelsData::<T>::contains_key(model_id.clone()),
				Error::<T>::ModelNotExist
			);

			ensure!(
				ModelPeersData::<T>::contains_key(model_id.clone(), account_id.clone()),
				Error::<T>::ModelPeerNotExist
			);

			let model_peer = ModelPeersData::<T>::get(model_id.clone(), account_id.clone());

			let min_required_consensus_inclusion_epochs = MinRequiredPeerConsensusInclusionEpochs::<T>::get();

			// Check if model peer is eligible to be included in consensus data submissions
			let is_included: bool = block >= Self::get_eligible_epoch_block(
				consensus_blocks_interval, 
				model_peer.initialized, 
				min_required_consensus_inclusion_epochs
			);

			// If a model peer can be included in consensus they must wait until `can_remove_or_update_model_peer()` is true
			// to self-remove their model peer
			//
			// If a model peer isn't included in consensus then removing won't disrupt anything
			if is_included {
				// Ensure updates are happening before consensus submissions
				// Ensure removing during model peer removal range of epoch
				// This ensures model peers submitting consensus data will not be interupted
				// if a model peer exits on a block another peer is submitting data on as it can
				// revert the model peers submission due to data array length requirements
				ensure!(
					Self::can_remove_or_update_model_peer(block, consensus_blocks_interval),
					Error::<T>::InvalidRemoveOrUpdateModelPeerBlock
				);
			}

			// We don't check consensus steps here because a model peers stake isn't included in calculating rewards 
			// that hasn't reached their consensus submission epoch yet
			
			Self::do_remove_model_peer(block, model_id.clone(), account_id.clone());

			Ok(())
		}

		/// Remove a model peer that has surpassed the max penalties allowed
		#[pallet::call_index(7)]
		#[pallet::weight({0})]
		pub fn remove_account_model_peers(
			origin: OriginFor<T>, 
			account_id: T::AccountId, 
		) -> DispatchResult {
			ensure_signed(origin.clone())?;

			let consensus_blocks_interval: u64 = ConsensusBlocksInterval::<T>::get();
			let block: u64 = Self::get_current_block_as_u64();

			// Ensure consensus isn't being formed or emissions are being generated
			ensure!(
				Self::can_remove_or_update_model_peer(block, consensus_blocks_interval),
				Error::<T>::InvalidRemoveOrUpdateModelPeerBlock
			);

			// Ensure account is not eligible to be a model peer
			ensure!(
				!Self::is_account_eligible(account_id.clone()),
				Error::<T>::AccountEligible
			);

			Self::do_remove_account_model_peers(block, account_id);

			Ok(())
		}
		
		/// Update model peer port
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::update_port())]
		pub fn update_port(
			origin: OriginFor<T>, 
			model_id: u32, 
			port: u16,
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin.clone())?;

			// --- Ensure model exists
			ensure!(
				ModelsData::<T>::contains_key(model_id.clone()),
				Error::<T>::ModelNotExist
			);

			// --- Ensure account has peer
			ensure!(
				ModelPeersData::<T>::contains_key(model_id.clone(), account_id.clone()),
				Error::<T>::ModelPeerNotExist
			);

			let model_peer: ModelPeer<T::AccountId> = ModelPeersData::<T>::get(model_id.clone(), account_id.clone());

			// Validate port
			ensure!(Self::validate_port(port.clone()),  Error::<T>::InvalidPort);

			// ====================
			// Insert peer into storage
			// ====================
			ModelPeersData::<T>::mutate(
				model_id.clone(),
				account_id.clone(),
				|params: &mut ModelPeer<T::AccountId>| {
					params.port = port.clone();
				}
			);

			Ok(())
		}

		/// Increase stake towards the specified model ID
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::add_to_stake())]
		// #[pallet::weight({0})]
		pub fn add_to_stake(
			origin: OriginFor<T>, 
			model_id: u32,
			stake_to_be_added: u128,
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin.clone())?;
			// Each account can only have one peer
			// Staking is accounted for per account_id per model_id
			// We only check that origin exists within ModelPeersData

			let consensus_blocks_interval: u64 = ConsensusBlocksInterval::<T>::get();
			let block: u64 = Self::get_current_block_as_u64();

			// Ensure consensus isn't being formed or emissions are being generated
			ensure!(
				!Self::is_in_consensus_steps(block, consensus_blocks_interval),
				Error::<T>::InvalidSubmitConsensusBlock
			);

			// --- Ensure model exists
			ensure!(
				ModelsData::<T>::contains_key(model_id.clone()),
				Error::<T>::ModelNotExist
			);

			// --- Ensure account has peer
			ensure!(
				ModelPeersData::<T>::contains_key(model_id.clone(), account_id.clone()),
				Error::<T>::ModelPeerNotExist
			);
			
			Self::do_add_stake(
				origin, 
				model_id,
				account_id.clone(),
				stake_to_be_added,
			)
		}

		/// Remove stake balance
		/// If account is a current model peer on the model ID they can only remove up to minimum required balance
		// Decrease stake on accounts peer if minimum required isn't surpassed
		// to-do: if removed through consensus, add removed_block to storage and require time 
		//				to pass until they can remove their stake
		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::remove_stake())]
		// #[pallet::weight({0})]
		pub fn remove_stake(
			origin: OriginFor<T>, 
			model_id: u32, 
			stake_to_be_removed: u128
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin.clone())?;

			// // Ensure consensus isn't being formed or emissions are being generated
			// ensure!(
			// 	!Self::is_in_consensus_steps(block, consensus_blocks_interval),
			// 	Error::<T>::InvalidSubmitConsensusBlock
			// );

      // Get ModelAccount (this is not deleted until stake == 0)
			let model_accounts: BTreeMap<T::AccountId, u64> = ModelAccount::<T>::get(model_id.clone());

			// Check if removed all stake yet
			let has_model_account: bool = match model_accounts.get(&account_id.clone()) {
				Some(_) => true,
				None => false,
			};

			// If ModelAccount doesn't exist this means they have been removed due their staking balance is at zero
			// Once balance is at zero they are removed from ModelAccount in `do_remove_stake()`
			ensure!(
				has_model_account,
				Error::<T>::ModelPeerNotExist
			);

			let block_initialized_or_removed: u64 = match model_accounts.get(&account_id.clone()) {
				Some(block_initialized_or_removed) => *block_initialized_or_removed,
				None => 0,
			};
			let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<T>::get();

			let consensus_blocks_interval: u64 = ConsensusBlocksInterval::<T>::get();
			let block: u64 = Self::get_current_block_as_u64();

			// Ensure min required epochs have surpassed to unstake
			// Based on either initialized block or removal block
			ensure!(
				block >= Self::get_eligible_epoch_block(
					consensus_blocks_interval, 
					block_initialized_or_removed, 
					min_required_unstake_epochs
				),
				Error::<T>::RequiredUnstakeEpochsNotMet
			);

			// If account is a peer they can remove stake up to minimum required stake balance
			// Else they can remove entire balance because they are not hosting models according to consensus
			//		They are removed in `do_remove_model_peer()` when self or consensus removed
			let is_peer: bool = match ModelPeersData::<T>::try_get(model_id.clone(), account_id.clone()) {
				Ok(_) => true,
				Err(()) => false,
			};

			// Remove stake
			// 		if_peer: cannot remove stake below minimum required stake
			// 		else: can remove total stake balance
			// if balance is zero then ModelAccount is removed
			Self::do_remove_stake(
				origin, 
				model_id.clone(),
				account_id,
				is_peer,
				stake_to_be_removed,
			)
		}

		// Testing purposes only
		#[pallet::call_index(11)]
		#[pallet::weight({0})]
		pub fn form_consensus(origin: OriginFor<T>) -> DispatchResult {
			let block: u64 = Self::get_current_block_as_u64();

			let consensus_blocks_interval: u64 = ConsensusBlocksInterval::<T>::get();
  
			if block % consensus_blocks_interval == 0 {
			}

			ensure!(block % consensus_blocks_interval == 0, "e");

			Self::form_peer_consensus(block);
			Ok(())
		}
	
		// Testing purposes only
		#[pallet::call_index(12)]
		#[pallet::weight({0})]
		pub fn do_generate_emissions(origin: OriginFor<T>) -> DispatchResult {
			let block: u64 = Self::get_current_block_as_u64();

			let consensus_blocks_interval: u64 = ConsensusBlocksInterval::<T>::get();
	
			ensure!((block - 1) % consensus_blocks_interval == 0, "Error block generate_emissions");

			Self::generate_emissions();

			let _ = ModelPeerConsensusResults::<T>::clear(u32::MAX, None);
			let _ = PeerConsensusEpochSubmitted::<T>::clear(u32::MAX, None);
			let _ = PeerConsensusEpochUnconfirmed::<T>::clear(u32::MAX, None);
			let _ = ModelTotalConsensusSubmits::<T>::clear(u32::MAX, None);
			let _ = ModelConsensusEpochUnconfirmedCount::<T>::clear(u32::MAX, None);

			Ok(())
		}

		// Testing purposes only
		#[pallet::call_index(13)]
		#[pallet::weight({0})]
		pub fn vote_model(origin: OriginFor<T>, model_path: Vec<u8>) -> DispatchResult {
			ModelActivated::<T>::insert(model_path.clone(), true);
			Ok(())
		}

		// Testing purposes only
		#[pallet::call_index(14)]
		#[pallet::weight({0})]
		pub fn vote_model_out(origin: OriginFor<T>, model_path: Vec<u8>) -> DispatchResult {
			ModelActivated::<T>::insert(model_path.clone(), false);
			Ok(())
		}

		// Testing purposes only
		#[pallet::call_index(15)]
		#[pallet::weight({0})]
		pub fn form_consensus_no_consensus_weight_test(origin: OriginFor<T>) -> DispatchResult {
			let block: u64 = Self::get_current_block_as_u64();

			let consensus_blocks_interval: u64 = ConsensusBlocksInterval::<T>::get();
  
			if block % consensus_blocks_interval == 0 {
			}

			if (block - 1) % consensus_blocks_interval == 0 {
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn is_model_eligible(
			model_id: u32, 
			model_path: Vec<u8>, 
			model_initialized: u64
		) -> (bool, Vec<u8>) {
			let mut reason_for_removal: Vec<u8> = Vec::new();

			// 1.
			// check model voted out
			// let voted: bool = match ModelVoteOut::<T>::try_get(model_path.clone()) {
			// 	Ok(vote) => vote,
			// 	Err(()) => false,
			// };

			let activated: bool = match ModelActivated::<T>::try_get(model_path.clone()) {
				Ok(is_active) => is_active,
				Err(()) => false,
			};

			// Push into reason
			if !activated {
				reason_for_removal.push(1)
			}

			// 2.
			// Model can reach max zero consensus count
			let zero_consensus_epochs: u32 = ModelConsensusEpochsErrors::<T>::get(model_id.clone());
			let max_zero_consensus_epochs: u32 = MaxModelConsensusEpochsErrors::<T>::get();
			let too_many_max_consensus_epochs: bool = zero_consensus_epochs > max_zero_consensus_epochs;

			// Push into reason
			if too_many_max_consensus_epochs {
				reason_for_removal.push(2)
			}

			// 3.
			// Check if model is offline too many times
			let is_offline: bool = false;

			// Push into reason
			if is_offline {
				reason_for_removal.push(3)
			}

			// 4.
			// Check if model has min amount of peers
			// If min peers are not met and initialization epochs has surpassed
			// then model can be removed
			let total_model_peers: u32 = TotalModelPeers::<T>::get(model_id.clone());
			let min_model_peers: u32 = MinModelPeers::<T>::get();
			let mut has_min_peers: bool = true;
			if total_model_peers < min_model_peers {
				let block: u64 = Self::get_current_block_as_u64();
				let consensus_blocks_interval: u64 = ConsensusBlocksInterval::<T>::get();
				let model_peers_initialization_epochs: u64 = ModelPeersInitializationEpochs::<T>::get();
				// Ensure initialization epochs have passed
				// If not return false
				let has_min_peers: bool = block < Self::get_eligible_epoch_block(
					consensus_blocks_interval, 
					model_initialized, 
					model_peers_initialization_epochs
				);
				// Push into reason
				if !has_min_peers {
					reason_for_removal.push(4)
				}	
			}

			(!activated || too_many_max_consensus_epochs || is_offline || !has_min_peers, reason_for_removal)
		}
	}
	
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
			let block: u64 = Self::convert_current_block_as_u64(block_number);

			let consensus_blocks_interval: u64 = ConsensusBlocksInterval::<T>::get();
  
			// Form peer consensus at the beginning of each epoch on the last epochs data
			if block >= consensus_blocks_interval && block % consensus_blocks_interval == 0 {
				Self::form_peer_consensus(block);
				return Weight::from_parts(38_499_025_000, 6832080)
					.saturating_add(T::DbWeight::get().reads(3171_u64))
					.saturating_add(T::DbWeight::get().writes(2011_u64));
			}

			// Run the block succeeding form consensus
			if (block - 1) >= consensus_blocks_interval && (block - 1) % consensus_blocks_interval == 0 {
				Self::generate_emissions();

				// reset consensus storage
				let _ = ModelPeerConsensusResults::<T>::clear(u32::MAX, None);
				let _ = PeerConsensusEpochSubmitted::<T>::clear(u32::MAX, None);
				let _ = PeerConsensusEpochUnconfirmed::<T>::clear(u32::MAX, None);
				let _ = ModelTotalConsensusSubmits::<T>::clear(u32::MAX, None);
				let _ = ModelConsensusEpochUnconfirmedCount::<T>::clear(u32::MAX, None);				

				return Weight::from_parts(37_324_694_000, 6723567)
					.saturating_add(T::DbWeight::get().reads(3137_u64))
					.saturating_add(T::DbWeight::get().writes(3123_u64));
				}
	
			return Weight::from_parts(8_060_000, 1565)
				.saturating_add(T::DbWeight::get().reads(1_u64))
		}

		fn offchain_worker(block_number: BlockNumberFor<T>) {
			// designated for testnet v2.0
			//
			// Call peers at random to ensure model is running
			// Submit a prompt/hash/code/etc. and expect specific response
			// Increment errors or wrong responses to both models and peers
			// ...
		}
	}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub model_path: Vec<u8>,
		pub model_peers: Vec<(T::AccountId, Vec<u8>, PeerId, Vec<u8>, u16)>,
		pub accounts: Vec<T::AccountId>,
		pub blank: Option<T::AccountId>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			let model_id = 1;

			let model_data = ModelData {
				id: model_id.clone(),
				path: self.model_path.clone(),
				initialized: 0,
			};

			// Store unique path
			ModelPaths::<T>::insert(self.model_path.clone(), model_id.clone());
			// Store model data
			ModelsData::<T>::insert(model_id.clone(), model_data.clone());

			TotalModels::<T>::mutate(|n: &mut u32| *n += 1);
			StakeVaultBalance::<T>::mutate(|n: &mut u128| *n += 10000000000000000000);
			ModelActivated::<T>::insert(self.model_path.clone(), true);

			let mut count = 0;
			for (account_id, model_path, peer_id, ip, port) in &self.model_peers {
				// for running benchmarks set to `count >= 0`
				// for testing model validators
				// 0-100 get balance
				// 0-50 are peers on initialization
				if count >= 50 {
					break
				}	

				log::info!("BuildGenesisConfig peer_id: {:?}", peer_id);
	
				// version 2
				let model_peer: ModelPeer<T::AccountId> = ModelPeer {
					account_id: account_id.clone(),
					peer_id: peer_id.clone(),
					ip: ip.clone(),
					port: port.clone(),
					initialized: 0,
				};
				ModelPeersData::<T>::insert(model_id.clone(), account_id.clone(), model_peer.clone());

				// Insert model peer account to keep peer_ids unique within models
				ModelPeerAccount::<T>::insert(model_id.clone(), peer_id.clone(), account_id.clone());

				// let mut model_accounts: BTreeSet<T::AccountId> = ModelAccount::<T>::get(model_id.clone());
				// let model_account_id: Option<&T::AccountId> = model_accounts.get(&account_id.clone());
				// model_accounts.insert(account_id.clone());
				// ModelAccount::<T>::insert(model_id.clone(), model_accounts);

				let mut model_accounts: BTreeMap<T::AccountId, u64> = ModelAccount::<T>::get(model_id.clone());
				let model_account: Option<&u64> = model_accounts.get(&account_id.clone());
				model_accounts.insert(account_id.clone(), 0);
				ModelAccount::<T>::insert(model_id.clone(), model_accounts);
		
				TotalModelPeers::<T>::mutate(model_id.clone(), |n: &mut u32| *n += 1);

				// Stake
				let stake_amount: u128 = 10000000000000000000;
				AccountModelStake::<T>::insert(
					account_id.clone(),
					model_id.clone(),
					stake_amount,
				);
		
				// -- Increase account_id total stake
				TotalAccountStake::<T>::mutate(account_id.clone(), |n: &mut u128| *n += stake_amount.clone());
		
				// -- Increase total stake overall
				TotalStake::<T>::mutate(|n: &mut u128| *n += stake_amount.clone());
		
				// -- Increase total model stake
				TotalModelStake::<T>::mutate(model_id.clone(), |n: &mut u128| *n += stake_amount.clone());

				AccountModels::<T>::append(account_id.clone(), model_id.clone());

				count += 1;
			}
		}
	}
}

// Staking logic from rewards pallet
impl<T: Config> IncreaseStakeVault for Pallet<T> {
	fn increase_stake_vault(amount: u128) -> DispatchResult {
		StakeVaultBalance::<T>::mutate(|n: &mut u128| *n += amount);
		Ok(())
	}
}
pub trait IncreaseStakeVault {
	fn increase_stake_vault(amount: u128) -> DispatchResult;
}

// Voting logic for testnet v2.0
impl<T: Config> ModelVote for Pallet<T> {
	fn vote_model_in(path: Vec<u8>) -> DispatchResult {
		ModelActivated::<T>::insert(path.clone(), true);
		Ok(())
	}
	fn vote_model_out(path: Vec<u8>) -> DispatchResult {
		ModelActivated::<T>::insert(path.clone(), false);
		Ok(())
	}
	fn vote_activated(path: Vec<u8>, value: bool) -> DispatchResult {
		ModelActivated::<T>::insert(path.clone(), value);
		Ok(())
	}
}

pub trait ModelVote {
	fn vote_model_in(path: Vec<u8>) -> DispatchResult;
	fn vote_model_out(path: Vec<u8>) -> DispatchResult;
	fn vote_activated(path: Vec<u8>, value: bool) -> DispatchResult;
}

// Admin logic
impl<T: Config> AdminInterface for Pallet<T> {
	fn set_vote_model_in(path: Vec<u8>) -> DispatchResult {
		Self::set_vote_model_in(path)
	}
	fn set_vote_model_out(path: Vec<u8>) -> DispatchResult {
		Self::set_vote_model_out(path)
	}
	fn set_max_models(value: u32) -> DispatchResult {
		Self::set_max_models(value)
	}
	fn set_min_model_peers(value: u32) -> DispatchResult {
		Self::set_min_model_peers(value)
	}
	fn set_max_model_peers(value: u32) -> DispatchResult {
		Self::set_max_model_peers(value)
	}
	fn set_min_stake_balance(value: u128) -> DispatchResult {
		Self::set_min_stake_balance(value)
	}
	fn set_tx_rate_limit(value: u64) -> DispatchResult {
		Self::set_tx_rate_limit(value)
	}
	fn set_max_consensus_epochs_errors(value: u32) -> DispatchResult {
		Self::set_max_consensus_epochs_errors(value)
	}
	fn set_min_required_model_consensus_submit_epochs(value: u64) -> DispatchResult {
		Self::set_min_required_model_consensus_submit_epochs(value)
	}
	fn set_min_required_peer_consensus_submit_epochs(value: u64) -> DispatchResult {
		Self::set_min_required_peer_consensus_submit_epochs(value)
	}
	fn set_min_required_peer_consensus_inclusion_epochs(value: u64) -> DispatchResult {
		Self::set_min_required_peer_consensus_inclusion_epochs(value)
	}
	fn set_max_outlier_delta_percent(value: u8) -> DispatchResult {
		Self::set_max_outlier_delta_percent(value)
	}
	fn set_model_peer_consensus_submit_percent_requirement(value: u128) -> DispatchResult {
		Self::set_model_peer_consensus_submit_percent_requirement(value)
	}
	fn set_consensus_blocks_interval(value: u64) -> DispatchResult {
		Self::set_consensus_blocks_interval(value)
	}
	fn set_peer_removal_threshold(value: u128) -> DispatchResult {
		Self::set_peer_removal_threshold(value)
	}
	fn set_max_model_rewards_weight(value: u128) -> DispatchResult {
		Self::set_max_model_rewards_weight(value)
	}
	fn set_stake_reward_weight(value: u128) -> DispatchResult {
		Self::set_stake_reward_weight(value)
	}
	fn set_model_per_peer_init_cost(value: u128) -> DispatchResult {
		Self::set_model_per_peer_init_cost(value)
	}
	fn set_model_consensus_unconfirmed_threshold(value: u128) -> DispatchResult {
		Self::set_model_consensus_unconfirmed_threshold(value)
	}
	fn set_remove_model_peer_epoch_percentage(value: u128) -> DispatchResult {
		Self::set_remove_model_peer_epoch_percentage(value)
	}
}

pub trait AdminInterface {
	fn set_vote_model_in(path: Vec<u8>) -> DispatchResult;
	fn set_vote_model_out(path: Vec<u8>) -> DispatchResult;
	fn set_max_models(value: u32) -> DispatchResult;
	fn set_min_model_peers(value: u32) -> DispatchResult;
	fn set_max_model_peers(value: u32) -> DispatchResult;
	fn set_min_stake_balance(value: u128) -> DispatchResult;
	fn set_tx_rate_limit(value: u64) -> DispatchResult;
	fn set_max_consensus_epochs_errors(value: u32) -> DispatchResult;
	fn set_min_required_model_consensus_submit_epochs(value: u64) -> DispatchResult;
	fn set_min_required_peer_consensus_submit_epochs(value: u64) -> DispatchResult;
	fn set_min_required_peer_consensus_inclusion_epochs(value: u64) -> DispatchResult;
	fn set_max_outlier_delta_percent(value: u8) -> DispatchResult;
	fn set_model_peer_consensus_submit_percent_requirement(value: u128) -> DispatchResult;
	fn set_consensus_blocks_interval(value: u64) -> DispatchResult;
	fn set_peer_removal_threshold(value: u128) -> DispatchResult;
	fn set_max_model_rewards_weight(value: u128) -> DispatchResult;
	fn set_stake_reward_weight(value: u128) -> DispatchResult;
	fn set_model_per_peer_init_cost(value: u128) -> DispatchResult;
	fn set_model_consensus_unconfirmed_threshold(value: u128) -> DispatchResult;
	fn set_remove_model_peer_epoch_percentage(value: u128) -> DispatchResult;
}