use crate::validator_whitelist::WhitelistError;
use ink::primitives::AccountId;

#[ink::trait_definition]
pub trait ValidatorWhitelist {
    #[ink(message, selector = 1)]
    fn init_add_validator(&mut self, validator: AccountId) -> Result<(), WhitelistError>;
    #[ink(message, selector = 2)]
    fn remove_validator_by_agent(
        &mut self,
        agent: AccountId,
        slash: bool,
    ) -> Result<(), WhitelistError>;
}
