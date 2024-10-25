use crate::errors::RegistryError;
use crate::registry::{Agent, RoleType};
use ink::{primitives::AccountId, prelude::vec::Vec};

#[ink::trait_definition]
pub trait IRegistry {
    #[ink(message, payable, selector = 1)]
    fn add_agent(
        &mut self,
        admin: AccountId,
        validator: AccountId,
    ) -> Result<AccountId, RegistryError>;
    #[ink(message, selector = 2)]
    fn update_agents(
        &mut self,
        accounts: Vec<AccountId>,
        new_weights: Vec<u64>,
    ) -> Result<(), RegistryError>;
    #[ink(message, selector = 3)]
    fn remove_agent(&mut self, account: AccountId) -> Result<(), RegistryError>;
    #[ink(message, selector = 4)]
    fn get_agents(&self) -> (u64, Vec<Agent>);

    #[ink(message)]
    fn transfer_role(
        &mut self,
        role_type: RoleType,
        new_account: AccountId,
    ) -> Result<(), RegistryError>;

    #[ink(message)]
    fn transfer_role_admin(
        &mut self,
        role_type: RoleType,
        new_account: AccountId,
    ) -> Result<(), RegistryError>;

    #[ink(message)]
    fn set_code(&mut self, code_hash: [u8; 32]) -> Result<(), RegistryError>;

    #[ink(message)]
    fn set_agent_code(&mut self, nomination_agent_hash: [u8; 32]) -> Result<(), RegistryError>;

    #[ink(message)]
    fn get_role(&self, role_type: RoleType) -> AccountId;

    #[ink(message)]
    fn get_role_admin(&self, role_type: RoleType) -> AccountId;

    #[ink(message)]
    fn get_max_agents(&self) -> u32;
}
