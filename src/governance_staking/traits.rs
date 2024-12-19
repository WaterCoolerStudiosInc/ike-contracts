use crate::staking::StakingError;
use ink::primitives::AccountId;
#[ink::trait_definition]
pub trait Staking {
    #[ink(message, selector = 1)]
    fn update_rewards_rate(&mut self, new_rate: u128) -> Result<(), StakingError>;
    #[ink(message, selector = 10)]
    fn onboard_validator(&mut self, validator: AccountId) -> Result<(), StakingError>;
    #[ink(message, selector = 11)]
    fn disable_validator(&mut self, agent: AccountId, slash: bool) -> Result<(), StakingError>;
}
