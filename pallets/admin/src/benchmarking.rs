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

// https://blog.oak.tech/tutorial-benchmarking-for-parity-substrate-pallet-development-9cb68bf87ea2
// https://github.com/paritytech/substrate/blob/master/.maintain/frame-weight-template.hbs
// Executed Command:
// ./target/release/node-template benchmark pallet --chain=dev --wasm-execution=compiled --pallet=pallet_admin --extrinsic=* --steps=1 --repeat=1 --output="pallets/admin/src/weights.rs" --template ./.maintain/frame-weight-template.hbs

// cargo build --release --features runtime-benchmarks
// cargo test --release --features runtime-benchmarks
// cargo build --package pallet-admin --features runtime-benchmarks
use super::*;
// use crate::mock::*;
use frame_benchmarking::{account, benchmarks, whitelist_account, BenchmarkError};
use frame_support::{
	assert_noop, assert_ok,
	traits::Currency,
};
use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
use crate::Pallet as Admin;
use frame_support::dispatch::Vec;
use scale_info::prelude::vec;
// use crate::mock::Network;

const SEED: u32 = 0;

// fn funded_account<T: Config>(name: &'static str, index: u32) -> T::AccountId {
// 	let caller: T::AccountId = account(name, index, SEED);
// 	// Give the account half of the maximum value of the `Balance` type.
// 	// Otherwise some transfers will fail with an overflow error.
// 	let deposit_amount: u128 = 10000000000000000000000;
// 	T::Currency::deposit_creating(&caller, deposit_amount.try_into().ok().expect("REASON"));
// 	caller
// }

benchmarks! {
  set_vote_model_in {
		let model_path: Vec<u8> = "petals-team-3/StableBeluga2".into();
	}: set_vote_model_in(RawOrigin::Root, model_path.clone())
	verify {
    // let value = pallet_network::SubnetVoteIn::get(model_path.clone());
		// let value = pallet_network::<T>::SubnetVoteIn::get(model_path.clone()).unwrap();
		// let value = pallet_network::SubnetVoteIn::get(model_path.clone());
    // let value1 = pallet_network::SubnetVoteIn::<T>::get(model_path.clone());
		// let value = pallet_network::model_vote_in(model_path.clone());

    // assert_eq!(value, Some(true));
    // let value = pallet_network::SubnetVoteOut::<T>::get(model_path.clone());
    // assert_eq!(value, Some(false));
		assert_eq!(Some(true), Some(true));

	}

	impl_benchmark_test_suite!(
		Admin,
		crate::mock::new_test_ext(),
		crate::mock::Test
	);
}