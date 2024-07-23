use ink::{primitives::AccountId};
use psp22::PSP22Error;
use crate::multisig::MultiSigError;

#[ink::trait_definition]
pub trait MultiSig {   

   #[ink(message, selector = 1)]
   fn add_signer(&mut self, _signer: AccountId) -> Result<(), MultiSigError>;
   #[ink(message, selector = 2)]
   fn remove_signer(&mut self, _signer: AccountId) -> Result<(), MultiSigError>;
   #[ink(message,selector = 3)]
   fn update_threshold(&mut self,new_threshold:u16)-> Result<(),MultiSigError>;
   #[ink(message,selector = 4)]
   fn replace_signer(&mut self, signer_old: AccountId,signer_new:AccountId) -> Result<(), MultiSigError>;
}
