#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub mod traits;

pub type Time = u64;
pub type PropId = u128;
pub type NftId = u128;
pub type Weight = u128;

#[ink::contract]
pub mod governance {
    use super::*;
    use ink::{
        codegen::{EmitEvent, StaticEnv},
        contract_ref,
        env::{debug_println, Error as InkEnvError},
        prelude::{format, string::String, vec::Vec},
        reflect::ContractEventBase,
        storage::Mapping,
        ToAccountId,
    };

    use governance_council::traits::ICouncil as Council;
    use governance_council::CouncilRef;
    use governance_nft::traits::IGovernanceNFT as GovernanceNFT;
    use governance_nft::GovernanceNFTRef;
    use governance_staking::{Staking, StakingRef};
    use psp22::{PSP22Error, PSP22};
    use psp34::PSP34;
    use traits::IGovernance;
    use vault::traits::IVault;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum GovernanceError {
        RegistryFailure,
        CouncilError,
        VaultFailure,
        Unauthorized,
        InvalidInput,
        InvalidVotePeriodUpdate,
        InvalidVoteWeight,
        MaxProposals,
        ExistingProposal,
        NonExistingProposal,
        ProposalVotingInactive,
        ProposalActive,
        ProposalNotExecutable,
        DoubleVote,
        TransferError,
        NFTError,
        StakingError,
        TokenError(PSP22Error),
        InkEnvError(String),
        TransferLockError,
        TransferAlreadyUnlocked,
        TransferAlreadyLocked,
    }

    impl From<InkEnvError> for GovernanceError {
        fn from(e: InkEnvError) -> Self {
            GovernanceError::InkEnvError(format!("{:?}", e))
        }
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, Clone, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum PropType {
        // Transfer Azero from governance
        TransferFunds(AccountId, Balance, AccountId),
        // Transfer psp22 token from governance
        NativeTokenTransfer(AccountId, Balance),
        // update tokens per millisecond for staker in staking contract
        ChangeStakingRewardRate(u128),
        // Update the bond requirement [Ike-deposit, A0-deposit] to become a validator
        UpdateValidatorStakeRequirement(Option<Balance>, Option<Balance>),
        // Update the threshold stake required to become a self-delegator
        UpdateRespresentativeStakeThreshold(Balance),
        // Update the delegation fees in staking contract
        UpdateDelegationFees(u128),
        // Onboard a validator [validator, agent_admin]
        AddValidator(AccountId, AccountId),
        // Add to council
        AddCouncilMember(AccountId),
        // remove then add to council
        ReplaceCouncilMember(AccountId, AccountId),
        // remove from council
        RemoveCouncilMember(AccountId),
        // change threshold for council acceptance
        ChangeCouncilThreshold(u16),
        // change vault fee
        FeeChange(u16),
        // change  governance proposal acceptance weight requirement
        AcceptanceWeightUpdate(Weight),
        // change vote period delay
        VoteDelayUpdate(Time),
        // update voting period
        VotePeriodUpdate(Time),
        // update threshold proposals
        UpdateRejectThreshhold(Weight),
        // update execution threshold for proposals
        UpdateExecThreshhold(Weight),
        // update governance code logic
        SetCodeHash([u8; 32]),
        // Unlock Transfer for governance nft
        UnlockTransfer(),
        // Lock Transfer for governance nft
        LockTransfer(),
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, Clone, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum ProposalState {
        Created,
        Active,
        Executable,
        Expired,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, Clone, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Vote {
        Pro,
        Con,
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TokenTransfer {
        token: AccountId,
        amount: Balance,
        to: AccountId,
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Proposal {
        pub creation_timestamp: Time,
        pub creator_id: NftId,
        pub prop_id: PropId,
        pub prop_type: PropType,
        pub pro_vote_count: Weight,
        pub con_vote_count: Weight,
        pub vote_start: Time,
        pub vote_end: Time,
    }

    #[ink(storage)]
    pub struct Governance {
        pub admin: Option<AccountId>,
        pub gov_nft: AccountId,
        pub vault: AccountId,
        pub staking: AccountId,
        pub council: AccountId,
        pub execution_threshold: Weight,
        pub rejection_threshold: Weight,
        pub acceptance_threshold: Weight,
        pub voting_delay: Time,
        pub voting_period: Time,
        pub proposals: Vec<Proposal>,
        pub voted: Mapping<(PropId, NftId), ()>,
        pub validator_whitelist: Mapping<(AccountId, AccountId), ()>, // key: [validator, agent_admin]
        pub prop_nonce: PropId,
    }

    pub const DAY: Time = 86400 * 1000;
    pub const MIN_VOTING_DELAY: Time = 1 * DAY;
    pub const MAX_VOTING_DELAY: Time = 7 * DAY;
    pub const MIN_VOTING_PERIOD: Time = 5 * DAY;
    pub const MAX_VOTING_PERIOD: Time = 14 * DAY;

    type Event = <Governance as ContractEventBase>::Type;

    #[ink(event)]
    pub struct ProposalCreated {
        proposal: Proposal,
    }

    #[ink(event)]
    pub struct ProposalCancelled {
        id: PropId,
    }

    #[ink(event)]
    pub struct VoteSubmitted {
        proposal_id: PropId,
        nft_id: NftId,
        pro_vote: Vote,
    }

    #[ink(event)]
    pub struct ProposalRejected {
        proposal: Proposal,
    }

    #[ink(event)]
    pub struct ProposalExecuted {
        proposal: Proposal,
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

        fn validate_vote_delay_update(&self, update: Time) -> bool {
            update > MIN_VOTING_DELAY && update < MAX_VOTING_DELAY
        }

        fn validate_vote_period_update(&self, update: Time) -> bool {
            update > MIN_VOTING_PERIOD && update < MAX_VOTING_PERIOD
        }

        fn check_ownership(&self, id: NftId, user: AccountId) -> bool {
            let nft: contract_ref!(PSP34) = self.gov_nft.into();
            nft.owner_of(psp34::Id::U128(id)) == Some(user)
        }

        fn query_vote_weight(&self, id: NftId) -> Weight {
            let nft: contract_ref!(GovernanceNFT) = self.gov_nft.into();
            nft.get_governance_data(id).unwrap().vote_weight
        }

        fn get_proposal_state(&self, prop: &Proposal, current_time: Time) -> ProposalState {
            debug_println!("{}{}", prop.vote_end, "vote end");
            debug_println!("{}{}", prop.vote_start, "vote start");
            debug_println!("{}{}", current_time, "current time");

            if current_time < prop.vote_start {
                ProposalState::Created
            } else if current_time < prop.vote_end {
                ProposalState::Active
            } else if self.is_executable(prop.pro_vote_count, prop.con_vote_count) {
                ProposalState::Executable
            } else {
                ProposalState::Expired
            }
        }

        fn is_executable(&self, pro_votes: Weight, con_votes: Weight) -> bool {
            ((pro_votes + con_votes) >= self.execution_threshold) && (pro_votes > con_votes)
        }

        // fn generate_proposal_id(&self, time_stamp: u64, creator_id: u128) -> String {
        //     let encodable = (time_stamp, creator_id);
        //     let mut output = <Sha2x256 as HashOutput>::Type::default();
        //     hash_encoded::<Sha2x256, _>(&encodable, &mut output);
        //     String::from_utf8(output.to_vec()).unwrap()
        // }

        fn update_vault_fee(&self, new_fee: &u16) -> Result<(), GovernanceError> {
            let mut vault: contract_ref!(IVault) = self.vault.into();
            vault
                .adjust_fee(*new_fee)
                .map_err(|_| GovernanceError::VaultFailure)
        }

        fn remove_expired_proposals(&mut self, current_time: Time) -> Vec<Proposal> {
            debug_println!("{}", current_time);
            let (expired, active) =
                self.proposals.clone().into_iter().partition(|p| {
                    self.get_proposal_state(p, current_time) == ProposalState::Expired
                });
            debug_println!("{}{:?}", "removed proposal", expired);
            debug_println!("{}{:?}", "active proposal", active);
            self.proposals = active;

            if !expired.is_empty() {
                Self::emit_event(
                    Self::env(),
                    Event::ProposalsExpired(ProposalsExpired {
                        proposals: expired.clone(),
                    }),
                );
            }

            expired
        }

        fn remove_council_member(&self, member: &AccountId) -> Result<(), GovernanceError> {
            let mut council: contract_ref!(Council) = self.council.into();
            council
                .remove_signer(*member)
                .map_err(|_| GovernanceError::CouncilError)
        }

        fn add_council_member(&self, member: &AccountId) -> Result<(), GovernanceError> {
            let mut council: contract_ref!(Council) = self.council.into();
            council
                .add_signer(*member)
                .map_err(|_| GovernanceError::CouncilError)
        }

        fn change_council_threshold(&self, update: u16) -> Result<(), GovernanceError> {
            let mut council: contract_ref!(Council) = self.council.into();
            council
                .update_threshold(update)
                .map_err(|_| GovernanceError::CouncilError)
        }

        fn replace_council_member(
            &self,
            member: &AccountId,
            new_member: AccountId,
        ) -> Result<(), GovernanceError> {
            let mut council: contract_ref!(Council) = self.council.into();
            council
                .replace_signer(*member, new_member)
                .map_err(|_| GovernanceError::CouncilError)
        }

        fn update_staking_rewards(&self, new_reward: u128) -> Result<(), GovernanceError> {
            let mut staking: contract_ref!(Staking) = self.staking.into();
            staking
                .update_rewards_rate(new_reward)
                .map_err(|_| GovernanceError::StakingError)
        }

        fn update_validator_stake_requirement(
            &self,
            ike: Option<Balance>,
            a0: Option<Balance>,
        ) -> Result<(), GovernanceError> {
            let mut staking: contract_ref!(Staking) = self.staking.into();
            staking
                .update_validator_stake_requirement(ike, a0)
                .map_err(|_| GovernanceError::StakingError)
        }

        fn update_representative_stake_threshold(
            &self,
            amount: Balance,
        ) -> Result<(), GovernanceError> {
            let mut staking: contract_ref!(Staking) = self.staking.into();
            staking
                .update_representative_stake_threshold(amount)
                .map_err(|_| GovernanceError::StakingError)
        }

        fn update_delegation_fees(&self, fees: u128) -> Result<(), GovernanceError> {
            let mut staking: contract_ref!(Staking) = self.staking.into();
            staking
                .update_delegation_fees(fees)
                .map_err(|_| GovernanceError::StakingError)
        }

        fn add_validator(
            &mut self,
            validator: AccountId,
            agent_admin: AccountId,
        ) -> Result<(), GovernanceError> {
            self.validator_whitelist
                .insert((validator, agent_admin), &());
            Ok(())
        }

        fn update_reject_threshold(&mut self, update: Weight) {
            self.rejection_threshold = update;
        }

        fn update_execution_threshold(&mut self, update: Weight) {
            self.execution_threshold = update;
        }

        fn update_acceptance_threshold(&mut self, update: Weight) {
            self.acceptance_threshold = update;
        }

        fn update_voting_period(&mut self, update: Time) {
            self.voting_period = update;
        }

        fn update_voting_delay(&mut self, update: Time) {
            self.voting_delay = update;
        }

        fn transfer_native_funds(
            &self,
            to: AccountId,
            amount: Balance,
        ) -> Result<(), GovernanceError> {
            Self::env()
                .transfer(to, amount)
                .map_err(|_| GovernanceError::TransferError)
        }

        fn set_code_internal(&mut self, code_hash: [u8; 32]) -> Result<(), GovernanceError> {
            ink::env::set_code_hash(&code_hash)?;
            Ok(())
        }

        fn unlock_transfer(&self) -> Result<(), GovernanceError> {
            let mut gov_nft: contract_ref!(GovernanceNFT) = self.gov_nft.into();
            if !gov_nft.is_collection_locked() {
                return Err(GovernanceError::TransferAlreadyUnlocked);
            }
            gov_nft
                .unlock_transfer()
                .map_err(|_| GovernanceError::TransferLockError)
        }

        fn lock_transfer(&self) -> Result<(), GovernanceError> {
            let mut gov_nft: contract_ref!(GovernanceNFT) = self.gov_nft.into();
            if gov_nft.is_collection_locked() {
                return Err(GovernanceError::TransferAlreadyLocked);
            }
            gov_nft
                .lock_transfer()
                .map_err(|_| GovernanceError::TransferLockError)
        }

        fn remove_proposal(&mut self, prop_id: PropId) -> Result<(), GovernanceError> {
            let update = self
                .proposals
                .iter()
                .filter(|p| p.prop_id != prop_id)
                .cloned()
                .collect();
            self.proposals = update;

            Ok(())
        }

        fn execute_proposal(&mut self, proposal: Proposal) -> Result<(), GovernanceError> {
            match proposal.prop_type {
                PropType::TransferFunds(token, amount, to) => {
                    self.transfer_psp22_from(token, &Self::env().account_id(), &to, amount)?
                }
                PropType::NativeTokenTransfer(to, funds) => {
                    self.transfer_native_funds(to, funds)?
                }
                PropType::AcceptanceWeightUpdate(update) => {
                    self.update_acceptance_threshold(update)
                }
                PropType::UpdateRejectThreshhold(update) => self.update_reject_threshold(update),
                PropType::UpdateExecThreshhold(update) => self.update_execution_threshold(update),
                PropType::VoteDelayUpdate(update) => {
                    debug_println!("executing delay update {}", update);
                    self.update_voting_delay(update)
                }
                PropType::VotePeriodUpdate(update) => self.update_voting_period(update),
                PropType::AddCouncilMember(member) => self.add_council_member(&member)?,
                PropType::ReplaceCouncilMember(member, replacement) => {
                    self.replace_council_member(&member, replacement)?
                }
                PropType::RemoveCouncilMember(member) => self.remove_council_member(&member)?,
                PropType::ChangeCouncilThreshold(update) => {
                    self.change_council_threshold(update)?
                }
                PropType::FeeChange(new_fee) => self.update_vault_fee(&new_fee)?,
                PropType::ChangeStakingRewardRate(new_rate) => {
                    debug_println!("executing staking update {}", new_rate);
                    self.update_staking_rewards(new_rate)?
                }
                PropType::UpdateValidatorStakeRequirement(ike, a0) => {
                    self.update_validator_stake_requirement(ike, a0)?
                }
                PropType::UpdateRespresentativeStakeThreshold(amount) => {
                    self.update_representative_stake_threshold(amount)?
                }
                PropType::UpdateDelegationFees(fees) => self.update_delegation_fees(fees)?,
                PropType::AddValidator(validator, agent_admin) => {
                    self.add_validator(validator, agent_admin)?
                }
                PropType::SetCodeHash(code_hash) => self.set_code_internal(code_hash)?,
                PropType::UnlockTransfer() => self.unlock_transfer()?,
                PropType::LockTransfer() => self.lock_transfer()?,
            };

            Self::emit_event(
                Self::env(),
                Event::ProposalExecuted(ProposalExecuted { proposal }),
            );
            Ok(())
        }

        fn handle_proposal_rejection(&mut self, index: usize) {
            let proposal = self.proposals[index].clone();
            if proposal.con_vote_count >= self.rejection_threshold {
                debug_println!("removing at index {}", index);
                self.proposals.swap_remove(index);

                Self::emit_event(
                    Self::env(),
                    Event::ProposalRejected(ProposalRejected { proposal }),
                );
            }
        }

        fn transfer_psp22_from(
            &self,
            contract: AccountId,
            from: &AccountId,
            to: &AccountId,
            amount: Balance,
        ) -> Result<(), GovernanceError> {
            let mut token: contract_ref!(PSP22) = contract.into();
            token
                .transfer_from(*from, *to, amount, Vec::new())
                .map_err(GovernanceError::TokenError)
        }
    }

    impl Governance {
        #[ink(constructor)]
        pub fn new(
            vault: AccountId,
            registry: AccountId,
            governance_token: AccountId,
            council_hash: Hash,
            gov_nft_hash: Hash,
            staking_hash: Hash,
            exec_threshold: Weight,
            reject_threshold: Weight,
            acc_threshold: Weight,
            staking_reward_pool: Balance,
            interest_rate: u128,
            signers: Vec<AccountId>,
        ) -> Self {
            let caller = Self::env().caller();
            let governor = Self::env().account_id();

            let mut council_ref = CouncilRef::new(caller, governor, vault, signers)
                .endowment(0)
                .code_hash(council_hash)
                .salt_bytes(&[5_u8.to_le_bytes().as_ref(), caller.as_ref()].concat()[..4])
                .instantiate();

            let mut nft_ref: GovernanceNFTRef = GovernanceNFTRef::new(governor)
                .endowment(0)
                .code_hash(gov_nft_hash)
                .salt_bytes(&[7_u8.to_le_bytes().as_ref(), caller.as_ref()].concat()[..4])
                .instantiate();

            let staking_ref = StakingRef::new(
                caller,
                governance_token,
                registry,
                governor,
                nft_ref.clone(),
                staking_reward_pool,
                interest_rate,
                CouncilRef::to_account_id(&council_ref),
            )
            .endowment(0)
            .code_hash(staking_hash)
            .salt_bytes(&[9_u8.to_le_bytes().as_ref(), caller.as_ref()].concat()[..4])
            .instantiate();

            let staking_address = StakingRef::to_account_id(&staking_ref);

            //if let Err(e) = council.remove_signer(*member) {
            //    return Err(GovernanceError::CouncilError);
            //}

            council_ref.set_gov_staking(staking_address).unwrap();
            nft_ref.set_admin(staking_address).unwrap();

            Self {
                admin: Some(caller),
                gov_nft: GovernanceNFTRef::to_account_id(&nft_ref),
                vault,
                council: CouncilRef::to_account_id(&council_ref),
                staking: staking_address,
                execution_threshold: exec_threshold,
                rejection_threshold: reject_threshold,
                acceptance_threshold: acc_threshold,
                voting_delay: 2 * DAY,
                voting_period: 7 * DAY,
                proposals: Vec::new(),
                voted: Mapping::new(),
                validator_whitelist: Mapping::new(),
                prop_nonce: 1_u128,
            }
        }

        #[ink(message)]
        pub fn set_code_hash(&mut self, code_hash: [u8; 32]) -> Result<(), GovernanceError> {
            self.only_admin()?;
            self.set_code_internal(code_hash)
        }

        #[ink(message)]
        pub fn whitelist_validator(
            &mut self,
            validator: AccountId,
            agent_admin: AccountId,
        ) -> Result<(), GovernanceError> {
            self.only_admin()?;
            self.add_validator(validator, agent_admin)
        }

        #[ink(message)]
        pub fn is_validator_whitelisted(
            &self, 
            validator: AccountId, 
            agent_admin: AccountId
        ) -> bool {
            self.validator_whitelist.contains((validator, agent_admin))
        }

        #[ink(message)]
        pub fn transfer_admin_role(
            &mut self,
            new_admin: Option<AccountId>,
        ) -> Result<(), GovernanceError> {
            self.only_admin()?;
            self.admin = new_admin;
            Ok(())
        }

        fn only_admin(&self) -> Result<(), GovernanceError> {
            match Some(self.env().caller()) == self.admin {
                true => Ok(()),
                false => Err(GovernanceError::Unauthorized),
            }
        }
    }

    impl IGovernance for Governance {
        #[ink(message)]
        fn get_council(&self) -> AccountId {
            self.council
        }

        #[ink(message)]
        fn get_staking(&self) -> AccountId {
            self.staking
        }

        #[ink(message)]
        fn get_voting_delay(&self) -> Time {
            self.voting_delay
        }

        #[ink(message)]
        fn get_voting_period(&self) -> Time {
            self.voting_period
        }

        #[ink(message)]
        fn get_execution_threshold(&self) -> Weight {
            self.execution_threshold
        }

        #[ink(message)]
        fn get_rejection_threshold(&self) -> Weight {
            self.rejection_threshold
        }

        #[ink(message)]
        fn get_acceptance_threshold(&self) -> Weight {
            self.acceptance_threshold
        }

        #[ink(message)]
        fn get_proposal_by_id(&self, id: PropId) -> Option<Proposal> {
            self.proposals.iter().find(|p| p.prop_id == id).cloned()
        }

        #[ink(message)]
        fn get_all_proposals(&self) -> Vec<Proposal> {
            self.proposals.clone()
        }

        #[ink(message)]
        fn get_proposal_by_nft(&self, id: NftId) -> Option<Proposal> {
            self.proposals.iter().find(|p| p.creator_id == id).cloned()
        }

        #[ink(message, selector = 33)]
        fn get_active_proposal_status_by_nft(&self, id: NftId) -> bool {
            let current_time = Self::env().block_timestamp();
            let prop = self.proposals.iter().find(|p| p.creator_id == id);

            match prop {
                None => false,
                Some(prop) => self.get_proposal_state(prop, current_time) != ProposalState::Expired,
            }
        }

        #[ink(message)]
        fn create_proposal(
            &mut self,
            prop: PropType,
            nft_id: NftId,
        ) -> Result<(), GovernanceError> {
            let current_time = Self::env().block_timestamp();

            // clean expired proposals
            self.remove_expired_proposals(current_time);

            if !self.check_ownership(nft_id, Self::env().caller()) {
                return Err(GovernanceError::Unauthorized);
            }
            if self.query_vote_weight(nft_id) < self.acceptance_threshold {
                return Err(GovernanceError::InvalidVoteWeight);
            }
            if self.proposals.len() == 100 {
                return Err(GovernanceError::MaxProposals);
            }

            let vote_update_check = match prop {
                PropType::VoteDelayUpdate(update) => self.validate_vote_delay_update(update),
                PropType::VotePeriodUpdate(update) => self.validate_vote_period_update(update),
                _ => true,
            };
            if !vote_update_check {
                return Err(GovernanceError::InvalidVotePeriodUpdate);
            }

            if self.proposals.iter().any(|p| p.creator_id == nft_id) {
                debug_println!("found a duplicate");
                return Err(GovernanceError::ExistingProposal);
            }

            // Generate Unique ID for proposals
            let prop_id = self.prop_nonce;
            self.prop_nonce += 1;

            let new_prop = Proposal {
                creation_timestamp: current_time,
                creator_id: nft_id,
                prop_type: prop,
                prop_id,
                pro_vote_count: 0u128,
                con_vote_count: 0u128,
                vote_start: current_time + self.voting_delay,
                vote_end: current_time + self.voting_delay + self.voting_period,
            };
            self.proposals.push(new_prop.clone());

            debug_println!("{:?}{}", self.proposals.to_vec(), "Props");
            Self::emit_event(
                Self::env(),
                Event::ProposalCreated(ProposalCreated { proposal: new_prop }),
            );

            Ok(())
        }

        #[ink(message)]
        fn vote(
            &mut self,
            prop_id: PropId,
            nft_id: NftId,
            pro: Vote,
        ) -> Result<(), GovernanceError> {
            let current_time = Self::env().block_timestamp();
            if !self.check_ownership(nft_id, Self::env().caller()) {
                return Err(GovernanceError::Unauthorized);
            }

            let index = self
                .proposals
                .iter()
                .position(|p| p.prop_id == prop_id)
                .ok_or(GovernanceError::NonExistingProposal)?;
            let proposal = &self.proposals[index];

            if self.get_proposal_state(proposal, current_time) != ProposalState::Active {
                debug_println!(
                    "{:?}{}",
                    self.get_proposal_state(proposal, current_time),
                    "ProposalState"
                );
                return Err(GovernanceError::ProposalVotingInactive);
            }

            if self.voted.contains((prop_id, nft_id)) {
                return Err(GovernanceError::DoubleVote);
            }

            self.voted.insert((prop_id, nft_id), &());
            let weight = self.query_vote_weight(nft_id);
            match pro {
                Vote::Pro => self.proposals[index].pro_vote_count += weight,
                Vote::Con => {
                    self.proposals[index].con_vote_count += weight;
                    self.handle_proposal_rejection(index)
                }
            };

            Self::emit_event(
                Self::env(),
                Event::VoteSubmitted(VoteSubmitted {
                    proposal_id: prop_id,
                    nft_id,
                    pro_vote: pro,
                }),
            );
            Ok(())
        }

        #[ink(message)]
        fn complete_proposal(&mut self, prop_id: PropId) -> Result<(), GovernanceError> {
            let current_time = Self::env().block_timestamp();

            let proposal = self
                .get_proposal_by_id(prop_id)
                .ok_or(GovernanceError::NonExistingProposal)?;

            match self.get_proposal_state(&proposal, current_time) {
                ProposalState::Executable => {
                    self.execute_proposal(proposal)?;
                    self.remove_proposal(prop_id)
                }
                _ => Err(GovernanceError::ProposalNotExecutable),
            }
        }

        #[ink(message)]
        fn cancel_proposal(&mut self, prop_id: PropId) -> Result<(), GovernanceError> {
            let current_time = Self::env().block_timestamp();

            let proposal = self
                .get_proposal_by_id(prop_id)
                .ok_or(GovernanceError::NonExistingProposal)?;

            if !self.check_ownership(proposal.creator_id, Self::env().caller()) {
                return Err(GovernanceError::Unauthorized);
            }

            match self.get_proposal_state(&proposal, current_time) {
                ProposalState::Created => {
                    self.remove_proposal(prop_id)?;
                    Self::emit_event(
                        Self::env(),
                        Event::ProposalCancelled(ProposalCancelled { id: prop_id }),
                    );
                    Ok(())
                }
                ProposalState::Active => Err(GovernanceError::ProposalActive),
                _ => {
                    self.remove_expired_proposals(current_time);
                    Ok(())
                }
            }
        }

        #[ink(message, selector = 100)]
        fn consume_validator_whitelist(
            &mut self,
            validator: AccountId,
            agent_admin: AccountId,
        ) -> Result<(), GovernanceError> {
            let caller = self.env().caller();
            if caller != self.staking {
                return Err(GovernanceError::Unauthorized);
            }

            if !self.validator_whitelist.contains((validator, agent_admin)) {
                return Err(GovernanceError::InvalidInput);
            }

            self.validator_whitelist.remove((validator, agent_admin));
            Ok(())
        }
    }
}
