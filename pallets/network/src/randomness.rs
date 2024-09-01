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

impl<T: Config> Pallet<T> {

	// If using len() for `max`, avoid overflow by `-1` 
	pub fn get_random_number(max: u32, seed: u32) -> u32 {
		if max == 0 {
			return 0
		}
		let mut random_number = Self::generate_random_number(seed);

		// Best effort attempt to remove bias from modulus operator.
		let mut i = 1;
		let mut found = false;
		while !found {
			if random_number < u32::MAX - u32::MAX % max {
				found = true;
				break
			}

			random_number = Self::generate_random_number(i);

			i += 1;
		}

		random_number % max
	}

	/// Generate a random number from a given seed.
	/// Note that there is potential bias introduced by using modulus operator.
	/// You should call this function with different seed values until the random
	/// number lies within `u32::MAX - u32::MAX % n`.
	/// TODO: deal with randomness freshness
	/// https://github.com/paritytech/substrate/issues/8311
  /// This is not a secure random number generator but serves its purpose for choosing random numbers
	pub fn generate_random_number(seed: u32) -> u32 {
		let (random_seed, _) = T::Randomness::random(&(T::PalletId::get(), seed).encode());
		let random_number = <u32>::decode(&mut random_seed.as_ref())
			.expect("secure hashes should always be bigger than u32; qed");

		random_number
	}
}