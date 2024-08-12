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
//
// Enables accounts to delegate stake to subnets for a portion of emissions

use super::*;

impl<T: Config> Pallet<T> {
  pub fn do_add_delegate_stake(
    origin: T::RuntimeOrigin,
    subnet_id: u32,
    hotkey: T::AccountId,
    delegate_stake_to_be_added: u128,
  ) -> DispatchResult {
    let account_id: T::AccountId = ensure_signed(origin)?;

    let delegate_stake_as_balance = Self::u128_to_balance(delegate_stake_to_be_added);

    ensure!(
      delegate_stake_as_balance.is_some(),
      Error::<T>::CouldNotConvertToBalance
    );

    let account_delegate_stake_shares: u128 = AccountSubnetDelegateStakeShares::<T>::get(&account_id, subnet_id.clone());
    let total_model_delegated_stake_shares = TotalSubnetDelegateStakeShares::<T>::get(subnet_id.clone());
    let total_model_delegated_stake_balance = TotalSubnetDelegateStakeBalance::<T>::get(subnet_id.clone());

    // --- Get accounts current balance
    let account_delegate_stake_balance = Self::convert_to_balance(
      account_delegate_stake_shares,
      total_model_delegated_stake_shares,
      total_model_delegated_stake_balance
    );

    // ensure!(
    //   account_delegate_stake_balance != 0,
    //   Error::<T>::InsufficientBalanceToSharesConversion
    // );

    ensure!(
      account_delegate_stake_balance.saturating_add(delegate_stake_to_be_added) <= MaxDelegateStakeBalance::<T>::get(),
      Error::<T>::MaxDelegatedStakeReached
    );

    // --- Ensure the callers account_id has enough delegate_stake to perform the transaction.
    ensure!(
      Self::can_remove_balance_from_coldkey_account(&account_id, delegate_stake_as_balance.unwrap()),
      Error::<T>::NotEnoughBalanceToStake
    );
  
    // to-do: add AddStakeRateLimit instead of universal rate limiter
    //        this allows peers to come in freely
    let block: u64 = Self::get_current_block_as_u64();
    ensure!(
      !Self::exceeds_tx_rate_limit(Self::get_last_tx_block(&account_id), block),
      Error::<T>::TxRateLimitExceeded
    );

    // --- Ensure the remove operation from the account_id is a success.
    ensure!(
      Self::remove_balance_from_coldkey_account(&account_id, delegate_stake_as_balance.unwrap()) == true,
      Error::<T>::BalanceWithdrawalError
    );
  
    // --- Get amount to be added as shares based on stake to balance added to account
    let mut delegate_stake_to_be_added_as_shares = Self::convert_to_shares(
      delegate_stake_to_be_added,
      total_model_delegated_stake_shares,
      total_model_delegated_stake_balance
    );

    // --- Mitigate inflation attack
    if total_model_delegated_stake_shares == 0 {
      TotalSubnetDelegateStakeShares::<T>::mutate(subnet_id.clone(), |mut n| *n += 1000);
      delegate_stake_to_be_added_as_shares = delegate_stake_to_be_added_as_shares.saturating_sub(1000);
    }
    
    // --- Check rounding errors
    ensure!(
      delegate_stake_to_be_added_as_shares != 0,
      Error::<T>::CouldNotConvertToShares
    );

    Self::increase_account_delegate_stake_shares(
      &account_id,
      subnet_id, 
      delegate_stake_to_be_added,
      delegate_stake_to_be_added_as_shares,
    );

    // Set last block for rate limiting
    Self::set_last_tx_block(&account_id, block);

    Self::deposit_event(Event::DelegateStakeAdded(subnet_id, account_id, delegate_stake_to_be_added));

    Ok(())
  }

  pub fn do_remove_delegate_stake(
    origin: T::RuntimeOrigin, 
    subnet_id: u32,
    hotkey: T::AccountId,
    delegate_stake_shares_to_be_removed: u128,
    // delegate_stake_to_be_removed: u128,
  ) -> DispatchResult {
    let account_id: T::AccountId = ensure_signed(origin)?;

    // --- Ensure that the delegate_stake amount to be removed is above zero.
    ensure!(
      delegate_stake_shares_to_be_removed > 0,
      Error::<T>::NotEnoughStakeToWithdraw
    );

    let account_delegate_stake_shares: u128 = AccountSubnetDelegateStakeShares::<T>::get(&account_id, subnet_id.clone());

    log::error!("delegate_stake_shares_to_be_removed {:?}", delegate_stake_shares_to_be_removed);
    log::error!("account_delegate_stake_shares       {:?}", account_delegate_stake_shares);

    // --- Ensure that the account has enough delegate_stake to withdraw.
    ensure!(
      account_delegate_stake_shares >= delegate_stake_shares_to_be_removed,
      Error::<T>::NotEnoughStakeToWithdraw
    );
      
    let total_model_delegated_stake_shares = TotalSubnetDelegateStakeShares::<T>::get(subnet_id.clone());
    let total_model_delegated_stake_balance = TotalSubnetDelegateStakeBalance::<T>::get(subnet_id.clone());

    // --- Get accounts current balance
    let delegate_stake_to_be_removed = Self::convert_to_balance(
      account_delegate_stake_shares,
      total_model_delegated_stake_shares,
      total_model_delegated_stake_balance
    );

    // --- Ensure that we can convert this u128 to a balance.
    let delegate_stake_to_be_added_as_currency = Self::u128_to_balance(delegate_stake_to_be_removed);
    ensure!(
      delegate_stake_to_be_added_as_currency.is_some(),
      Error::<T>::CouldNotConvertToBalance
    );

    let block: u64 = Self::get_current_block_as_u64();
    ensure!(
      !Self::exceeds_tx_rate_limit(Self::get_last_tx_block(&account_id), block),
      Error::<T>::TxRateLimitExceeded
    );

    // --- 7. We remove the balance from the hotkey.
    Self::decrease_account_delegate_stake_shares(&account_id, subnet_id, delegate_stake_to_be_removed, delegate_stake_shares_to_be_removed);

    let remaining_account_delegate_stake_shares: u128 = AccountSubnetDelegateStakeShares::<T>::get(&account_id, subnet_id);
    
    // --- 9. We add the balancer to the account_id.  If the above fails we will not credit this account_id.
    Self::add_balance_to_coldkey_account(&account_id, delegate_stake_to_be_added_as_currency.unwrap());
    
    // Set last block for rate limiting
    Self::set_last_tx_block(&account_id, block);

    Self::deposit_event(Event::DelegateStakeRemoved(subnet_id, account_id, delegate_stake_to_be_removed));

    Ok(())
  }

  pub fn increase_account_delegate_stake_shares(
    account_id: &T::AccountId,
    subnet_id: u32, 
    amount: u128,
    shares: u128,
  ) {
    // -- increase account subnet staking shares balance
    AccountSubnetDelegateStakeShares::<T>::insert(
      account_id,
      subnet_id.clone(),
      AccountSubnetDelegateStakeShares::<T>::get(account_id, subnet_id).saturating_add(shares),
    );

    // -- increase total subnet delegate stake balance
    TotalSubnetDelegateStakeBalance::<T>::mutate(subnet_id.clone(), |mut n| *n += amount);

    // -- increase total subnet delegate stake shares
    TotalSubnetDelegateStakeShares::<T>::mutate(subnet_id.clone(), |mut n| *n += shares);
  }
  
  pub fn decrease_account_delegate_stake_shares(
    account_id: &T::AccountId,
    subnet_id: u32, 
    amount: u128,
    shares: u128,
  ) {
    // -- decrease account subnet staking shares balance
    AccountSubnetDelegateStakeShares::<T>::insert(
      account_id,
      subnet_id.clone(),
      AccountSubnetDelegateStakeShares::<T>::get(account_id, subnet_id).saturating_sub(shares),
    );

    // -- increase total subnet delegate stake balance
    TotalSubnetDelegateStakeBalance::<T>::mutate(subnet_id.clone(), |mut n| *n += amount);

    // -- decrease total subnet delegate stake shares
    TotalSubnetDelegateStakeShares::<T>::mutate(subnet_id.clone(), |mut n| *n -= shares);
  }

  // fn can_remove_balance_from_coldkey_account(
  //   account_id: &T::AccountId,
  //   amount: <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance,
  // ) -> bool {
  //   let current_balance = Self::get_coldkey_balance(account_id);
  //   if amount > current_balance {
  //     return false;
  //   }

  //   // This bit is currently untested. @todo
  //   let new_potential_balance = current_balance - amount;
  //   let can_withdraw = T::Currency::ensure_can_withdraw(
  //     &account_id,
  //     amount,
  //     WithdrawReasons::except(WithdrawReasons::TIP),
  //     new_potential_balance,
  //   )
  //   .is_ok();
  //   can_withdraw
  // }

  // fn remove_balance_from_coldkey_account(
  //   account_id: &T::AccountId,
  //   amount: <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance,
  // ) -> bool {
  //   return match T::Currency::withdraw(
  //     &account_id,
  //     amount,
  //     WithdrawReasons::except(WithdrawReasons::TIP),
  //     ExistenceRequirement::KeepAlive,
  //   ) {
  //     Ok(_result) => true,
  //     Err(_error) => false,
  //   };
  // }

  /// Rewards are deposited here
  pub fn increase_delegated_stake(
    subnet_id: u32,
    amount: u128,
  ) {
    // -- increase total subnet delegate stake 
    TotalSubnetDelegateStakeBalance::<T>::mutate(subnet_id.clone(), |mut n| *n += amount);
  }

  // pub fn add_balance_to_coldkey_account(
  //   account_id: &T::AccountId,
  //   amount: <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance,
  // ) {
  //   T::Currency::deposit_creating(&account_id, amount); // Infallibe
  // }

  // pub fn get_coldkey_balance(
  //   account_id: &T::AccountId,
  // ) -> <<T as pallet::Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance {
  //   return T::Currency::free_balance(&account_id);
  // }

  // pub fn u128_to_balance(
  //   input: u128,
  // ) -> Option<
  //   <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance,
  // > {
  //   input.try_into().ok()
  // }

  pub fn get_delegate_stake_balance(
    subnet_id: u32,
    account_id: &T::AccountId,
  ) -> u128 {
    0
  }

  pub fn get_delegate_shares_balance(
    subnet_id: u32,
    account_id: &T::AccountId,
  ) -> u128 {
    0
  }

  pub fn convert_to_balance(
    shares: u128,
    total_shares: u128,
    total_balance: u128
  ) -> u128 {
    log::error!("convert_to_balance shares        {:?}", shares);
    log::error!("convert_to_balance total_shares  {:?}", total_shares);
    log::error!("convert_to_balance total_balance {:?}", total_balance);
    if total_shares == 0 {
      return shares;
    }
    // shares.saturating_mul(total_balance).saturating_div(total_shares)
    shares.saturating_mul(total_balance.saturating_div(total_shares))
  }

  pub fn convert_to_shares(
    balance: u128,
    total_shares: u128,
    total_balance: u128
  ) -> u128 {
    log::error!("convert_to_shares balance        {:?}", balance);
    log::error!("convert_to_shares total_shares   {:?}", total_shares);
    log::error!("convert_to_shares total_balance  {:?}", total_balance);
    if total_shares == 0 {
      return balance;
    }
    // balance.saturating_mul(total_shares).saturating_div(total_balance)
    balance.saturating_mul(total_shares.saturating_div(total_balance))
  }
}