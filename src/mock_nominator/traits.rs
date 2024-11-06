use crate::errors::RuntimeError;
use ink::primitives::AccountId;

#[ink::trait_definition]
pub trait INominationAgent {
    #[ink(message, payable, selector = 1)]
    fn deposit(&mut self) -> Result<(), RuntimeError>;

    #[ink(message, selector = 2)]
    fn start_unbond(&mut self, amount: u128) -> Result<(), RuntimeError>;

    #[ink(message, selector = 3)]
    fn withdraw_unbonded(&mut self) -> Result<(), RuntimeError>;

    #[ink(message, selector = 4)]
    fn compound(&mut self) -> Result<u128, RuntimeError>;

    #[ink(message, selector = 12)]
    fn get_staked_value(&self) -> u128;

    #[ink(message, selector = 13)]
    fn get_unbonding_value(&self) -> u128;

    #[ink(message)]
    fn get_vault(&self) -> AccountId;

    #[ink(message)]
    fn get_admin(&self) -> AccountId;

    #[ink(message)]
    fn get_validator(&self) -> AccountId;

    #[ink(message, selector = 101)]
    fn destroy(&mut self) -> Result<(), RuntimeError>;

    #[ink(message, selector = 102)]
    fn admin_withdraw_bond(&mut self, to: AccountId) -> Result<u128, RuntimeError>;

    #[ink(message, selector = 999)]
    fn set_code(&mut self, code_hash: [u8; 32]) -> Result<(), RuntimeError>;
}
