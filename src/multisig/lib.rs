#![cfg_attr(not(feature = "std"), no_std, no_main)]
mod traits;
pub use traits::MultiSig;

#[ink::contract]
mod multisig {
    use ink::{
        codegen::EmitEvent, contract_ref, env::{debug_println, Error as InkEnvError}, prelude::{format, string::String, vec::Vec},  reflect::ContractEventBase, storage::Mapping, ToAccountId
    };
    use registry::{registry::RegistryError, Registry};
    use vault::Vault;

    #[ink(storage)]
    pub struct MultiSig {
        pub admin: AccountId,
        pub vault:AccountId,
        pub registry:AccountId,
        pub signers: Vec<AccountId>,
        pub threshold: u16,
        pub creation_time:u64
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum MultiSigError {
        SignerNotFound,
        VaultFailure,
        RegistryFailure,
        Unauthorized,
        InvalidInput,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum PropType {
        FeeUpdate,
        WeightUpdate,
        ValidatorAdd,
        ValidatorRemove,
    }
    #[ink(event)]
    pub struct SignerAdded {
        #[ink(topic)]
        signer: AccountId,
    }

    #[ink(event)]
    pub struct SignerRemoved {
        #[ink(topic)]
        signer: AccountId,
    }
    // Fee update process
    // Users have a vote weight based on token holdings
    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct FeeProposal {
        new_fee: u16,
        vote_points: u128,
    }
    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct WeightProposal {
        accounts: Vec<AccountId>,
        weights: Vec<u64>,
    }
    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct WeightUpdate {
        accounts: Vec<AccountId>,
        weights: Vec<u64>,
    }
    type Event = <MultiSig as ContractEventBase>::Type;
    // internal calls
    impl MultiSig {
        fn emit_event<EE>(emitter: EE, event: Event)
        where
            EE: EmitEvent<MultiSig>,
        {
            emitter.emit_event(event);
        }
        fn update_registry_weights(
            &self,
            accounts: Vec<AccountId>,
            new_weights: Vec<u64>,
        ) -> Result<(), MultiSigError> {
            let mut registry: contract_ref!(Registry) = self.registry.into();
            if let Err(e) = registry.update_agents(accounts, new_weights) {
                return Err(MultiSigError::RegistryFailure);
            }
            Ok(())
        }
        fn add_registry_agent(
            &self,
            account: AccountId,
            new_weight: u64,
        ) -> Result<(), MultiSigError> {
            let mut registry: contract_ref!(Registry) = self.registry.into();
            if let Err(e) = registry.add_agent(account, new_weight) {
                return Err(MultiSigError::RegistryFailure);
            }
            Ok(())
        }
        fn remove_agent(&self, account: AccountId) -> Result<(), MultiSigError> {
            let mut registry: contract_ref!(Registry) = self.registry.into();
            if let Err(e) = registry.remove_agent(account) {
                return Err(MultiSigError::RegistryFailure);
            }
            Ok(())
        }
        fn update_vault_fees(&self, new_fee: u16) -> Result<(), MultiSigError> {
            let mut vault: contract_ref!(Vault) = self.vault.into();
            if let Err(e) = vault.adjust_fee(new_fee) {
                return Err(MultiSigError::VaultFailure);
            }
            Ok(())
        }
       

        fn validate_weight_update(&self, update_request: Vec<u64>) -> bool {
            let mut registry: contract_ref!(Registry) = self.registry.into();
            let registry_weights = registry.get_agents().unwrap();
            if (update_request.len() == registry_weights.1.len()) {
                true
            } else {
                false
            }
        }
    }
    impl MultiSig {
        #[ink(constructor)]
        pub fn new(_admin: AccountId,_registry:AccountId,_vault:AccountId) -> Self {
            Self {
                admin: _admin,
                registry:_registry,
                vault:_vault,
                signers: Vec::new(),
                threshold: 1000,
                creation_time:Self::env().block_timestamp()
            }
        }
        #[ink(message,selector = 1)]
        pub fn add_signer(&mut self, _signer: AccountId) -> Result<(), MultiSigError> {
            let caller = Self::env().caller();
            if caller != self.admin {
                return Err(MultiSigError::Unauthorized);
            }
            self.signers.push(_signer);
            Self::emit_event(
                Self::env(),
                Event::SignerAdded(SignerAdded {
                   signer:_signer
                }),
            );
            Ok(())
        }
        #[ink(message,selector = 2)]
        pub fn remove_signer(&mut self, _signer: AccountId) -> Result<(), MultiSigError> {
            let caller = Self::env().caller();
            if caller != self.admin {
                return Err(MultiSigError::Unauthorized);
            }
            if let Some(index) = self.signers.iter().position(|a| *a == _signer) {
                self.signers.remove(index);
                Self::emit_event(
                    Self::env(),
                    Event::SignerRemoved(SignerRemoved {
                       signer:_signer
                    }),
                );
            } else {
                return Err(MultiSigError::SignerNotFound);
            }
            Ok(())
        }
       
        #[ink(message,selector = 3)]
        pub fn update_threshold(&mut self,new_threshold:u16)-> Result<(),MultiSigError>{
            let caller = Self::env().caller();
            if caller != self.admin {
                return Err(MultiSigError::Unauthorized);
            }   
            self.threshold=new_threshold;
            Ok(())
        }
    }
}
