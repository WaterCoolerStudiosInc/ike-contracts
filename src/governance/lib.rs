#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod governance {
    use ::vault::Vault;
    use ink::{
        codegen::EmitEvent,
        contract_ref,
        env::{debug_println, Error as InkEnvError},
        prelude::{format, string::String, vec::Vec},
        reflect::ContractEventBase,
        storage::Mapping,
        ToAccountId,
    };
    use psp22::PSP22;
    use psp34::PSP34;
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum GovernanceError {
        RegistryFailure,
        VaultFailure,
        Unauthorized,
        InvalidInput,
    }
    #[derive(Debug, PartialEq, Eq, scale::Encode,Clone, scale::Decode)]
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
    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    struct TokenTransfer {
        token: AccountId,
        amount: u128,
        to: AccountId,
    }
    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    struct Proposal {
        creation_timestamp: u64,
        creator_id: u128,
        prop_type: PropType,
        vote_count: u128,
        vote_start: u64,
        vote_end: u64,
    }
    #[ink(storage)]
    pub struct Governance {
        pub gov_nft: AccountId,
        pub proposal_threshold: u128, // threshold of votes to pass
        pub vote_threshold: u16,      //
        pub creation_time: u64,
        pub prop_delay: u64,
        pub voting_period: u64,
        pub proposals: Vec<Proposal>,
        pub prop_timeout: Mapping<u128, u64>,
    }
    pub const DAY: u64 = 86400 * 1000;
    type Event = <Governance as ContractEventBase>::Type;
    #[ink(event)]
    pub struct ProposlCreated {
        id:u128
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
        #[ink(constructor)]
        pub fn new(
            _gov_nft: AccountId,
            prop_threshold: u128,
            _vote_threshold: u16,
            prop_delay: u64,
            voting_period: u64,
        ) -> Self {
            Self {
                gov_nft: _gov_nft,
                proposal_threshold: prop_threshold,
                vote_threshold: _vote_threshold,
                creation_time: Self::env().block_timestamp(),
                prop_delay: 3 * DAY,
                voting_period: 7 * DAY,
                proposals: Vec::new(),
                prop_timeout: Mapping::new(),
            }
        }
        #[ink(message)]
        pub fn create_proposal(
            &mut self,
            prop: PropType,
            nft_id: u128,
        ) -> Result<(), GovernanceError> {
            Ok(())
        }
    }
}
