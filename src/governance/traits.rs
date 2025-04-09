use super::*;
use crate::governance::{GovernanceError, PropType, Proposal, Vote};
use ink::{prelude::vec::Vec, primitives::AccountId};

#[ink::trait_definition]
pub trait IGovernance {
    #[ink(message)]
    fn get_council(&self) -> AccountId;

    #[ink(message)]
    fn get_staking(&self) -> AccountId;

    #[ink(message)]
    fn get_voting_delay(&self) -> Time;

    #[ink(message)]
    fn get_voting_period(&self) -> Time;

    #[ink(message)]
    fn get_execution_threshold(&self) -> u128;

    #[ink(message)]
    fn get_rejection_threshold(&self) -> u128;

    #[ink(message)]
    fn get_acceptance_threshold(&self) -> u128;

    #[ink(message)]
    fn get_max_proposals(&self) -> u16;

    #[ink(message)]
    fn get_proposal_by_id(&self, id: PropId) -> Option<Proposal>;

    #[ink(message)]
    fn get_all_proposals(&self) -> Vec<Proposal>;

    #[ink(message)]
    fn get_proposal_by_nft(&self, id: NftId) -> Option<Proposal>;

    #[ink(message, selector = 33)]
    fn get_active_proposal_status_by_nft(&self, id: NftId) -> bool;

    #[ink(message)]
    fn create_proposal(&mut self, prop: PropType, nft_id: NftId) -> Result<(), GovernanceError>;

    #[ink(message)]
    fn vote(&mut self, prop_id: PropId, nft_id: NftId, pro: Vote) -> Result<(), GovernanceError>;

    #[ink(message)]
    fn complete_proposal(&mut self, prop_id: PropId) -> Result<(), GovernanceError>;

    #[ink(message)]
    fn cancel_proposal(&mut self, prop_id: PropId) -> Result<(), GovernanceError>;

    #[ink(message, selector = 100)]
    fn consume_validator_whitelist(
        &mut self,
        validator: AccountId,
        agent_admin: AccountId,
    ) -> Result<(), GovernanceError>;
}
