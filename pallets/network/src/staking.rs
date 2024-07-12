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
    pub fn do_add_stake(
        origin: T::RuntimeOrigin,
        model_id: u32,
        _hotkey: T::AccountId,
        stake_to_be_added: u128
    ) -> DispatchResult {
        let account_id: T::AccountId = ensure_signed(origin)?;

        let stake_as_balance = Self::u128_to_balance(stake_to_be_added);

        ensure!(stake_as_balance.is_some(), Error::<T>::CouldNotConvertToBalance);

        let account_stake_balance: u128 = AccountModelStake::<T>::get(&account_id, model_id);

        ensure!(
            account_stake_balance.saturating_add(stake_to_be_added) >= MinStakeBalance::<T>::get(),
            Error::<T>::MinStakeNotReached
        );

        ensure!(
            account_stake_balance.saturating_add(stake_to_be_added) <= MaxStakeBalance::<T>::get(),
            Error::<T>::MaxStakeReached
        );

        // --- Ensure the callers account_id has enough stake to perform the transaction.
        ensure!(
            Self::can_remove_balance_from_coldkey_account(&account_id, stake_as_balance.unwrap()),
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
            Self::remove_balance_from_coldkey_account(&account_id, stake_as_balance.unwrap()) ==
                true,
            Error::<T>::BalanceWithdrawalError
        );

        Self::increase_account_stake(&account_id, model_id, stake_to_be_added);

        // Set last block for rate limiting
        Self::set_last_tx_block(&account_id, block);

        Self::deposit_event(Event::StakeAdded(model_id, account_id, stake_to_be_added));

        Ok(())
    }

    pub fn do_remove_stake(
        origin: T::RuntimeOrigin,
        model_id: u32,
        _hotkey: T::AccountId,
        is_peer: bool,
        stake_to_be_removed: u128
    ) -> DispatchResult {
        let account_id: T::AccountId = ensure_signed(origin)?;

        // --- Ensure that the stake amount to be removed is above zero.
        ensure!(stake_to_be_removed > 0, Error::<T>::NotEnoughStakeToWithdraw);

        let account_stake_balance: u128 = AccountModelStake::<T>::get(
            &account_id,
            model_id.clone()
        );

        // --- Ensure that the account has enough stake to withdraw.
        ensure!(account_stake_balance >= stake_to_be_removed, Error::<T>::NotEnoughStakeToWithdraw);

        // if user is still a peer in consensus they must keep the required minimum balance
        if is_peer {
            ensure!(
                account_stake_balance.saturating_sub(stake_to_be_removed) >=
                    MinStakeBalance::<T>::get(),
                Error::<T>::MinStakeNotReached
            );
        }

        // --- Ensure that we can conver this u128 to a balance.
        let stake_to_be_added_as_currency = Self::u128_to_balance(stake_to_be_removed);
        ensure!(stake_to_be_added_as_currency.is_some(), Error::<T>::CouldNotConvertToBalance);

        let block: u64 = Self::get_current_block_as_u64();
        ensure!(
            !Self::exceeds_tx_rate_limit(Self::get_last_tx_block(&account_id), block),
            Error::<T>::TxRateLimitExceeded
        );

        // --- 7. We remove the balance from the hotkey.
        Self::decrease_account_stake(&account_id, model_id, stake_to_be_removed);

        let remaining_account_stake_balance: u128 = AccountModelStake::<T>::get(
            &account_id,
            model_id
        );

        // --- 8. If model stake balance is zero, remove from ModelAccount
        if remaining_account_stake_balance == 0 {
            let mut model_accounts = ModelAccount::<T>::get(model_id);
            model_accounts.remove(&account_id);
            ModelAccount::<T>::insert(model_id.clone(), model_accounts);
        }

        // --- 9. We add the balancer to the account_id.  If the above fails we will not credit this account_id.
        Self::add_balance_to_coldkey_account(&account_id, stake_to_be_added_as_currency.unwrap());

        // Set last block for rate limiting
        Self::set_last_tx_block(&account_id, block);

        Self::deposit_event(Event::StakeRemoved(model_id, account_id, stake_to_be_removed));

        Ok(())
    }

    pub fn increase_account_stake(account_id: &T::AccountId, model_id: u32, amount: u128) {
        // -- increase account model staking balance
        AccountModelStake::<T>::insert(
            account_id,
            model_id.clone(),
            AccountModelStake::<T>::get(account_id, model_id).saturating_add(amount)
        );

        // -- increase account_id total stake
        TotalAccountStake::<T>::mutate(account_id, |n| {
            *n += amount;
        });

        // -- increase total model stake
        TotalModelStake::<T>::mutate(model_id.clone(), |n| {
            *n += amount;
        });

        // -- increase total stake overall
        TotalStake::<T>::mutate(|n| {
            *n += amount;
        });
    }

    pub fn decrease_account_stake(account_id: &T::AccountId, model_id: u32, amount: u128) {
        // -- decrease account model staking balance
        AccountModelStake::<T>::insert(
            account_id,
            model_id.clone(),
            AccountModelStake::<T>::get(account_id, model_id).saturating_sub(amount)
        );

        // -- decrease account_id total stake
        TotalAccountStake::<T>::mutate(account_id, |n| {
            *n -= amount;
        });

        // -- decrease total stake overall
        TotalStake::<T>::mutate(|n| {
            *n -= amount;
        });

        // -- decrease total model stake
        TotalModelStake::<T>::mutate(model_id.clone(), |n| {
            *n -= amount;
        });
    }

    pub fn can_remove_balance_from_coldkey_account(
        account_id: &T::AccountId,
        amount: <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance
    ) -> bool {
        let current_balance = Self::get_coldkey_balance(account_id);
        if amount > current_balance {
            return false;
        }

        // This bit is currently untested. @todo
        let new_potential_balance = current_balance - amount;
        let can_withdraw = T::Currency::ensure_can_withdraw(
            &account_id,
            amount,
            WithdrawReasons::except(WithdrawReasons::TIP),
            new_potential_balance
        ).is_ok();
        can_withdraw
    }

    pub fn remove_balance_from_coldkey_account(
        account_id: &T::AccountId,
        amount: <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance
    ) -> bool {
        return match
            T::Currency::withdraw(
                &account_id,
                amount,
                WithdrawReasons::except(WithdrawReasons::TIP),
                ExistenceRequirement::KeepAlive
            )
        {
            Ok(_result) => true,
            Err(_error) => false,
        };
    }

    pub fn add_balance_to_coldkey_account(
        account_id: &T::AccountId,
        amount: <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance
    ) {
        let _ = T::Currency::deposit_creating(&account_id, amount); // Infallibe
    }

    pub fn get_coldkey_balance(
        account_id: &T::AccountId
    ) -> <<T as pallet::Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance {
        return T::Currency::free_balance(&account_id);
    }

    // pub fn u64_to_balance(
    //   input: u64,
    // ) -> Option<
    //   <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance,
    // > {
    //   input.try_into().ok()
    // }

    pub fn u128_to_balance(
        input: u128
    ) -> Option<<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance> {
        input.try_into().ok()
    }
}
