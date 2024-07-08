use ink::{primitives::AccountId};
use psp34::PSP34Error;
use crate::governance_nft::GovernanceData;
#[ink::trait_definition]
pub trait GovernanceNFT{
    #[ink(message, selector = 1337)]
    fn mint(&mut self, to: AccountId, weight: u128) -> Result<u128, PSP34Error>;    
    #[ink(message, selector = 31337)]
    fn get_governance_data(&mut self, id:u128) -> GovernanceData;
    #[ink(message, selector = 8057)]
    fn burn(&mut self, account: AccountId, id: u128) -> Result<(), PSP34Error>; 
    #[ink(message, selector = 99)]
    fn increment_weight(&mut self,id:u128,weight:u128) -> Result<(), PSP34Error>;
    #[ink(message, selector = 77)]
    fn decrement_weight(&mut self,id:u128,weight:u128) -> Result<(), PSP34Error>;
    #[ink(message, selector = 17)]
    fn transfer_from(
        &mut self,
        from:AccountId,
        to: AccountId,
        id: u128,
        data: ink::prelude::vec::Vec<u8>,
    ) -> Result<(), PSP34Error> ;
}
