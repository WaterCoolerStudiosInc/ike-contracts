use crate::staking::StakingError;
#[ink::trait_definition]
pub trait Staking{
    #[ink(message,selector = 17)]
    fn update_rewards_rate(&mut self,new_rate:u128) -> Result<(), StakingError>;
}