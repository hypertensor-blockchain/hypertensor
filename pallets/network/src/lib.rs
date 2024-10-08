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
	traits::{tokens::WithdrawReasons, Get, Currency, ReservableCurrency, ExistenceRequirement, Randomness},
	pallet_prelude::{Weight, DispatchResultWithPostInfo},
	storage::bounded_vec::BoundedVec,
	PalletId
};
use sp_runtime::RuntimeDebug;
use scale_info::prelude::string::String;
use sp_core::OpaquePeerId as PeerId;
use sp_std::collections::{btree_map::BTreeMap, btree_set::BTreeSet};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, FromRepr};
// use sp_runtime::traits::{
// 	AppendZerosInput, Hash, IdentifyAccount, Saturating, StaticLookup, Verify, Zero,
// };
use sp_runtime::traits::{Verify, IdentifyAccount};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

mod utils;
mod math;
mod admin;
mod staking;
mod delegate_staking;
mod info;
mod accountant;
mod subnet_validator;
mod rewards;
mod proposal;
mod randomness;

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

    // type Currency: Currency<Self::AccountId> + Send + Sync;
    type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId> + Send + Sync;

		#[pallet::constant]
		type EpochLength: Get<u64>;

    #[pallet::constant]
    type StringLimit: Get<u32>;

    #[pallet::constant] // Initial transaction rate limit.
    type InitialTxRateLimit: Get<u64>;

		#[pallet::constant]
    type SecsPerBlock: Get<u64>;

		#[pallet::constant]
    type Year: Get<u64>;

		type OffchainSignature: Verify<Signer = Self::OffchainPublic> + Parameter;
		type OffchainPublic: IdentifyAccount<AccountId = Self::AccountId>;

		/// Something that provides randomness in the runtime.
		type Randomness: Randomness<Self::Hash, BlockNumberFor<Self>>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type SubnetInitializationCost: Get<u128>;
	}

	/// Events for the pallet.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// Subnets
		SubnetAdded { proposer: T::AccountId, activator: T::AccountId, subnet_id: u32, subnet_path: Vec<u8>, block: u64 },
		SubnetRemoved { account: T::AccountId, subnet_id: u32, subnet_path: Vec<u8>, reason: Vec<u8>, block: u64 },

		// Subnet Nodes
		SubnetNodeAdded { subnet_id: u32, account_id: T::AccountId, peer_id: PeerId, block: u64 },
		SubnetNodeUpdated { subnet_id: u32, account_id: T::AccountId, peer_id: PeerId, block: u64 },
		SubnetNodeRemoved { subnet_id: u32, account_id: T::AccountId, peer_id: PeerId, block: u64 },

		// Stake
		StakeAdded(u32, T::AccountId, u128),
		StakeRemoved(u32, T::AccountId, u128),

		DelegateStakeAdded(u32, T::AccountId, u128),
		DelegateStakeRemoved(u32, T::AccountId, u128),
		
		// Admin 
		SetVoteSubnetIn(Vec<u8>),
    SetVoteSubnetOut(Vec<u8>),
    SetMaxSubnets(u32),
    SetMinSubnetNodes(u32),
    SetMaxSubnetNodes(u32),
    SetMinStakeBalance(u128),
    SetTxRateLimit(u64),
    SetMaxZeroConsensusEpochs(u32),
    SetMinRequiredSubnetConsensusSubmitEpochs(u64),
    SetMinRequiredNodeConsensusSubmitEpochs(u64),
    SetMinRequiredNodeConsensusEpochs(u64),
		SetMinRequiredNodeAccountantEpochs(u64),
    SetMaximumOutlierDeltaPercent(u8),
    SetSubnetNodeConsensusSubmitPercentRequirement(u128),
    SetEpochLengthsInterval(u64),
    SetNodeRemovalThreshold(u128),
    SetMaxSubnetRewardsWeight(u128),
		SetStakeRewardWeight(u128),
		SetSubnetPerNodeInitCost(u128),
		SetSubnetConsensusUnconfirmedThreshold(u128),
		SetRemoveSubnetNodeEpochPercentage(u128),

		// Dishonesty Proposals
		DishonestSubnetNodeProposed { subnet_id: u32, account_id: T::AccountId, block: u64},
		DishonestSubnetNodeVote { subnet_id: u32, account_id: T::AccountId, voter_account_id: T::AccountId, block: u64 },
		DishonestAccountRemoved { subnet_id: u32, account_id: T::AccountId, block: u64},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Errors should have helpful documentation associated with them.

		/// Node hasn't been initialized for required epochs to submit consensus
		NodeConsensusSubmitEpochNotReached,
		/// Node hasn't been initialized for required epochs to be an accountant
		NodeAccountantEpochNotReached,
		/// Maximum subnets reached
		MaxSubnets,
		/// Account has subnet peer under subnet already
		SubnetNodeExist,
		/// Node ID already in use
		PeerIdExist,
		/// Node ID already in use
		PeerIdNotExist,
		/// Subnet peer doesn't exist
		SubnetNodeNotExist,
		/// Subnet already exists
		SubnetExist,
		/// Subnet doesn't exist
		SubnetNotExist,
		/// Minimum required subnet peers not reached
		SubnetNodesMin,
		/// Maximum allowed subnet peers reached
		SubnetNodesMax,
		/// Subnet has not been voted in
		SubnetNotVotedIn,
		/// Subnet not validated to be removed
		SubnetCantBeRemoved,
		/// Account is eligible
		AccountEligible,
		/// Account is ineligible
		AccountIneligible,
		// invalid submit consensus block
		/// Cannot submit consensus during invalid blocks
		InvalidSubmitEpochLength,
		/// Cannot remove subnet peer during invalid blocks
		InvalidRemoveOrUpdateSubnetNodeBlock,
		/// Transaction rate limiter exceeded
		TxRateLimitExceeded,
		/// PeerId format invalid
		InvalidPeerId,
		/// The provided signature is incorrect.
		WrongSignature,

		// Admin
		/// Consensus block epoch_length invalid, must reach minimum
		InvalidEpochLengthsInterval,
		/// Invalid maximimum subnets, must not exceed maximum allowable
		InvalidMaxSubnets,
		/// Invalid min subnet peers, must not be less than minimum allowable
		InvalidMinSubnetNodes,
		/// Invalid maximimum subnet peers, must not exceed maximimum allowable
		InvalidMaxSubnetNodes,
		/// Invalid minimum stake balance, must be greater than or equal to minimim required stake balance
		InvalidMinStakeBalance,
		/// Invalid percent number, must be in 1e4 format. Used for elements that only require correct format
		InvalidPercent,
		/// Invalid subnet peer consensus submit percent requirement
		InvalidSubnetNodeConsensusSubmitPercentRequirement,
		/// Invalid percent number based on MinSubnetNodes as `min_value = 1 / MinSubnetNodes`
		// This ensures it's possible to form consensus to remove peers
		InvalidNodeRemovalThreshold,
		/// Invalid maximimum zero consensus epochs, must not exceed maximum allowable
		InvalidMaxZeroConsensusEpochs,
		/// Invalid subnet consensus `submit` epochs, must be greater than 2 and greater than MinRequiredNodeConsensusSubmitEpochs
		InvalidSubnetConsensusSubmitEpochs,
		/// Invalid peer consensus `inclusion` epochs, must be greater than 0 and less than MinRequiredNodeConsensusSubmitEpochs
		InvalidNodeConsensusInclusionEpochs,
		/// Invalid peer consensus `submit` epochs, must be greater than 1 and greater than MinRequiredNodeConsensusInclusionEpochs
		InvalidNodeConsensusSubmitEpochs,
		/// Invalid peer consensus `dishonesty` epochs, must be greater than 2 and greater than MinRequiredNodeConsensusSubmitEpochs
		InvalidNodeConsensusDishonestyEpochs,
		/// Invalid max outlier delta percentage, must be in format convertible to f64
		InvalidMaxOutlierDeltaPercent,
		/// Invalid subnet per peer init cost, must be greater than 0 and less than 1000
		InvalidSubnetPerNodeInitCost,
		/// Invalid subnet consensus uncunfirmed threshold, must be in 1e4 format
		InvalidSubnetConsensusUnconfirmedThreshold,
		/// Invalid remove subnet peer epoch percentage, must be in 1e4 format and greater than 20.00
		InvalidRemoveSubnetNodeEpochPercentage,
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
		NotEnoughStakeToWithdraw,
		MaxStakeReached,
		// if min stake not met on both stake and unstake
		MinStakeNotReached,
		// delegate staking
		CouldNotConvertToShares,
		// 
		MaxDelegatedStakeReached,
		//
		RequiredDelegateUnstakeEpochsNotMet,
		// Conversion to balance was zero
		InsufficientBalanceToSharesConversion,
		// consensus
		SubnetInitializeRequirement,
		ConsensusDataInvalidLen,
		/// Invalid consensus score, must be in 1e4 format and greater than 0
		InvalidScore,
		/// Consensus data already submitted
		ConsensusDataAlreadySubmitted,
		/// Consensus data already unconfirmed
		ConsensusDataAlreadyUnconfirmed,

		/// Math multiplication overflow
		MathMultiplicationOverflow,

		/// Dishonesty on subnet and account proposed
		DishonestyVoteAlreadyProposed,

		/// Dishonesty vote period already completed
		DishonestyVotePeriodCompleted,
		
		/// Dishonesty vote not proposed
		DishonestyVoteNotProposed,

		/// Dishonesty voting either not exists or voting period is over
		DishonestyVotingPeriodOver,

		/// Dishonesty voting not over
		DishonestyVotingPeriodNotOver,

		/// Dishonesty voting either not exists or voting period is over
		DishonestyVotingDuplicate,

		/// Not enough balance to withdraw bid for proposal
		NotEnoughBalanceToBid,

		QuorumNotReached,

		/// Dishonest propsal type
		PropsTypeInvalid,

		ProposalNotExist,
		ProposalNotChallenged,
		ProposalChallenged,
		ProposalChallengePeriodPassed,
		PropsalAlreadyChallenged,
		NotChallenger,
		ChallengePeriodPassed,
		DuplicateVote,
		NotAccountant,
		InvalidAccountantDataId,
		InvalidAccountantData,
		DataEmpty,

		InvalidSubnetRewardsSubmission,


		// Validation and Attestation
		/// Subnet rewards data already submitted by validator
		SubnetRewardsAlreadySubmitted,
		/// Not epoch validator
		InvalidValidator,
		/// Already attested validator data
		AlreadyAttested,
		/// Invalid rewards data length
		InvalidRewardsDataLength,
		/// Invalid block for submitting data
		InvalidBlock,


		ProposalInvalid,
		NotDefendant,
		NotPlaintiff,
		ProposalUnchallenged,
		/// Subnet node as defendant has proposal activated already
		NodeHasActiveProposal,
	}
	
	// Used for decoding API data - not in use in v1.0
	#[derive(Deserialize, Serialize)]
	struct SerdePeerId {
		peer_uid: String,
	}

	// gib_per_block is used to calculate how many blocks are in the model to get how many nodes are required
	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct ModelType {
		pub title: Vec<u8>, // Unique title of model type: LLAMA-NF4
		pub precision: u128, // bit precision: 4
		pub default_memory: u128, // default memory for node: 64
    pub gib_b: u128, // GiB per billion parameters: 0.7 for LLAMA-NF4
		pub rewards_per_billion: u128, // reward per billion per epoch
		pub rewards_per_node: u128, // reward per billion per epoch: 9_363_000_000_000_000
	}

	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct SubnetData {
		pub id: u32,
		pub path: Vec<u8>,
		pub min_nodes: u32,
		pub target_nodes: u32,
		pub memory_mb: u128,
		pub initialized: u64,
	}

	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct PreSubnetData {
		pub path: Vec<u8>,
		pub memory_mb: u128,
	}
	
	/// Data for subnet held to be compared when adding a subnet to the network
	// This is the data from the democracy voting pallet
	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct VoteSubnetData {
		pub data: PreSubnetData,
		pub active: bool,
	}

	// The submit consensus data format
	// Scoring is calculated off-chain between subnet peers hosting AI subnets together
	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct SubnetNodeData {
		pub peer_id: PeerId,
		pub score: u128,
	}

	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct SubnetNode<AccountId> {
		pub account_id: AccountId,
		pub peer_id: PeerId,
		pub initialized: u64,
	}

	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct RewardsData<AccountId> {
		pub validator: AccountId, // Chosen validator of the epoch
		pub nodes_count: u32, // Number of nodes expected to submit attestations
		pub sum: u128, // Sum of the data scores
		pub attests: BTreeSet<AccountId>, // Count of attestations of the submitted data
		pub data: Vec<SubnetNodeData>, // Data submitted by chosen validator
	}

	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct SubnetNodeConsensusResultsParams<AccountId> {
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

	// `data` is an arbitrary vec of data for subnets to use for validation
	// It's up to each subnet to come up with their own format that fits within the BoundedVec
	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct AccountantDataNodeParams {
		pub peer_id: PeerId,
		pub data: BoundedVec<u8, DefaultAccountantDataNodeParamsMaxLimit>,
	}

	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct AccountantDataParams<AccountId> {
		pub accountant: AccountId,
		pub block: u64,
		pub epoch: u32,
		pub data: Vec<AccountantDataNodeParams>,
	}

  #[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct VotesParams {
    pub yay: u128,
		pub nay: u128,
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
  pub enum VoteType {
    Yay,
    Nay,
  }

	// The `amount` parameter is the bidding amount to both initialize the proposal and to challenge the proposal
	// The winner gets their bid back and the losers bid gets distributed to everyone in consensus including proposer
	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct DishonestyProposalParams<AccountId> {
		pub subnet_id: u32, // subnet ID this proposal is taking place on
		pub proposal_type: PropsType,
		pub proposer: AccountId, // account proposing dishonesty vote proposal
		pub total_accountants: u32, // total accountants at time of proposal
		pub account_id: AccountId, // account deemed dishonest
		pub peer_id: PeerId, // peer deemed dishonest
		pub bid: u128, // bid amount for proposal at the time of proposal
		pub total_votes: u32, // total sum of yay and nay votes combined
		pub votes: VotesParams, // yay:nay votes
		pub voters: Vec<AccountId>,	// list of voters
		pub yay_voters: Vec<AccountId>,	// list of yay voters
		pub nay_voters: Vec<AccountId>,	// list of nay voters
		pub start_block: u64, // block of proposal
		pub challenge_block: u64, // block of challenging proposal
		pub data: Vec<u8>, // arbitrary data for subnets to use
		pub accountant_data_id: Option<u32>, // if proposal type is dishonest account data, must submit accountant data ID
	}

	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct ProposalParams<AccountId> {
		pub subnet_id: u32,
		pub plaintiff: AccountId,
		pub defendant: AccountId,
		pub plaintiff_bond: u128,
		pub defendant_bond: u128,
		pub eligible_voters: BTreeMap<AccountId, u64>, // Those eligible to vote at time of the proposal
		pub votes: VoteParams2<AccountId>,
		pub start_block: u64,
		pub challenge_block: u64,
		pub plaintiff_data: Vec<u8>,
		pub defendant_data: Vec<u8>,
		pub complete: bool,
	}

	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct VoteParams2<AccountId> {
		pub yay: BTreeSet<AccountId>,
		pub nay: BTreeSet<AccountId>,
	}

	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct VoteParams<AccountId> {
		pub subnet_id: u32,
		pub peer_id: PeerId,
		pub plaintiff_stake: u128,
		pub defendant_stake: u128,
		pub total_votes: u32,
		pub votes: Vec<AccountId>,
		pub start_block: u64,
		pub data: Vec<u8>,
	}

	// #[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub enum ConsensusType {
		Null,
    Submit,
    Unconfirm,
	}

	// Parameters for each subnet peers consensus data
	// It will store the most recent block they submitted and the type of submit
	#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct ConsensusSubmissionDataParams<ConsensusType> {
		block: u64,
		consensus_type: ConsensusType,
	}

	// DishonestAccountant
	// 		- Accountant chosen to validate nodes on epochs data is dishonest
	//		- This prevents lazy accountants
	// DishonestSubnetNode
	//		- Accountant validating nodes proposing a node is dishonest
	//		- Any accountant-eligible node can propose dishonesty
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
  pub enum PropsType {
    None,
    DishonestAccountant, // Accountant chosen to validate nodes on epochs data is dishonest
    DishonestSubnetNode, // Accountant validating nodes proposing a node is dishonest
  }

  impl Default for PropsType {
    fn default() -> Self {
      PropsType::None
    }
  }

	// types
	#[pallet::type_value]
	pub fn DefaultAccountId<T: Config>() -> T::AccountId {
		T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap()
	}
	#[pallet::type_value]
	pub fn DefaultNodeRemovalThreshold<T: Config>() -> u128 {
		7500
	}
	#[pallet::type_value]
	pub fn DefaultNodeAgainstConsensusRemovalThreshold<T: Config>() -> u128 {
		2500
	}
	#[pallet::type_value]
	pub fn DefaultAccountTake<T: Config>() -> u128 {
		0
	}
	// The consensus data format
	//
	// `account_id`
	// 	• The AccountId of the subnet peer
	// `peer_id`
	// 	• The PeerId of the subnet peer
	// `scores`
	// 	• The scores of each subnet peer submitting data on the subnet peer
	// `score`
	// 	• The final score calculated from all `scores`
	// `successful`
	// 	• The count of subnet peers that submitted data on the subnet peer
	// `successful_consensus`
	// 	• Array of each subnet peer that submitted data on the subnet peer
	// `unsuccessful`
	// 	• The count of subnet peers that didn't submit data on the subnet peer
	// `unsuccessful_consensus`
	// 	• Array of each subnet peer that didn't submit data on the subnet peer
	// `total_submits`
	// 	• Count of all submits
	#[pallet::type_value]
	pub fn DefaultSubnetNodeConsensusResults<T: Config>() -> SubnetNodeConsensusResultsParams<T::AccountId> {
		return SubnetNodeConsensusResultsParams {
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
	#[pallet::type_value]
	pub fn DefaultDishonestyProposal<T: Config>() -> DishonestyProposalParams<T::AccountId> {
		return DishonestyProposalParams {
			subnet_id: 0,
			proposal_type: PropsType::None,
			proposer: T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap(),
			total_accountants: 0,
			account_id: T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap(),
			peer_id: PeerId(Vec::new()),
			bid: 0,
			total_votes: 0,
			votes: VotesParams {
				yay: 0,
				nay: 0,
			},
			voters: Vec::new(),
			yay_voters: Vec::new(),
			nay_voters: Vec::new(),
			start_block: 0,
			challenge_block: 0,
			data: Vec::new(),
			accountant_data_id: None,
		};
	}

	#[pallet::type_value]
	pub fn DefaultProposalParams<T: Config>() -> ProposalParams<T::AccountId> {
		return ProposalParams {
			subnet_id: 0,
			plaintiff: T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap(),
			defendant: T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap(),
			plaintiff_bond: 0,
			defendant_bond: 0,
			eligible_voters: BTreeMap::new(),
			votes: VoteParams2 {
				yay: BTreeSet::new(),
				nay: BTreeSet::new(),
			},
			start_block: 0,
			challenge_block: 0,
			plaintiff_data: Vec::new(),
			defendant_data: Vec::new(),
			complete: false,
		};
	}
	#[pallet::type_value]
	pub fn DefaultVoteParams2<T: Config>() -> VoteParams2<T::AccountId> {
		return VoteParams2 {
			yay: BTreeSet::new(),
			nay: BTreeSet::new(),
		};
	}

	// #[pallet::type_value]
	// pub fn DefaultVoteParams<T: Config>() -> VoteParams<T::AccountId> {
	// 	return VoteParams {
	// 		subnet_id: 0,
	// 		peer_id: PeerId(Vec::new()),
	// 		plaintiff_stake: 0,
	// 		defendant_stake: 0,
	// 		total_votes: 0,
	// 		votes: Vec::new(),
	// 		start_block: 0,
	// 		data: Vec::new(),
	// 	};
	// }
	// #[pallet::type_value]
	// pub fn DefaultSubnetData<T: Config>() -> SubnetData {
	// 	return SubnetData {
	// 		id: 0,
	// 		path: Vec::new(),
	// 		initialized: 0,
	// 	};
	// }
	#[pallet::type_value]
	pub fn DefaultSubnetNode<T: Config>() -> SubnetNode<T::AccountId> {
		return SubnetNode {
			account_id: T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap(),
			peer_id: PeerId(Vec::new()),
			initialized: 0,
		};
	}
	#[pallet::type_value]
	pub fn DefaultConsensusSubmissionData<T: Config>() -> ConsensusSubmissionDataParams<ConsensusType> {
		return ConsensusSubmissionDataParams {
			block: 0,
			consensus_type: ConsensusType::Null,
		};
	}
	#[pallet::type_value]
	pub fn DefaultDishonestyVotingPeriod<T: Config>() -> u64 {
		// 7 days
		100800
	}
	#[pallet::type_value]
	pub fn DefaultChallengePeriod<T: Config>() -> u64 {
		// 7 days
		1000
	}
	#[pallet::type_value]
	pub fn DefaultDishonestyProposalQuorum<T: Config>() -> u128 {
		// 7 days
		75000
	}
	#[pallet::type_value]
	pub fn DefaultDishonestyProposalConsensusThreshold<T: Config>() -> u128 {
		// 7 days
		9000
	}
	#[pallet::type_value]
	pub fn DefaultProposalBidAmount<T: Config>() -> u128 {
		1e+18 as u128
	}
	/// Must be greater than MinRequiredNodeConsensusSubmitEpochs
	#[pallet::type_value]
	pub fn DefaultMinRequiredSubnetConsensusSubmitEpochs<T: Config>() -> u64 {
		4
	}
	/// Must be less than MinRequiredNodeConsensusSubmitEpochs
	#[pallet::type_value]
	pub fn DefaultMinRequiredNodeConsensusInclusionEpochs<T: Config>() -> u64 {
		2
	}	
	/// Must be less than MinRequiredSubnetConsensusSubmitEpochs
	/// Must be greater than MinRequiredNodeConsensusInclusionEpochs
	#[pallet::type_value]
	pub fn DefaultMinRequiredNodeConsensusSubmitEpochs<T: Config>() -> u64 {
		3
	}
	/// Must be greater than or equal to DefaultMinRequiredNodeConsensusSubmitEpochs
	#[pallet::type_value]
	pub fn DefaultMinRequiredNodeAccountantEpochs<T: Config>() -> u64 {
		6
	}
	// Testnet 30 mins per epoch
	// Mainnet 120 minutes per epoch at 1200 blocks per epoch
	// #[pallet::type_value]
	// pub fn DefaultEpochLength<T: Config>() -> u64 {
	// 	100
	// }
	#[pallet::type_value]
	pub fn DefaultSubnetNodesInitializationEpochs<T: Config>() -> u64 {
		100
	}
	#[pallet::type_value]
	pub fn DefaultRemoveSubnetNodeEpochPercentage<T: Config>() -> u128 {
		2000
	}
	#[pallet::type_value]
	pub fn DefaultMinRequiredUnstakeEpochs<T: Config>() -> u64 {
		12
	}
	#[pallet::type_value]
	pub fn DefaultMinRequiredDelegateUnstakeEpochs<T: Config>() -> u64 {
		21
	}
	#[pallet::type_value]
	pub fn DefaultMinSubnetNodes() -> u32 {
		3
	}
	#[pallet::type_value]
	pub fn DefaultOptimalSubnets<T: Config>() -> u32 {
		12
	}
	#[pallet::type_value]
	pub fn DefaultOptimalNodesPerSubnet<T: Config>() -> u32 {
		32
	}
	#[pallet::type_value]
	pub fn DefaultInflationUpperBound<T: Config>() -> u128 {
		10000
	}
	#[pallet::type_value]
	pub fn DefaultInflationLowerBound<T: Config>() -> u128 {
		8000
	}
	#[pallet::type_value]
	pub fn DefaultTimeDecay<T: Config>() -> u64 {
		10000
	}
	#[pallet::type_value]
	pub fn DefaultLastSubnetInitializedBlock<T: Config>() -> u64 {
		0
	}
	#[pallet::type_value]
	pub fn DefaultMaxSubnets<T: Config>() -> u32 {
		32
	}
	#[pallet::type_value]
	pub fn DefaultSubnetPerNodeInitCost<T: Config>() -> u128 {
		// 28e+18 as u128
		// 0 as u128
		10e+18 as u128
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
		280000000000000000000000
	}
	#[pallet::type_value]
	pub fn DefaultMinStakeBalance<T: Config>() -> u128 {
		1000e+18 as u128
	}
	#[pallet::type_value]
	pub fn DefaultMaxDelegateStakeBalance<T: Config>() -> u128 {
		280000000000000000000000
	}
	#[pallet::type_value]
	pub fn DefaultMinDelegateStakeBalance<T: Config>() -> u128 {
		1000e+18 as u128
	}
	#[pallet::type_value]
	pub fn DefaultDelegateStakeRewardsPercentage() -> u128 {
		1100
	}
	#[pallet::type_value]
	pub fn DefaultStakeRewardWeight<T: Config>() -> u128 {
		4000
	}
	#[pallet::type_value]
	pub fn DefaultMaxSubnetRewardsWeight<T: Config>() -> u128 {
		4800
	}	
	#[pallet::type_value]
	pub fn DefaultSubnetNodeConsensusPercentRequirement<T: Config>() -> u8 {
		75
	}
	#[pallet::type_value]
	pub fn DefaultSubnetNodeConsensusSubmitPercentRequirement<T: Config>() -> u128 {
		5100
	}
	#[pallet::type_value]
	pub fn DefaultMaximumOutlierDeltaPercent<T: Config>() -> u8 {
		// @to-do: Update to u128 (10000 == 100.00) for accuracy
		1
	}
	#[pallet::type_value]
	pub fn DefaultMaxAccountPenaltyCount<T: Config>() -> u32 {
		12
	}
	#[pallet::type_value]
	pub fn DefaultMaxSubnetNodeConsecutiveConsensusNotSent<T: Config>() -> u32 {
		2
	}
	#[pallet::type_value]
	pub fn DefaultSubnetConsensusUnconfirmedThreshold<T: Config>() -> u128 {
		5100
	}
	#[pallet::type_value]
	pub fn DefaultMaxSubnetConsensusUnconfirmedConsecutiveEpochs<T: Config>() -> u32 {
		2
	}
	#[pallet::type_value]
	pub fn DefaultNodeConsensusEpochSubmitted<T: Config>() -> bool {
		false
	}
	#[pallet::type_value]
	pub fn DefaultNodeConsensusEpochUnconfirmed<T: Config>() -> bool {
		false
	}
	#[pallet::type_value]
	pub fn DefaultMaxZeroConsensusEpochs<T: Config>() -> u32 {
		100
	}
	#[pallet::type_value]
	pub fn DefaultMaxSubnetConsensusEpochsErrors<T: Config>() -> u32 {
		100
	}
	#[pallet::type_value]
	pub fn DefaultMaxSubnetResponseErrors<T: Config>() -> u32 {
		100
	}
	#[pallet::type_value]
	pub fn DefaultSubnetsInConsensus<T: Config>() -> Vec<u32> {
		Vec::new()
	}
	#[pallet::type_value]
	pub fn DefaultSubnetConsecutiveEpochsThreshold<T: Config>() -> u32 {
		100
	}
	#[pallet::type_value]
	pub fn DefaultSubnetNodeConsecutiveConsensusSent<T: Config>() -> u32 {
		0
	}
	#[pallet::type_value]
	pub fn DefaultSubnetNodeConsecutiveConsensusNotSent<T: Config>() -> u32 {
		2
	}
	#[pallet::type_value]
	pub fn DefaultAccountantDataChallengePeriod<T: Config>() -> u64 {
		1000
	}
	#[pallet::type_value]
	pub fn DefaultAccountantData<T: Config>() -> AccountantDataParams<T::AccountId> {
		return AccountantDataParams {
			accountant: T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap(),
			block: 0,
			epoch: 0,
			data: Vec::new(),
		};
	}
	#[pallet::type_value]
	pub fn DefaultAccountantDataNodeParamsMaxLimit() -> u32 {
		1024_u32
	}
	#[pallet::type_value]
	pub fn DefaultSubnetRewardsSubmission<T: Config>() -> RewardsData<T::AccountId> {
		return RewardsData {
			validator: T::AccountId::decode(&mut TrailingZeroInput::zeroes()).unwrap(),
			nodes_count: 0,
			sum: 0,
			attests: BTreeSet::new(),
			data: Vec::new(),
		}
	}
	#[pallet::type_value]
	pub fn DefaultTrue() -> bool {
		true
	}
	#[pallet::type_value]
	pub fn DefaultFalse() -> bool {
		false
	}
	#[pallet::type_value]
	pub fn DefaultBaseReward() -> u128 {
		1e+18 as u128
	}
	#[pallet::type_value]
	pub fn DefaultBaseRewardPerMillion() -> u128 {
		1e+18 as u128
	}
	#[pallet::type_value]
	pub fn DefaultBaseRewardPerGB() -> u128 {
		1e+18 as u128
	}
	#[pallet::type_value]
	pub fn DefaultBaseSubnetReward() -> u128 {
		9e+18 as u128
	}
	#[pallet::type_value]
	pub fn DefaultBaseSubnetDelegateReward() -> u128 {
		4e+18 as u128
	}
	#[pallet::type_value]
	pub fn DefaultMinAttestationPercentage() -> u128 {
		6600
	}
	#[pallet::type_value]
	pub fn DefaultSlashPercentage() -> u128 {
		312
	}
	#[pallet::type_value]
	pub fn DefaultMaxSlashAmount() -> u128 {
		1e+18 as u128
	}
	#[pallet::type_value]
	pub fn DefaultMaxSubnetNodes() -> u32 {
		96
	}
	#[pallet::type_value]
	pub fn DefaultSubnetNodeClassEpochs() -> u64 {
		2
	}
	#[pallet::type_value]
	pub fn DefaultSubnetNodesClasses<T: Config>() -> BTreeMap<T::AccountId, u64> {
		BTreeMap::new()
	}
	#[pallet::type_value]
	pub fn DefaultZeroU32() -> u32 {
		0
	}
	#[pallet::type_value]
	pub fn DefaultMaxSequentialAbsentSubnetNode() -> u32 {
		3
	}
	#[pallet::type_value]
	pub fn DefaultBaseSubnetNodeMemoryMB() -> u128 {
		16000
	}
	#[pallet::type_value]
	pub fn DefaultMaxSubnetMemoryMB() -> u128 {
		500000
	}
	#[pallet::type_value]
	pub fn DefaultTargetSubnetNodesMultiplier() -> u128 {
		3333
	}
	#[pallet::type_value]
	pub fn DefaultTargetAccountantsLength() -> u32 {
		2
	}
	#[pallet::type_value]
	pub fn DefaultVoteSubnetData() -> VoteSubnetData {
		let pre_subnet_data = PreSubnetData {
			path: Vec::new(),
			memory_mb: 0,
		};
		return VoteSubnetData {
			data: pre_subnet_data,
			active: false,
		}
	}
	#[pallet::type_value]
	pub fn DefaultProposalsCount() -> u32 {
		0
	}

	


	
	

	
	/// If subnet is activate for rewards or general blockchain interfacing
	// subnet_path => boolean
	// #[pallet::storage]
	// pub type SubnetActivated<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, bool>;

	// #[pallet::storage]
	// pub type SubnetActivated<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, VoteSubnetData>;

	#[pallet::storage] // subnet ID => account_id
	pub type SubnetActivated<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		Vec<u8>,
		VoteSubnetData,
		ValueQuery,
		DefaultVoteSubnetData,
	>;

	// #[pallet::storage] // account_id --> [model_ids]
	// pub type SubnetActivated<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, VoteSubnetData, ValueQuery>;

	/// Max subnets at any given time
	#[pallet::storage]
	#[pallet::getter(fn max_models)]
	pub type MaxSubnets<T> = StorageValue<_, u32, ValueQuery, DefaultMaxSubnets<T>>;

	// Ensures no duplicate subnet paths within the network at one time
	// If a subnet path is voted out, it can be voted up later on and any
	// stakes attached to the subnet_id won't impact the re-initialization
	// of the subnet path.
	#[pallet::storage]
	#[pallet::getter(fn models_v3)] // subnet_path --> subnet_id
	pub type SubnetPaths<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, u32>;

	/// Mapping of each subnet stored by ID, uniqued by `SubnetPaths`
	// Stores subnet data by a unique id
	#[pallet::storage] // subnet_id => data struct
	pub type SubnetsData<T: Config> = StorageMap<_, Blake2_128Concat, u32, SubnetData>;

	/// Cost to initialize a new subnet based on count of current subnet peers
	// Should be `cost * live_peers_count`
	// See `get_model_initialization_cost()`
	#[pallet::storage]
	pub type SubnetPerNodeInitCost<T> = StorageValue<_, u128, ValueQuery, DefaultSubnetPerNodeInitCost<T>>;
	
	// Percentage of the beginning of an epoch for subnet peer to exit blockchain storage
	// At the beginning of each epoch, subnet peers can exit the blockchain, but only within this time frame
	// represented as a percentage of the epoch
	// We only allow peers to update or remove themselves in order to not disrupt the consensus
	#[pallet::storage]
	pub type RemoveSubnetNodeEpochPercentage<T> = StorageValue<_, u128, ValueQuery, DefaultRemoveSubnetNodeEpochPercentage<T>>;

	/// Count of subnets
	#[pallet::storage]
	#[pallet::getter(fn total_models)]
	pub type TotalSubnets<T> = StorageValue<_, u32, ValueQuery>;

	/// Amount of epochs a subnet has to acquire submittable subnet peers based on the MinSubnetNodes
	// If MinSubnetNodes is not reached by this time anyone can remove the subnet
	#[pallet::storage]
	pub type SubnetNodesInitializationEpochs<T> = StorageValue<_, u64, ValueQuery, DefaultSubnetNodesInitializationEpochs<T>>;

	// Minimum amount of peers required per subnet
	// required for subnet activity
	#[pallet::storage]
	#[pallet::getter(fn min_subnet_nodes)]
	pub type MinSubnetNodes<T> = StorageValue<_, u32, ValueQuery, DefaultMinSubnetNodes>;

	// Maximim peers in a subnet at any given time
	#[pallet::storage]
	#[pallet::getter(fn max_subnet_nodes)]
	pub type MaxSubnetNodes<T> = StorageValue<_, u32, ValueQuery, DefaultMaxSubnetNodes>;

	// Data per subnet peer
	// This is the main logic for subnet peers
	#[pallet::storage] // subnet_id --> account_id --> data
	#[pallet::getter(fn subnet_nodes)]
	pub type SubnetNodesData<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		SubnetNode<T::AccountId>,
		ValueQuery,
		DefaultSubnetNode<T>,
	>;

	// Tracks each subnet an account is a subnet peer on
	// This is used as a helper when removing accounts from all subnets they are peers on
	#[pallet::storage] // account_id --> [model_ids]
	pub type AccountSubnets<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Vec<u32>, ValueQuery>;

	// Total count of subnet peers within a subnet
	#[pallet::storage] // model_uid --> peer_data
	#[pallet::getter(fn total_subnet_nodes)]
	pub type TotalSubnetNodes<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, u32, ValueQuery>;

	// Used for unique peer_ids
	#[pallet::storage] // subnet_id --> account_id --> peer_id
	#[pallet::getter(fn subnet_node_account)]
	pub type SubnetNodeAccount<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		PeerId,
		T::AccountId,
		ValueQuery,
		DefaultAccountId<T>,
	>;

	// This can only be removed by accounts themselves and will be stored even after a subnet is removed until each account removed its stake
	// Used primarily for staking as a fail safe if subnets or peers get removed due to being out of consensus
	// Unlike SubnetNodesData this never deletes until staking is 0
	// u64 is either the initialized block or the removal block
	//		Updates to block of add or remove peer, whichever is latest
	//		This works with MinRequiredUnstakeEpochs
	#[pallet::storage] // subnet_id --> (account_id, (initialized or removal block))
	pub type SubnetAccount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		BTreeMap<T::AccountId, u64>,
		ValueQuery,
	>;

	// Amount of epochs for removed subnets peers required to unstake
	#[pallet::storage]
	pub type MinRequiredUnstakeEpochs<T> = StorageValue<_, u64, ValueQuery, DefaultMinRequiredUnstakeEpochs<T>>;

	// Total stake sum of all peers in all subnets
	#[pallet::storage] // ( total_stake )
	#[pallet::getter(fn total_stake)]
	pub type TotalStake<T: Config> = StorageValue<_, u128, ValueQuery>;

	// Total stake sum of all peers in specified subnet
	#[pallet::storage] // model_uid --> peer_data
	#[pallet::getter(fn total_model_stake)]
	pub type TotalSubnetStake<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, u128, ValueQuery>;

	// An accounts stake per subnet
	#[pallet::storage] // account--> subnet_id --> u128
	#[pallet::getter(fn account_model_stake)]
	pub type AccountSubnetStake<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Identity,
		u32,
		u128,
		ValueQuery,
		DefaultAccountTake<T>,
	>;

	// An accounts stake across all subnets
	#[pallet::storage] // account_id --> all subnets balance
	#[pallet::getter(fn total_account_stake)]
	pub type TotalAccountStake<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u128, ValueQuery>;

	// Maximum stake balance per subnet
	// Only checked on `do_add_stake` and ``
	// A subnet staker can have greater than the max stake balance although any rewards
	// they would receive based on their stake balance will only account up to the max stake balance allowed
	#[pallet::storage]
	pub type MaxStakeBalance<T: Config> = StorageValue<_, u128, ValueQuery, DefaultMaxStakeBalance<T>>;

	// Minimum required subnet peer stake balance per subnet
	#[pallet::storage]
	pub type MinStakeBalance<T: Config> = StorageValue<_, u128, ValueQuery, DefaultMinStakeBalance<T>>;
		
	// Target stake balance per subnet node
	#[pallet::storage]
	pub type TargetStake<T: Config> = StorageValue<_, u128, ValueQuery, DefaultMinStakeBalance<T>>;

	// // Emissions calculations
	// #[pallet::storage]
	// pub type OptimalSubnets<T> = StorageValue<_, u32, ValueQuery, DefaultOptimalSubnets<T>>;

	// #[pallet::storage]
	// pub type OptimalNodesPerSubnet<T> = StorageValue<_, u32, ValueQuery, DefaultOptimalNodesPerSubnet<T>>;
	
	#[pallet::storage]
	pub type InflationUpperBound<T> = StorageValue<_, u128, ValueQuery, DefaultInflationUpperBound<T>>;

	#[pallet::storage]
	pub type InflationLowerBound<T> = StorageValue<_, u128, ValueQuery, DefaultInflationLowerBound<T>>;

	#[pallet::storage]
	pub type TimeDecay<T> = StorageValue<_, u64, ValueQuery, DefaultTimeDecay<T>>;

	#[pallet::storage]
	pub type LastSubnetInitializedBlock<T> = StorageValue<_, u64, ValueQuery, DefaultLastSubnetInitializedBlock<T>>;
	
	// Delegate staking logic 

	// Amount of epochs for subnet delegate stakers required to unstake
	#[pallet::storage]
	pub type MinRequiredDelegateUnstakeEpochs<T> = StorageValue<_, u64, ValueQuery, DefaultMinRequiredDelegateUnstakeEpochs<T>>;

	// Total stake sum of all peers in specified subnet
	#[pallet::storage] // model_uid --> peer_data
	pub type TotalSubnetDelegateStakeShares<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, u128, ValueQuery>;

	// Total stake sum of all peers in specified subnet
	#[pallet::storage] // model_uid --> peer_data
	pub type TotalSubnetDelegateStakeBalance<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, u128, ValueQuery>;

	// An accounts stake per subnet
	#[pallet::storage] // account --> subnet_id --> u128
	pub type AccountSubnetDelegateStakeShares<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Identity,
		u32,
		u128,
		ValueQuery,
		DefaultAccountTake<T>,
	>;

	#[pallet::storage]
	pub type MaxDelegateStakeBalance<T: Config> = StorageValue<_, u128, ValueQuery, DefaultMaxDelegateStakeBalance<T>>;

	#[pallet::storage]
	pub type MinDelegateStakeBalance<T: Config> = StorageValue<_, u128, ValueQuery, DefaultMinDelegateStakeBalance<T>>;

	#[pallet::storage] // subnet_id --> (account_id, (initialized or removal block))
	pub type SubnetAccountDelegateStake<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		BTreeMap<T::AccountId, u64>,
		ValueQuery,
	>;

	// Percentage of epoch rewards that go towards delegate stake pools
	#[pallet::storage]
	pub type DelegateStakeRewardsPercentage<T: Config> = StorageValue<_, u128, ValueQuery, DefaultDelegateStakeRewardsPercentage>;

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
	pub type SubnetNodeConsensusEpoch<T> = StorageValue<_, u64, ValueQuery>;

	// Vector of model_ids stored during `form_consensus()` then used to generate rewards in ``
	// This is by default an empty vector and resets back to an empty vector each time `` is called
	#[pallet::storage]
	pub type SubnetsInConsensus<T> = StorageValue<_, Vec<u32>, ValueQuery, DefaultSubnetsInConsensus<T>>;

	// total consensus submits and unconfirms on epoch, reset each epoch
	#[pallet::storage]
	#[pallet::getter(fn model_total_consensus_submits)] 
	pub type SubnetTotalConsensusSubmits<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;

	// The threshold of epochs for a subnets consensus successes to reach to increment error count down
	#[pallet::storage]
	pub type SubnetConsecutiveEpochsThreshold<T> = StorageValue<_, u32, ValueQuery, DefaultSubnetConsecutiveEpochsThreshold<T>>;

	// The total count of successful epochs in a row
	#[pallet::storage]
	pub type SubnetConsecutiveSuccessfulEpochs<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;

	// Max epochs where consensus isn't formed before subnet being removed
	#[pallet::storage]
	pub type MaxSubnetConsensusEpochsErrors<T> = StorageValue<_, u32, ValueQuery, DefaultMaxSubnetConsensusEpochsErrors<T>>;
	
	// Count of epochs a subnet has consensus errors
	// This can incrase on the following issues:
	//	1. Not enough submit-able peers submitted consensus data.
	//	2. The subnet doesn't reach the required 0.01% stake balance towards the subnet versus all other live subnets.
	//	3. The subnet consensus submission data is `unconfirmed` sequentially too many times based on
	//				MaxSubnetConsensusUnconfirmedConsecutiveEpochs
	//				SubnetConsensusUnconfirmedConsecutiveEpochsCount
	//
	#[pallet::storage] // subnet_id => count
	pub type SubnetConsensusEpochsErrors<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;

	// Max epochs where consensus isn't formed before subnet being removed
	#[pallet::storage]
	pub type MaxSubnetPenalties<T> = StorageValue<_, u32, ValueQuery, DefaultMaxSubnetConsensusEpochsErrors<T>>;
	
	// Count of epochs a subnet has consensus errors
	#[pallet::storage] // subnet_id => count
	pub type SubnetPenaltyCount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;
	
	// The Max errors from expected return values when calling a subnet
	// *NOT IMPLEMENTED YET
	#[pallet::storage]
	pub type MaxSubnetResponseErrors<T> = StorageValue<_, u32, ValueQuery, DefaultMaxZeroConsensusEpochs<T>>;

	// Tracks errors from expected return values when calling a subnet
	// Stored count of subnet response errors
	// Ran through offchain worker
	// Stored by validator
	// *NOT IMPLEMENTED YET
	#[pallet::storage] // subnet_id --> errors count
	pub type SubnetResponseErrors<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;
	
	// The max errors from expected return values when calling a subnet through a subnet peer's server
	// *NOT IMPLEMENTED YET
	#[pallet::storage]
	pub type MaxAccountResponseErrors<T> = StorageValue<_, u32, ValueQuery, DefaultMaxZeroConsensusEpochs<T>>;

	// Tracks errors from expected return values when calling a subnet through a subnet peer's server
	// Stored count of account subnet response errors
	// Ran through offchain worker
	// Stored by validator
	// *NOT IMPLEMENTED YET
	#[pallet::storage] // subnet_id --> errors count
	pub type AccountResponseErrors<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		u32,
		ValueQuery,
	>;

	// If subnet peer sent consensus data during epoch
	#[pallet::storage] // subnet_id --> account -> boolean
	pub type NodeConsensusEpochSubmitted<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		bool,
		ValueQuery,
		DefaultNodeConsensusEpochSubmitted<T>,
	>;

	// If subnet subnet peer unconfirmed consensus data during epoch
	//
	// There should be no incentive to unconfirm consensus data outside of subnets out of a healthy state
	// If unconfirmed, subnet peer receives no rewards
	#[pallet::storage]
	pub type NodeConsensusEpochUnconfirmed<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		bool,
		ValueQuery,
		DefaultNodeConsensusEpochUnconfirmed<T>,
	>;

	// Works alongside SubnetConsensusUnconfirmedThreshold
	// Count of subnet peers that confirm consensus should be formed
	#[pallet::storage]
	pub type SubnetConsensusEpochSubmitCount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;
		
	// Percentage (100.00 as 10000) of subnet peers submitting consensus to deem a subnet in an error or alike state
	// If enough error submissions come in then consensus is skipped for the subnet
	// This is an important feature in case a subnet is unhealthy nearing the end of the epoch
	// This avoids peers that submit consensus data later in the epoch that cannot query accurate subnet peer scores from
	// the decentralized machine-learning subnet hosting network from submitting illegitimate consensus data
	#[pallet::storage]
	pub type SubnetConsensusUnconfirmedThreshold<T> = StorageValue<_, u128, ValueQuery, DefaultSubnetConsensusUnconfirmedThreshold<T>>;

	// Works alongside SubnetConsensusUnconfirmedThreshold
	// Count of subnet peers that confirm consensus should not be formed
	#[pallet::storage]
	pub type SubnetConsensusEpochUnconfirmedCount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;

	// The max epochs to a subnet can sequentially be unconfirmed before incrementing SubnetConsensusEpochsErrors
	// Increments SubnetConsensusEpochsErrors is SubnetConsensusUnconfirmedConsecutiveEpochsCount > MaxSubnetConsensusUnconfirmedConsecutiveEpochs
	#[pallet::storage]
	pub type MaxSubnetConsensusUnconfirmedConsecutiveEpochs<T> = StorageValue<_, u32, ValueQuery, DefaultMaxSubnetConsensusUnconfirmedConsecutiveEpochs<T>>;

	// The sequential count of epochs a subnet has unconfirmed its consensus data
	// This resets on a successful consensus to zero
	#[pallet::storage]
	pub type SubnetConsensusUnconfirmedConsecutiveEpochsCount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;

	// The maximum amount of times in a row a subnet peer can miss consensus before incrementing AccountPenaltyCount
	#[pallet::storage]
	pub type MaxSubnetNodeConsecutiveConsensusNotSent<T> = StorageValue<
		_, 
		u32, 
		ValueQuery, 
		DefaultMaxSubnetNodeConsecutiveConsensusNotSent<T>
	>;
	
	// Count of how many times in a row a subnet peer missed consensus
	#[pallet::storage] // account_id --> u32
	pub type SubnetNodeConsecutiveConsensusNotSent<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		u32,
		ValueQuery,
		DefaultSubnetNodeConsecutiveConsensusNotSent<T>,
	>;

	// Count of how many times in a row a subnet peer missed consensus
	// *NOT IN USE
	#[pallet::storage] // account_id --> u32
	pub type LatestSubnetNodeConsensusSubmissionData<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		ConsensusSubmissionDataParams<ConsensusType>,
		ValueQuery,
		DefaultConsensusSubmissionData<T>,
	>;
	
	// The maximum amount of times in a row a subnet peer can miss consensus before incrementing AccountPenaltyCount
	#[pallet::storage]
	pub type SubnetNodeConsecutiveConsensusSentThreshold<T> = StorageValue<
		_, 
		u32, 
		ValueQuery, 
		DefaultMaxSubnetNodeConsecutiveConsensusNotSent<T>
	>;
	
	// Count of how many times in a row a subnet peer successfully submitted consensus
	// When submitted enough times in a row, a subnet peer can have their penalties incremented down
	// based on SubnetNodeConsecutiveConsensusSentThreshold
	#[pallet::storage]
	pub type SubnetNodeConsecutiveConsensusSent<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		u32,
		ValueQuery,
		DefaultSubnetNodeConsecutiveConsensusSent<T>,
	>;
	
	// Epochs required from subnet initialization block to accept consensus submissions
	// Epochs required based on EpochLength
	// Each epoch is EpochLength
	// Min required epochs for a subnet to be in storage for based on initialized
	#[pallet::storage]
	#[pallet::getter(fn min_required_model_consensus_submit_epochs)]
	pub type MinRequiredSubnetConsensusSubmitEpochs<T> = StorageValue<_, u64, ValueQuery, DefaultMinRequiredSubnetConsensusSubmitEpochs<T>>;

	// Epochs required from peer initialization block to submit consensus
	// Epochs required based on EpochLength
	// Each epoch is EpochLength
	// This must always be at least 1 epoch
	// Must always be greater than MinRequiredNodeConsensusInclusionEpochs
	//
	// This is used to ensure peers aren't misusing add_subnet_node() function
	// Combined with MinRequiredNodeConsensusInclusionEpochs peers are required to be
	// in consensus before they can submit any data.
	// Rewards are emitted if required epochs are reached, submitted, and is in consensus
	// Node won't receive rewards if they don't meet this requirement
	#[pallet::storage]
	#[pallet::getter(fn min_required_peer_consensus_submit_epochs)]
	pub type MinRequiredNodeConsensusSubmitEpochs<T> = StorageValue<_, u64, ValueQuery, DefaultMinRequiredNodeConsensusSubmitEpochs<T>>;

	// Epochs required to be included in consensus
	// Epochs required based on EpochLength
	// Each epoch is EpochLength
	// This must always be at least 1 epoch
	// Must always be less than MinRequiredNodeConsensusSubmitEpochs
	//
	// This is used to ensure peers aren't misusing add_subnet_node() function
	// If a peer is not hosting a subnet theoretically consensus submitters will
	// have them removed before they are able to submit consensus data.
	#[pallet::storage]
	#[pallet::getter(fn min_required_consensus_epochs)]
	pub type MinRequiredNodeConsensusInclusionEpochs<T> = StorageValue<_, u64, ValueQuery, DefaultMinRequiredNodeConsensusInclusionEpochs<T>>;

	// Epochs required to be able to propose and vote on subnet peer dishonesty
	#[pallet::storage]
	pub type MinRequiredNodeAccountantEpochs<T> = StorageValue<_, u64, ValueQuery, DefaultMinRequiredNodeAccountantEpochs<T>>;

	// Consensus data submitted and filtered per epoch
	#[pallet::storage] // subnet => account_id => consensus results
	#[pallet::getter(fn subnet_node_consensus_results)]
	pub type SubnetNodeConsensusResults<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		SubnetNodeConsensusResultsParams<T::AccountId>,
		ValueQuery,
		DefaultSubnetNodeConsensusResults<T>,
	>;

	// Total count of proposals used for unique identifiers
	#[pallet::storage]
	pub type DishonestyProposalsCount<T> = StorageValue<_, u32, ValueQuery>;

	// Track total active proposals
	#[pallet::storage]
	pub type ActiveDishonestyProposalsCount<T> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage] // Period in blocks for votes after challenge
	pub type VotingPeriod<T> = StorageValue<_, u64, ValueQuery, DefaultDishonestyVotingPeriod<T>>;

	#[pallet::storage] // Period in blocks after proposal to challenge proposal
	pub type ChallengePeriod<T> = StorageValue<_, u64, ValueQuery, DefaultChallengePeriod<T>>;

	#[pallet::storage] // How many votes are needed
	pub type ProposalQuorum<T> = StorageValue<_, u128, ValueQuery, DefaultDishonestyProposalQuorum<T>>;

	// Consensus required to pass proposal
	#[pallet::storage]
	pub type ProposalConsensusThreshold<T> = StorageValue<_, u128, ValueQuery, DefaultDishonestyProposalConsensusThreshold<T>>;

	#[pallet::storage] // subnet_id => proposal_id => proposal parameters
	pub type DishonestyProposal<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		u32,
		DishonestyProposalParams<T::AccountId>,
		ValueQuery,
		DefaultDishonestyProposal<T>,
	>;

	// #[pallet::storage] // subnet => account_id => proposals
	// pub type Proposals<T: Config> = StorageDoubleMap<
	// 	_,
	// 	Blake2_128Concat,
	// 	u32,
	// 	Identity,
	// 	T::AccountId,
	// 	VoteParams<T::AccountId>,
	// 	ValueQuery,
	// 	DefaultVoteParams<T>,
	// >;

	#[pallet::storage] // subnet => proposal_id => proposal
	pub type Proposals<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		u32,
		ProposalParams<T::AccountId>,
		ValueQuery,
		DefaultProposalParams<T>,
	>;

	#[pallet::storage] 
	pub type ProposalsCount<T> = StorageValue<_, u32, ValueQuery, DefaultProposalsCount>;

	// Amount required to put up as a proposer and challenger
	#[pallet::storage] 
	pub type ProposalBidAmount<T> = StorageValue<_, u128, ValueQuery, DefaultProposalBidAmount<T>>;

	#[pallet::storage]
	pub type AccountantDataCount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u32,
		ValueQuery,
	>;

	#[pallet::storage] // subnet ID => data_id => data
	pub type AccountantData<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		u32,
		AccountantDataParams<T::AccountId>,
		ValueQuery,
		DefaultAccountantData<T>,
	>;

	#[pallet::storage] // subnet ID => account_id
	pub type CurrentAccountant<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		T::AccountId,
		ValueQuery,
		DefaultAccountId<T>,
	>;

	// The current logic uses one accountant
	// This allows multiple accountants per epoch
	#[pallet::storage]
	pub type CurrentAccountant2<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		BTreeMap<T::AccountId, bool>,
		ValueQuery,
		// DefaultAccountId<T>,
	>;

	#[pallet::storage]
	pub type TargetAccountantsLength<T> = StorageValue<_, u32, ValueQuery, DefaultTargetAccountantsLength>;

	#[pallet::storage] // subnet ID => epoch  => data
	pub type CurrentAccountants<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		u32,
		BTreeMap<T::AccountId, bool>,
	>;

	// Time in blocks from data submission to challenge accountants data submission
	#[pallet::storage] 
	pub type AccountantDataChallengePeriod<T> = StorageValue<_, u64, ValueQuery, DefaultAccountantDataChallengePeriod<T>>;

	// Maximum delta a score can be from the average without incurring penalties
	#[pallet::storage]
	pub type MaximumOutlierDeltaPercent<T> = StorageValue<_, u8, ValueQuery, DefaultMaximumOutlierDeltaPercent<T>>;

	// Maximum subnet peer penalty count
	#[pallet::storage]
	pub type MaxAccountPenaltyCount<T> = StorageValue<_, u32, ValueQuery, DefaultMaxAccountPenaltyCount<T>>;

	// Count of times a peer is against consensus
	// This includes:
	// 1. being against other peers that conclude another peer is out of consensus
	// 2. being against other peers that conclude another peer is in consensus
	// 3. score delta is too high on consensus data submission
	// 4. not submitting consensus data
	#[pallet::storage] // account_id --> u32
	#[pallet::getter(fn subnet_node_penalty_count)]
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
	#[pallet::getter(fn subnet_node_consensus_submit_percent_requirement)]
	pub type SubnetNodeConsensusSubmitPercentRequirement<T: Config> = 
		StorageValue<_, u128, ValueQuery, DefaultSubnetNodeConsensusSubmitPercentRequirement<T>>;
		

	// Blocks per consensus form
	// #[pallet::storage]
	// pub type EpochLength<T: Config> = StorageValue<_, u64, ValueQuery, DefaultEpochLength<T>>;

	// Consensus threshold percentage for peer to be removed
	// If a peer is not sent in by enough of other peers based on NodeRemovalThreshold
	// They will be removed as a peer and will not longer generate incentives
	#[pallet::storage]
	pub type NodeRemovalThreshold<T: Config> = StorageValue<_, u128, ValueQuery, DefaultNodeRemovalThreshold<T>>;

	// Threshold percentage for peer to be removed
	// If a peer is against consensus in relation to the count of all consensus submissions
	// They will be removed as a peer and will not longer generate incentives
	// e.g. If a peer is against consensus passed the threshold on one epoch, they will gain
	//			AccountPenaltyCount's and also be removed as a subnet peer
	#[pallet::storage]
	pub type NodeAgainstConsensusRemovalThreshold<T: Config> = StorageValue<_, u128, ValueQuery, DefaultNodeAgainstConsensusRemovalThreshold<T>>;

	// // The max amount of times a peer can reach the NodeRemovalThreshold before being removed
	// // It's possible for a peer to not be "ONLINE" while they are online because their peer
	// #[pallet::storage]
	// pub type MaxNodeOutOfConsensusCount<T> = StorageValue<_, u32, ValueQuery>;

	// #[pallet::storage] // account_id --> u32
	// pub type NodeOutOfConsensusCount<T: Config> = StorageMap<
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
	#[pallet::storage] // maximum percentage of rewards a subnet can have per epoch
	pub type MaxSubnetRewardsWeight<T> = StorageValue<_, u128, ValueQuery, DefaultMaxSubnetRewardsWeight<T>>;

	// #[pallet::storage] // subnet ID => epoch  => data
	// pub type SubnetRewardsSubmission<T: Config> = StorageDoubleMap<
	// 	_,
	// 	Blake2_128Concat,
	// 	u32,
	// 	Identity,
	// 	u32,
	// 	RewardsData<T::AccountId>,
	// 	ValueQuery,
	// 	DefaultSubnetRewardsSubmission<T>,
	// >;

	#[pallet::storage] // subnet ID => epoch  => data
	pub type SubnetRewardsSubmission<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		u32,
		RewardsData<T::AccountId>,
	>;
	
	// The account responsible for validating the epochs rewards data
	#[pallet::storage] // subnet ID => epoch  => data
	pub type SubnetRewardsValidator<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		u32,
		T::AccountId,
		// ValueQuery,
		// DefaultAccountId<T>
	>;

	#[pallet::storage] // account_id --> u32
	pub type AttestedEpoch<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		bool,
		ValueQuery,
		DefaultFalse,
	>;

	#[pallet::storage] // title => data
	pub type ModelTypes<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		ModelType,
		ValueQuery
	>;


	// Base reward per subnet
	#[pallet::storage]
	pub type BaseSubnetReward<T> = StorageValue<_, u128, ValueQuery, DefaultBaseSubnetReward>;

	// Base reward premium per subnet based on delegate staking portions
	#[pallet::storage]
	pub type BaseSubnetDelegateReward<T> = StorageValue<_, u128, ValueQuery, DefaultBaseSubnetDelegateReward>;
	
	// Base reward per epoch for validators and accountants
	// This is the base reward to subnet validators on successful attestation
	// This is the base reward to accountants when they agree to validation data.?
	#[pallet::storage]
	pub type BaseReward<T> = StorageValue<_, u128, ValueQuery, DefaultBaseReward>;
	
	// Base reward per MB per epoch based on 4,380 MB per year
	#[pallet::storage]
	pub type BaseRewardPerGB<T> = StorageValue<_, u128, ValueQuery, DefaultBaseRewardPerGB>;

	
	// #[pallet::storage]
	// pub type BaseRewardPerGFLOP<T> = StorageValue<_, u128, ValueQuery, DefaultBaseReward>;

	// #[pallet::storage]
	// pub type MinSubnetNodesPerMillion<T> = StorageValue<_, u128, ValueQuery, DefaultMinSubnetNodes>;

	#[pallet::storage]
	pub type SlashPercentage<T> = StorageValue<_, u128, ValueQuery, DefaultSlashPercentage>;

	#[pallet::storage]
	pub type MaxSlashAmount<T> = StorageValue<_, u128, ValueQuery, DefaultMaxSlashAmount>;

	#[pallet::storage]
	pub type MaxSequentialAbsentSubnetNode<T> = StorageValue<_, u32, ValueQuery, DefaultMaxSequentialAbsentSubnetNode>;

	// If subnet node is absent from inclusion in consensus information or attestings
	#[pallet::storage] // subnet_id -> class_id -> BTreeMap(account_id, block)
	pub type SequentialAbsentSubnetNode<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		T::AccountId,
		u32,
		ValueQuery,
		DefaultZeroU32,
	>;

	// 
	#[pallet::storage]
	pub type MinAttestationPercentage<T> = StorageValue<_, u128, ValueQuery, DefaultMinAttestationPercentage>;
	
	#[pallet::storage]
	pub type BaseSubnetNodeMemoryMB<T> = StorageValue<_, u128, ValueQuery, DefaultBaseSubnetNodeMemoryMB>;

	#[pallet::storage]
	pub type MaxSubnetMemoryMB<T> = StorageValue<_, u128, ValueQuery, DefaultMaxSubnetMemoryMB>;

	#[pallet::storage]
	pub type TargetSubnetNodesMultiplier<T> = StorageValue<_, u128, ValueQuery, DefaultTargetSubnetNodesMultiplier>;

	#[derive(EnumIter, FromRepr, Copy, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
  pub enum SubnetNodeClass {
    Idle,
    Included,
		Submittable,
		Accountant
  }

	impl SubnetNodeClass {
    pub fn index(&self) -> usize {
			*self as usize
    }
	}

	// How many epochs until an account can reach the next node class
	// e.g. Idle 			2 epochs => account must be Idle for 2 epochs from their initialization epoch
	//			Included	2 epochs => account must be Included for 2 epochs from their initialization epoch
	#[pallet::storage] // subnet => account_id
	pub type SubnetNodeClassEpochs<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		SubnetNodeClass,
		u64,
		ValueQuery,
		DefaultSubnetNodeClassEpochs
	>;

	#[pallet::storage] // subnet_id -> class_id -> BTreeMap(account_id, block)
	pub type SubnetNodesClasses<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Identity,
		SubnetNodeClass,
		BTreeMap<T::AccountId, u64>,
		ValueQuery,
		DefaultSubnetNodesClasses<T>,
	>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Remove a subnet if the subnet has been voted out
		/// This can be done by anyone as long as the required conditions pass
		#[pallet::call_index(0)]
		#[pallet::weight({0})]
		pub fn remove_subnet(
			origin: OriginFor<T>, 
			subnet_id: u32,
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

			ensure!(
				SubnetsData::<T>::contains_key(subnet_id),
				Error::<T>::SubnetNotExist
			);

			let subnet = SubnetsData::<T>::get(subnet_id).unwrap();
			let subnet_path: Vec<u8> = subnet.path;
			let model_initialized: u64 = subnet.initialized;

			// ----
			// Subnets can be removed by
			// 		1. Subnet can be voted off
			//		2. Subnet can reach max zero consensus count
			//		3. Subnet can be offline too many times
			//		4. Subnet has min peers after initialization period
			// ----

			let mut reason_for_removal: Vec<u8> = Vec::new();

			// 1.
			// Check subnet voted out
			let activated: bool = match SubnetActivated::<T>::try_get(subnet_path.clone()) {
				Ok(data) => data.active,
				Err(()) => false,
			};

			// Push into reason
			if !activated {
				reason_for_removal.push(1)
			}

			// 2.
			// Subnet can reach max zero consensus count
			let zero_consensus_epochs: u32 = SubnetConsensusEpochsErrors::<T>::get(subnet_id);
			let max_zero_consensus_epochs: u32 = MaxSubnetConsensusEpochsErrors::<T>::get();
			let too_many_max_consensus_epochs: bool = zero_consensus_epochs > max_zero_consensus_epochs;

			// Push into reason
			if too_many_max_consensus_epochs {
				reason_for_removal.push(2)
			}

			// 3.
			// Check if subnet is offline too many times
			let is_offline: bool = false;

			// Push into reason
			if is_offline {
				reason_for_removal.push(3)
			}

			// 4.
			// Check if subnet has min amount of peers
			// If min peers are not met and initialization epochs has surpassed
			// then subnet can be removed
			let total_subnet_nodes: u32 = TotalSubnetNodes::<T>::get(subnet_id);
			let min_subnet_nodes: u32 = MinSubnetNodes::<T>::get();
			let block: u64 = Self::get_current_block_as_u64();
			let mut has_min_peers: bool = true;
			if total_subnet_nodes < min_subnet_nodes {
				let epoch_length: u64 = T::EpochLength::get();
				let subnet_nodes_initialization_epochs: u64 = SubnetNodesInitializationEpochs::<T>::get();
				// Ensure initialization epochs have passed
				// If not return false
				let initialized: bool = block < Self::get_eligible_epoch_block(
					epoch_length, 
					model_initialized, 
					subnet_nodes_initialization_epochs
				);
				// Push into reason
				if !initialized {
					reason_for_removal.push(4)
				}	
			}

			// Must have at least one of the possible reasons to be removed
			ensure!(
				!activated || too_many_max_consensus_epochs || is_offline || !has_min_peers,
				Error::<T>::SubnetCantBeRemoved
			);

			// Remove unique path
			SubnetPaths::<T>::remove(subnet_path.clone());
			// Remove subnet data
			SubnetsData::<T>::remove(subnet_id);

			// We don't subtract TotalSubnets since it's used for ids

			// Remove all peers data
			let _ = SubnetNodesData::<T>::clear_prefix(subnet_id, u32::MAX, None);
			let _ = TotalSubnetNodes::<T>::remove(subnet_id);
			let _ = SubnetNodeAccount::<T>::clear_prefix(subnet_id, u32::MAX, None);

			// Remove all subnet consensus data
			let _ = SubnetPenaltyCount::<T>::remove(subnet_id);
			let _ = SubnetNodesClasses::<T>::clear_prefix(subnet_id, u32::MAX, None);
	
			Self::deposit_event(Event::SubnetRemoved { 
				account: account_id, 
				subnet_id: subnet_id, 
				subnet_path: subnet_path.clone(),
				reason: reason_for_removal,
				block: block
			});

			Ok(())
		}

		/// Add a subnet peer that is currently hosting an AI subnet (or a peer in DHT)
		/// A minimum stake balance is required
		// Before adding subnet peer you must become a peer hosting the subnet of choice
		// This fn will claim your peer_id and associate it with your account as peer_id => account_id
		// If this reverts due to `SubnetNodeExist` you must remove the peer node and try again with a new peer_id
		// It's possible someone can claim the peer_id before you do
		// due to the requirement of staking this is an unlikely scenario.
		// Once you claim the peer_id, no one else can claim it.
		// After RequiredSubnetNodeEpochs pass and the peer is in consensus, rewards will be emitted to the account
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::add_subnet_node())]
		// #[pallet::weight({0})]
		pub fn add_subnet_node(
			origin: OriginFor<T>, 
			subnet_id: u32, 
			peer_id: PeerId, 
			stake_to_be_added: u128,
			// signature: T::OffchainSignature,
			// signer: T::AccountId,
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin.clone())?;

			let epoch_length: u64 = T::EpochLength::get();
			let block: u64 = Self::get_current_block_as_u64();

			ensure!(
				SubnetsData::<T>::contains_key(subnet_id),
				Error::<T>::SubnetNotExist
			);

			// Ensure account is eligible
			ensure!(
				Self::is_account_eligible(account_id.clone()),
				Error::<T>::AccountIneligible
			);
			
			// Ensure max peers isn't surpassed
			let total_subnet_nodes: u32 = TotalSubnetNodes::<T>::get(subnet_id);
			let max_subnet_nodes: u32 = MaxSubnetNodes::<T>::get();
			let max_subnet_nodes: u32 = MaxSubnetNodes::<T>::get();
			ensure!(
				total_subnet_nodes < max_subnet_nodes,
				Error::<T>::SubnetNodesMax
			);

			// Unique subnet_id -> AccountId
			// Ensure account doesn't already have a peer within subnet
			ensure!(
				!SubnetNodesData::<T>::contains_key(subnet_id, account_id.clone()),
				Error::<T>::SubnetNodeExist
			);

			// Unique subnet_id -> PeerId
			// Ensure peer ID doesn't already exist within subnet regardless of account_id
			let peer_exists: bool = match SubnetNodeAccount::<T>::try_get(subnet_id, peer_id.clone()) {
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

			// ====================
			// Initiate stake logic
			// ====================
			Self::do_add_stake(
				origin.clone(), 
				subnet_id,
				account_id.clone(),
				stake_to_be_added,
			).map_err(|e| e)?;

			// To ensure the AccountId that owns the PeerId, they must sign the PeerId for others to verify
			// This ensures others cannot claim to own a PeerId they are not the owner of
			// Self::validate_signature(&Encode::encode(&peer_id), &signature, &signer)?;

			// ========================
			// Insert peer into storage
			// ========================
			let subnet_node: SubnetNode<T::AccountId> = SubnetNode {
				account_id: account_id.clone(),
				peer_id: peer_id.clone(),
				initialized: block,
			};
			// Insert SubnetNodesData with account_id as key
			SubnetNodesData::<T>::insert(subnet_id, account_id.clone(), subnet_node);

			// Insert subnet peer account to keep peer_ids unique within subnets
			SubnetNodeAccount::<T>::insert(subnet_id, peer_id.clone(), account_id.clone());

			// Insert unstaking reinforcements
			// This data is specifically used for allowing unstaking after being removed
			// SubnetAccount is not removed from storage until the peer has unstaked their entire stake balance
			// This stores the block they are initialized at
			// If removed, the initialized block will be replace with the removal block
			let mut model_accounts: BTreeMap<T::AccountId, u64> = SubnetAccount::<T>::get(subnet_id);
			// let model_account: Option<&u64> = model_accounts.get(&account_id.clone());
			let block_initialized_or_removed: u64 = match model_accounts.get(&account_id.clone()) {
				Some(block_initialized_or_removed) => *block_initialized_or_removed,
				None => 0,
			};

			// If previously removed or removed themselves
			// Ensure they have either unstaked or have waited enough epochs to unstake
			// to readd themselves as a subnet peer
			if block_initialized_or_removed != 0 {
				let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<T>::get();
				// Ensure min required epochs have surpassed to unstake
				// Based on either initialized block or removal block
				ensure!(
					block >= Self::get_eligible_epoch_block(
						epoch_length, 
						block_initialized_or_removed, 
						min_required_unstake_epochs
					),
					Error::<T>::RequiredUnstakeEpochsNotMet
				);	
			}

			// Update to current block
			model_accounts.insert(account_id.clone(), block);
			SubnetAccount::<T>::insert(subnet_id, model_accounts);

			if let Ok(mut node_class) = SubnetNodesClasses::<T>::try_get(subnet_id, SubnetNodeClass::Idle) {
				node_class.insert(account_id.clone(), block);
				SubnetNodesClasses::<T>::insert(subnet_id, SubnetNodeClass::Idle, node_class);	
			} else {
				// If new subnet, initialize classes
				let mut node_class: BTreeMap<T::AccountId, u64> = BTreeMap::new();
				node_class.insert(account_id.clone(), block);
				SubnetNodesClasses::<T>::insert(subnet_id, SubnetNodeClass::Idle, node_class);	
			}

			// let mut node_class: BTreeMap<T::AccountId, u64> = SubnetNodesClasses::<T>::get(subnet_id, SubnetNodeClass::Idle).unwrap();
			// node_class.insert(account_id.clone(), block);
			// SubnetNodesClasses::<T>::insert(subnet_id, SubnetNodeClass::Idle, node_class);

			// Add subnet_id to account
			// Account can only have a subnet peer per subnet so we don't check if it exists
			AccountSubnets::<T>::append(account_id.clone(), subnet_id);

			// Increase total subnet peers
			TotalSubnetNodes::<T>::mutate(subnet_id, |n: &mut u32| *n += 1);

			Self::deposit_event(
				Event::SubnetNodeAdded { 
					subnet_id: subnet_id, 
					account_id: account_id.clone(), 
					peer_id: peer_id.clone(),
					block: block
				}
			);

			Ok(())
		}

		/// Update a subnet peer
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::update_subnet_node())]
		// #[pallet::weight({0})]
		pub fn update_subnet_node(
			origin: OriginFor<T>, 
			subnet_id: u32, 
			peer_id: PeerId,
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

			// --- Ensure subnet exists
			ensure!(
				SubnetsData::<T>::contains_key(subnet_id),
				Error::<T>::SubnetNotExist
			);

			ensure!(
				SubnetNodesData::<T>::contains_key(subnet_id, account_id.clone()),
				Error::<T>::SubnetNodeNotExist
			);
			
			// Unique subnet_id -> PeerId
			// Ensure peer ID doesn't already exist within subnet regardless of account_id
			let peer_exists: bool = match SubnetNodeAccount::<T>::try_get(subnet_id, peer_id.clone()) {
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
				
			let subnet_node = SubnetNodesData::<T>::get(subnet_id, account_id.clone());

			let block: u64 = Self::get_current_block_as_u64();
			let epoch_length: u64 = T::EpochLength::get();

			// Ensure updates are happening before consensus submissions
			ensure!(
				Self::can_remove_or_update_subnet_node(block, epoch_length),
				Error::<T>::InvalidRemoveOrUpdateSubnetNodeBlock
			);

			let submit_epochs = SubnetNodeClassEpochs::<T>::get(SubnetNodeClass::Submittable);

			// Check if subnet peer is eligible for consensus submission
			//
			// Updating a peer_id can only be done if the subnet peer can submit consensus data
			// Otherwise they must remove their peer and start a new one
			//
			// This is a backup incase subnets go down and subnet hosters all need to spin up
			// new nodes under new peer_id's
			ensure!(
				Self::is_epoch_block_eligible(
					block, 
					epoch_length, 
					submit_epochs, 
					subnet_node.initialized
				),
				Error::<T>::NodeConsensusSubmitEpochNotReached
			);

			// ====================
			// Mutate peer_id into storage
			// ====================
			SubnetNodesData::<T>::mutate(
				subnet_id,
				account_id.clone(),
				|params: &mut SubnetNode<T::AccountId>| {
					params.peer_id = peer_id.clone();
				}
			);

			// Update unique subnet peer_id
			SubnetNodeAccount::<T>::insert(subnet_id, peer_id.clone(), account_id.clone());

			Self::deposit_event(
				Event::SubnetNodeUpdated { 
					subnet_id: subnet_id, 
					account_id: account_id.clone(), 
					peer_id: peer_id.clone(),
					block: block
				}
			);

			Ok(())
		}

		/// Remove your subnet peer
		/// Unstaking must be done seperately
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::remove_subnet_node())]
		pub fn remove_subnet_node(
			origin: OriginFor<T>, 
			subnet_id: u32, 
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

			let epoch_length: u64 = T::EpochLength::get();
			let block: u64 = Self::get_current_block_as_u64();

			// --- Ensure subnet exists
			ensure!(
				SubnetsData::<T>::contains_key(subnet_id),
				Error::<T>::SubnetNotExist
			);

			ensure!(
				SubnetNodesData::<T>::contains_key(subnet_id, account_id.clone()),
				Error::<T>::SubnetNodeNotExist
			);

			// TODO: Track removal of subnet nodes following validator consensus data submission per epoch

			// We don't check consensus steps here because a subnet peers stake isn't included in calculating rewards 
			// that hasn't reached their consensus submission epoch yet
			
			Self::do_remove_subnet_node(block, subnet_id, account_id.clone());

			Ok(())
		}

		/// Remove a subnet peer that has surpassed the max penalties allowed
		// This is redundant 
		#[pallet::call_index(4)]
		#[pallet::weight({0})]
		pub fn remove_account_subnet_nodes(
			origin: OriginFor<T>, 
			account_id: T::AccountId, 
		) -> DispatchResult {
			ensure_signed(origin)?;

			let block: u64 = Self::get_current_block_as_u64();

			// We can skip `can_remove_or_update_subnet_node` because they should not be
			// included in consensus data

			// Ensure account is not eligible to be a subnet peer
			ensure!(
				!Self::is_account_eligible(account_id.clone()),
				Error::<T>::AccountEligible
			);

			Self::do_remove_account_subnet_nodes(block, account_id);

			Ok(())
		}
		
		/// Increase stake towards the specified subnet ID
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::add_to_stake())]
		// #[pallet::weight({0})]
		pub fn add_to_stake(
			origin: OriginFor<T>, 
			subnet_id: u32,
			stake_to_be_added: u128,
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin.clone())?;
			// Each account can only have one peer
			// Staking is accounted for per account_id per subnet_id
			// We only check that origin exists within SubnetNodesData

			let epoch_length: u64 = T::EpochLength::get();
			let block: u64 = Self::get_current_block_as_u64();

			// Ensure consensus isn't being formed or emissions are being generated
			ensure!(
				!Self::is_in_consensus_steps(block, epoch_length),
				Error::<T>::InvalidSubmitEpochLength
			);

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
			
			Self::do_add_stake(
				origin, 
				subnet_id,
				account_id.clone(),
				stake_to_be_added,
			)
		}

		/// Remove stake balance
		/// If account is a current subnet peer on the subnet ID they can only remove up to minimum required balance
		// Decrease stake on accounts peer if minimum required isn't surpassed
		// to-do: if removed through consensus, add removed_block to storage and require time 
		//				to pass until they can remove their stake
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::remove_stake())]
		// #[pallet::weight({0})]
		pub fn remove_stake(
			origin: OriginFor<T>, 
			subnet_id: u32, 
			stake_to_be_removed: u128
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin.clone())?;

			// // Ensure consensus isn't being formed or emissions are being generated
			// ensure!(
			// 	!Self::is_in_consensus_steps(block, epoch_length),
			// 	Error::<T>::InvalidSubmitEpochLength
			// );

      // Get SubnetAccount (this is not deleted until stake == 0)
			let model_accounts: BTreeMap<T::AccountId, u64> = SubnetAccount::<T>::get(subnet_id);

			// Check if removed all stake yet
			let has_model_account: bool = match model_accounts.get(&account_id.clone()) {
				Some(_) => true,
				None => false,
			};

			// If SubnetAccount doesn't exist this means they have been removed due their staking balance is at zero
			// Once balance is at zero they are removed from SubnetAccount in `do_remove_stake()`
			ensure!(
				has_model_account,
				Error::<T>::SubnetNodeNotExist
			);

			let block_initialized_or_removed: u64 = match model_accounts.get(&account_id.clone()) {
				Some(block_initialized_or_removed) => *block_initialized_or_removed,
				None => 0,
			};
			let min_required_unstake_epochs = MinRequiredUnstakeEpochs::<T>::get();

			let epoch_length: u64 = T::EpochLength::get();
			let block: u64 = Self::get_current_block_as_u64();

			// Ensure min required epochs have surpassed to unstake
			// Based on either initialized block or removal block
			ensure!(
				block >= Self::get_eligible_epoch_block(
					epoch_length, 
					block_initialized_or_removed, 
					min_required_unstake_epochs
				),
				Error::<T>::RequiredUnstakeEpochsNotMet
			);

			// If account is a peer they can remove stake up to minimum required stake balance
			// Else they can remove entire balance because they are not hosting subnets according to consensus
			//		They are removed in `do_remove_subnet_node()` when self or consensus removed
			let is_peer: bool = match SubnetNodesData::<T>::try_get(subnet_id, account_id.clone()) {
				Ok(_) => true,
				Err(()) => false,
			};

			// Remove stake
			// 		if_peer: cannot remove stake below minimum required stake
			// 		else: can remove total stake balance
			// if balance is zero then SubnetAccount is removed
			Self::do_remove_stake(
				origin, 
				subnet_id,
				account_id,
				is_peer,
				stake_to_be_removed,
			)
		}

		/// Increase stake towards the specified subnet ID
		#[pallet::call_index(7)]
		// #[pallet::weight(T::WeightInfo::add_to_stake())]
		#[pallet::weight({0})]
		pub fn add_to_delegate_stake(
			origin: OriginFor<T>, 
			subnet_id: u32,
			stake_to_be_added: u128,
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin.clone())?;

			// --- Ensure subnet exists
			ensure!(
				SubnetsData::<T>::contains_key(subnet_id),
				Error::<T>::SubnetNotExist
			);

			// Update accounts subnet stake add block
			let mut model_account_delegate_stakes: BTreeMap<T::AccountId, u64> = SubnetAccountDelegateStake::<T>::get(subnet_id);
			let block: u64 = Self::get_current_block_as_u64();

			// Insert or update the accounts subnet stake add block
			model_account_delegate_stakes.insert(account_id.clone(), block);
			SubnetAccount::<T>::insert(subnet_id, model_account_delegate_stakes);

			Self::do_add_delegate_stake(
				origin, 
				subnet_id,
				account_id.clone(),
				stake_to_be_added,
			)
		}

		/// Remove stake balance
		/// If account is a current subnet peer on the subnet ID they can only remove up to minimum required balance
		// Decrease stake on accounts peer if minimum required isn't surpassed
		// to-do: if removed through consensus, add removed_block to storage and require time 
		//				to pass until they can remove their stake
		#[pallet::call_index(8)]
		// #[pallet::weight(T::WeightInfo::remove_stake())]
		#[pallet::weight({0})]
		pub fn remove_delegate_stake(
			origin: OriginFor<T>, 
			subnet_id: u32, 
			stake_to_be_removed: u128
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin.clone())?;

			let min_required_delegate_unstake_epochs = MinRequiredDelegateUnstakeEpochs::<T>::get();
			let epoch_length: u64 = T::EpochLength::get();
			let block: u64 = Self::get_current_block_as_u64();

			let mut model_account_delegate_stakes: BTreeMap<T::AccountId, u64> = SubnetAccountDelegateStake::<T>::get(subnet_id);

			let block_added: u64 = match model_account_delegate_stakes.get(&account_id.clone()) {
				Some(block_added) => *block_added,
				None => 0,
			};

			// We don't ensure! if the account add block is zero 
			// If they have no stake, it will be ensure!'d in the delegate_staking.rs

			// Ensure min required epochs have surpassed to unstake
			// Based on either initialized block or removal block
			ensure!(
				block >= Self::get_eligible_epoch_block(
					epoch_length, 
					block_added, 
					min_required_delegate_unstake_epochs
				),
				Error::<T>::RequiredDelegateUnstakeEpochsNotMet
			);

			// Remove stake
			Self::do_remove_delegate_stake(
				origin, 
				subnet_id,
				account_id,
				stake_to_be_removed,
			)
		}

		/// Delete proposals that are no longer live
		#[pallet::call_index(9)]
		#[pallet::weight({0})]
		pub fn submit_accountant_data(
			origin: OriginFor<T>, 
			subnet_id: u32,
			data: Vec<AccountantDataNodeParams>,
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;
			let block: u64 = Self::get_current_block_as_u64();
			let epoch_length: u64 = T::EpochLength::get();
			let epoch: u64 = block / epoch_length;

			Self::do_submit_accountant_data(
				account_id,
				subnet_id,
				epoch as u32,	
				data,	
			)
		}

		/// Delete proposals that are no longer live
		#[pallet::call_index(10)]
		#[pallet::weight({0})]
		pub fn validate(
			origin: OriginFor<T>, 
			subnet_id: u32,
			data: Vec<SubnetNodeData>,
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

			let block: u64 = Self::get_current_block_as_u64();
			let epoch_length: u64 = T::EpochLength::get();
			let epoch: u64 = block / epoch_length;

			Self::do_validate(
				subnet_id, 
				account_id,
				block,
				epoch_length,
				epoch as u32,
				data,
			)
		}

		#[pallet::call_index(11)]
		#[pallet::weight({0})]
		pub fn attest(
			origin: OriginFor<T>, 
			subnet_id: u32,
		) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;

			let block: u64 = Self::get_current_block_as_u64();
			let epoch_length: u64 = T::EpochLength::get();
			let epoch: u64 = block / epoch_length;

			Self::do_attest(
				subnet_id, 
				account_id,
				block, 
				epoch_length,
				epoch as u32,
			)
		}

		#[pallet::call_index(12)]
		#[pallet::weight({0})]
		pub fn propose(
			origin: OriginFor<T>, 
			subnet_id: u32,
			peer_id: PeerId,
			data: Vec<u8>,
	) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;
	
			Self::do_propose(
				account_id,
				subnet_id,
				peer_id,
				data
			)
		}

		#[pallet::call_index(13)]
		#[pallet::weight({0})]
		pub fn challenge_proposal(
			origin: OriginFor<T>, 
			subnet_id: u32,
			proposal_id: u32,
			data: Vec<u8>,
	) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;
	
			Self::do_challenge_proposal(
				account_id,
				subnet_id,
				proposal_id,
				data
			)
		}

		#[pallet::call_index(14)]
		#[pallet::weight({0})]
		pub fn vote(
			origin: OriginFor<T>, 
			subnet_id: u32,
			proposal_id: u32,
			vote: VoteType
	) -> DispatchResult {
			let account_id: T::AccountId = ensure_signed(origin)?;
	
			Self::do_vote(
				account_id,
				subnet_id,
				proposal_id,
				vote
			)
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn activate_subnet(
			activator: T::AccountId,
			proposer: T::AccountId,
			subnet_data: PreSubnetData,
		) -> DispatchResult {
			// let activator: T::AccountId = ensure_signed(activator)?;

			// Ensure path is unique
			ensure!(
				!SubnetPaths::<T>::contains_key(subnet_data.clone().path),
				Error::<T>::SubnetExist
			);

			// not needed since can only be called by logic in subnet democracy pallet for activating new subnets
			// let activated: bool = match SubnetActivated::<T>::try_get(subnet_data.clone().path) {
			// 	Ok(data) => data.active,
			// 	Err(()) => false,
			// };

			// // Ensure subnet voted in
			// ensure!(
			// 	activated,
			// 	Error::<T>::SubnetNotVotedIn
			// );

			// Ensure max subnets not reached
			// Get total live subnets
			let total_subnets: u32 = (SubnetsData::<T>::iter().count()).try_into().unwrap();
			let max_subnets: u32 = MaxSubnets::<T>::get();
			ensure!(
				total_subnets < max_subnets,
				Error::<T>::MaxSubnets
			);

			let block: u64 = Self::get_current_block_as_u64();
			let model_cost: u128 = Self::get_model_initialization_cost(block);

			if model_cost > 0 {
				// unreserve from proposer
				let model_cost_as_balance = Self::u128_to_balance(model_cost);

				ensure!(
					Self::can_remove_balance_from_coldkey_account(&proposer, model_cost_as_balance.unwrap()),
					Error::<T>::NotEnoughBalanceToStake
				);
		
				ensure!(
					Self::remove_balance_from_coldkey_account(&proposer, model_cost_as_balance.unwrap()) == true,
					Error::<T>::BalanceWithdrawalError
				);

				// Send portion to stake rewards vault
				// Send portion to treasury

				// increase stake balance with subnet initialization cost
				StakeVaultBalance::<T>::mutate(|n: &mut u128| *n += model_cost);
			}

			// Get total subnets ever
			let subnet_len: u32 = TotalSubnets::<T>::get();
			// Start the model_ids at 1
			let subnet_id = subnet_len + 1;
			
			let base_node_memory: u128 = BaseSubnetNodeMemoryMB::<T>::get();
	
			let min_subnet_nodes: u32 = Self::get_min_subnet_nodes(base_node_memory, subnet_data.memory_mb);
			let target_subnet_nodes: u32 = Self::get_target_subnet_nodes(base_node_memory, min_subnet_nodes);
	
			let subnet_data = SubnetData {
				id: subnet_id,
				path: subnet_data.clone().path,
				min_nodes: min_subnet_nodes,
				target_nodes: target_subnet_nodes,
				memory_mb: subnet_data.memory_mb,  
				initialized: block,
			};

			// Store unique path
			SubnetPaths::<T>::insert(subnet_data.clone().path, subnet_id);
			// Store subnet data
			SubnetsData::<T>::insert(subnet_id, subnet_data.clone());
			// Increase total subnets. This is used for unique Subnet IDs
			TotalSubnets::<T>::mutate(|n: &mut u32| *n += 1);

			LastSubnetInitializedBlock::<T>::set(block);

			Self::deposit_event(Event::SubnetAdded { 
				proposer: proposer, 
				activator: activator,
				subnet_id: subnet_id, 
				subnet_path: subnet_data.clone().path,
				block: block
			});

			Ok(())
		}

		pub fn deactivate_subnet(
			deactivator: T::AccountId,
			proposer: T::AccountId,
			subnet_data: PreSubnetData,
		) -> DispatchResult {
			// let account_id: T::AccountId = ensure_signed(origin)?;
			ensure!(
				SubnetPaths::<T>::contains_key(subnet_data.clone().path),
				Error::<T>::SubnetNotExist
			);

			let subnet_id = SubnetPaths::<T>::get(subnet_data.clone().path).unwrap();

			ensure!(
				SubnetsData::<T>::contains_key(subnet_id),
				Error::<T>::SubnetNotExist
			);

			let subnet = SubnetsData::<T>::get(subnet_id).unwrap();
			let subnet_path: Vec<u8> = subnet.path;

			// ----
			// Subnets can be removed by
			// 		1. Subnet can be voted off
			//		2. Subnet can reach max zero consensus count
			//		3. Subnet can be offline too many times
			//		4. Subnet has min peers after initialization period
			// ----

			let mut reason_for_removal: Vec<u8> = Vec::new();

			// // 1.
			// // Check subnet voted out
			// let activated: bool = match SubnetActivated::<T>::try_get(subnet_path.clone()) {
			// 	Ok(data) => data.active,
			// 	Err(()) => false,
			// };

			// // Push into reason
			// if !activated {
			// 	reason_for_removal.push(1)
			// }

			// 2.
			// // Subnet can reach max zero consensus count
			// let zero_consensus_epochs: u32 = SubnetConsensusEpochsErrors::<T>::get(subnet_id);
			// let max_zero_consensus_epochs: u32 = MaxSubnetConsensusEpochsErrors::<T>::get();
			// let too_many_max_consensus_epochs: bool = zero_consensus_epochs > max_zero_consensus_epochs;

			// // Push into reason
			// if too_many_max_consensus_epochs {
			// 	reason_for_removal.push(2)
			// }

			// 3.
			// // Check if subnet is offline too many times
			// let is_offline: bool = false;

			// // Push into reason
			// if is_offline {
			// 	reason_for_removal.push(3)
			// }

			// 4.
			// Check if subnet has min amount of peers
			// If min peers are not met and initialization epochs has surpassed
			// then subnet can be removed
			// let total_subnet_nodes: u32 = TotalSubnetNodes::<T>::get(subnet_id);
			// let min_subnet_nodes: u32 = subnet.min_nodes;
			// let block: u64 = Self::get_current_block_as_u64();
			// let mut has_min_peers: bool = true;
			// if total_subnet_nodes < min_subnet_nodes {
			// 	let model_initialized: u64 = subnet.initialized;
			// 	let epoch_length: u64 = T::EpochLength::get();
			// 	let subnet_nodes_initialization_epochs: u64 = SubnetNodesInitializationEpochs::<T>::get();
			// 	// Ensure initialization epochs have passed
			// 	// If not return false
			// 	let initialized: bool = block < Self::get_eligible_epoch_block(
			// 		epoch_length, 
			// 		model_initialized, 
			// 		subnet_nodes_initialization_epochs
			// 	);
			// 	// Push into reason
			// 	if !initialized {
			// 		reason_for_removal.push(4)
			// 	}	
			// }

			// Must have at least one of the possible reasons to be removed
			// ensure!(
			// 	!activated || too_many_max_consensus_epochs || is_offline || !has_min_peers,
			// 	Error::<T>::SubnetCantBeRemoved
			// );

			// Remove unique path
			SubnetPaths::<T>::remove(subnet_path.clone());
			// Remove subnet data
			SubnetsData::<T>::remove(subnet_id);

			// We don't subtract TotalSubnets since it's used for ids

			// Remove all peers data
			let _ = SubnetNodesData::<T>::clear_prefix(subnet_id, u32::MAX, None);
			let _ = TotalSubnetNodes::<T>::remove(subnet_id);
			let _ = SubnetNodeAccount::<T>::clear_prefix(subnet_id, u32::MAX, None);

			// Remove all subnet consensus data
			let _ = SubnetPenaltyCount::<T>::remove(subnet_id);
			let _ = SubnetNodesClasses::<T>::clear_prefix(subnet_id, u32::MAX, None);
	
			Self::deposit_event(Event::SubnetRemoved { 
				account: deactivator, 
				subnet_id: subnet_id, 
				subnet_path: subnet_path.clone(),
				reason: reason_for_removal,
				block: 0
			});

			Ok(())
		}

		fn is_model_eligible(
			subnet_id: u32, 
			subnet_path: Vec<u8>, 
			model_initialized: u64
		) -> (bool, Vec<u8>) {
			let mut reason_for_removal: Vec<u8> = Vec::new();

			// 1.
			// check subnet voted out
			// let voted: bool = match SubnetVoteOut::<T>::try_get(subnet_path.clone()) {
			// 	Ok(vote) => vote,
			// 	Err(()) => false,
			// };

			let activated: bool = match SubnetActivated::<T>::try_get(subnet_path.clone()) {
				Ok(data) => data.active,
				Err(()) => false,
			};

			// Push into reason
			if !activated {
				reason_for_removal.push(1)
			}

			// 2.
			// Subnet can reach max zero consensus count
			let zero_consensus_epochs: u32 = SubnetConsensusEpochsErrors::<T>::get(subnet_id);
			let max_zero_consensus_epochs: u32 = MaxSubnetConsensusEpochsErrors::<T>::get();
			let too_many_max_consensus_epochs: bool = zero_consensus_epochs > max_zero_consensus_epochs;

			// Push into reason
			if too_many_max_consensus_epochs {
				reason_for_removal.push(2)
			}

			// 3.
			// Check if subnet is offline too many times
			let is_offline: bool = false;

			// Push into reason
			if is_offline {
				reason_for_removal.push(3)
			}

			// 4.
			// Check if subnet has min amount of peers
			// If min peers are not met and initialization epochs has surpassed
			// then subnet can be removed
			let total_subnet_nodes: u32 = TotalSubnetNodes::<T>::get(subnet_id);
			let min_subnet_nodes: u32 = MinSubnetNodes::<T>::get();
			let mut has_min_peers: bool = true;
			if total_subnet_nodes < min_subnet_nodes {
				let block: u64 = Self::get_current_block_as_u64();
				let epoch_length: u64 = T::EpochLength::get();
				let subnet_nodes_initialization_epochs: u64 = SubnetNodesInitializationEpochs::<T>::get();
				// Ensure initialization epochs have passed
				// If not return false
				let has_min_peers: bool = block < Self::get_eligible_epoch_block(
					epoch_length, 
					model_initialized, 
					subnet_nodes_initialization_epochs
				);
				// Push into reason
				if !has_min_peers {
					reason_for_removal.push(4)
				}	
			}

			(!activated || too_many_max_consensus_epochs || is_offline || !has_min_peers, reason_for_removal)
		}

		/// Validates the signature of the given data with the provided signer's account ID.
		///
		/// # Errors
		///
		/// This function returns a [`WrongSignature`](crate::Error::WrongSignature) error if the
		/// signature is invalid or the verification process fails.
		pub fn validate_signature(
			data: &Vec<u8>,
			signature: &T::OffchainSignature,
			signer: &T::AccountId,
		) -> DispatchResult {
			if signature.verify(&**data, &signer) {
				return Ok(())
			}

			// NOTE: for security reasons modern UIs implicitly wrap the data requested to sign into
			// <Bytes></Bytes>, that's why we support both wrapped and raw versions.
			let prefix = b"<Bytes>";
			let suffix = b"</Bytes>";
			let mut wrapped: Vec<u8> = Vec::with_capacity(data.len() + prefix.len() + suffix.len());
			wrapped.extend(prefix);
			wrapped.extend(data);
			wrapped.extend(suffix);

			ensure!(signature.verify(&*wrapped, &signer), Error::<T>::WrongSignature);

			Ok(())
		}
	}
	
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
			let block: u64 = Self::convert_block_as_u64(block_number);
			let epoch_length: u64 = T::EpochLength::get();
  
			// Form peer consensus at the beginning of each epoch on the last epochs data
			if block >= epoch_length && block % epoch_length == 0 {
				log::info!("Rewarding subnets...");
				let epoch: u64 = block / epoch_length;

				// Reward subnets for the previous epoch
				Self::reward_subnets(block, (epoch - 1) as u32, epoch_length);

				// --- Update subnet nodes classifications
				Self::shift_node_classes(block, epoch_length);

				return Weight::from_parts(207_283_478_000, 22166406)
					.saturating_add(T::DbWeight::get().reads(18250_u64))
					.saturating_add(T::DbWeight::get().writes(12002_u64));
			}

			// Run the block succeeding form consensus
			if (block - 1) >= epoch_length && (block - 1) % epoch_length == 0 {
				log::info!("Updating classes and choosing subnet validators...");
				let epoch: u64 = block / epoch_length;

				// Choose validators and accountants for the current epoch
				Self::do_choose_validator_and_accountants(block, epoch as u32, epoch_length);

				return Weight::from_parts(153_488_564_000, 21699450)
					.saturating_add(T::DbWeight::get().reads(6118_u64))
					.saturating_add(T::DbWeight::get().writes(6082_u64));
			}
	
			return Weight::from_parts(8_054_000, 1638)
				.saturating_add(T::DbWeight::get().reads(1_u64))
		}

		// fn on_idle(block_number: BlockNumberFor<T>) {

		// }

		// fn offchain_worker(block_number: BlockNumberFor<T>) {
			// designated for testnet v2.0
			//
			// Call peers at random to ensure subnet is running
			// Submit a prompt/hash/code/etc. and expect specific response
			// Increment errors or wrong responses to both subnets and peers
			// ...
		// }
	}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub subnet_path: Vec<u8>,
		pub memory_mb: u128,
		pub subnet_nodes: Vec<(T::AccountId, Vec<u8>, PeerId)>,
		pub accounts: Vec<T::AccountId>,
		pub blank: Option<T::AccountId>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			SubnetNodeClassEpochs::<T>::insert(SubnetNodeClass::Idle, 2);
			SubnetNodeClassEpochs::<T>::insert(SubnetNodeClass::Included, 4);
			SubnetNodeClassEpochs::<T>::insert(SubnetNodeClass::Submittable, 6);
			SubnetNodeClassEpochs::<T>::insert(SubnetNodeClass::Accountant, 8);

			let min_required_model_consensus_submit_epochs: u64 = MinRequiredSubnetConsensusSubmitEpochs::<T>::get();
			let min_required_peer_consensus_submit_epochs: u64 = MinRequiredNodeConsensusSubmitEpochs::<T>::get();
			let min_required_peer_consensus_inclusion_epochs: u64 = MinRequiredNodeConsensusInclusionEpochs::<T>::get();
			let min_required_peer_accountant_epochs: u64 = MinRequiredNodeAccountantEpochs::<T>::get();

			let requirement_one: bool = min_required_model_consensus_submit_epochs > min_required_peer_consensus_submit_epochs;
			let requirement_two: bool = min_required_peer_consensus_submit_epochs > min_required_peer_consensus_inclusion_epochs;
			let requirement_three: bool = min_required_peer_accountant_epochs >= min_required_peer_consensus_submit_epochs;
			
			if !requirement_one || !requirement_two || !requirement_three {
				log::error!("Build error code 001, check `fn build`");
				if !(requirement_one) {
					log::error!("MinRequiredSubnetConsensusSubmitEpochs is not greater than MinRequiredNodeConsensusSubmitEpochs");
				}
				if !(requirement_two) {
					log::error!("MinRequiredNodeConsensusSubmitEpochs is not greater than MinRequiredNodeConsensusInclusionEpochs");
				}
				if !(requirement_three) {
					log::error!("MinRequiredNodeAccountantEpochs is not greater than or equal to MinRequiredNodeConsensusSubmitEpochs");
				}
			}

			return; // remove this when testing subnets to pre-initialize subnet if required for testing

			let subnet_id = 1;

			let base_node_memory: u128 = BaseSubnetNodeMemoryMB::<T>::get();

			// --- Get min nodes based on default memory settings
			let real_min_subnet_nodes: u128 = self.memory_mb.clone() / base_node_memory;
			let mut min_subnet_nodes: u32 = MinSubnetNodes::<T>::get();
			if real_min_subnet_nodes as u32 > min_subnet_nodes {
				min_subnet_nodes = real_min_subnet_nodes as u32;
			}
				
			let target_subnet_nodes: u32 = (min_subnet_nodes as u128).saturating_mul(TargetSubnetNodesMultiplier::<T>::get()).saturating_div(10000) as u32 + min_subnet_nodes;

			let model_data = SubnetData {
				id: subnet_id,
				path: self.subnet_path.clone(),
				min_nodes: min_subnet_nodes,
				target_nodes: target_subnet_nodes,
				memory_mb: self.memory_mb.clone(),
				initialized: 0,
			};

			// Activate subnet
			let pre_subnet_data = PreSubnetData {
				path: self.subnet_path.clone(),
				memory_mb: self.memory_mb.clone(),
			};
		
			let vote_subnet_data = VoteSubnetData {
				data: pre_subnet_data,
				active: true,
			};

			SubnetActivated::<T>::insert(self.subnet_path.clone(), vote_subnet_data);
			// Store unique path
			SubnetPaths::<T>::insert(self.subnet_path.clone(), subnet_id);
			// Store subnet data
			SubnetsData::<T>::insert(subnet_id, model_data.clone());
			// Increase total subnets count
			TotalSubnets::<T>::mutate(|n: &mut u32| *n += 1);

			StakeVaultBalance::<T>::mutate(|n: &mut u128| *n += 10000000000000000000);
			let mut count = 0;
			for (account_id, subnet_path, peer_id) in &self.subnet_nodes {
				// for running benchmarks set to `count >= 0`
				// for testing subnet validators
				// 0-100 get balance
				// 0-50 are peers on initialization
				if count >= 50 {
					break
				}

				log::info!("BuildGenesisConfig peer_id: {:?}", peer_id);
	
				// version 2
				let subnet_node: SubnetNode<T::AccountId> = SubnetNode {
					account_id: account_id.clone(),
					peer_id: peer_id.clone(),
					initialized: 0,
				};
				SubnetNodesData::<T>::insert(subnet_id, account_id.clone(), subnet_node.clone());

				// Insert subnet peer account to keep peer_ids unique within subnets
				SubnetNodeAccount::<T>::insert(subnet_id, peer_id.clone(), account_id.clone());

				// let mut model_accounts: BTreeSet<T::AccountId> = SubnetAccount::<T>::get(subnet_id);
				// let model_account_id: Option<&T::AccountId> = model_accounts.get(&account_id.clone());
				// model_accounts.insert(account_id.clone());
				// SubnetAccount::<T>::insert(subnet_id, model_accounts);

				let mut model_accounts: BTreeMap<T::AccountId, u64> = SubnetAccount::<T>::get(subnet_id);
				let model_account: Option<&u64> = model_accounts.get(&account_id.clone());
				model_accounts.insert(account_id.clone(), 0);
				SubnetAccount::<T>::insert(subnet_id, model_accounts);
		
				TotalSubnetNodes::<T>::mutate(subnet_id, |n: &mut u32| *n += 1);

				// Stake
				let stake_amount: u128 = MinStakeBalance::<T>::get();
				AccountSubnetStake::<T>::insert(
					account_id.clone(),
					subnet_id,
					stake_amount,
				);
		
				// -- Increase account_id total stake
				TotalAccountStake::<T>::mutate(account_id.clone(), |n: &mut u128| *n += stake_amount.clone());
		
				// -- Increase total stake overall
				TotalStake::<T>::mutate(|n: &mut u128| *n += stake_amount.clone());
		
				// -- Increase total subnet stake
				TotalSubnetStake::<T>::mutate(subnet_id, |n: &mut u128| *n += stake_amount.clone());

				if let Ok(mut node_class) = SubnetNodesClasses::<T>::try_get(subnet_id, SubnetNodeClass::Idle) {
					node_class.insert(account_id.clone(), 0);
					SubnetNodesClasses::<T>::insert(subnet_id, SubnetNodeClass::Idle, node_class);	
				} else {
					// If new subnet, initialize classes
					let mut node_class: BTreeMap<T::AccountId, u64> = BTreeMap::new();
					node_class.insert(account_id.clone(), 0);
					SubnetNodesClasses::<T>::insert(subnet_id, SubnetNodeClass::Idle, node_class);	
				}
	
				AccountSubnets::<T>::append(account_id.clone(), subnet_id);

				count += 1;
			}
		}
	}
}

/// Return the weight of a dispatch call result as an `Option`.
///
/// Will return the weight regardless of what the state of the result is.
fn get_result_weight(result: DispatchResultWithPostInfo) -> Option<Weight> {
	match result {
		Ok(post_info) => post_info.actual_weight,
		Err(err) => err.post_info.actual_weight,
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


impl<T: Config<AccountId = AccountId>, AccountId> SubnetVote<AccountId> for Pallet<T> {
	fn vote_model_in(vote_subnet_data: VoteSubnetData) -> DispatchResult {
		SubnetActivated::<T>::insert(vote_subnet_data.clone().data.path, vote_subnet_data.clone());
		Ok(())
	}
	fn vote_model_out(vote_subnet_data: VoteSubnetData) -> DispatchResult {
		SubnetActivated::<T>::insert(vote_subnet_data.clone().data.path, vote_subnet_data.clone());
		Ok(())
	}
	fn vote_activated(activator: AccountId, path: Vec<u8>, proposer: AccountId, vote_subnet_data: VoteSubnetData) -> DispatchResult {
		SubnetActivated::<T>::insert(path, vote_subnet_data.clone());

		Self::activate_subnet(
			activator, 
			proposer,
			vote_subnet_data.clone().data,
		)
	}
	fn vote_deactivated(deactivator: AccountId, path: Vec<u8>, proposer: AccountId, vote_subnet_data: VoteSubnetData) -> DispatchResult {
		SubnetActivated::<T>::insert(path, vote_subnet_data.clone());

		Self::deactivate_subnet(
			deactivator, 
			proposer,
			vote_subnet_data.clone().data,
		)
	}

	fn get_total_models() -> u32 {
		TotalSubnets::<T>::get()
	}
	fn get_model_initialization_cost() -> u128 {
		let block: u64 = Self::get_current_block_as_u64();
		Self::get_model_initialization_cost(block)
	}
	fn get_model_path_exist(path: Vec<u8>) -> bool {
		if SubnetPaths::<T>::contains_key(path) {
			true
		} else {
			false
		}
	}
	fn get_model_id_by_path(path: Vec<u8>) -> u32 {
		if !SubnetPaths::<T>::contains_key(path.clone()) {
			return 0
		} else {
			return SubnetPaths::<T>::get(path.clone()).unwrap()
		}
	}
	fn get_model_id_exist(id: u32) -> bool {
		if SubnetsData::<T>::contains_key(id) {
			true
		} else {
			false
		}
	}
	// Should never be called unless contains_key is confirmed
	fn get_model_data(id: u32) -> SubnetData {
		SubnetsData::<T>::get(id).unwrap()
	}
	// fn get_min_subnet_nodes() -> u32 {
	// 	MinSubnetNodes::<T>::get()
	// }
	fn get_max_subnet_nodes() -> u32 {
		MaxSubnetNodes::<T>::get()
	}
	fn get_min_stake_balance() -> u128 {
		MinStakeBalance::<T>::get()
	}
	fn is_submittable_subnet_node_account(account_id: AccountId) -> bool {
		true
	}
	fn is_model_initialized(id: u32) -> bool {
		let model_data = SubnetsData::<T>::get(id).unwrap();
		let model_initialized = model_data.initialized;

		let epoch_length: u64 = T::EpochLength::get();
		let min_required_model_consensus_submit_epochs = MinRequiredSubnetConsensusSubmitEpochs::<T>::get();
		let block: u64 = Self::get_current_block_as_u64();

		block >= Self::get_eligible_epoch_block(
			epoch_length, 
			model_initialized, 
			min_required_model_consensus_submit_epochs
		)
	}
	fn get_total_model_errors(id: u32) -> u32 {
		SubnetConsensusEpochsErrors::<T>::get(id)
	}
	fn get_min_subnet_nodes(memory_mb: u128) -> u32 {
		let base_node_memory: u128 = BaseSubnetNodeMemoryMB::<T>::get();
		Self::get_min_subnet_nodes(base_node_memory, memory_mb)
	}
	fn get_target_subnet_nodes(min_subnet_nodes: u32) -> u32 {
		let base_node_memory: u128 = BaseSubnetNodeMemoryMB::<T>::get();
		Self::get_target_subnet_nodes(base_node_memory, min_subnet_nodes)
	}
}

pub trait SubnetVote<AccountId> {
	fn vote_model_in(vote_subnet_data: VoteSubnetData) -> DispatchResult;
	fn vote_model_out(vote_subnet_data: VoteSubnetData) -> DispatchResult;
	fn vote_activated(activator: AccountId, path: Vec<u8>, proposer: AccountId, vote_subnet_data: VoteSubnetData) -> DispatchResult;
	fn vote_deactivated(deactivator: AccountId, path: Vec<u8>, proposer: AccountId, vote_subnet_data: VoteSubnetData) -> DispatchResult;
	fn get_total_models() -> u32;
	fn get_model_initialization_cost() -> u128;
	fn get_model_path_exist(path: Vec<u8>) -> bool;
	fn get_model_id_by_path(path: Vec<u8>) -> u32;
	fn get_model_id_exist(id: u32) -> bool;
	fn get_model_data(id: u32) -> SubnetData;
	fn get_max_subnet_nodes() -> u32;
	fn get_min_stake_balance() -> u128;
	fn is_submittable_subnet_node_account(account_id: AccountId) -> bool;
	fn is_model_initialized(id: u32) -> bool;
	fn get_total_model_errors(id: u32) -> u32;
	fn get_min_subnet_nodes(memory_mb: u128) -> u32;
	fn get_target_subnet_nodes(min_subnet_nodes: u32) -> u32;
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
	fn set_min_subnet_nodes(value: u32) -> DispatchResult {
		Self::set_min_subnet_nodes(value)
	}
	fn set_max_subnet_nodes(value: u32) -> DispatchResult {
		Self::set_max_subnet_nodes(value)
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
	fn set_min_required_peer_consensus_dishonesty_epochs(value: u64) -> DispatchResult {
		Self::set_min_required_peer_consensus_dishonesty_epochs(value)
	}
	fn set_max_outlier_delta_percent(value: u8) -> DispatchResult {
		Self::set_max_outlier_delta_percent(value)
	}
	fn set_subnet_node_consensus_submit_percent_requirement(value: u128) -> DispatchResult {
		Self::set_subnet_node_consensus_submit_percent_requirement(value)
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
	fn set_remove_subnet_node_epoch_percentage(value: u128) -> DispatchResult {
		Self::set_remove_subnet_node_epoch_percentage(value)
	}
}

pub trait AdminInterface {
	fn set_vote_model_in(path: Vec<u8>) -> DispatchResult;
	fn set_vote_model_out(path: Vec<u8>) -> DispatchResult;
	fn set_max_models(value: u32) -> DispatchResult;
	fn set_min_subnet_nodes(value: u32) -> DispatchResult;
	fn set_max_subnet_nodes(value: u32) -> DispatchResult;
	fn set_min_stake_balance(value: u128) -> DispatchResult;
	fn set_tx_rate_limit(value: u64) -> DispatchResult;
	fn set_max_consensus_epochs_errors(value: u32) -> DispatchResult;
	fn set_min_required_model_consensus_submit_epochs(value: u64) -> DispatchResult;
	fn set_min_required_peer_consensus_submit_epochs(value: u64) -> DispatchResult;
	fn set_min_required_peer_consensus_inclusion_epochs(value: u64) -> DispatchResult;
	fn set_min_required_peer_consensus_dishonesty_epochs(value: u64) -> DispatchResult;	
	fn set_max_outlier_delta_percent(value: u8) -> DispatchResult;
	fn set_subnet_node_consensus_submit_percent_requirement(value: u128) -> DispatchResult;
	fn set_consensus_blocks_interval(value: u64) -> DispatchResult;
	fn set_peer_removal_threshold(value: u128) -> DispatchResult;
	fn set_max_model_rewards_weight(value: u128) -> DispatchResult;
	fn set_stake_reward_weight(value: u128) -> DispatchResult;
	fn set_model_per_peer_init_cost(value: u128) -> DispatchResult;
	fn set_model_consensus_unconfirmed_threshold(value: u128) -> DispatchResult;
	fn set_remove_subnet_node_epoch_percentage(value: u128) -> DispatchResult;
}