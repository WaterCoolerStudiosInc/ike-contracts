use crate::multisig::Action;
use crate::multisig::MultiSigError;
use ink::prelude::{string::String,vec, vec::Vec};
use ink::primitives::AccountId;
#[ink::trait_definition]
pub trait IMultiSig {
    #[ink(message, selector = 1)]
    fn add_signer(&mut self, _signer: AccountId) -> Result<(), MultiSigError>;
    #[ink(message, selector = 2)]
    fn remove_signer(&mut self, _signer: AccountId) -> Result<(), MultiSigError>;
    #[ink(message, selector = 3)]
    fn update_threshold(&mut self, new_threshold: u16) -> Result<(), MultiSigError>;
    #[ink(message, selector = 4)]
    fn replace_signer(
        &mut self,
        signer_old: AccountId,
        signer_new: AccountId,
    ) -> Result<(), MultiSigError>;
    #[ink(message, selector = 7)]
    fn endorse_proposal(&mut self, action: Action) -> Result<(), MultiSigError>;
    #[ink(message, selector = 8)]
    fn get_signers(&self) -> Vec<AccountId>;
    #[ink(message, selector = 9)]
    fn set_whitelist(&mut self, new_list: AccountId) -> Result<(), MultiSigError>;
}
