#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod governance {
    use std::env::current_dir;

    use ::vault::Vault;
    use governance_nft::GovernanceNFT;
    use ink::{
        codegen::EmitEvent,
        contract_ref,
        env::{
            debug_println,
            hash::{HashOutput, Sha2x256},
            hash_encoded, Error as InkEnvError,
        },
        prelude::{format, string::String, vec::Vec},
        reflect::ContractEventBase,
        storage::Mapping,
        ToAccountId,
    };
    use psp22::{PSP22Error, PSP22};
    use psp34::{Id, PSP34};

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum GovernanceError {
        RegistryFailure,
        VaultFailure,
        Unauthorized,
        InvalidInput,
        InvalidVoteWeight,
        MaxProposals,
        ExistingProposal,
        NonExistingProposal,
        ProposalInactive,
        DoubleVote,
        TokenError(PSP22Error),
    }
    #[derive(Debug, PartialEq, Eq, scale::Encode, Clone, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum PropType {
        TransferFunds(TokenTransfer),
        AddCouncilMember(AccountId),
        RemoveCouncilMember(AccountId),
        ThresholdChange(u16),
        FeeChange(u16),
    }
    #[derive(Debug, PartialEq, Eq, scale::Encode, Clone, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum ProposalState {
        Created,
        Active,
        Expired,
    }
    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TokenTransfer {
        token: AccountId,
        amount: u128,
        to: AccountId,
    }
    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Proposal {
        pub creation_timestamp: u64,
        pub creator_id: u128,
        pub prop_id: String,
        pub prop_type: PropType,
        pub pro_vote_count: u128,
        pub con_vote_count: u128,
        pub vote_start: u64,
        pub vote_end: u64,
    }
    #[ink(storage)]
    pub struct Governance {
        pub gov_nft: AccountId,
        pub vault: AccountId,
        pub execution_threshold: u128,
        pub rejection_threshold: u128,  // threshold of votes to pass
        pub acceptance_threshold: u128, //
        pub creation_time: u64,
        pub voting_delay: u64,
        pub voting_period: u64,
        pub proposals: Vec<Proposal>,
        pub last_proposal: Mapping<u128, u64>,
        pub voted: Mapping<(String, u128),bool>,
    }
    pub const DAY: u64 = 86400 * 1000;
    pub const MIN_VOTING_DELAY: u64 = 1 * DAY;
    pub const MAX_VOTING_DELAY: u64 = 7 * DAY;
    type Event = <Governance as ContractEventBase>::Type;
    #[ink(event)]
    pub struct ProposlCreated {
        id: u128,
    }
    #[ink(event)]
    pub struct ProposlRejected {
        id: u128,
    }
    #[ink(event)]
    pub struct ProposlExecuted {
        id: u128,
    }
    #[ink(event)]
    pub struct ProposalsExpired {
        proposals: Vec<Proposal>,
    }
    impl Governance {
        fn emit_event<EE>(emitter: EE, event: Event)
        where
            EE: EmitEvent<Governance>,
        {
            emitter.emit_event(event);
        }

        fn check_ownership(&self, id: u128, user: AccountId) -> bool {
            let mut nft: contract_ref!(PSP34) = self.gov_nft.into();
            let owner = nft.owner_of(psp34::Id::U128((id))).unwrap();
            owner == user
        }
        fn query_weight(&self, id: u128) -> u128 {
            let mut nft: contract_ref!(GovernanceNFT) = self.gov_nft.into();
            let data = nft.get_governance_data(id);
            data.vote_weight
        }
        fn get_proposal_state(&self, prop: Proposal, current_time: u64) -> ProposalState {
            match current_time {
                current_time if current_time < prop.vote_start => ProposalState::Created,
                current_time if current_time > prop.vote_start && current_time < prop.vote_end => {
                    ProposalState::Active
                }
                _ => ProposalState::Expired,
            }
        }
        fn update_vault_fees(&self, new_fee: u16) -> Result<(), GovernanceError> {
            let mut vault: contract_ref!(Vault) = self.vault.into();
            if let Err(e) = vault.adjust_fee(new_fee) {
                return Err(GovernanceError::VaultFailure);
            }
            Ok(())
        }
        fn remove_expired_proposals(&mut self, current_time: u64) -> Vec<Proposal> {
            let (active, expired) = self
                .proposals
                .clone()
                .into_iter()
                .partition(|p| p.vote_end <= current_time);
            self.proposals = active;
            expired
        }
        fn handle_pro_vote(&mut self, index: usize, weight: u128) ->  Result<(), GovernanceError> {
            if self.proposals[index].pro_vote_count + weight >= self.execution_threshold {
                match &self.proposals[index].prop_type {
                    PropType::TransferFunds(TokenTransfer) => self.transfer_psp22_from(
                        TokenTransfer.token,
                        &Self::env().account_id(),
                        &TokenTransfer.to,
                        TokenTransfer.amount,
                    )?,
                    _ => (),
                };
                self.proposals.swap_remove(index);
            } else {
                self.proposals[index].pro_vote_count += weight;
            }
            Ok(())
        }
        fn handle_con_vote(&mut self, index: usize, weight: u128) -> Result<(), GovernanceError>{
            if self.proposals[index].pro_vote_count + weight > self.rejection_threshold {
                self.proposals.swap_remove(index);
            } else {
                self.proposals[index].con_vote_count += weight;
            }
            Ok(())
        }
        fn transfer_psp22_from(
            &self,
            contract: AccountId,
            from: &AccountId,
            to: &AccountId,
            amount: Balance,
        ) -> Result<(), GovernanceError> {
            let mut token: contract_ref!(PSP22) = contract.into();
            if let Err(e) = token.transfer_from(*from, *to, amount, Vec::new()) {
                return Err(GovernanceError::TokenError(e));
            }
            Ok(())
        }
        #[ink(constructor)]
        pub fn new(
            vault: AccountId,
            _gov_nft: AccountId,
            exec_threshold: u128,
            reject_threshold: u128,
            acc_threshold: u128,
            prop_delay: u64,
            voting_period: u64,
        ) -> Self {
            Self {
                gov_nft: _gov_nft,
                vault: vault,
                execution_threshold: exec_threshold,
                rejection_threshold: reject_threshold,
                acceptance_threshold: acc_threshold,
                creation_time: Self::env().block_timestamp(),
                voting_delay: 2 * DAY,
                voting_period: 7 * DAY,
                proposals: Vec::new(),
                last_proposal: Mapping::new(),
                voted: Mapping::new(),
            }
        }
        #[ink(message)]
        pub fn create_proposal(
            &mut self,
            prop: PropType,
            nft_id: u128,
        ) -> Result<(), GovernanceError> {
            let current_time = Self::env().block_timestamp();
            self.remove_expired_proposals(current_time);
            if self.check_ownership(nft_id, Self::env().caller()) != true {
                return Err(GovernanceError::Unauthorized);
            }
            if self.query_weight(nft_id) < self.acceptance_threshold {
                return Err(GovernanceError::InvalidVoteWeight);
            }
            if self.proposals.len() == 100 {
                return Err(GovernanceError::MaxProposals);
            }
            if self
                .proposals
                .clone()
                .into_iter()
                .filter(|p| p.creator_id == nft_id)
                .collect::<Vec<Proposal>>()
                .len()
                > 0
            {
                return Err(GovernanceError::ExistingProposal);
            }
            let encodable = (Self::env().block_timestamp(), nft_id);
            let mut output = <Sha2x256 as HashOutput>::Type::default();
            hash_encoded::<Sha2x256, _>(&encodable, &mut output);
            let key_string = String::from_utf8(output.to_vec()).unwrap();
            self.proposals.push(Proposal {
                creation_timestamp: Self::env().block_timestamp(),
                creator_id: nft_id,
                prop_type: prop,
                prop_id: key_string,
                pro_vote_count: 0u128,
                con_vote_count: 0u128,
                vote_start: Self::env().block_timestamp() + self.voting_delay,
                vote_end: Self::env().block_timestamp() + self.voting_delay + self.voting_period,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn vote(
            &mut self,
            prop_id: String,
            nft_id: u128,
            pro: bool,
        ) -> Result<(), GovernanceError> {
            let current_time = Self::env().block_timestamp();
            if self.check_ownership(nft_id, Self::env().caller()) != true {
                return Err(GovernanceError::Unauthorized);
            }
            let weight = self.query_weight(nft_id);
            let index = self
                .proposals
                .clone()
                .into_iter()
                .position(|p| p.prop_id == prop_id)
                .unwrap();
            let mut proposal = self.proposals[index].clone();

            if self.get_proposal_state(proposal, current_time) != ProposalState::Active {
                return Err(GovernanceError::ProposalInactive);
            }
            if self.voted.get((prop_id.clone(),nft_id)).unwrap() {
                return Err(GovernanceError::DoubleVote);
            }
            self.voted.insert((prop_id,nft_id),&true);
            match pro {
                true => self.handle_pro_vote(index,weight)?,
                false => self.handle_con_vote(index,weight)?,
            };

            Ok(())
        }
    }
}
