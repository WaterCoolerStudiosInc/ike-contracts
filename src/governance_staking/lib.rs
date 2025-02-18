#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub mod traits;
pub use crate::staking::StakingRef;
pub use crate::traits::Staking;

#[ink::contract]
pub mod staking {
    use ink::contract_ref;
    use ink::reflect::ContractEventBase;
    use ink::ToAccountId;
    use ink::{
        codegen::EmitEvent,
        env::debug_println,
        env::{
            call::{build_call, ExecutionInput, Selector},
            DefaultEnvironment,
        },
        prelude::vec,
        prelude::vec::Vec,
        storage::Mapping,
    };
    use num_bigint::BigUint;
    use num_traits::cast::ToPrimitive;

    use governance_nft::traits::{GovernanceData, IGovernanceNFT};
    use governance_nft::GovernanceNFTRef;
    use psp22::{PSP22Error, PSP22};
    use psp34::{Id, PSP34Error};
    use registry::traits::IRegistry;

    pub const DAY: u64 = 86400 * 1000;
    pub const WITHDRAW_DELAY: u64 = 14 * DAY;
    pub const MAX_VALIDATORS: u8 = 5;
    pub const BIPS: u128 = 10000000;
    const UPDATE_SELECTOR: Selector = Selector::new([0, 0, 0, 2]);
    const AGENT_SELECTOR: Selector = Selector::new([0, 0, 0, 4]);
    const ADD_SELECTOR: Selector = Selector::new([0, 0, 0, 1]);

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum StakingError {
        InvalidInput,
        Unauthorized,
        InvalidTimeWindow,
        NftLocked,
        NFTError(PSP34Error),
        TokenError(PSP22Error),
        InternalError(RuntimeError),
        InvalidCreateDeposit,
        InvalidStake,
        InvalidPermissions,
        InvalidRepresentative,
        InvalidRequest,
        DuplicateRequest,
        AlreadyOnList,
        RegistryError,
        NoChange,
        NotFound,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum RuntimeError {
        CallRuntimeFailed,
        Unauthorized,
    }

    #[ink(storage)]
    pub struct Staking {
        creation_time: u64,
        governor: AccountId,
        registry: AccountId,
        reward_token_balance: u128,
        staked_token_balance: u128,
        rewards_per_second: u128,
        reward_stake_accumulation: u128,
        accumulated_rewards: u128,
        last_accumulation_update: u64,
        governance_council: AccountId,
        governance_token: AccountId,
        nft: GovernanceNFTRef,
        cast_distribution: Mapping<u128, Vec<(AccountId, u128)>>,
        voting_delegations: Mapping<u128, (u128, u128, u128)>, // (delegatee, amount, nonce)
        redelegate_requests: Mapping<u128, (u64, u128)>,
        voting_delegations_nonce: Mapping<u128, u128>,
        governance_nfts: Mapping<AccountId, Vec<u128>>,
        unstake_requests: Mapping<u128, UnstakeRequest>,
        last_reward_claim: Mapping<u128, u64>,
        deployed_validators: Vec<Validator>,
        token_stake_amount: u128,
        create_deposit: u128,
        existential_deposit: u128,
        treasury: AccountId,
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum CastType {
        Direct(Vec<(AccountId, u128)>),
        Delegate(u128),
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    struct UnstakeRequest {
        pub time: u64,
        pub token_value: u128,
        pub owner: AccountId,
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Agent {
        pub address: AccountId,
        pub weight: u128,
        pub disabled: bool,
    }

    #[ink(event)]
    pub struct TokensWrapped {
        staker: AccountId,
        amount: u128,
        nft: u128,
    }

    #[ink(event)]
    pub struct StakeAdded {
        staker: AccountId,
        amount: u128,
        nft: u128,
    }

    #[ink(event)]
    pub struct StakeRemoved {
        staker: AccountId,
        amount: u128,
        nft: u128,
    }

    #[ink(event)]
    pub struct UnwrapRequestCreated {
        staker: AccountId,
        nft: u128,
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct WeightUpdate {
        pub agent: AccountId,
        pub weight: u128,
        pub increase: bool,
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Validator {
        validator: AccountId,
        agent: AccountId,
        admin: AccountId,
        nft_id: u128,
    }

    type Event = <Staking as ContractEventBase>::Type;

    #[ink(impl)]
    impl Staking {
        pub fn pro_rata(&self, a: u128, b: u128, c: u128) -> u128 {
            let result = BigUint::from(a) * BigUint::from(b) / BigUint::from(c);
            BigUint::to_u128(&result).unwrap()
        }

        pub fn query_nft_proposal_lock(&self, nft_id: u128) -> bool {
            build_call::<DefaultEnvironment>()
                .call(self.governor)
                .exec_input(ExecutionInput::new(Selector::new([0, 0, 0, 33])).push_arg(nft_id))
                .transferred_value(0)
                .returns::<bool>()
                .invoke()
        }

        pub fn update_registry_weights(
            &mut self,
            agents: &[(AccountId, u128)],
            mut value: u128,
            increase: bool,
            safe_check: bool,
        ) -> Result<(), StakingError> {
            let mut sum: u128 = 0;
            let mut update_list = Vec::new();

            if agents.len() > 5 {
                return Err(StakingError::InvalidInput);
            }

            let current_agents = match safe_check {
                true => self.get_agents()?,
                false => vec![],
            };

            for agent in agents.iter() {
                sum += agent.1;

                let amt = self.pro_rata(value, agent.1, BIPS);
                if safe_check && !self.is_disabled(agent.0, &current_agents) {
                    update_list.push(WeightUpdate {
                        agent: agent.0,
                        weight: amt,
                        increase,
                    });
                    value -= amt;
                }
            }
            if sum != BIPS {
                return Err(StakingError::InvalidInput);
            }

            // Add remaining (dust) value to the 1st agent from the `agents` list if it's part of the updated_list
            match update_list.first() {
                Some(WeightUpdate { agent, .. }) if agent == &agents[0].0 => {
                    update_list[0].weight += value
                }
                _ => {}
            };

            if let Err(e) = self.call_registry_update(update_list) {
                return Err(StakingError::InternalError(e));
            }
            Ok(())
        }

        pub fn new_cast_distribution(
            &mut self,
            nft_id: u128,
            value: u128,
            cast: CastType,
        ) -> Result<(), StakingError> {
            let (weights, safe_check) = match cast {
                CastType::Direct(weights) => (weights, false),
                CastType::Delegate(nft) => (
                    self.cast_distribution
                        .get(nft)
                        .ok_or(StakingError::InvalidInput)?,
                    true,
                ),
            };

            self.cast_distribution.insert(nft_id, &weights);
            self.update_registry_weights(&weights, value, true, safe_check)
        }

        pub fn add_cast_distribution(
            &mut self,
            nft_id: u128,
            value: u128,
        ) -> Result<(), StakingError> {
            let cast = self
                .cast_distribution
                .get(nft_id)
                .ok_or(StakingError::InvalidInput)?;
            self.update_registry_weights(&cast, value, true, true)
        }

        pub fn remove_cast_distribution(
            &mut self,
            nft_id: u128,
            value: u128,
        ) -> Result<(), StakingError> {
            let cast = self
                .cast_distribution
                .get(nft_id)
                .ok_or(StakingError::InvalidInput)?;
            self.update_registry_weights(&cast, value, false, true)
        }

        fn emit_event<EE>(emitter: EE, event: Event)
        where
            EE: EmitEvent<Staking>,
        {
            emitter.emit_event(event);
        }

        fn transfer_psp22_from(
            &self,
            from: &AccountId,
            to: &AccountId,
            amount: Balance,
        ) -> Result<(), StakingError> {
            let mut token: contract_ref!(PSP22) = self.governance_token.into();
            if let Err(e) = token.transfer_from(*from, *to, amount, Vec::new()) {
                return Err(StakingError::TokenError(e));
            }
            Ok(())
        }

        fn burn_psp34(&mut self, from: AccountId, nft_id: u128) -> Result<(), StakingError> {
            if let Err(e) = self.nft.burn(from, nft_id) {
                return Err(StakingError::NFTError(e));
            }
            Ok(())
        }

        fn call_increment_weights(
            &mut self,
            nft_id: u128,
            stake_weight: u128,
            vote_weight: u128,
        ) -> Result<(), StakingError> {
            self.nft
                .increment_weights(nft_id, stake_weight, vote_weight)
                .map_err(StakingError::NFTError)
        }

        fn call_registry_update(&mut self, values: Vec<WeightUpdate>) -> Result<(), RuntimeError> {
            build_call::<DefaultEnvironment>()
                .call(self.registry)
                .exec_input(ExecutionInput::new(UPDATE_SELECTOR).push_arg(values))
                .transferred_value(0)
                .returns::<Result<(), RuntimeError>>()
                .invoke()
        }

        fn get_agents(&self) -> Result<Vec<Agent>, StakingError> {
            build_call::<DefaultEnvironment>()
                .call(self.registry)
                .exec_input(ExecutionInput::new(AGENT_SELECTOR))
                .transferred_value(0)
                .returns::<Result<Vec<Agent>, RuntimeError>>()
                .invoke()
                .map_err(StakingError::InternalError)
        }

        fn is_disabled(&self, agent: AccountId, agents: &[Agent]) -> bool {
            match agents.iter().find(|a| a.address == agent) {
                Some(agent) => agent.disabled,
                None => true,
            }
        }

        fn mint_psp34(
            &mut self,
            to: AccountId,
            stake_weight: u128,
            vote_weight: u128,
        ) -> Result<u128, StakingError> {
            self.nft
                .mint(to, stake_weight, vote_weight)
                .map_err(StakingError::NFTError)
        }

        fn decrease_vote_weight(
            &mut self,
            nft_id: u128,
            vote_weight: u128,
        ) -> Result<(), StakingError> {
            self.nft
                .decrement_vote_weight(nft_id, vote_weight)
                .map_err(StakingError::NFTError)
        }

        fn update_stake_accumulation(&mut self, curr_time: u64) -> Result<(), StakingError> {
            self.accumulated_rewards +=
                ((curr_time - self.last_accumulation_update) as u128) * self.rewards_per_second;
            self.reward_stake_accumulation +=
                self.staked_token_balance * ((curr_time - self.last_accumulation_update) as u128);
            self.last_accumulation_update = curr_time;
            Ok(())
        }

        fn calculate_reward_share(
            &self,
            curr_time: u64,
            last_update: u64,
            stake_balance: u128,
        ) -> u128 {
            debug_println!("{}{}", curr_time, " CURRTIME");
            debug_println!("{}{}", last_update, " UPDATE");
            debug_println!("{}{}", stake_balance, " STAKE");
            debug_println!("{}{}", self.accumulated_rewards, " ACCUMULATED");
            debug_println!("{}{}", self.reward_stake_accumulation, " REWARD");
            let user_stake_weight = stake_balance * ((curr_time - last_update) as u128);
            self.pro_rata(
                self.accumulated_rewards,
                user_stake_weight,
                self.reward_stake_accumulation,
            )
        }

        fn transfer_psp34(
            &mut self,
            from: &AccountId,
            to: &AccountId,
            nft_id: Balance,
        ) -> Result<(), StakingError> {
            self.nft
                .transfer_from(*from, *to, Id::U128(nft_id), Vec::new())
                .map_err(StakingError::NFTError)
        }

        fn call_add_agent(
            &self,
            admin: AccountId,
            validator: AccountId,
            pool_create_amount: u128,
            existential_deposit: u128,
        ) -> Result<AccountId, StakingError> {
            let transfer_amount = pool_create_amount + existential_deposit;
            build_call::<DefaultEnvironment>()
                .call(self.registry)
                .exec_input(
                    ExecutionInput::new(ADD_SELECTOR)
                        .push_arg(admin)
                        .push_arg(validator),
                )
                .transferred_value(transfer_amount)
                .returns::<Result<AccountId, RuntimeError>>()
                .invoke()
                .map_err(StakingError::InternalError)
        }

        fn call_disable_validator(&self, agent: AccountId) -> Result<(), StakingError> {
            let mut registry: contract_ref!(IRegistry) = self.registry.into();
            if registry.disable_agent(agent).is_err() {
                return Err(StakingError::RegistryError);
            }
            Ok(())
        }

        fn is_self_delegator(&self, nft_id: u128) -> bool {
            if self.voting_delegations.contains(nft_id) {
                return false;
            }
            let Some(data) = self.nft.get_governance_data(nft_id) else {
                return false;
            };

            data.vote_weight >= data.stake_weight
        }

        fn is_still_same_pool(&self, delegatee: u128, nonce: u128) -> bool {
            // Check the self-delegator nonce matches the record
            let latest_nonce = self.get_voting_delegation_nonce(delegatee);
            latest_nonce == nonce
        }

        fn get_voting_delegation_nonce(&self, nft_id: u128) -> u128 {
            self.voting_delegations_nonce
                .get(nft_id)
                .unwrap_or_default()
        }

        fn get_governance_data(&self, nft_id: u128) -> Result<GovernanceData, StakingError> {
            self.nft
                .get_governance_data(nft_id)
                .ok_or(StakingError::NotFound)
        }

        fn only_token_owner(&self, nft_id: u128) -> Result<(), StakingError> {
            let caller = self.env().caller();
            match self.nft.owner_of_id(nft_id) == Some(caller) {
                true => Ok(()),
                false => Err(StakingError::Unauthorized),
            }
        }
    }

    impl Staking {
        #[ink(constructor)]
        pub fn new(
            governance_token: AccountId,
            registry: AccountId,
            governor: AccountId,
            governance_nft: GovernanceNFTRef,
            interest_rate: u128,
            governance_council: AccountId,
        ) -> Self {
            let now = Self::env().block_timestamp();

            Self {
                creation_time: now,
                governor,
                registry,
                reward_token_balance: 0_u128,
                staked_token_balance: 0_u128,
                rewards_per_second: interest_rate,
                reward_stake_accumulation: 0,
                accumulated_rewards: 0,
                last_accumulation_update: now,
                governance_council,
                governance_token,
                nft: governance_nft,
                cast_distribution: Mapping::new(),
                voting_delegations: Mapping::new(),
                redelegate_requests: Mapping::new(),
                voting_delegations_nonce: Mapping::new(),
                governance_nfts: Mapping::new(),
                unstake_requests: Mapping::new(),
                last_reward_claim: Mapping::new(),
                deployed_validators: Vec::new(),
                token_stake_amount: 100000_u128,
                create_deposit: 100_000_000_000_000_u128,
                existential_deposit: 500_u128,
                treasury: governance_council,
            }
        }

        #[ink(message)]
        pub fn get_interest_rate(&self) -> u128 {
            self.rewards_per_second
        }

        #[ink(message)]
        pub fn get_governance_nft(&self) -> AccountId {
            GovernanceNFTRef::to_account_id(&self.nft)
        }

        #[ink(message)]
        pub fn get_voting_delegation(&self, nft_id: u128) -> Option<(u128, u128, u128)> {
            self.voting_delegations.get(nft_id)
        }

        #[ink(message, selector = 1)]
        pub fn update_rewards_rate(&mut self, new_rate: u128) -> Result<(), StakingError> {
            let caller = Self::env().caller();
            if caller != self.governor {
                return Err(StakingError::Unauthorized);
            }
            let now = Self::env().block_timestamp();
            self.update_stake_accumulation(now)?;
            self.rewards_per_second = new_rate;
            Ok(())
        }

        #[ink(message, selector = 2)]
        pub fn wrap_tokens(
            &mut self,
            token_value: u128,
            to: Option<AccountId>,
            validator_cast: CastType,
            vote_delegation: Option<u128>,
        ) -> Result<(), StakingError> {
            debug_println!("ADDing Value {}", token_value);

            let caller = Self::env().caller();
            let now = Self::env().block_timestamp();
            self.transfer_psp22_from(&caller, &Self::env().account_id(), token_value)?;
            self.update_stake_accumulation(now)?;
            self.staked_token_balance += token_value;

            let recipient = to.unwrap_or(caller);
            let minted_nft = self.mint_psp34(recipient, token_value, 0)?;
            let vote_delegation = vote_delegation.unwrap_or(minted_nft);
            self.call_increment_weights(vote_delegation, 0, token_value)?;

            if vote_delegation != minted_nft {
                if !self.is_self_delegator(vote_delegation) {
                    return Err(StakingError::InvalidRepresentative);
                }
                let nonce = self.get_voting_delegation_nonce(vote_delegation);
                self.voting_delegations
                    .insert(minted_nft, &(vote_delegation, token_value, nonce));
            }

            self.new_cast_distribution(minted_nft, token_value, validator_cast)?;

            Self::emit_event(
                Self::env(),
                Event::TokensWrapped(TokensWrapped {
                    staker: caller,
                    amount: token_value,
                    nft: minted_nft,
                }),
            );
            Ok(())
        }

        #[ink(message, selector = 3)]
        pub fn update_cast(
            &mut self,
            nft_id: u128,
            validator_cast: CastType,
        ) -> Result<(), StakingError> {
            self.only_token_owner(nft_id)?;

            let data = self.get_governance_data(nft_id)?;
            // deallocate current cast weights
            self.remove_cast_distribution(nft_id, data.stake_weight)?;
            self.new_cast_distribution(nft_id, data.stake_weight, validator_cast)
        }

        #[ink(message, selector = 4)]
        pub fn start_vote_redelegate(
            &mut self,
            nft_id: u128,
            delegatee: u128,
        ) -> Result<(), StakingError> {
            self.only_token_owner(nft_id)?;
            let now = Self::env().block_timestamp();

            if self.query_nft_proposal_lock(nft_id) {
                return Err(StakingError::NftLocked);
            }
            if self.redelegate_requests.contains(nft_id) {
                return Err(StakingError::DuplicateRequest);
            }
            let data = self.get_governance_data(nft_id)?;
            debug_println!("Current NFT Governance DATA {:?}", &data);
            let current = self.voting_delegations.get(nft_id);
            if let Some(curr) = current {
                if delegatee == curr.0 {
                    return Err(StakingError::NoChange);
                }
                debug_println!("Current delegation values being updated {:?}", curr);
                if self.is_still_same_pool(curr.0, curr.2) {
                    self.decrease_vote_weight(curr.0, curr.1)?;
                }
                self.voting_delegations.remove(nft_id);
            } else if delegatee == nft_id {
                return Err(StakingError::NoChange);
            }
            if data.vote_weight != 0 {
                self.decrease_vote_weight(nft_id, data.vote_weight)?;
            }
            self.redelegate_requests.insert(nft_id, &(now, delegatee));

            let prev_nonce = self.get_voting_delegation_nonce(nft_id);
            self.voting_delegations_nonce
                .insert(nft_id, &(prev_nonce + 1));

            Ok(())
        }

        #[ink(message)]
        pub fn update_vote_redelegate(
            &mut self,
            nft_id: u128,
            new_delegatee: u128,
        ) -> Result<(), StakingError> {
            self.only_token_owner(nft_id)?;

            let Some((time, _)) = self.redelegate_requests.get(nft_id) else {
                return Err(StakingError::InvalidRequest);
            };
            self.redelegate_requests
                .insert(nft_id, &(time, new_delegatee));

            Ok(())
        }

        #[ink(message, selector = 5)]
        pub fn complete_vote_redelegate(&mut self, nft_id: u128) -> Result<(), StakingError> {
            self.only_token_owner(nft_id)?;

            if self.query_nft_proposal_lock(nft_id) {
                return Err(StakingError::NftLocked);
            }

            let req = self
                .redelegate_requests
                .get(nft_id)
                .ok_or(StakingError::InvalidRequest)?;
            self.redelegate_requests.remove(nft_id);
            let now = Self::env().block_timestamp();
            if now - req.0 < 14 * DAY {
                return Err(StakingError::InvalidInput);
            }
            let data = self.get_governance_data(nft_id)?;

            //let current = self.voting_delegations.get(nft_id);

            self.call_increment_weights(req.1, 0, data.stake_weight)?;
            if nft_id != req.1 {
                if !self.is_self_delegator(req.1) {
                    return Err(StakingError::InvalidRepresentative);
                }

                let nonce = self.get_voting_delegation_nonce(req.1);
                self.voting_delegations
                    .insert(nft_id, &(req.1, data.stake_weight, nonce));
            }

            Ok(())
        }

        #[ink(message, selector = 6)]
        pub fn add_stake_value(
            &mut self,
            token_value: u128,
            nft_id: u128,
        ) -> Result<(), StakingError> {
            let caller = Self::env().caller();
            let now = Self::env().block_timestamp();

            self.transfer_psp22_from(&caller, &Self::env().account_id(), token_value)?;
            self.update_stake_accumulation(now)?;
            self.claim_staking_rewards(nft_id)?; // should come before `add_cast_distribution` call
            self.add_cast_distribution(nft_id, token_value)?;
            self.staked_token_balance += token_value;

            if let Some(vote_delegation) = self.voting_delegations.get(nft_id) {
                debug_println!("ADDing Delegation Value {}", token_value);
                let update = vote_delegation.1 + token_value;

                if self.is_still_same_pool(vote_delegation.0, vote_delegation.2) {
                    self.call_increment_weights(vote_delegation.0, 0, token_value)?;
                }

                self.call_increment_weights(nft_id, token_value, 0)?;
                self.voting_delegations
                    .insert(nft_id, &(vote_delegation.0, update, vote_delegation.2));
            } else if self.redelegate_requests.contains(nft_id) {
                // To avoid breaking 1-role-1-representative constraint and double-voting;
                // new voting_weight is activated alongside redelegation-completion
                self.call_increment_weights(nft_id, token_value, 0)?;
            } else {
                debug_println!("Adding Value With No Delegation {}", token_value);
                self.call_increment_weights(nft_id, token_value, token_value)?;
            }

            Ok(())
        }

        #[ink(message, selector = 7)]
        pub fn claim_staking_rewards(&mut self, nft_id: u128) -> Result<(), StakingError> {
            let now = Self::env().block_timestamp();
            self.update_stake_accumulation(now)?;
            let data = self.get_governance_data(nft_id)?;
            let last_claim = self
                .last_reward_claim
                .get(nft_id)
                .unwrap_or(data.block_created);
            let reward = self.calculate_reward_share(now, last_claim, data.stake_weight);
            self.add_cast_distribution(nft_id, reward)?;
            self.last_reward_claim.insert(nft_id, &now);
            if let Some(vote_delegation) = self.voting_delegations.get(nft_id) {
                if self.is_still_same_pool(vote_delegation.0, vote_delegation.2) {
                    self.call_increment_weights(vote_delegation.0, 0, reward)?;
                }
                self.call_increment_weights(nft_id, reward, 0)?;
                self.voting_delegations.insert(
                    nft_id,
                    &(
                        vote_delegation.0,
                        vote_delegation.1 + reward,
                        vote_delegation.2,
                    ),
                );
            } else if self.redelegate_requests.contains(nft_id) {
                // To avoid breaking 1-role-1-representative constraint and double-voting;
                // new voting_weight is activated alongside redelegation-completion
                self.call_increment_weights(nft_id, reward, 0)?;
            } else {
                self.call_increment_weights(nft_id, reward, reward)?;
            }

            self.staked_token_balance += reward;
            self.reward_stake_accumulation += reward;

            Ok(())
        }

        #[ink(message, selector = 8)]
        pub fn create_unwrap_request(&mut self, nft_id: u128) -> Result<(), StakingError> {
            self.only_token_owner(nft_id)?;

            let now = Self::env().block_timestamp();
            let caller = Self::env().caller();
            let data = self.get_governance_data(nft_id)?;
            if self.query_nft_proposal_lock(nft_id) {
                return Err(StakingError::NftLocked);
            }
            let delegations = self.voting_delegations.get(nft_id);
            if let Some(d) = delegations {
                self.voting_delegations.remove(nft_id); // optional-housekeeping
                if self.is_still_same_pool(d.0, d.2) {
                    self.decrease_vote_weight(d.0, d.1)?
                }
            }
            self.update_stake_accumulation(now)?;
            self.remove_cast_distribution(nft_id, data.stake_weight)?;
            let last_claim = self
                .last_reward_claim
                .get(nft_id)
                .unwrap_or(data.block_created);

            let reward = self.calculate_reward_share(now, last_claim, data.stake_weight);
            debug_println!("{}{:?}", "reward earned ", reward);
            self.staked_token_balance -= data.stake_weight;
            self.unstake_requests.insert(
                nft_id,
                &UnstakeRequest {
                    time: now,
                    token_value: data.stake_weight + reward,
                    owner: caller,
                },
            );
            self.last_reward_claim.insert(nft_id, &now); // optional-housekeeping
            self.redelegate_requests.remove(nft_id); // optional-housekeeping
            self.cast_distribution.remove(nft_id); // optional-housekeeping

            // This helps prevent delegator of this nft (if a rep) from getting stuck
            let prev_nonce = self.get_voting_delegation_nonce(nft_id);
            self.voting_delegations_nonce
                .insert(nft_id, &(prev_nonce + 1));

            self.burn_psp34(caller, nft_id)?;
            Ok(())
        }

        #[ink(message, selector = 9)]
        pub fn complete_unwrap_request(&mut self, nft_id: u128) -> Result<(), StakingError> {
            let now = Self::env().block_timestamp();
            let caller = Self::env().caller();
            let data = self
                .unstake_requests
                .get(nft_id)
                .ok_or(StakingError::InvalidRequest)?;

            if now < data.time + WITHDRAW_DELAY {
                return Err(StakingError::InvalidTimeWindow);
            }
            if data.owner != caller {
                return Err(StakingError::Unauthorized);
            }

            self.unstake_requests.remove(nft_id);
            self.transfer_psp22_from(&Self::env().account_id(), &caller, data.token_value)?;

            Ok(())
        }

        #[ink(message, payable, selector = 10)]
        pub fn onboard_validator(&mut self, validator: AccountId) -> Result<(), StakingError> {
            //let data = self.nft.get_governance_data(id).unwrap();
            let now = Self::env().block_timestamp();
            let caller = Self::env().caller();

            self.transfer_psp22_from(&caller, &Self::env().account_id(), self.token_stake_amount)?;
            self.update_stake_accumulation(now)?;
            self.staked_token_balance += self.token_stake_amount;
            // self.transfer_psp34(&caller, &Self::env().account_id(), id)?;
            let minted_nft = self.mint_psp34(
                Self::env().account_id(),
                self.token_stake_amount,
                self.token_stake_amount,
            )?;
            let azero = Self::env().transferred_value();

            if azero != self.create_deposit + self.existential_deposit {
                return Err(StakingError::InvalidCreateDeposit);
            }

            if self
                .deployed_validators
                .iter()
                .any(|p| p.validator == validator)
            {
                return Err(StakingError::AlreadyOnList);
            }

            let new_agent = self.call_add_agent(
                validator,
                caller,
                self.create_deposit,
                self.existential_deposit,
            )?;

            // Cast NFT Weight to new agent
            let cast = CastType::Direct(vec![(new_agent, BIPS)]);
            self.new_cast_distribution(minted_nft, self.token_stake_amount, cast)?;

            self.deployed_validators.push(Validator {
                validator,
                agent: new_agent,
                admin: caller,
                nft_id: minted_nft,
            });

            Ok(())
        }

        //Validator addition flow
        // Step 1. Call Registry AddAgent  Existential Deposit:,
        //Mainnet
        //staking.minNominatorBond: 2,000,000,000,000,000
        //balances.existentialDeposit: 500
        //Testnet
        //taking.minNominatorBond: 100,000,000,000,000
        //balances.existentialDeposit: 500
        // Step 2. Initialize Agent call with poolid and Account in nomination pool contract

        #[ink(message, selector = 11)]
        pub fn disable_validator(
            &mut self,
            agent: AccountId,
            slash: bool,
        ) -> Result<(), StakingError> {
            let caller = Self::env().caller();
            if caller != self.governance_council {
                return Err(StakingError::InvalidPermissions);
            }

            let validator_info = self
                .deployed_validators
                .iter()
                .find(|p| p.agent == agent)
                .ok_or(StakingError::InvalidInput)?;
            self.call_disable_validator(agent)?;

            let recipient = match slash {
                true => self.treasury,
                false => validator_info.admin,
            };

            self.transfer_psp34(&Self::env().account_id(), &recipient, validator_info.nft_id)?;

            self.deployed_validators = self
                .deployed_validators
                .iter()
                .filter(|v| v.agent != agent)
                .cloned()
                .collect();

            Ok(())
        }
    }
}
