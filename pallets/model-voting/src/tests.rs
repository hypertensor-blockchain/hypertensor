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
use frame_support::{
	assert_noop, assert_ok, assert_err
};
use log::info;
use sp_core::{H256, U256};
use frame_support::traits::Currency;

type AccountIdOf<Test> = <Test as frame_system::Config>::AccountId;

fn account(id: u8) -> AccountIdOf<Test> {
	[id; 32].into()
}

#[test]
fn test_set_propose_model_vote_in() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "".into();
  })
}

fn test_propose_model_vote_out() {
  new_test_ext().execute_with(|| {
    let model_id: u32 = 1;
  })
}

fn test_vote() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "".into();
  })
}

fn test_activate_model() {
  new_test_ext().execute_with(|| {
    let model_path: Vec<u8> = "".into();
  })
}

fn test_deactivate_model() {
  new_test_ext().execute_with(|| {
    let model_id: u32 = 1;
  })
}