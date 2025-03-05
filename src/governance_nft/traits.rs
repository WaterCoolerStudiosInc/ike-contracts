pub use crate::governance_nft::GovernanceData;
use ink::primitives::AccountId;
use psp34::Id;
use psp34::PSP34Error;

#[ink::trait_definition]
pub trait IGovernanceNFT {
    #[ink(message, selector = 1337)]
    fn mint(
        &mut self,
        to: AccountId,
        stake_weight: u128,
        vote_weight: u128,
    ) -> Result<u128, PSP34Error>;

    #[ink(message, selector = 31337)]
    fn get_governance_data(&self, id: u128) -> Option<GovernanceData>;

    #[ink(message, selector = 8057)]
    fn burn(&mut self, account: AccountId, id: u128) -> Result<(), PSP34Error>;

    #[ink(message, selector = 89)]
    fn increment_weights(
        &mut self,
        id: u128,
        stake_weight: u128,
        vote_weight: u128,
    ) -> Result<(), PSP34Error>;

    #[ink(message, selector = 99)]
    fn decrement_vote_weight(&mut self, id: u128, weight: u128) -> Result<(), PSP34Error>;

    #[ink(message, selector = 47)]
    fn unlock_transfer(&mut self) -> Result<(), PSP34Error>;

    #[ink(message, selector = 91)]
    fn lock_transfer(&mut self) -> Result<(), PSP34Error>;

    #[ink(message, selector = 69)]
    fn is_collection_locked(&self) -> bool;

    #[ink(message, selector = 17)]
    fn transfer_from(
        &mut self,
        from: AccountId,
        to: AccountId,
        id: Id,
        data: ink::prelude::vec::Vec<u8>,
    ) -> Result<(), PSP34Error>;

    #[ink(message)]
    fn owner_of_id(&self, id: u128) -> Option<AccountId>;

    #[ink(message, selector = 8888)]
    fn set_admin(&mut self, new_admin: AccountId) -> Result<(), PSP34Error>;
}
