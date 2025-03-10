#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub mod traits;
pub use crate::staking::StakingRef;
pub use crate::traits::Staking;

pub type Time = u64;
pub type NftId = u128;
pub type Nonce = u128;
pub type Bips = u128;

#[ink::contract]
pub mod staking {
    use super::*;
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
    use psp34::{Id, PSP34Error, PSP34};
    use registry::traits::IRegistry;

    pub const DAY: Time = 86400 * 1000;
    pub const WITHDRAW_DELAY: Time = 14 * DAY;
    pub const MAX_VALIDATORS: u8 = 5;
    pub const BIPS: Bips = 10000000;
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
        creation_time: Time,
        governor: AccountId,
        registry: AccountId,
        reward_token_balance: Balance,
        staked_token_balance: Balance,
        rewards_per_second: Balance,
        reward_stake_accumulation: Balance,
        accumulated_rewards: Balance,
        last_accumulation_update: Time,
        governance_council: AccountId,
        governance_token: AccountId,
        nft: GovernanceNFTRef,
        cast_distribution: Mapping<NftId, Vec<(AccountId, Bips)>>,
        voting_delegations: Mapping<NftId, (NftId, Nonce)>, // (delegatee, nonce)
        redelegate_requests: Mapping<NftId, (Time, NftId)>,
        voting_delegations_nonce: Mapping<NftId, Nonce>,
        unstake_requests: Mapping<NftId, UnstakeRequest>,
        last_reward_claim: Mapping<NftId, Time>,
        deployed_validators: Vec<Validator>,
        representative_stake_threshold: Balance,
        token_stake_amount: Balance,
        create_deposit: Balance,
        existential_deposit: Balance,
        treasury: AccountId,
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum CastType {
        Direct(Vec<(AccountId, Bips)>),
        Delegate(NftId),
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    struct UnstakeRequest {
        pub time: Time,
        pub token_value: Balance,
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
        amount: Balance,
        nft: NftId,
    }

    #[ink(event)]
    pub struct StakeAdded {
        staker: AccountId,
        amount: Balance,
        nft: NftId,
    }

    #[ink(event)]
    pub struct StakeRemoved {
        staker: AccountId,
        amount: Balance,
        nft: NftId,
    }

    #[ink(event)]
    pub struct UnwrapRequestCreated {
        staker: AccountId,
        nft: NftId,
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
        nft_id: NftId,
    }

    type Event = <Staking as ContractEventBase>::Type;

    #[ink(impl)]
    impl Staking {
        pub fn pro_rata(&self, a: u128, b: u128, c: u128) -> u128 {
            if a == 0 || b == 0 {
                return 0;
            }
            let result = BigUint::from(a) * BigUint::from(b) / BigUint::from(c);
            BigUint::to_u128(&result).expect("overflow")
        }

        pub fn update_registry_weights(
            &mut self,
            agents: &[(AccountId, Bips)],
            mut value: Balance,
            increase: bool,
            safe_check: bool,
        ) -> Result<(), StakingError> {
            let mut sum: Bips = 0;
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
            nft_id: NftId,
            value: Balance,
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
            nft_id: NftId,
            value: Balance,
        ) -> Result<(), StakingError> {
            let cast = self
                .cast_distribution
                .get(nft_id)
                .ok_or(StakingError::InvalidInput)?;
            self.update_registry_weights(&cast, value, true, true)
        }

        pub fn remove_cast_distribution(
            &mut self,
            nft_id: NftId,
            value: Balance,
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

        fn burn_psp34(&mut self, from: AccountId, nft_id: NftId) -> Result<(), StakingError> {
            if let Err(e) = self.nft.burn(from, nft_id) {
                return Err(StakingError::NFTError(e));
            }
            Ok(())
        }

        fn call_increment_weights(
            &mut self,
            nft_id: NftId,
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
        ) -> Result<NftId, StakingError> {
            self.nft
                .mint(to, stake_weight, vote_weight)
                .map_err(StakingError::NFTError)
        }

        fn decrease_vote_weight(
            &mut self,
            nft_id: NftId,
            vote_weight: u128,
        ) -> Result<(), StakingError> {
            self.nft
                .decrement_vote_weight(nft_id, vote_weight)
                .map_err(StakingError::NFTError)
        }

        fn update_stake_accumulation(&mut self, curr_time: Time) -> Result<(), StakingError> {
            self.accumulated_rewards +=
                ((curr_time - self.last_accumulation_update) as u128) * self.rewards_per_second;

            self.reward_stake_accumulation +=
                self.staked_token_balance * ((curr_time - self.last_accumulation_update) as u128);
            self.last_accumulation_update = curr_time;
            Ok(())
        }

        fn calculate_reward_share(
            &self,
            curr_time: Time,
            last_update: Time,
            stake_balance: Balance,
        ) -> Balance {
            debug_println!("{}{}", curr_time, " CURRTIME");
            debug_println!("{}{}", last_update, " UPDATE");
            debug_println!("{}{}", stake_balance, " STAKE");
            debug_println!("{}{}", self.accumulated_rewards, " ACCUMULATED");
            debug_println!("{}{}", self.reward_stake_accumulation, " REWARD");

            let user_stake_weight = stake_balance * (curr_time.saturating_sub(last_update) as u128);
            self.pro_rata(
                self.accumulated_rewards,
                user_stake_weight,
                self.reward_stake_accumulation,
            )
        }

        fn transfer_psp34(&mut self, to: &AccountId, nft_id: Balance) -> Result<(), StakingError> {
            self.nft
                .transfer(*to, Id::U128(nft_id), Vec::new())
                .map_err(StakingError::NFTError)
        }

        fn call_add_agent(
            &self,
            admin: AccountId,
            validator: AccountId,
            pool_create_amount: Balance,
            existential_deposit: Balance,
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

        fn is_self_delegator(&self, nft_id: NftId) -> bool {
            if self.voting_delegations.contains(nft_id) {
                return false;
            }
            let Some(data) = self.nft.get_governance_data(nft_id) else {
                return false;
            };

            data.vote_weight >= data.stake_weight
        }

        fn is_still_same_pool(&self, delegatee: NftId, nonce: Nonce) -> bool {
            // Check the self-delegator nonce matches the record
            let latest_nonce = self.get_voting_delegation_nonce(delegatee);
            latest_nonce == nonce
        }

        fn get_voting_delegation_nonce(&self, nft_id: NftId) -> Nonce {
            self.voting_delegations_nonce
                .get(nft_id)
                .unwrap_or_default()
        }

        fn get_governance_data(&self, nft_id: NftId) -> Result<GovernanceData, StakingError> {
            self.nft
                .get_governance_data(nft_id)
                .ok_or(StakingError::NotFound)
        }

        fn only_token_owner(&self, nft_id: NftId) -> Result<(), StakingError> {
            let caller = self.env().caller();
            match self.nft.owner_of_id(nft_id) {
                Some(owner) if owner == self.env().account_id() => {
                    // A validator nft
                    if let Some(v) = self.deployed_validators.iter().find(|v| v.nft_id == nft_id) {
                        if v.admin == caller {
                            return Ok(());
                        }
                    }
                    Err(StakingError::Unauthorized)
                }
                Some(owner) if owner == caller => Ok(()),
                _ => Err(StakingError::Unauthorized),
            }
        }

        fn only_governor(&self) -> Result<(), StakingError> {
            match self.env().caller() == self.governor {
                true => Ok(()),
                false => Err(StakingError::Unauthorized),
            }
        }

        fn nft_proposal_lock(&self, nft_id: NftId) -> Result<(), StakingError> {
            let is_locked = build_call::<DefaultEnvironment>()
                .call(self.governor)
                .exec_input(ExecutionInput::new(Selector::new([0, 0, 0, 33])).push_arg(nft_id))
                .transferred_value(0)
                .returns::<bool>()
                .invoke();

            match is_locked {
                true => Err(StakingError::NftLocked),
                false => Ok(()),
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
            reward_token_balance: Balance,
            interest_rate: Balance,
            governance_council: AccountId,
        ) -> Self {
            let now = Self::env().block_timestamp();

            Self {
                creation_time: now,
                governor,
                registry,
                reward_token_balance,
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
                unstake_requests: Mapping::new(),
                last_reward_claim: Mapping::new(),
                deployed_validators: Vec::new(),
                representative_stake_threshold: 0,
                token_stake_amount: 100_000_u128, // FIXME: doesn't consider the decimals
                create_deposit: 100_000_000_000_000_u128,
                existential_deposit: 500_u128,
                treasury: governance_council,
            }
        }

        #[ink(message)]
        pub fn get_interest_rate(&self) -> Balance {
            self.rewards_per_second
        }

        #[ink(message)]
        pub fn get_governance_nft(&self) -> AccountId {
            GovernanceNFTRef::to_account_id(&self.nft)
        }

        #[ink(message)]
        pub fn get_voting_delegation(&self, nft_id: NftId) -> Option<(NftId, Nonce)> {
            self.voting_delegations.get(nft_id)
        }

        #[ink(message)]
        pub fn get_reward_pool(&self) -> Balance {
            self.reward_token_balance
        }

        #[ink(message, selector = 0)]
        pub fn increase_reward_pool(&mut self, amount: Balance) -> Result<(), StakingError> {
            self.only_governor()?;

            self.reward_token_balance += amount;

            Ok(())
        }

        #[ink(message, selector = 1)]
        pub fn update_rewards_rate(&mut self, new_rate: Balance) -> Result<(), StakingError> {
            self.only_governor()?;

            let now = Self::env().block_timestamp();
            self.update_stake_accumulation(now)?;

            self.rewards_per_second = new_rate;
            Ok(())
        }

        #[ink(message, selector = 12)]
        pub fn update_validator_stake_requirement(
            &mut self,
            ike_deposit: Option<Balance>,
            azero_deposit: Option<Balance>,
        ) -> Result<(), StakingError> {
            self.only_governor()?;

            if let Some(amount) = ike_deposit {
                self.token_stake_amount = amount;
            }

            if let Some(amount) = azero_deposit {
                self.create_deposit = amount;
            }

            Ok(())
        }

        #[ink(message, selector = 13)]
        pub fn update_representative_stake_threshold(
            &mut self,
            amount: Balance,
        ) -> Result<(), StakingError> {
            self.only_governor()?;
            self.representative_stake_threshold = amount;
            Ok(())
        }

        #[ink(message, selector = 2)]
        pub fn wrap_tokens(
            &mut self,
            token_value: Balance,
            to: Option<AccountId>,
            validator_cast: CastType,
            vote_delegation: Option<NftId>,
        ) -> Result<(), StakingError> {
            let caller = Self::env().caller();
            let now = Self::env().block_timestamp();
            self.update_stake_accumulation(now)?;

            self.transfer_psp22_from(&caller, &Self::env().account_id(), token_value)?;
            self.staked_token_balance += token_value;
            self.reward_stake_accumulation += token_value * ((now - self.creation_time) as u128);

            let recipient = to.unwrap_or(caller);
            let minted_nft = self.mint_psp34(recipient, token_value, 0)?;
            let vote_delegation = vote_delegation.unwrap_or(minted_nft);

            if vote_delegation == minted_nft {
                if token_value < self.representative_stake_threshold {
                    return Err(StakingError::InvalidStake);
                }
            } else {
                if !self.is_self_delegator(vote_delegation) {
                    return Err(StakingError::InvalidRepresentative);
                }
                let nonce = self.get_voting_delegation_nonce(vote_delegation);
                self.voting_delegations
                    .insert(minted_nft, &(vote_delegation, nonce));
            }
            self.call_increment_weights(vote_delegation, 0, token_value)?;
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
            nft_id: NftId,
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
            nft_id: NftId,
            new_delegatee: NftId,
        ) -> Result<(), StakingError> {
            self.only_token_owner(nft_id)?;
            self.nft_proposal_lock(nft_id)?;
            let now = Self::env().block_timestamp();

            if self.redelegate_requests.contains(nft_id) {
                return Err(StakingError::DuplicateRequest);
            }

            let data = self.get_governance_data(nft_id)?;
            if new_delegatee == nft_id && data.stake_weight < self.representative_stake_threshold {
                return Err(StakingError::InvalidStake);
            }
            if data.vote_weight != 0 {
                self.decrease_vote_weight(nft_id, data.vote_weight)?;
            }

            let current_delegation = self.voting_delegations.get(nft_id);
            if let Some((current_delegatee, nonce)) = current_delegation {
                if new_delegatee == current_delegatee {
                    return Err(StakingError::NoChange);
                }
                if self.is_still_same_pool(current_delegatee, nonce) {
                    self.decrease_vote_weight(current_delegatee, data.vote_weight)?;
                }
                self.voting_delegations.remove(nft_id);
            } else if new_delegatee == nft_id {
                return Err(StakingError::NoChange);
            }

            let prev_nonce = self.get_voting_delegation_nonce(nft_id);
            self.voting_delegations_nonce
                .insert(nft_id, &(prev_nonce + 1));
            self.redelegate_requests
                .insert(nft_id, &(now, new_delegatee));

            Ok(())
        }

        #[ink(message)]
        pub fn update_vote_redelegate(
            &mut self,
            nft_id: NftId,
            new_delegatee: NftId,
        ) -> Result<(), StakingError> {
            self.only_token_owner(nft_id)?;

            let Some((time, _)) = self.redelegate_requests.get(nft_id) else {
                return Err(StakingError::InvalidRequest);
            };

            let data = self.get_governance_data(nft_id)?;
            if data.stake_weight < self.representative_stake_threshold {
                return Err(StakingError::InvalidStake);
            }

            self.redelegate_requests
                .insert(nft_id, &(time, new_delegatee));

            Ok(())
        }

        #[ink(message, selector = 5)]
        pub fn complete_vote_redelegate(&mut self, nft_id: NftId) -> Result<(), StakingError> {
            self.only_token_owner(nft_id)?;
            self.nft_proposal_lock(nft_id)?;

            let (time, delegatee) = self
                .redelegate_requests
                .get(nft_id)
                .ok_or(StakingError::InvalidRequest)?;
            self.redelegate_requests.remove(nft_id);

            let now = Self::env().block_timestamp();
            if now - time < 14 * DAY {
                return Err(StakingError::InvalidInput);
            }

            let data = self.get_governance_data(nft_id)?;
            if nft_id == delegatee {
                if data.stake_weight < self.representative_stake_threshold {
                    return Err(StakingError::InvalidStake);
                }
            } else {
                if !self.is_self_delegator(delegatee) {
                    return Err(StakingError::InvalidRepresentative);
                }

                let nonce = self.get_voting_delegation_nonce(delegatee);
                self.voting_delegations.insert(nft_id, &(delegatee, nonce));
            }
            self.call_increment_weights(delegatee, 0, data.stake_weight)?;

            Ok(())
        }

        #[ink(message, selector = 6)]
        pub fn add_stake_value(
            &mut self,
            token_value: Balance,
            nft_id: NftId,
            withdraw_current_yield: bool,
        ) -> Result<(), StakingError> {
            let caller = Self::env().caller();
            let now = Self::env().block_timestamp();
            self.update_stake_accumulation(now)?;
            self.claim_staking_rewards(nft_id, withdraw_current_yield)?; // should come before incrementing stake_weight

            self.transfer_psp22_from(&caller, &Self::env().account_id(), token_value)?;
            self.staked_token_balance += token_value;
            self.reward_stake_accumulation += token_value * ((now - self.creation_time) as u128);

            self.add_cast_distribution(nft_id, token_value)?;

            if let Some((delegatee, nonce)) = self.voting_delegations.get(nft_id) {
                if self.is_still_same_pool(delegatee, nonce) {
                    self.call_increment_weights(delegatee, 0, token_value)?;
                }
                self.call_increment_weights(nft_id, token_value, 0)?;
                self.voting_delegations.insert(nft_id, &(delegatee, nonce));
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
        pub fn claim_staking_rewards(
            &mut self,
            nft_id: NftId,
            withdraw_yield: bool,
        ) -> Result<(), StakingError> {
            self.only_token_owner(nft_id)?;
            let now = Self::env().block_timestamp();
            self.update_stake_accumulation(now)?;

            let data = self.get_governance_data(nft_id)?;

            let last_claim = self
                .last_reward_claim
                .get(nft_id)
                .unwrap_or(data.block_created);
            let mut reward = self.calculate_reward_share(now, last_claim, data.stake_weight);
            self.last_reward_claim.insert(nft_id, &now);

            if self.reward_token_balance <= reward {
                // discuss: call to gov_token to get the latest reward reserve balance?
                self.rewards_per_second = 0;
                reward = self.reward_token_balance;
            }
            self.reward_token_balance -= reward;

            match withdraw_yield {
                true => {
                    self.transfer_psp22_from(
                        &self.env().account_id(),
                        &self.env().caller(),
                        reward,
                    )?;
                }
                false => {
                    self.add_cast_distribution(nft_id, reward)?;

                    if let Some((delegatee, nonce)) = self.voting_delegations.get(nft_id) {
                        if self.is_still_same_pool(delegatee, nonce) {
                            self.call_increment_weights(delegatee, 0, reward)?;
                        }
                        self.call_increment_weights(nft_id, reward, 0)?;
                        self.voting_delegations.insert(nft_id, &(delegatee, nonce));
                    } else if self.redelegate_requests.contains(nft_id) {
                        // To avoid breaking 1-role-1-representative constraint and double-voting;
                        // new voting_weight is activated alongside redelegation-completion
                        self.call_increment_weights(nft_id, reward, 0)?;
                    } else {
                        self.call_increment_weights(nft_id, reward, reward)?;
                    }

                    self.staked_token_balance += reward;
                    self.reward_stake_accumulation += reward * ((now - self.creation_time) as u128);
                }
            }

            Ok(())
        }

        #[ink(message, selector = 8)]
        pub fn create_unwrap_request(&mut self, nft_id: NftId) -> Result<(), StakingError> {
            self.only_token_owner(nft_id)?;
            self.nft_proposal_lock(nft_id)?;

            let caller = Self::env().caller();
            let now = Self::env().block_timestamp();
            self.update_stake_accumulation(now)?;

            let data = self.get_governance_data(nft_id)?;
            self.remove_cast_distribution(nft_id, data.stake_weight)?;

            let last_claim = self
                .last_reward_claim
                .get(nft_id)
                .unwrap_or(data.block_created);
            let mut reward = self.calculate_reward_share(now, last_claim, data.stake_weight);

            if self.reward_token_balance <= reward {
                // discuss: call to gov_token to get the latest reward reserve balance?
                self.rewards_per_second = 0;
                reward = self.reward_token_balance;
            }
            self.reward_token_balance -= reward;

            let delegations = self.voting_delegations.get(nft_id);
            if let Some((delegatee, nonce)) = delegations {
                if self.is_still_same_pool(delegatee, nonce) {
                    self.decrease_vote_weight(delegatee, data.vote_weight)?
                }
                self.voting_delegations.remove(nft_id); // optional-housekeeping
            }

            self.staked_token_balance -= data.stake_weight;

            // Possible heuristics to adjust the weights
            // 1. Don't make any changes (=> future reward is always less than the ideal yield)
            // 2. Remove the utilised range (=> future reward can yield higher than ideal returns)
            // 3. Only account for utilised range (middle ground) (ACTIVE)
            self.reward_stake_accumulation -= data.stake_weight * ((data.block_created - self.creation_time) as u128);

            self.unstake_requests.insert(
                nft_id,
                &UnstakeRequest {
                    time: now,
                    token_value: data.stake_weight + reward,
                    owner: caller,
                },
            );

            // This helps prevent delegator of this nft (if a rep) from getting stuck
            let prev_nonce = self.get_voting_delegation_nonce(nft_id);
            self.voting_delegations_nonce
                .insert(nft_id, &(prev_nonce + 1));

            // optional-housekeeping
            self.redelegate_requests.remove(nft_id);
            self.cast_distribution.remove(nft_id);
            self.last_reward_claim.insert(nft_id, &now);

            self.burn_psp34(caller, nft_id)
        }

        #[ink(message, selector = 9)]
        pub fn complete_unwrap_request(&mut self, nft_id: NftId) -> Result<(), StakingError> {
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
            let caller = Self::env().caller();
            let now = Self::env().block_timestamp();
            self.update_stake_accumulation(now)?;

            self.transfer_psp22_from(&caller, &Self::env().account_id(), self.token_stake_amount)?;
            self.staked_token_balance += self.token_stake_amount;

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
                caller,
                validator,
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

            self.transfer_psp34(&recipient, validator_info.nft_id)?;

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
