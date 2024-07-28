use super::*;

impl<T: Config> Pallet<T> {
  pub fn vote(
    proposal_id: u32,
    account_id: T::AccountId
  ) -> DispatchResult {

    Ok(())
  }
}