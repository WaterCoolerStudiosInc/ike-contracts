#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub mod traits;
pub use governance_council::CouncilRef;
pub use traits::ICouncil;

#[ink::contract]
mod governance_council {
    use super::ICouncil;
    use core::fmt::Error;
    use ink::{
        codegen::EmitEvent,
        contract_ref,
        env::{
            debug_println,
            hash::{HashOutput, Sha2x256},
            hash_encoded,
        },
        prelude::{string::String, string::ToString, vec, vec::Vec},
        reflect::ContractEventBase,
        storage::Mapping,
    };

    use governance_staking::traits::Staking;

    use registry::traits::IRegistry;

    #[ink(storage)]
    pub struct Council {
        pub admin: AccountId,
        pub gov_staking: AccountId,
        pub registry: AccountId,
        pub signers: Vec<AccountId>,
        pub threshold: u16,
        pub creation_time: u64,
        pub used_nonces: Mapping<u128, bool>,
        pub proposals: Mapping<[u8; 32], Proposal>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum CouncilError {
        SignerNotFound,
        SignerAlreadyExists,
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
    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Proposal {
        action: Action,
        proposers: Vec<AccountId>,
    }
    #[derive(Debug, PartialEq, Eq, scale::Encode, Clone, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Action {
        RemoveValidator(AccountId, bool),
        CompleteRemoveValidator(AccountId),
    }
    #[ink(event)]
    pub struct ProposalCreated {
        proposal: Proposal,
    }
    #[ink(event)]
    pub struct ProposalUpdated {
        proposal: Proposal,
    }
    #[ink(event)]
    pub struct ProposalExecuted {
        proposal: Proposal,
    }
    type Event = <Council as ContractEventBase>::Type;
    // internal calls
    impl Council {
        fn emit_event<EE>(emitter: EE, event: Event)
        where
            EE: EmitEvent<Council>,
        {
            emitter.emit_event(event);
        }

        fn hash_remove(&self, validator: AccountId, slash: bool) -> [u8; 32] {
            let encodable = (validator, slash);
            let mut output = <Sha2x256 as HashOutput>::Type::default();
            hash_encoded::<Sha2x256, _>(&encodable, &mut output);
            output
        }
        fn hash_complete(&self, validator: AccountId) -> [u8; 32] {
            let encodable = (validator);
            let mut output = <Sha2x256 as HashOutput>::Type::default();
            hash_encoded::<Sha2x256, _>(&encodable, &mut output);
            output
        }
        fn hash_execution(&self, tx: Action) -> Result<[u8; 32], Error> {
            match tx {
                Action::RemoveValidator(validator, slash) => {
                    Ok(self.hash_remove(validator, slash))
                }
                Action::CompleteRemoveValidator(validator) => {
                    Ok(self.hash_complete(validator))
                }
            }
        }

        fn execute_disable(&self, validator: AccountId, slash: bool) -> Result<(), CouncilError> {
            let mut gov_staking: contract_ref!(Staking) = self.gov_staking.into();
            if let Err(_) = gov_staking.disable_validator(validator, slash) {
                return Err(CouncilError::VaultFailure);
            }
            Ok(())
        }
        fn complete_removal(&self, validator: AccountId) -> Result<(), CouncilError> {
            let mut registry: contract_ref!(IRegistry) = self.registry.into();
            if let Err(_) = registry.remove_agent(validator) {
                return Err(CouncilError::VaultFailure);
            }
            Ok(())
        }
        fn execute(&self, tx: Action) -> Result<(), CouncilError> {
            match tx {
                Action::RemoveValidator(validator, slash) => self.execute_disable(validator, slash),
                Action::CompleteRemoveValidator(validator) => self.complete_removal(validator),
            }
        }
    
        fn is_signer(&self, acc: &AccountId) -> bool {
            self.signers.iter().any(|a| a == acc)
        }
    }
    impl Council {
        #[ink(constructor)]
        pub fn new(
            _admin: AccountId,
            _registry: AccountId,
            gov_staking: AccountId,
            initial_signers: Vec<AccountId>,
        ) -> Self {
            Self {
                admin: _admin,
                registry: _registry,
                gov_staking,
                signers: initial_signers,
                threshold: 3,
                creation_time: Self::env().block_timestamp(),
                used_nonces: Mapping::new(),
                proposals: Mapping::new(),
            }
        }
    }
    impl ICouncil for Council {
        #[ink(message, selector = 1)]
        fn add_signer(&mut self, _signer: AccountId) -> Result<(), CouncilError> {
            let caller = Self::env().caller();
            if caller != self.admin {
                return Err(CouncilError::Unauthorized);
            }
            if self.is_signer(&_signer) {
                return Err(CouncilError::SignerAlreadyExists)
            }
            self.signers.push(_signer);
            Self::emit_event(
                Self::env(),
                Event::SignerAdded(SignerAdded { signer: _signer }),
            );
            Ok(())
        }
        #[ink(message, selector = 2)]
        fn remove_signer(&mut self, _signer: AccountId) -> Result<(), CouncilError> {
            let caller = Self::env().caller();
            if caller != self.admin {
                return Err(CouncilError::Unauthorized);
            }
            if let Some(index) = self.signers.iter().position(|a| *a == _signer) {
                self.signers.remove(index);
                Self::emit_event(
                    Self::env(),
                    Event::SignerRemoved(SignerRemoved { signer: _signer }),
                );
            } else {
                return Err(CouncilError::SignerNotFound);
            }
            Ok(())
        }

        #[ink(message, selector = 3)]
        fn update_threshold(&mut self, new_threshold: u16) -> Result<(), CouncilError> {
            let caller = Self::env().caller();
            if caller != self.admin {
                return Err(CouncilError::Unauthorized);
            }
            self.threshold = new_threshold;
            Ok(())
        }

        #[ink(message, selector = 4)]
        fn replace_signer(
            &mut self,
            signer_old: AccountId,
            signer_new: AccountId,
        ) -> Result<(), CouncilError> {
            let caller = Self::env().caller();
            if caller != self.admin {
                return Err(CouncilError::Unauthorized);
            }
            if self.is_signer(&signer_new) {
                return Err(CouncilError::SignerAlreadyExists);
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
                return Err(CouncilError::SignerNotFound);
            }
            Ok(())
        }

        #[ink(message, selector = 7)]
        fn endorse_proposal(&mut self, action: Action) -> Result<(), CouncilError> {
            let hash: [u8; 32] = self
                .hash_execution(action.clone())
                .unwrap();
            let caller = Self::env().caller();
            let existing = self.proposals.get(hash);
            let signers = self.signers.clone();

            if !signers.contains(&caller) {
                return Err(CouncilError::Unauthorized);
            }
            if let Some(mut existing) = existing {
                let mut curr_proposers = existing.proposers.clone();

                if curr_proposers.contains(&caller) {
                    return Err(CouncilError::Unauthorized);
                }
                // remove booted signers from the proposers
                curr_proposers.retain(|&x| signers.contains(&x));

                if curr_proposers.len() as u16 + 1_u16 == self.threshold {
                    debug_println!("{}", "executing");
                    Self::emit_event(
                        Self::env(),
                        Event::ProposalExecuted(ProposalExecuted {
                            proposal: existing.clone(),
                        }),
                    );
                    self.proposals.remove(hash);
                    self.execute(existing.action)?;
                } else {
                    curr_proposers.push(caller);
                    existing.proposers = curr_proposers;
                    Self::emit_event(
                        Self::env(),
                        Event::ProposalUpdated(ProposalUpdated {
                            proposal: existing.clone(),
                        }),
                    );
                    self.proposals.insert(hash, &existing);
                }
            } else {
                debug_println!("{}", "add new proposal");

                self.proposals.insert(
                    hash,
                    &Proposal {
                        action: action.clone(),
                        proposers: vec![caller],
                    },
                );
                Self::emit_event(
                    Self::env(),
                    Event::ProposalCreated(ProposalCreated {
                        proposal: Proposal {
                            action,
                            proposers: vec![caller],
                        },
                    }),
                );
            }
            Ok(())
        }

        #[ink(message, selector = 8)]
        fn get_signers(&self) -> Vec<AccountId> {
            self.signers.clone()
        }
        #[ink(message, selector = 9)]
        fn set_gov_staking(&mut self, new_account: AccountId) -> Result<(), CouncilError> {
            let caller = Self::env().caller();
            if caller != self.admin {
                return Err(CouncilError::Unauthorized);
            }
            self.gov_staking = new_account;
            Ok(())
        }
    }
}
