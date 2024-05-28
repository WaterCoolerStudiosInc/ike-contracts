use ink::{primitives::AccountId};
use psp34::PSP34Error;
use crate::governance_nft::GovernanceData;
#[ink::trait_definition]
pub trait GovernanceNFT{
    #[ink(message, selector = 1337)]
    fn mint(&mut self, to: AccountId, value: u128) -> Result<(), PSP34Error>;
    #[ink(message, selector = 31337)]
    fn get_governance_data(&mut self, id:u128) -> GovernanceData;
    #[ink(message, selector = 8057)]
    fn burn(&mut self, account: AccountId, id: u128) -> Result<(), PSP34Error>; 

}
