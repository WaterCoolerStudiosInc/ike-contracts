#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub mod traits;
pub use crate::governance_council::CouncilRef;
pub use traits::ICouncil;

#[ink::contract]
mod governance_council {
    use super::ICouncil;
    use ink::{
        codegen::EmitEvent,
        contract_ref,
        env::{
            debug_println,
            hash::{HashOutput, Sha2x256},
            hash_encoded,
        },
        prelude::{vec, vec::Vec},
        reflect::ContractEventBase,
        storage::Mapping,
    };

    use governance_staking::traits::Staking;

    #[ink(storage)]
    pub struct Council {
        pub admin: Option<AccountId>,
        pub governor: AccountId,
        pub gov_staking: AccountId,
        pub signers: Vec<AccountId>,
        pub threshold: u16,
        pub proposals: Mapping<[u8; 32], Proposal>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum CouncilError {
        SignerNotFound,
        SignerAlreadyExists,
        VaultFailure,
        Unauthorized,
        InvalidInput,
        UsedNonce,
        EnvError,
        StorageOverflow,
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
        threshold: u16,
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
    #[ink(impl)]
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
            let encodable = (validator,);
            let mut output = <Sha2x256 as HashOutput>::Type::default();
            hash_encoded::<Sha2x256, _>(&encodable, &mut output);
            output
        }

        fn hash_execution(&self, tx: &Action) -> [u8; 32] {
            match *tx {
                Action::RemoveValidator(validator, slash) => self.hash_remove(validator, slash),
                Action::CompleteRemoveValidator(validator) => self.hash_complete(validator),
            }
        }

        fn execute_disable(&self, validator: AccountId, slash: bool) -> Result<(), CouncilError> {
            let mut gov_staking: contract_ref!(Staking) = self.gov_staking.into();
            gov_staking
                .disable_validator(validator, slash)
                .map_err(|_| CouncilError::VaultFailure)
        }

        fn complete_removal(&self, validator: AccountId) -> Result<(), CouncilError> {
            let mut gov_staking: contract_ref!(Staking) = self.gov_staking.into();
            gov_staking
                .remove_agent(validator)
                .map_err(|_| CouncilError::VaultFailure)
        }

        fn execute(&self, tx: &Action) -> Result<(), CouncilError> {
            match *tx {
                Action::RemoveValidator(validator, slash) => self.execute_disable(validator, slash),
                Action::CompleteRemoveValidator(validator) => self.complete_removal(validator),
            }
        }

        fn create_new_proposal(
            &mut self,
            hash: [u8; 32],
            creator: AccountId,
            action: &Action,
        ) -> Result<(), CouncilError> {
            debug_println!("{}", "add new proposal");

            let proposal = Proposal {
                action: action.clone(),
                threshold: self.threshold,
                proposers: vec![creator],
            };

            Self::emit_event(
                Self::env(),
                Event::ProposalCreated(ProposalCreated {
                    proposal: Proposal {
                        action: action.clone(),
                        threshold: self.threshold,
                        proposers: vec![creator],
                    },
                }),
            );

            if self.threshold <= 1 {
                self.execute(&proposal.action)?;
                Self::emit_event(
                    Self::env(),
                    Event::ProposalExecuted(ProposalExecuted { proposal }),
                );
            } else {
                self.proposals.insert(hash, &proposal);
            }

            Ok(())
        }

        fn is_signer(&self, acc: &AccountId) -> bool {
            self.signers.contains(acc)
        }

        fn get_signer_index(&self, acc: &AccountId) -> Option<usize> {
            self.signers.iter().position(|a| a == acc)
        }

        fn only_governor(&self) -> Result<(), CouncilError> {
            match self.env().caller() == self.governor {
                true => Ok(()),
                false => Err(CouncilError::Unauthorized),
            }
        }

        fn only_admin(&self) -> Result<(), CouncilError> {
            match Some(self.env().caller()) == self.admin {
                true => Ok(()),
                false => Err(CouncilError::Unauthorized),
            }
        }
    }

    impl Council {
        #[ink(constructor)]
        pub fn new(
            admin: AccountId,
            governor: AccountId,
            gov_staking: AccountId,
            initial_signers: Vec<AccountId>,
        ) -> Self {
            Self {
                admin: Some(admin),
                governor,
                gov_staking,
                signers: initial_signers,
                threshold: 3,
                proposals: Mapping::new(),
            }
        }

        #[ink(message)]
        pub fn set_code_hash(&mut self, code_hash: [u8; 32]) -> Result<(), CouncilError> {
            self.only_admin()?;
            ink::env::set_code_hash(&code_hash).map_err(|_| CouncilError::EnvError)
        }

        #[ink(message)]
        pub fn transfer_admin_role(
            &mut self,
            new_admin: Option<AccountId>,
        ) -> Result<(), CouncilError> {
            self.only_admin()?;
            self.admin = new_admin;
            Ok(())
        }

        #[ink(message)]
        pub fn get_admin(&self) -> Option<AccountId> {
            self.admin
        }

        #[ink(message)]
        pub fn get_governor(&self) -> AccountId {
            self.governor
        }

        #[ink(message)]
        pub fn get_staking(&self) -> AccountId {
            self.gov_staking
        }

        #[ink(message)]
        pub fn get_threshold(&self) -> u16 {
            self.threshold
        }

        #[ink(message)]
        pub fn get_proposal(&self, hash: [u8; 32]) -> Option<Proposal> {
            self.proposals.get(hash)
        }
    }

    impl ICouncil for Council {
        #[ink(message, selector = 1)]
        fn add_signer(&mut self, signer: AccountId) -> Result<(), CouncilError> {
            self.only_governor()?;

            if self.is_signer(&signer) {
                return Err(CouncilError::SignerAlreadyExists);
            }

            self.signers.push(signer);
            Self::emit_event(Self::env(), Event::SignerAdded(SignerAdded { signer }));
            Ok(())
        }

        #[ink(message, selector = 2)]
        fn remove_signer(&mut self, signer: AccountId) -> Result<(), CouncilError> {
            self.only_governor()?;

            match self.get_signer_index(&signer) {
                None => return Err(CouncilError::SignerNotFound),
                Some(index) => {
                    self.signers.remove(index);
                    Self::emit_event(Self::env(), Event::SignerRemoved(SignerRemoved { signer }));
                }
            }

            let len = self.signers.len();
            if len < self.threshold as usize {
                self.threshold = len.try_into().map_err(|_| CouncilError::StorageOverflow)?;
            }

            Ok(())
        }

        #[ink(message, selector = 3)]
        fn update_threshold(&mut self, new_threshold: u16) -> Result<(), CouncilError> {
            self.only_governor().or_else(|_| self.only_admin())?;
            if new_threshold as usize > self.signers.len(){
                return Err(CouncilError::InvalidInput);
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
            self.only_governor()?;

            if self.is_signer(&signer_new) {
                return Err(CouncilError::SignerAlreadyExists);
            }

            match self.get_signer_index(&signer_old) {
                None => Err(CouncilError::SignerNotFound),
                Some(index) => {
                    self.signers[index] = signer_new;
                    Self::emit_event(
                        Self::env(),
                        Event::SignerReplaced(SignerReplaced {
                            removed: signer_old,
                            added: signer_new,
                        }),
                    );
                    Ok(())
                }
            }
        }

        #[ink(message, selector = 7)]
        fn endorse_proposal(&mut self, action: Action) -> Result<(), CouncilError> {
            let hash = self.hash_execution(&action);
            let caller = Self::env().caller();
            let signers = &self.signers;

            if !signers.contains(&caller) {
                return Err(CouncilError::Unauthorized);
            }

            match self.proposals.get(hash) {
                None => self.create_new_proposal(hash, caller, &action)?,
                Some(mut proposal) => {
                    let curr_proposers = &mut proposal.proposers;

                    // remove booted signers from the proposers
                    curr_proposers.retain(|x| signers.contains(x));

                    // Already endorsed
                    if curr_proposers.contains(&caller) {
                        return Err(CouncilError::Unauthorized);
                    }

                    if curr_proposers.len() as u16 + 1_u16 >= proposal.threshold {
                        debug_println!("{}", "executing");
                        self.proposals.remove(hash);
                        self.execute(&proposal.action)?;

                        Self::emit_event(
                            Self::env(),
                            Event::ProposalExecuted(ProposalExecuted { proposal }),
                        );
                    } else {
                        curr_proposers.push(caller);
                        self.proposals.insert(hash, &proposal);

                        Self::emit_event(
                            Self::env(),
                            Event::ProposalUpdated(ProposalUpdated { proposal }),
                        );
                    }
                }
            }

            Ok(())
        }

        #[ink(message, selector = 8)]
        fn get_signers(&self) -> Vec<AccountId> {
            self.signers.clone()
        }

        #[ink(message, selector = 9)]
        fn set_gov_staking(&mut self, new_account: AccountId) -> Result<(), CouncilError> {
            self.only_governor()?;
            self.gov_staking = new_account;
            Ok(())
        }
    }
}
