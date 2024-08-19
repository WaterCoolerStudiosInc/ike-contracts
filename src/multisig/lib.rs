#![cfg_attr(not(feature = "std"), no_std, no_main)]
mod traits;
pub use crate::multisig::MultiSigRef;
pub use traits::MultiSig;
#[ink::contract]
mod multisig {
    use core::fmt::Error;

    use ink::{
        codegen::EmitEvent,
        contract_ref,
        env::{
            hash::{HashOutput, Sha2x256},
            hash_encoded,
        },
        prelude::{string::String, vec::Vec},
        reflect::ContractEventBase,
        storage::Mapping,
    };
    use registry::Registry;
    use validator_whitelist::ValidatorWhitelist;
    #[ink(storage)]
    pub struct MultiSig {
        pub admin: AccountId,
        pub whitelist: AccountId,
        pub registry: AccountId,
        pub signers: Vec<AccountId>,
        pub threshold: u16,
        pub creation_time: u64,
        pub used_nonces: Mapping<String, bool>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum MultiSigError {
        SignerNotFound,
        VaultFailure,
        RegistryFailure,
        Unauthorized,
        InvalidInput,
        UsedNonce,
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

    #[ink(event)]
    pub struct SignerReplaced {
        #[ink(topic)]
        removed: AccountId,
        added: AccountId,
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
    #[derive(Debug, PartialEq, Eq, scale::Encode, Clone, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Action {
        UpdateValidators(WeightUpdate),
        AddValidator(AccountId),
        RemoveValidator(AccountId, bool),
        InitValidator(AccountId, u32),
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
        fn hash_update(&self, update: WeightUpdate, nonce: String) -> [u8; 32] {
            let encodable: (Vec<ink::primitives::AccountId>, Vec<u64>, String) =
                (update.accounts, update.weights, nonce);
            let mut output = <Sha2x256 as HashOutput>::Type::default();
            hash_encoded::<Sha2x256, _>(&encodable, &mut output);
            output
        }
        fn hash_initiate(&self, validator: AccountId, pool_id: u32, nonce: String) -> [u8; 32] {
            let encodable = (validator, pool_id, nonce);
            let mut output = <Sha2x256 as HashOutput>::Type::default();
            hash_encoded::<Sha2x256, _>(&encodable, &mut output);
            output
        }
        fn hash_addition(&self, validator: AccountId, nonce: String) -> [u8; 32] {
            let encodable = (validator, nonce);
            let mut output = <Sha2x256 as HashOutput>::Type::default();
            hash_encoded::<Sha2x256, _>(&encodable, &mut output);
            output
        }
        fn hash_remove(&self, validator: AccountId, slash: bool, nonce: String) -> [u8; 32] {
            let encodable = (validator, slash, nonce);
            let mut output = <Sha2x256 as HashOutput>::Type::default();
            hash_encoded::<Sha2x256, _>(&encodable, &mut output);
            output
        }
        fn hash_execution(&self, tx: Action, nonce: String) -> Result<[u8; 32], Error> {
            match tx {
                Action::UpdateValidators(weight_update) => {
                    Ok(self.hash_update(weight_update, nonce))
                }
                Action::AddValidator(validator) => Ok(self.hash_addition(validator, nonce)),
                Action::RemoveValidator(validator, slash) => {
                    Ok(self.hash_remove(validator, slash, nonce))
                }
                Action::InitValidator(validator, pool_id) => {
                    Ok(self.hash_initiate(validator, pool_id, nonce))
                }
            }
        }
        fn execute_add(&self, validator: AccountId) -> Result<(), MultiSigError> {
            let mut whitelist: contract_ref!(ValidatorWhitelist) = self.whitelist.into();
            if let Err(e) = whitelist.init_add_validator(validator) {
                return Err(MultiSigError::VaultFailure);
            }
            Ok(())
        }
        fn execute_remove(&self, validator: AccountId, slash: bool) -> Result<(), MultiSigError> {
            let mut whitelist: contract_ref!(ValidatorWhitelist) = self.whitelist.into();
            if let Err(e) = whitelist.remove_validator_by_agent(validator, slash) {
                return Err(MultiSigError::VaultFailure);
            }
            Ok(())
        }
        fn execute_init(&self, validator: AccountId, pool_id: u32) -> Result<(), MultiSigError> {
            let mut registry: contract_ref!(Registry) = self.registry.into();
            if let Err(e) = registry.initialize_agent(validator, pool_id) {
                return Err(MultiSigError::RegistryFailure);
            }
            Ok(())
        }
        fn execute_update(
            &self,
            agents: Vec<AccountId>,
            weights: Vec<u64>,
        ) -> Result<(), MultiSigError> {
            let mut registry: contract_ref!(Registry) = self.registry.into();
            if let Err(e) = registry.update_agents(agents, weights) {
                return Err(MultiSigError::RegistryFailure);
            }
            Ok(())
        }
        fn execute(&self, tx: Action) -> Result<(), MultiSigError> {
            match tx {
                Action::UpdateValidators(weight_update) => {
                    self.execute_update(weight_update.accounts, weight_update.weights)
                }
                Action::AddValidator(validator) => self.execute_add(validator),
                Action::RemoveValidator(validator, slash) => {
                    self.execute_remove(validator, slash)
                }
                Action::InitValidator(validator, pool_id) => {
                    self.execute_init(validator, pool_id)
                }
            }
        }
    }
    impl MultiSig {
        #[ink(constructor)]
        pub fn new(_admin: AccountId, _registry: AccountId, _whitelist: AccountId) -> Self {
            Self {
                admin: _admin,
                registry: _registry,
                whitelist: _whitelist,
                signers: Vec::new(),
                threshold: 5,
                creation_time: Self::env().block_timestamp(),
                used_nonces: Mapping::new(),
            }
        }
        fn recover_signer(&self, message_hash: &[u8; 32], signature: [u8; 65]) -> AccountId {
            let mut pub_key = [0; 33];
            ink::env::ecdsa_recover(&signature, &message_hash, &mut pub_key)
                .unwrap_or_else(|err| panic!("recover failed: {err:?}"));
            let mut signature_account_id = [0; 32];
            <ink::env::hash::Blake2x256 as ink::env::hash::CryptoHash>::hash(
                &pub_key,
                &mut signature_account_id,
            );

            signature_account_id.into()
        }

        #[ink(message, selector = 1)]
        pub fn add_signer(&mut self, _signer: AccountId) -> Result<(), MultiSigError> {
            let caller = Self::env().caller();
            if caller != self.admin {
                return Err(MultiSigError::Unauthorized);
            }
            self.signers.push(_signer);
            Self::emit_event(
                Self::env(),
                Event::SignerAdded(SignerAdded { signer: _signer }),
            );
            Ok(())
        }
        #[ink(message, selector = 2)]
        pub fn remove_signer(&mut self, _signer: AccountId) -> Result<(), MultiSigError> {
            let caller = Self::env().caller();
            if caller != self.admin {
                return Err(MultiSigError::Unauthorized);
            }
            if let Some(index) = self.signers.iter().position(|a| *a == _signer) {
                self.signers.remove(index);
                Self::emit_event(
                    Self::env(),
                    Event::SignerRemoved(SignerRemoved { signer: _signer }),
                );
            } else {
                return Err(MultiSigError::SignerNotFound);
            }
            Ok(())
        }

        #[ink(message, selector = 3)]
        pub fn update_threshold(&mut self, new_threshold: u16) -> Result<(), MultiSigError> {
            let caller = Self::env().caller();
            if caller != self.admin {
                return Err(MultiSigError::Unauthorized);
            }
            self.threshold = new_threshold;
            Ok(())
        }

        #[ink(message, selector = 4)]
        pub fn replace_signer(
            &mut self,
            signer_old: AccountId,
            signer_new: AccountId,
        ) -> Result<(), MultiSigError> {
            let caller = Self::env().caller();
            if caller != self.admin {
                return Err(MultiSigError::Unauthorized);
            }
            if let Some(index) = self.signers.iter().position(|a| *a == signer_old) {
                self.signers.remove(index);
                self.signers.push(signer_new);
                Self::emit_event(
                    Self::env(),
                    Event::SignerReplaced(SignerReplaced {
                        removed: signer_old,
                        added: signer_new,
                    }),
                );
            } else {
                return Err(MultiSigError::SignerNotFound);
            }
            Ok(())
        }
        #[ink(message)]
        pub fn execute_transaction(
            &mut self,
            action: Action,
            signatures: Vec<[u8; 65]>,
            nonce: String,
        ) -> Result<(), MultiSigError> {
            if ((signatures.len() as u16) < self.threshold)
                || (signatures.len() > self.signers.len())
            {
                return Err(MultiSigError::InvalidInput);
            }
            if self.used_nonces.get(&nonce).unwrap() {
                return Err(MultiSigError::UsedNonce);
            }
            let mut _signers = self.signers.clone();
            let action_hash = self.hash_execution(action.clone(), nonce.clone()).unwrap();
            let count = 0;
            for signature in signatures {
                let _signer = self.recover_signer(&action_hash, signature);
                if !_signers.contains(&_signer) {
                    return Err(MultiSigError::InvalidInput);
                }
                _signers.retain(|&x| x != _signer);
            }
            self.used_nonces.insert(nonce, &true);
            self.execute(action)?; 
            Ok(())
        }
    }
}
