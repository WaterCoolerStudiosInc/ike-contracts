use crate::governance::{GovernanceError, PropType, Proposal, Vote};
use ink::{primitives::AccountId, prelude::vec::Vec};

#[ink::trait_definition]

pub trait IGovernaance {
    #[ink(message)]
    fn get_council(&self) -> AccountId;
    #[ink(message)]
    fn get_staking(&self) -> AccountId;
    #[ink(message)]
    fn get_voting_delay(&self) -> u64;
    #[ink(message)]
    fn get_voting_period(&self) -> u64;
    #[ink(message)]
    fn get_execution_threshold(&self) -> u128;
    #[ink(message)]
    fn get_rejection_threshold(&self) -> u128;
    #[ink(message)]
    fn get_acceptance_threshold(&self) -> u128;
    #[ink(message)]
    fn get_proposal_by_id(&self, id: u128) -> Option<Proposal>;
    #[ink(message)]
    fn get_all_proposals(&self) -> Vec<Proposal>;
    #[ink(message)]
    fn get_proposal_by_nft(&self, id: u128) -> Option<Proposal>;
    #[ink(message, selector = 33)]
    fn get_active_proposal_status_by_nft(&self, id: u128) -> bool;

    #[ink(message)]
    fn create_proposal(&mut self, prop: PropType, nft_id: u128) -> Result<(), GovernanceError>;

    #[ink(message)]
    fn vote(&mut self, prop_id: u128, nft_id: u128, pro: Vote) -> Result<(), GovernanceError>;
    #[ink(message)]
    fn complete_proposal(&mut self, prop_id: u128) -> Result<(), GovernanceError>;
    #[ink(message)]
    fn cancel_proposal(&mut self, prop_id: u128) -> Result<(), GovernanceError>;
}
