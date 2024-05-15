use ink::{primitives::AccountId};
use psp22::PSP22Error;
use crate::data::VaultError;

#[ink::trait_definition]
pub trait Vault {   

    #[ink(message, selector = 7)]
    fn adjust_fee(&mut self, new_fee: u16) -> Result<(), VaultError>;
}
