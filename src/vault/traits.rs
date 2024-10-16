use crate::data::{Balance, UnlockRequest};
use crate::errors::VaultError;
use ink::{
    primitives::AccountId,
    prelude::vec::Vec,
};

#[ink::trait_definition]
pub trait RateProvider {
    // Get "rate" of a particular token with respect to a given base token.
    // For instance, in the context of liquid staking, the base token could be the native token of the chain and the rate,
    // at a particular point of time would be the price of the yield bearing liquid staking token in terms of the base token.
    // The rate is supposed to have precision of RATE_DECIMALS=12 decimal places. So if the rate is 1.5, it should be represented as 1.5 * 10^12.
    // Note that the rate is expected to be a number relatively close to 1.0. More specifically, with the selected precision, the maximum
    // supported rate is of the order of 10^8, but in practice one would expect (get_rate() / 10^12) \in [0.001, 1000.0].
    #[ink(message)]
    fn get_rate(&mut self) -> u128;
}

#[ink::trait_definition]
pub trait IVault {
    #[ink(message, payable)]
    fn stake(&mut self) -> Result<u128, VaultError>;

    #[ink(message, payable)]
    fn stake_with_referral(&mut self, referral_id: AccountId) -> Result<u128, VaultError>;

    #[ink(message)]
    fn request_unlock(&mut self, shares: u128) -> Result<(), VaultError>;

    #[ink(message)]
    fn delegate_withdraw_unbonded(&mut self) -> Result<(), VaultError>;

    #[ink(message)]
    fn redeem(&mut self, user: AccountId, unlock_id: u64) -> Result<(), VaultError>;

    #[ink(message)]
    fn redeem_with_withdraw(&mut self, user: AccountId, unlock_id: u64) -> Result<(), VaultError>;

    #[ink(message)]
    fn compound(&mut self) -> Result<Balance, VaultError>;

    #[ink(message)]
    fn withdraw_fees(&mut self) -> Result<(), VaultError>;

    #[ink(message)]
    fn set_code(&mut self, code_hash: [u8; 32]) -> Result<(), VaultError>;

    #[ink(message)]
    fn disable_set_code(&mut self) -> Result<(), VaultError>;

    #[ink(message)]
    fn adjust_fee(&mut self, new_fee: u16) -> Result<(), VaultError>;

    #[ink(message)]
    fn get_role_adjust_fee(&self) -> AccountId;

    #[ink(message)]
    fn transfer_role_adjust_fee(&mut self, new_account: AccountId) -> Result<(), VaultError>;

    #[ink(message)]
    fn get_role_fee_to(&self) -> AccountId;

    #[ink(message)]
    fn transfer_role_fee_to(&mut self, new_account: AccountId) -> Result<(), VaultError>;

    #[ink(message)]
    fn get_role_set_code(&self) -> Option<AccountId>;

    #[ink(message)]
    fn transfer_role_set_code(&mut self, new_account: AccountId) -> Result<(), VaultError>;

    #[ink(message)]
    fn get_total_pooled(&self) -> Balance;

    #[ink(message)]
    fn get_total_shares(&self) -> u128;

    #[ink(message)]
    fn get_current_virtual_shares(&self) -> u128;

    #[ink(message)]
    fn get_fee_percentage(&self) -> u16;

    #[ink(message)]
    fn get_share_token_contract(&self) -> AccountId;

    #[ink(message)]
    fn get_registry_contract(&self) -> AccountId;

    #[ink(message)]
    fn get_shares_from_azero(&self, azero: Balance) -> u128;

    #[ink(message)]
    fn get_azero_from_shares(&self, shares: u128) -> Balance;

    #[ink(message)]
    fn get_unlock_requests(&self, user: AccountId) -> Vec<UnlockRequest>;

    #[ink(message)]
    fn get_weight_imbalances(&self, total_pooled: u128) -> (u128, u128, Vec<u128>, Vec<i128>);
}
