#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));
// https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/explorer

pub mod impls;
// mod tracks;
// pub use tracks::TracksInfo;

// use core::u32::MIN;

use pallet_grandpa::AuthorityId as GrandpaId;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		AccountIdLookup, BlakeTwo256, Block as BlockT, IdentifyAccount, NumberFor, One, Verify,
	},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, MultiSignature
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;


// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, parameter_types,
	traits::{
		ConstBool, ConstU128, ConstU32, ConstU64, ConstU8, KeyOwnerProofSystem, Randomness,
		StorageInfo, EitherOf
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND
		},
		IdentityFee, Weight
	},
	StorageValue,
	PalletId,
};
pub use frame_system::Call as SystemCall;
use frame_system::{EnsureRoot, EnsureSigned};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{ConstFeeMultiplier, CurrencyAdapter, Multiplier};
use codec::Encode;

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};
use sp_runtime::SaturatedConversion;

/// Import custom pallets.
pub use pallet_network;
pub use pallet_multisig;
pub use pallet_model_voting;
// pub use pallet_model_voting_v2;
// pub use pallet_offchain_worker;

// testing


/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

pub type AccountPublic = <Signature as Verify>::Signer;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
		}
	}
}

// To learn more about runtime versioning, see:
// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("node-template"),
	impl_name: create_runtime_str!("node-template"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 100,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub const YEAR: BlockNumber = DAYS * 365;

pub const SECS_PER_BLOCK: u64 = MILLISECS_PER_BLOCK / 1000;

// Est. halving per x years
// pub const BLOCKS_PER_HALVING: BlockNumber = YEAR * 4 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const BLOCKS_PER_HALVING: BlockNumber = YEAR * 1;

// max supply 2.8m 2800000000000000000000000
// pub const TARGET_MAX_TOTAL_SUPPLY: u64 = 28_000_000_000_000_000;
pub const TARGET_MAX_TOTAL_SUPPLY: u128 = 2_800_000_000_000_000_000_000_000;

// initial reward per block first halving
// pub const INITIAL_REWARD_PER_BLOCK: u64 = (TARGET_MAX_TOTAL_SUPPLY / 2) / BLOCKS_PER_HALVING as u64;
pub const INITIAL_REWARD_PER_BLOCK: u128 = (TARGET_MAX_TOTAL_SUPPLY / 2) / BLOCKS_PER_HALVING as u128;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::with_sensible_defaults(
			Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
			NORMAL_DISPATCH_RATIO,
		);
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u8 = 42;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = frame_support::traits::Everything;
	/// The block type for the runtime.
	type Block = Block;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = BlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The set code logic, just the default since we're not a parachain.
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = (1) as Balance * 2_000 * 10_000 + (88 as Balance) * 100 * 10_000;
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = (0) as Balance * 2_000 * 10_000 + (32 as Balance) * 100 * 10_000;
	pub const MaxSignatories: u32 = 100;
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<32>;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type WeightInfo = ();
	type MaxAuthorities = ConstU32<32>;
	type MaxSetIdSessionEntries = ConstU64<0>;

	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
}

/// Existential deposit.
pub const EXISTENTIAL_DEPOSIT: u128 = 500;

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type MaxHolds = ();
}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = CurrencyAdapter<Balances, crate::impls::DealWithFees>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

// // Configure the pallet network.
// parameter_types! {
// 	pub const NetworkInitialBondsMovingAverage: u64 = 900_000;
// 	pub const NetworkInitialMaxAllowedUids: u16 = 4096;
// 	pub const NetworkInitialIssuance: u64 = 0;
// 	pub const NetworkInitialEmissionValue: u16 = 0;
// 	pub const NetworkInitialMaxAllowedValidators: u16 = 128;
// 	pub const NetworkInitialBurn: u64 = 1_000_000_000; // 1 tao
// 	pub const NetworkInitialMinBurn: u64 = 1_000_000_000; // 1 tao
// 	pub const NetworkInitialMaxBurn: u64 = 100_000_000_000; // 100 tao
// 	// pub const NetworkInitialTxRateLimit: u64 = 1000;
// 	pub const NetworkInitialMaxRegistrationsPerBlock: u16 = 1;
// 	pub const NetworkInitialMinLockCost: u64 = 1_000_000_000_000; // 1000 TAO
// 	pub const NetworkInitialModelLimit: u16 = 12;
// 	pub const NetworkInitialSubnetNodeLimit: u16 = 12;
// 	pub const NetworkInitialNetworkRateLimit: u64 = 1 * 7200;
// 	pub const NetworkInitialStringLimit: u32 = 1000;
// }

// authority
pub struct AuraAccountAdapter;
impl frame_support::traits::FindAuthor<AccountId> for AuraAccountAdapter {
	fn find_author<'a, I>(digests: I) -> Option<AccountId>
		where I: 'a + IntoIterator<Item=(frame_support::ConsensusEngineId, &'a [u8])>
	{
		pallet_aura::AuraAuthorId::<Runtime>::find_author(digests).and_then(|k| {
			AccountId::try_from(k.as_ref()).ok()
		})
	}
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = AuraAccountAdapter;
	type EventHandler =  ();
}

parameter_types! {
	pub const HalvingInterval: BlockNumber = BLOCKS_PER_HALVING;
	pub const InitialBlockSubsidy: u128 = INITIAL_REWARD_PER_BLOCK;
}

// rewards
impl pallet_rewards::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type FindAuthor = AuraAccountAdapter;
	type HalvingInterval = HalvingInterval;
	type InitialBlockSubsidy = InitialBlockSubsidy;
	type IncreaseStakeVault = Network;
}

// admin
impl pallet_admin::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type NetworkAdminInterface = Network;
	type SubnetVotingAdminInterface = SubnetVoting;
}

// scheduler
// parameter_types! {
// 	pub const MaxScheduledPerBlock: u32 = 50;
// 	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
// }

// impl pallet_scheduler::Config for Runtime {
// 	type RuntimeEvent = RuntimeEvent;
// 	type RuntimeOrigin = RuntimeOrigin;
// 	type PalletsOrigin = OriginCaller;
// 	type RuntimeCall = RuntimeCall;
// 	type MaximumWeight = MaximumSchedulerWeight;
// 	type ScheduleOrigin = EnsureRoot<AccountId>;
// 	type OriginPrivilegeCmp = frame_support::traits::EqualPrivilegeOnly;
// 	type MaxScheduledPerBlock = MaxScheduledPerBlock;
// 	type WeightInfo = ();
// 	type Preimages = ();
// }

// // democracy
// parameter_types! {
// 	// pub const LaunchPeriod: BlockNumber = 10 * DAYS;
// 	pub const LaunchPeriod: BlockNumber = 2 * MINUTES;
// 	pub const VotingPeriod: BlockNumber = 2 * MINUTES;
// 	pub const FastTrackVotingPeriod: BlockNumber = 2;
// 	pub const InstantAllowed: bool = true;
// 	// pub const MinimumDeposit: Balance = 100;
// 	pub const MinimumDeposit: Balance = 1;
// 	pub const EnactmentPeriod: BlockNumber = 5;
// 	pub const CooloffPeriod: BlockNumber = 5;
// 	pub const MaxVotes: u32 = 100;
// 	pub const MaxProposal: u32 = 100;
// 	pub const MaxDeposits: u32 = 100;
// }

// impl pallet_democracy::Config for Runtime {
// 	type RuntimeEvent = RuntimeEvent;
// 	type Currency = Balances;
// 	type EnactmentPeriod = EnactmentPeriod;
// 	type LaunchPeriod = LaunchPeriod;
// 	type VotingPeriod = VotingPeriod;
// 	type VoteLockingPeriod = EnactmentPeriod;
// 	type MinimumDeposit = MinimumDeposit;
// 	type ExternalOrigin = EnsureRoot<Self::AccountId>;
// 	type ExternalMajorityOrigin = EnsureRoot<Self::AccountId>;
// 	type ExternalDefaultOrigin = EnsureRoot<Self::AccountId>;
// 	type FastTrackOrigin = EnsureRoot<Self::AccountId>;
// 	type InstantOrigin = EnsureRoot<Self::AccountId>;
// 	type InstantAllowed = InstantAllowed;
// 	type FastTrackVotingPeriod = FastTrackVotingPeriod;
// 	type CancellationOrigin = EnsureRoot<Self::AccountId>;
// 	type BlacklistOrigin = EnsureRoot<Self::AccountId>;
// 	type CancelProposalOrigin = EnsureRoot<Self::AccountId>;
// 	type VetoOrigin = EnsureSigned<Self::AccountId>;
// 	type CooloffPeriod = CooloffPeriod;
// 	type Slash = ();
// 	type Scheduler = Scheduler;
// 	type PalletsOrigin = OriginCaller;
// 	type MaxVotes = MaxVotes;
// 	type WeightInfo = ();
// 	type MaxProposals = MaxProposal;
// 	type MaxDeposits = MaxDeposits;
// 	type Preimages = Preimage;
// 	type MaxBlacklisted = ();
// 	type SubmitOrigin = EnsureSigned<AccountId>;
// }

// preimage
parameter_types! {
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub const PreimageBaseDeposit: Balance = 1;
	pub const PreimageByteDeposit: Balance = 1;
}

impl pallet_preimage::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type BaseDeposit = PreimageBaseDeposit;
	type ByteDeposit = PreimageByteDeposit;
}

// conviction voting
// parameter_types! {
// 	// pub const VoteLockingPeriod: BlockNumber = 1 * DAYS;
// 	pub const VoteLockingPeriod: BlockNumber = 2 * MINUTES;
// }

// // https://github.com/paritytech/substrate/blob/master/bin/node/runtime/src/lib.rs#L832
// impl pallet_conviction_voting::Config for Runtime {
// 	type WeightInfo = pallet_conviction_voting::weights::SubstrateWeight<Runtime>;
// 	type RuntimeEvent = RuntimeEvent;
// 	type Currency = Balances;
// 	type Polls = Referenda;
// 	type MaxTurnout = frame_support::traits::TotalIssuanceOf<Balances, Self::AccountId>;
// 	type MaxVotes = ConstU32<512>;
// 	type VoteLockingPeriod = VoteLockingPeriod;
// }

// impl pallet_model_voting_v2::Config for Runtime {
// 	type WeightInfo = pallet_model_voting_v2::weights::SubstrateWeight<Runtime>;
// 	type RuntimeEvent = RuntimeEvent;
// 	type Currency = Balances;
// 	type Polls = Referenda;
// 	type MaxTurnout = frame_support::traits::TotalIssuanceOf<Balances, Self::AccountId>;
// 	type MaxVotes = ConstU32<512>;
// 	type VoteLockingPeriod = VoteLockingPeriod;
// }

// // referenda
// parameter_types! {
// 	pub const AlarmInterval: BlockNumber = 10;
// 	pub SubmissionDeposit: Balance = 10;
// 	// pub const UndecidingTimeout: BlockNumber = 14 * DAYS;
// 	pub const UndecidingTimeout: BlockNumber = 4 * MINUTES;
// }

// pub struct TracksInfo;
// impl pallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
// 	type Id = u16;
// 	type RuntimeOrigin = <RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin;
// 	fn tracks() -> &'static [(Self::Id, pallet_referenda::TrackInfo<Balance, BlockNumber>)] {
// 		static DATA: [(u16, pallet_referenda::TrackInfo<Balance, BlockNumber>); 1] = [
// 			(
// 				0u16,
// 				pallet_referenda::TrackInfo {
// 					name: "root",
// 					max_deciding: 1,
// 					decision_deposit: 10,
// 					prepare_period: 4,
// 					decision_period: 4,
// 					confirm_period: 2,
// 					min_enactment_period: 4,
// 					min_approval: pallet_referenda::Curve::LinearDecreasing {
// 						length: Perbill::from_percent(100),
// 						floor: Perbill::from_percent(50),
// 						ceil: Perbill::from_percent(100),
// 					},
// 					min_support: pallet_referenda::Curve::LinearDecreasing {
// 						length: Perbill::from_percent(100),
// 						floor: Perbill::from_percent(0),
// 						ceil: Perbill::from_percent(100),
// 					},
// 				},
// 			)
// 		];
// 		&DATA[..]
// 	}
// 	fn track_for(id: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
// 		if let Ok(system_origin) = frame_system::RawOrigin::try_from(id.clone()) {
// 			match system_origin {
// 				frame_system::RawOrigin::Root => Ok(0),
// 				_ => Err(()),
// 			}
// 		} else {
// 			Err(())
// 		}
// 	}
// }
// pallet_referenda::impl_tracksinfo_get!(TracksInfo, Balance, BlockNumber);

// impl pallet_referenda::Config for Runtime {
// 	type WeightInfo = pallet_referenda::weights::SubstrateWeight<Self>;
// 	type RuntimeCall = RuntimeCall;
// 	type RuntimeEvent = RuntimeEvent;
// 	type Scheduler = Scheduler;
// 	type Currency = pallet_balances::Pallet<Self>;
// 	type SubmitOrigin = EnsureSigned<AccountId>;
// 	type CancelOrigin = EnsureRoot<AccountId>;
// 	type KillOrigin = EnsureRoot<AccountId>;
// 	type Slash = ();
// 	type Votes = pallet_model_voting_v2::VotesOf<Runtime>;
// 	type Tally = pallet_model_voting_v2::TallyOf<Runtime>;
// 	type SubmissionDeposit = SubmissionDeposit;
// 	type MaxQueued = ConstU32<100>;
// 	type UndecidingTimeout = UndecidingTimeout;
// 	type AlarmInterval = AlarmInterval;
// 	type Tracks = TracksInfo;
// 	type Preimages = Preimage;
// }

// //  old
// impl pallet_referenda::Config for Runtime {
// 	type AlarmInterval = AlarmInterval;
// 	type Currency = Balances;
// 	type CancelOrigin = EitherOf<EnsureRoot<Self::AccountId>, ReferendumKiller>;
// 	type KillOrigin = EitherOf<EnsureRoot<Self::AccountId>, ReferendumKiller>;
// 	type MaxQueued = ConstU32<100>;
// 	type Preimages = Preimage;
// 	type RuntimeCall = RuntimeCall;
// 	type RuntimeEvent = RuntimeEvent;
// 	type Scheduler = Scheduler;
// 	type Slash = ();
// 	type SubmissionDeposit = SubmissionDeposit;
// 	type SubmitOrigin = frame_system::EnsureSigned<Self::AccountId>;
// 	type Tally = pallet_conviction_voting::TallyOf<Self>;
// 	type Tracks = TracksInfo;
// 	type UndecidingTimeout = ConstU32<20>;
// 	type Votes = pallet_conviction_voting::VotesOf<Self>;
// 	type WeightInfo = ();
// }

parameter_types! {
	pub const InitialTxRateLimit: u64 = 0;
	pub const EpochLength: u64 = 10;
	pub const NetworkPalletId: PalletId = PalletId(*b"/network");
	pub const SubnetInitializationCost: u128 = 100_000_000_000_000_000_000;
}

impl pallet_network::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EpochLength = EpochLength;
	type StringLimit = ConstU32<100>;
	type InitialTxRateLimit = InitialTxRateLimit;
	type SecsPerBlock = ConstU64<{ SECS_PER_BLOCK as u64 }>;
	type Year = ConstU64<{ YEAR as u64 }>;
	type OffchainSignature = Signature;
	type OffchainPublic = AccountPublic;
	type Randomness = InsecureRandomnessCollectiveFlip;
	type PalletId = NetworkPalletId;
	type SubnetInitializationCost = SubnetInitializationCost;
}

parameter_types! {
	// pub const VotingPeriod: BlockNumber = DAYS * 21;
	// pub const EnactmentPeriod: BlockNumber = DAYS * 7;
	pub const MinProposalStake: u128 = 100_000_000_000_000_000_000; // 100 * 1e18

	// Testing
	pub const VotingPeriod: BlockNumber = 50; // ~5 minutes
	pub const EnactmentPeriod: BlockNumber = 30; // ~3 minutes
}

impl pallet_model_voting::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type SubnetVote = Network;
	type Currency = Balances;
	type MaxActivateProposals = ConstU32<1>;
	type MaxDeactivateProposals = ConstU32<32>;
	type MaxProposals = ConstU32<32>;
	type VotingPeriod = VotingPeriod;
	type EnactmentPeriod = EnactmentPeriod;
	type MinProposalStake = MinProposalStake;
}

// cargo check -p node-template-runtime --release
// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub struct Runtime {
		System: frame_system,
		InsecureRandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip,
		Timestamp: pallet_timestamp,
		Aura: pallet_aura,
		Grandpa: pallet_grandpa,
		Balances: pallet_balances,
		TransactionPayment: pallet_transaction_payment,
		Sudo: pallet_sudo,
		// Custom Logic
		Multisig: pallet_multisig,
		Authorship: pallet_authorship,
		Rewards: pallet_rewards,
		// Scheduler: pallet_scheduler,
		// Democracy: pallet_democracy,
		Preimage: pallet_preimage,
		// Referenda: pallet_referenda,
		// ConvictionVoting: pallet_conviction_voting,
		// ConvictionSubnetVoting: pallet_model_voting_v2,
		Network: pallet_network,
		Admin: pallet_admin,
		SubnetVoting: pallet_model_voting,
	}
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_grandpa, Grandpa]
		[pallet_balances, Balances]
		[pallet_timestamp, Timestamp]
		[pallet_model_voting, SubnetVoting]
		[pallet_network, Network]
	);
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> sp_consensus_grandpa::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: sp_consensus_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: sp_consensus_grandpa::SetId,
			_authority_id: GrandpaId,
		) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl network_custom_rpc_runtime_api::NetworkRuntimeApi<Block> for Runtime {
		fn get_subnet_nodes(model_id: u32) -> Vec<u8> {
			let result = Network::get_subnet_nodes(model_id);
			result.encode()
		}
		fn get_subnet_nodes_included(model_id: u32) -> Vec<u8> {
			let result = Network::get_subnet_nodes_included(model_id);
			result.encode()
		}
		fn get_subnet_nodes_submittable(model_id: u32) -> Vec<u8> {
			let result = Network::get_subnet_nodes_submittable(model_id);
			result.encode()
		}
		fn get_subnet_nodes_model_unconfirmed_count(model_id: u32) -> u32 {
			let result = Network::get_subnet_nodes_model_unconfirmed_count(model_id);
			result
			// result.encode()
		}
		fn get_consensus_data(model_id: u32, epoch: u32) -> Vec<u8> {
			let result = Network::get_consensus_data(model_id, epoch);
			result.encode()
		}
		fn get_accountant_data(model_id: u32, id: u32) -> Vec<u8> {
			let result = Network::get_accountant_data(model_id, id);
			result.encode()
		}
		fn get_minimum_subnet_nodes(subnet_id: u32, memory_mb: u128) -> u32 {
			let result = Network::get_minimum_subnet_nodes(subnet_id, memory_mb);
			result
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch, TrackedStorageKey};

			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			use frame_support::traits::WhitelistedStorageKeys;
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, BlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
		}
	}
}
