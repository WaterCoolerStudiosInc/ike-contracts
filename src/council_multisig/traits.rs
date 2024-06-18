use ink::{primitives::AccountId};
use psp22::PSP22Error;
use crate::MultiSigError;

#[ink::trait_definition]
pub trait MultiSig {   

   #[ink(message, selector = 1)]
   fn add_signer(&mut self, _signer: AccountId) -> Result<(), MultiSigError> 
   #[ink(message, selector = 2)]
   pub fn remove_signer(&mut self, _signer: AccountId) -> Result<(), MultiSigError> 
   #[ink(message,selector = 3)]
   pub fn update_threshold(&mut self,new_threshold:u16)-> Result<(),MultiSigError>
}
