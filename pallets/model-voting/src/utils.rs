use super::*;
use system::Config;

impl<T: Config> Pallet<T> {
  pub fn is_activation_proposal_ok(
    account_id: T::AccountId,
    path: Vec<u8>, 
    model_peers: Vec<ModelPeer<T::AccountId>>,
  ) -> bool {
    // Check path doesn't already exist in Network or ModelVoting
    // If it doesn't already exist, then it has either been not proposed or deactivated
    ensure!(
      !T::ModelVote::get_model_path_exist(path.clone()),
      Error::<T>::ModelPathExists
    );

    // // Ensure can propose new model path
    // let proposal_status = PropsPathStatus::<T>::get(path.clone());

    // ensure!(
    //   proposal_status != PropsStatus::ActivateVoting ||
    //   proposal_status != PropsStatus::DectivateVoting ||
    //   proposal_status != PropsStatus::Activated,
    //   Error::<T>::ProposalInvalid
    // );

    // // Ensure account has enough balance to pay cost of model initialization
    // let model_initialization_cost = T::ModelVote::get_model_initialization_cost();
    // let model_initialization_cost_as_balance = Self::u128_to_balance(model_initialization_cost);

    // ensure!(
    //   model_initialization_cost_as_balance.is_some(),
    //   Error::<T>::CouldNotConvertToBalance
    // );

    // let initializer_balance = T::Currency::free_balance(&account_id);
    // ensure!(
    //   model_initialization_cost_as_balance.unwrap() >= initializer_balance,
    //   Error::<T>::NotEnoughModelInitializationBalance
    // );

    // // Lock balance
    // // The final initialization fee may be more or less than the current initialization cost results
    // T::Currency::set_lock(
    //   MODEL_VOTING_ID,
    //   &account_id,
    //   model_initialization_cost_as_balance.unwrap(),
    //   WithdrawReasons::RESERVE
    // );
  
    // // Ensure account is already an existing peer and account eligible

    // // Ensure minimum peers required are already met before going forward
    // // @to-do: Get minimum model peers from network pallet
    // ensure!(
    //   model_peers.len() as u32 >= T::ModelVote::get_min_model_peers() && 
    //   model_peers.len() as u32 <= T::ModelVote::get_max_model_peers(),
    //   Error::<T>::ModelPeersMin
    // );

    // // Ensure peers have the minimum required stake balance
    // let min_stake: u128 = T::ModelVote::get_min_stake_balance();
    // let min_stake_as_balance = Self::u128_to_balance(min_stake);

    // ensure!(
    //   min_stake_as_balance.is_some(),
    //   Error::<T>::CouldNotConvertToBalance
    // );

    // for peer in model_peers.clone() {
    //   let peer_balance = T::Currency::free_balance(&peer.account_id);

    //   ensure!(
    //     peer_balance >= min_stake_as_balance.unwrap(),
    //     Error::<T>::NotEnoughMinStakeBalance
    //   );
    // }

    true
  }
}