use node_template_runtime::{
	AccountId, AuraConfig, BalancesConfig, GrandpaConfig, RuntimeGenesisConfig, Signature,
	SudoConfig, SystemConfig, WASM_BINARY,
	// DemocracyConfig, 
	NetworkConfig
};
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
// use sp_core::OpaquePeerId as PeerId;
use sp_core::OpaquePeerId;
// use multihash;
// use parity_multihash::Multihash;
// use parity_multihash::{encode, Hash, Multihash};
use sc_network::multiaddr::multihash::Multihash;
use sc_network::PeerId;
use sp_core::crypto::Ss58Codec;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

// type Multihash = parity_multihash::Multihash<64>;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
	(get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

pub fn authority_keys_from_ss58(s_aura: &str, s_grandpa: &str) -> (AuraId, GrandpaId) {
	(
		aura_from_ss58_addr(s_aura),
		grandpa_from_ss58_addr(s_grandpa),
	)
}

pub fn aura_from_ss58_addr(s: &str) -> AuraId {
	Ss58Codec::from_ss58check(s).unwrap()
}

pub fn grandpa_from_ss58_addr(s: &str) -> GrandpaId {
	Ss58Codec::from_ss58check(s).unwrap()
}

// fn account(id: u8) -> AccountId {
// 	[id; 32].into()
// }

// generate predictable peer ids
fn peer(id: u8) -> OpaquePeerId {
	let peer_id = format!("12D{id}KooWGFuUunX1AzAzjs3CgyqTXtPWX3AqRhJFbesGPGYHJQTP"); 
	// let peer_id = format!("1AfTrH75U9KU6R3FWUqBnFNz7ubzkXUhtKQprn3pxop4g{id}"); 
	OpaquePeerId(peer_id.into())
}

// fn peer(id: u8) -> OpaquePeerId {
// 	// let peer_id = [id; 32];
// 	// let zero = Multihash::wrap(0x0, &peer_id).expect("The digest size is never too large");
// 	// let peer = PeerId::from_multihash(zero).unwrap();
// 	// OpaquePeerId(peer.into())
// 	// let peer: Vec<u8> = PeerId::random().to_bytes();
// 	// println!("peer {:?}", peer);
// 	let peer = PeerId::random().to_base58();
// 	// println!("peer {:?}", peer);
// 	OpaquePeerId(peer.into())
// }

// fn peer(id: u8) -> PeerId {
// 	// let peer_id = rand::thread_rng().gen::<[u8; 32]>();
// 	let peer_id = [id; 32];
// 	// let peer: PeerId = encode(Hash::SHA2256, b"hello world").unwrap().into();
// 	// let peer: PeerId = Multihash::wrap(0x0, &peer_id).expect("The digest size is never too large");
// 	// peer
// 	// PeerId {
// 	// 	encode(Hash::SHA2256, b"hello world").unwrap().into()
// 	// }
// 	// encode(0x0.into(), &peer_id).unwrap()
// 	// Multihash::encode(0x0, &peer_id).into()
// 	let zero = Multihash::wrap(0x0, &peer_id).expect("The digest size is never too large").into();
// 	println!("zero {:?}", zero);
// 	// zero
// 	PeerId {
// 		0: zero,
// 	}

// 	// PeerId {
// 	// 	0: Multihash::wrap(0x0, &peer_id).expect("The digest size is never too large").into(),
// 	// }
// }

// fn peer(id: u8) -> PeerId {
// 	PeerId([id; 32].into())
// }


pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;
	let mut accounts = (0..90).map(|x| get_account_id_from_seed::<sr25519::Public>(&x.to_string())).collect::<Vec<_>>();
	let default_accounts = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
		get_account_id_from_seed::<sr25519::Public>("Charlie"),
		get_account_id_from_seed::<sr25519::Public>("Dave"),
		get_account_id_from_seed::<sr25519::Public>("Eve"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie"),
		get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
		get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
		get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
		get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
		get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
	];
	accounts.extend(default_accounts);

	Ok(ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				accounts.clone(),
				// (0..90).map(|x| get_account_id_from_seed::<sr25519::Public>(&x.to_string())).collect::<Vec<_>>(),
				// vec![
				// 	get_account_id_from_seed::<sr25519::Public>("Alice"),
				// 	get_account_id_from_seed::<sr25519::Public>("Bob"),
				// 	get_account_id_from_seed::<sr25519::Public>("Charlie"),
				// 	get_account_id_from_seed::<sr25519::Public>("Dave"),
				// 	get_account_id_from_seed::<sr25519::Public>("Eve"),
				// 	get_account_id_from_seed::<sr25519::Public>("Ferdie"),
				// 	get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
				// 	get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				// 	get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
				// 	get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
				// 	get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
				// 	get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
				// ],
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		None,
		// Properties
		None,
		// Extensions
		None,
	))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;
	let mut accounts = (0..90).map(|x| get_account_id_from_seed::<sr25519::Public>(&x.to_string())).collect::<Vec<_>>();
	let default_accounts = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
		get_account_id_from_seed::<sr25519::Public>("Charlie"),
		get_account_id_from_seed::<sr25519::Public>("Dave"),
		get_account_id_from_seed::<sr25519::Public>("Eve"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie"),
		get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
		get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
		get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
		get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
		get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
	];
	accounts.extend(default_accounts);

	Ok(ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				accounts.clone(),
				// (0..90).map(|x| get_account_id_from_seed::<sr25519::Public>(&x.to_string())).collect::<Vec<_>>(),
				// vec![
				// 	get_account_id_from_seed::<sr25519::Public>("Alice"),
				// 	get_account_id_from_seed::<sr25519::Public>("Bob"),
				// 	get_account_id_from_seed::<sr25519::Public>("Charlie"),
				// 	get_account_id_from_seed::<sr25519::Public>("Dave"),
				// 	get_account_id_from_seed::<sr25519::Public>("Eve"),
				// 	get_account_id_from_seed::<sr25519::Public>("Ferdie"),
				// 	get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
				// 	get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				// 	get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
				// 	get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
				// 	get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
				// 	get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
				// ],
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		None,
		None,
		// Extensions
		None,
	))
}

// -> Sudo only
// ./target/release/node-template build-spec --disable-default-bootnode --chain vitalik > vitalikSpec.json
// ./target/release/node-template build-spec --chain=vitalikSpec.json --raw --disable-default-bootnode > vitalikSpecRaw.json
pub fn vitalik_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Vitalik Testnet",
		// ID
		"vitalik_testnet",
		ChainType::Development,
		move || {
			vitalik_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![
					authority_keys_from_ss58(
						"5FtAdTm1ZFuyxuz39mWFZaaDF8925Pu62SvuF7svMQMSCcPF",
						"5Hp4uRdFD8NLXFmBRffS7wzAXnFvWJshR7pBdE9JhBg6Uqdg",
					),
					authority_keys_from_ss58(
						"5H9PKdBA6iosSyYbNfSqdn53DHjpKbrd1iefVq3bKjb6B2xj",
						"5C4ubi5694TjqSyFXHtAtYmj5d82sRN963h7tnE14cDfKL5x",
					),
				],
				// Sudo account
				AccountId::from_ss58check("5FtAdTm1ZFuyxuz39mWFZaaDF8925Pu62SvuF7svMQMSCcPF").unwrap(),
				// Pre-funded accounts
				(0..90).map(|x| get_account_id_from_seed::<sr25519::Public>(&x.to_string())).collect::<Vec<_>>(),
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		None,
		None,
		// Extensions
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
) -> RuntimeGenesisConfig {
	RuntimeGenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			..Default::default()
		},
		balances: BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			balances: {
				endowed_accounts.iter().cloned().map(|k| (k, 10000000000000000000000)).collect()
			},
		},
		aura: AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		},
		grandpa: GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
			..Default::default()
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key.clone()),
		},
		// democracy: DemocracyConfig::default(),
		transaction_payment: Default::default(),
		network: {
			let mut peer_index: u8 = 0;
			NetworkConfig {
			subnet_path: "petals-team/StableBeluga2".into(),
			subnet_nodes: endowed_accounts.iter().cloned().map(|k| {
				peer_index += 1;
				(
					k, 
					"petals-team/StableBeluga2".into(),
					peer(peer_index),
				)
			}).collect(),
			accounts: endowed_accounts.iter().cloned().map(|k| k).collect(),
			blank: {
				Some(root_key.clone())
			},
		}},
	}
}

/// Configure initial storage state for FRAME modules.
fn vitalik_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
) -> RuntimeGenesisConfig {
	RuntimeGenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			..Default::default()
		},
		balances: BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			balances: {
				endowed_accounts.iter().cloned().map(|k| (k, 10000000000000000000000)).collect()
			},
		},
		aura: AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		},
		grandpa: GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
			..Default::default()
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key.clone()),
		},
		// democracy: DemocracyConfig::default(),
		transaction_payment: Default::default(),
		network: {
			NetworkConfig {
				subnet_path: "bigscience/bloom-560m".into(),
				subnet_nodes: vec![],
				accounts: vec![],
				blank: {
					Some(root_key.clone())
				},
			}
		},
	}
}
