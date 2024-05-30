#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod governor {
    use ink::{
        codegen::EmitEvent, contract_ref, env::{debug_println, Error as InkEnvError}, prelude::{format, string::String, vec::Vec}, reflect::ContractEventBase, storage::Mapping, ToAccountId
    };
    use::registry::{Registry,registry::RegistryError};
    use::vault::{Vault};
    use psp22::{ PSP22};
    #[ink(storage)]
    pub struct Governor {
        // List of all nomination agents including their deployment and relative weight
        pub vault: AccountId,
        pub registry: AccountId,
        pub governance_token:AccountId,       
        pub epoch:u64,
        pub creation_time:u64,
        pub thresh_hold_weight:u128,
        pub thresh_hold_fees:u128,
        pub weight_proposals:Vec<WeightProposal>,
        pub fee_proposals:Vec<FeeProposal>
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum GovernorError{
        RegistryFailure,
        VaultFailure,
        Unauthorized,
        InvalidInput
    }
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum PropType{
        FeeUpdate,
        WeightUpdate,
        ValidatorAdd,
        ValidatorRemove
    }
    // Fee update process
    // Users have a vote weight based on token holdings
    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct FeeProposal{        
        new_fee:u16,
        vote_points:u128       
    }
    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
    pub struct WeightProposal{   
        accounts:Vec<AccountId>,     
        weights:Vec<u64>,        
        vote_points:u128
    }
    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
    pub struct WeightUpdate{
        accounts:Vec<AccountId>,     
        weights:Vec<u64>,
    }
    type Event = <Governor as ContractEventBase>::Type;
    #[ink(event)]
    pub struct Update {
        id: Balance,        
    }
    pub const DAY: u64 = 86400 * 1000;
    // internal calls
    impl Governor{
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
            if let Err(e) = registry.update_agents(accounts,new_weights) {
                return Err(GovernorError::RegistryFailure);
            }
            Ok(())
        }
        fn add_registry_agent(&self,account: AccountId,
            new_weight: u64)-> Result<(), GovernorError> {
            let mut registry: contract_ref!(Registry) = self.registry.into();
            if let Err(e) = registry.add_agent(account,new_weight) {
                return Err(GovernorError::RegistryFailure);
            }
            Ok(())
            
        }
        fn remove_agent(&self,account: AccountId,
            )-> Result<(), GovernorError> {
            let mut registry: contract_ref!(Registry) = self.registry.into();
            if let Err(e) = registry.remove_agent(account) {
                return Err(GovernorError::RegistryFailure);
            }
            Ok(())
            
        }
        fn update_vault_fees(&self,new_fee:u16)-> Result<(), GovernorError> {
            let mut vault: contract_ref!(Vault) = self.vault.into();
            if let Err(e) = vault.adjust_fee(new_fee) {
                return Err(GovernorError::VaultFailure);
            }
            Ok(())
        }
        fn get_curr_epoch(&self,current_time: Timestamp)->u64{
            (current_time-self.creation_time)/self.epoch
        }
        fn get_vote_weight(&self,epoch:u64,user:AccountId)->u128{
            let mut token: contract_ref!(PSP22) = self.governance_token.into();
            token.balance_of(user)
            
        }
        fn get_current_prop(&self,epoch:u64)->u16{
            1_u16
        }
        fn sort_weight_proposals(&self)->Vec<WeightProposal>{
            let mut props=self.weight_proposals.clone();
            props.sort_by(|a, b| b.vote_points.cmp(&a.vote_points));
            props
        }
        fn sort_fee_proposals(&self)->Vec<FeeProposal>{
            let mut props=self.fee_proposals.clone();
            props.sort_by(|a, b| b.vote_points.cmp(&a.vote_points));
            props
        }
        fn validate_weight_update(&self,update_request:Vec<u64>)->bool{
            let mut registry: contract_ref!(Registry) = self.registry.into();
            let registry_weights=registry.get_agents().unwrap();
            if(update_request.len()==registry_weights.1.len()){
                true
            }else{
                false
            }

        }
        fn add_weight_proposals(&mut self,updates:Vec<WeightUpdate>){
            for w in updates{
                let proposal= WeightProposal{
                    accounts:w.accounts,
                    weights:w.weights,
                    vote_points:0
                };
                self.weight_proposals.push(proposal);
            }
        }
        fn add_fee_proposals(&mut self,updates:Vec<u16>){
            for w in updates{
                let proposal= FeeProposal{
                    new_fee:w,
                    vote_points:0
                };
                self.fee_proposals.push(proposal);
            }
        }
        
    }
    impl Governor {
        #[ink(constructor)]
        pub fn new(
            _vault: AccountId,
            _registry: AccountId,  
             token:AccountId,           
            _weight_threshold:u128,
            _fee_threshold:u128,
            _fees:Vec<u16>,
            _weights:Vec<WeightUpdate>        
        ) -> Self {
            let mut fee_proposal:Vec<FeeProposal>=Vec::new();
            let mut weight_proposals:Vec<WeightProposal>=Vec::new();
            for w in _weights{
                let proposal= WeightProposal{
                    accounts:w.accounts,
                    weights:w.weights,
                    vote_points:0
                };
                weight_proposals.push(proposal);
            }
            for f in _fees{
                let proposal= FeeProposal{
                    new_fee:f,
                    vote_points:0
                };
                fee_proposal.push(proposal);
            }
            Self {
                vault:_vault,
                registry:_registry,
                governance_token:token,                
                epoch:DAY*3,
                creation_time:Self::env().block_timestamp(),
                thresh_hold_weight:_weight_threshold,
                thresh_hold_fees:_fee_threshold,
                weight_proposals,
                fee_proposals:fee_proposal
            }
        }

        // prop index input needed to verify user 
        #[ink(message)]
        pub fn proposal_weight_vote(&mut self,prop_index:u16)->Result<(), GovernorError> {
            let now = Self::env().block_timestamp();
            let epoch=self.get_curr_epoch(now);
            let user_weight=self.get_vote_weight(epoch,Self::env().caller());
            let current_prop:u16= self.get_current_prop(epoch);
            
            assert_eq!(prop_index,current_prop);
            let mut proposal= self.weight_proposals[prop_index as usize].clone();
            if proposal.vote_points+user_weight > self.thresh_hold_weight{
                self.update_registry_weights(proposal.accounts,proposal.weights)?;
            }else{
                self.weight_proposals[prop_index as usize].vote_points+=user_weight;
            }
           
            Ok(())
        }
        #[ink(message)]
        pub fn fee_weight_vote(&mut self,prop_index:u16)->Result<(), GovernorError> {
            let now = Self::env().block_timestamp();
            let epoch=self.get_curr_epoch(now);
            let user_weight=self.get_vote_weight(epoch,Self::env().caller());
            let current_prop:u16= self.get_current_prop(epoch);
            assert_eq!(prop_index,current_prop);
            let mut proposal= self.fee_proposals[prop_index as usize].clone();
            if proposal.vote_points+user_weight > self.thresh_hold_weight{
                self.update_vault_fees(proposal.new_fee)?;
            }else{
                self.fee_proposals[prop_index as usize].vote_points+=user_weight;
            }
           
            Ok(())
        }

    }
}
