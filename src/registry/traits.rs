use crate::registry::{Agent, RegistryError};
use ink::{prelude::vec::Vec, primitives::AccountId};

#[ink::trait_definition]
pub trait Registry {
    #[ink(message, selector = 1)]
    fn add_agent(
        &mut self,
        admin: AccountId,
        validator: AccountId,
        pool_create_amount: u128,
        existential_deposit: u128,
    ) -> Result<(), RegistryError>;
    #[ink(message, selector = 2)]
    fn update_agents(
        &mut self,
        accounts: Vec<AccountId>,
        new_weights: Vec<(u64,bool)>,
    ) -> Result<(), RegistryError>;
    #[ink(message, selector = 3)]
    fn remove_agent(&mut self, account: AccountId) -> Result<(), RegistryError>;
    #[ink(message, selector = 4)]
    fn get_agents(&self) -> Result<(u64, Vec<Agent>), RegistryError>;
    #[ink(message, selector = 5)]
    fn init_remove_agent(&mut self, account: AccountId) -> Result<(), RegistryError>;
}
