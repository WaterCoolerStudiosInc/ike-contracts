#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod multisig {
    use ink::{
        codegen::EmitEvent,
        contract_ref,
        env::{debug_println, Error as InkEnvError},
        prelude::{format, string::String, vec::Vec},
        reflect::ContractEventBase,
        storage::Mapping,
        ToAccountId,
    };
    use registry::{registry::RegistryError, Registry};
    use vault::Vault;

    #[ink(storage)]
    pub struct MultiSig {
        pub admin: AccountId,
        pub signers: Vec<AccountId>,
        pub threshold: u16,
        pub creation_time:u64
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum MultiSigError {
        SignerNotFound,
        VaultFailure,
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
    type Event = <Governor as ContractEventBase>::Type;
    // internal calls
    impl MultiSig {
        fn emit_event<EE>(emitter: EE, event: Event)
        where
            EE: EmitEvent<Governor>,
        {
            emitter.emit_event(event);
        }
        fn update_registry_weights(
            &self,
            accounts: Vec<AccountId>,
            new_weights: Vec<u64>,
        ) -> Result<(), GovernorError> {
            let mut registry: contract_ref!(Registry) = self.registry.into();
            if let Err(e) = registry.update_agents(accounts, new_weights) {
                return Err(GovernorError::RegistryFailure);
            }
            Ok(())
        }
        fn add_registry_agent(
            &self,
            account: AccountId,
            new_weight: u64,
        ) -> Result<(), GovernorError> {
            let mut registry: contract_ref!(Registry) = self.registry.into();
            if let Err(e) = registry.add_agent(account, new_weight) {
                return Err(GovernorError::RegistryFailure);
            }
            Ok(())
        }
        fn remove_agent(&self, account: AccountId) -> Result<(), GovernorError> {
            let mut registry: contract_ref!(Registry) = self.registry.into();
            if let Err(e) = registry.remove_agent(account) {
                return Err(GovernorError::RegistryFailure);
            }
            Ok(())
        }
        fn update_vault_fees(&self, new_fee: u16) -> Result<(), GovernorError> {
            let mut vault: contract_ref!(Vault) = self.vault.into();
            if let Err(e) = vault.adjust_fee(new_fee) {
                return Err(GovernorError::VaultFailure);
            }
            Ok(())
        }
        fn get_curr_epoch(&self, current_time: Timestamp) -> u64 {
            (current_time - self.creation_time) / self.epoch
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
        pub fn new(_admin: AccountId) -> Self {
            Self {
                admin: _admin,
                signers: Vec::new(),
                threshold: initial_roles,
                creation_time:Self::env().block_timestamp();
            }
        }
        #[message]
        pub fn remove_signer(&mut self, _signer: AccountId) -> Result<(), MultiSigError> {
            let caller = Self.env.caller();
            if caller != self.admin {
                return Err(MultiSigError::Unauthorized);
            }
            if let Some(index) = self.signers.iter().position(|a| a.address == _signer) {
                self.signers.remove(index);
                Self::env().emit_event(SignerRemoved { signer: _signer });
            } else {
                return Err(MultiSigError::SignerNotFound);
            }
            Ok(())
        }
        #[message]
        pub fn add_signer(&mut self, _signer: AccountId) -> Result<(), MultiSigError> {
            let caller = Self.env.caller();
            if caller != self.admin {
                return Err(MultiSigError::Unauthorized);
            }
            self.signers.push(_signer);
        }
    }
}
