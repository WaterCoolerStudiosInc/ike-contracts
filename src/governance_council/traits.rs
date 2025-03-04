use crate::governance_council::Action;
use crate::governance_council::CouncilError;
use ink::prelude::vec::Vec;
use ink::primitives::AccountId;

#[ink::trait_definition]
pub trait ICouncil {
    #[ink(message, selector = 1)]
    fn add_signer(&mut self, signer: AccountId) -> Result<(), CouncilError>;

    #[ink(message, selector = 2)]
    fn remove_signer(&mut self, signer: AccountId) -> Result<(), CouncilError>;

    #[ink(message, selector = 3)]
    fn update_threshold(&mut self, new_threshold: u16) -> Result<(), CouncilError>;

    #[ink(message, selector = 4)]
    fn replace_signer(
        &mut self,
        signer_old: AccountId,
        signer_new: AccountId,
    ) -> Result<(), CouncilError>;

    #[ink(message, selector = 7)]
    fn endorse_proposal(&mut self, action: Action) -> Result<(), CouncilError>;

    #[ink(message, selector = 8)]
    fn get_signers(&self) -> Vec<AccountId>;

    #[ink(message, selector = 9)]
    fn set_gov_staking(&mut self, new_account: AccountId) -> Result<(), CouncilError>;
}
