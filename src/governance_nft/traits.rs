use ink::{primitives::AccountId};
use psp22::PSP22Error;

#[ink::trait_definition]
pub trait GovernanceNFT{
    #[ink(message, selector = 7777)]
    fn mint(&mut self, to: AccountId, value: u128) -> Result<(), PSP22Error>;
}
