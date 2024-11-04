#![cfg_attr(not(feature = "std"), no_std, no_main)]
mod traits;
pub use crate::staking::StakingRef;
pub use crate::traits::Staking;

#[ink::contract]
mod staking {
    use ink::contract_ref;

    use num_bigint::BigUint;
    use num_traits::cast::ToPrimitive;

    use ink::reflect::ContractEventBase;
    use ink::ToAccountId;
    use ink::{
        codegen::EmitEvent,
        env::debug_println,
        env::{
            call::{build_call, ExecutionInput, Selector},
            DefaultEnvironment,
        },
        prelude::vec::Vec,
        storage::Mapping,
    };
    use psp22::{PSP22Error, PSP22};
    use psp34::PSP34Error;

    use governance_nft::GovernanceNFTRef;

    pub const DAY: u64 = 86400 * 1000;
    pub const WITHDRAW_DELAY: u64 = 14 * DAY;
    pub const MAX_VALIDATORS: u8 = 5;
    pub const BIPS: u128 = 10000000;
    const UPDATE_SELECTOR: Selector = Selector::new([0, 0, 0, 1]);
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum StakingError {
        InvalidInput,
        Unauthorized,
        InvalidTimeWindow,
        NftLocked,
        NFTError(PSP34Error),
        TokenError(PSP22Error),
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
        lst_accumulation_update: u64,
        owner: AccountId,
        governance_token: AccountId,
        nft: GovernanceNFTRef,
        cast_distribution: Mapping<u128, Vec<(AccountId, u128)>>,
        voting_delegations: Mapping<u128, (u128, u128)>,
        governance_nfts: Mapping<AccountId, Vec<u128>>,
        unstake_requests: Mapping<u128, UnstakeRequest>,
        last_reward_claim: Mapping<u128, u64>,
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
    type Event = <Staking as ContractEventBase>::Type;
    impl Staking {
        pub fn pro_rata(&self, a: u128, b: u128, c: u128) -> u128 {
            let result = BigUint::from(a) * BigUint::from(b) / BigUint::from(c);
            BigUint::to_u128(&result).unwrap()
        }
        pub fn query_nft_proposal_lock(&self, governance: AccountId, id: u128) -> bool {
            let call_result: bool = build_call::<DefaultEnvironment>()
                .call(governance)
                .exec_input(ExecutionInput::new(Selector::new([0, 0, 0, 33])).push_arg(id))
                .transferred_value(0)
                .returns::<bool>()
                .invoke();
            call_result
        }
        pub fn increase_registry_weights(
            &mut self,
            agents: Vec<(AccountId, u128)>,
            value: u128,
        ) -> Result<(), StakingError> {
            let mut sum: u128 = 0;
            let mut agents_list = Vec::new();
            let mut amounts_list = Vec::new();
            if agents.len() > 5 {
                return Err(StakingError::InvalidInput);
            }
            for agent in agents.into_iter() {
                sum += agent.1;
                agents_list.push(agent.0);
                let amt = self.pro_rata(value, agent.1 as u128, BIPS);
                amounts_list.push((amt, true));
            }
            if sum != BIPS {
                return Err(StakingError::InvalidInput);
            }
            self.call_registry_update(agents_list, amounts_list);
            Ok(())
        }
        pub fn decrease_registry_weights(
            &mut self,
            agents: Vec<(AccountId, u128)>,
            value: u128,
        ) -> Result<(), StakingError> {
            let mut sum: u128 = 0;
            let mut agents_list = Vec::new();
            let mut amounts_list = Vec::new();
            if agents.len() > 5 {
                return Err(StakingError::InvalidInput);
            }
            for agent in agents.into_iter() {
                sum += agent.1;
                agents_list.push(agent.0);
                let amt = self.pro_rata(value, agent.1 as u128, BIPS);
                amounts_list.push((amt, false));
            }
            if sum != BIPS {
                return Err(StakingError::InvalidInput);
            }
            self.call_registry_update(agents_list, amounts_list);
            Ok(())
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
        fn burn_psp34(&mut self, from: AccountId, id: u128) -> Result<(), StakingError> {
            if let Err(e) = self.nft.burn(from, id) {
                return Err(StakingError::NFTError(e));
            }
            Ok(())
        }
        fn call_registry_update(
            &mut self,
            agents: Vec<AccountId>,
            values: Vec<(u128, bool)>,
        ) -> Result<(), RuntimeError> {
            build_call::<DefaultEnvironment>()
                .call(self.registry)
                .exec_input(
                    ExecutionInput::new(UPDATE_SELECTOR)
                        .push_arg(agents)
                        .push_arg(values),
                )
                .transferred_value(0)
                .returns::<Result<(), RuntimeError>>()
                .invoke()
        }
        fn mint_psp34(
            &mut self,
            to: AccountId,
            stake_weight: u128,
            vote_weight: u128,
        ) -> Result<u128, StakingError> {
            let result = self.nft.mint(to, stake_weight, vote_weight);
            match result {
                Err(e) => return Err(StakingError::NFTError(e)),
                Ok(r) => Ok(r),
            }
        }
        fn decrease_vote_weight(
            &mut self,
            nft_id: u128,
            vote_weight: u128,
        ) -> Result<(), StakingError> {
            let result = self.nft.decrement_weight(nft_id, vote_weight);
            match result {
                Err(e) => return Err(StakingError::NFTError(e)),
                Ok(r) => Ok(r),
            }
        }

        fn update_stake_accumulation(&mut self, curr_time: u64) -> Result<(), StakingError> {
            self.accumulated_rewards +=
                ((curr_time - self.lst_accumulation_update) as u128) * self.rewards_per_second;
            self.reward_stake_accumulation +=
                self.staked_token_balance * ((curr_time - self.lst_accumulation_update) as u128);
            self.lst_accumulation_update = curr_time;
            Ok(())
        }
        fn calculate_reward_share(
            &mut self,
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
            (self.accumulated_rewards * user_stake_weight) / self.reward_stake_accumulation
            //0_u128
        }

        #[ink(constructor)]
        pub fn new(
            governance_token: AccountId,
            registry: AccountId,
            governor: AccountId,
            governance_nft: GovernanceNFTRef,
            interest_rate: u128,
        ) -> Self {
            let caller = Self::env().caller();
            let now = Self::env().block_timestamp();

            Self {
                creation_time: now,
                governor: governor,
                registry: registry,
                reward_token_balance: 0_u128,
                staked_token_balance: 0_u128,
                rewards_per_second: interest_rate,
                reward_stake_accumulation: 0,
                accumulated_rewards: 0,
                lst_accumulation_update: now,
                owner: caller,
                governance_token: governance_token,
                nft: governance_nft,
                cast_distribution: Mapping::new(),
                voting_delegations: Mapping::new(),
                governance_nfts: Mapping::new(),
                unstake_requests: Mapping::new(),
                last_reward_claim: Mapping::new(),
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
        #[ink(message, selector = 17)]
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
        #[ink(message)]
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

            let recipient: AccountId;
            if to.is_some() {
                recipient = to.unwrap();
            } else {
                recipient = caller;
            }

            let minted_nft;
            if vote_delegation.is_some() {
                minted_nft = self.mint_psp34(recipient, token_value, 0).unwrap();
                if let Err(e) = self
                    .nft
                    .increment_weight(vote_delegation.unwrap(), token_value)
                {
                    return Err(StakingError::NFTError(e));
                }
                self.voting_delegations
                    .insert(minted_nft, &(vote_delegation.unwrap(), token_value));
            } else {
                minted_nft = self
                    .mint_psp34(recipient, token_value, token_value)
                    .unwrap();
            }
            match validator_cast {
                CastType::Direct(weights) => {
                    self.cast_distribution.insert(minted_nft, &weights);
                    self.increase_registry_weights(weights, token_value);
                }
                CastType::Delegate(nft) => {
                    let d = self.cast_distribution.get(nft);
                    if let Some(dist) = d {
                        self.cast_distribution.insert(minted_nft, &dist);
                        self.increase_registry_weights(dist, token_value);
                    } else {
                        return Err(StakingError::InvalidInput);
                    }
                }
            }
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
        #[ink(message)]
        pub fn update_cast(
            &mut self,
            nft_id: u128,
            validator_cast: CastType,
        ) -> Result<(), StakingError> {
            let caller = Self::env().caller();
            let now = Self::env().block_timestamp();
            let current_cast = self.cast_distribution.get(nft_id).unwrap();
            let data = self.nft.get_governance_data(nft_id).unwrap();
            // deallocate current cast weights
            self.decrease_registry_weights(current_cast, data.stake_weight);

            match validator_cast {
                CastType::Direct(weights) => {
                    self.cast_distribution.insert(nft_id, &weights);
                    self.increase_registry_weights(weights, data.stake_weight);
                }
                CastType::Delegate(nft) => {
                    let d = self.cast_distribution.get(nft);
                    if let Some(dist) = d {
                        self.cast_distribution.insert(nft_id, &dist);
                        self.increase_registry_weights(dist, data.stake_weight);
                    } else {
                        return Err(StakingError::InvalidInput);
                    }
                }
            }
            Ok(())
        }

        #[ink(message)]
        pub fn add_stake_value(
            &mut self,
            token_value: u128,
            nft_id: u128,
        ) -> Result<(), StakingError> {
            debug_println!("ADDing Value {}", token_value);
            let caller = Self::env().caller();
            let now = Self::env().block_timestamp();
            self.transfer_psp22_from(&caller, &Self::env().account_id(), token_value)?;
            self.update_stake_accumulation(now)?;
            self.staked_token_balance += token_value;

            if let Err(e) = self.nft.increment_weight(nft_id, token_value) {
                return Err(StakingError::NFTError(e));
            }
            Ok(())
        }

        #[ink(message)]
        pub fn claim_staking_rewards(&mut self, token_id: u128) -> Result<(), StakingError> {
            let now = Self::env().block_timestamp();
            self.update_stake_accumulation(now)?;
            let data = self.nft.get_governance_data(token_id).unwrap();
            let last_claim = self
                .last_reward_claim
                .get(token_id)
                .unwrap_or(data.block_created);
            let reward = self.calculate_reward_share(now, last_claim, data.vote_weight);
            self.last_reward_claim.insert(token_id, &now);
            if let Err(e) = self.nft.increment_weight(token_id, reward) {
                return Err(StakingError::NFTError(e));
            }
            Ok(())
        }
        #[ink(message)]
        pub fn create_unwrap_request(&mut self, token_id: u128) -> Result<(), StakingError> {
            let now = Self::env().block_timestamp();
            let caller = Self::env().caller();
            let data = self.nft.get_governance_data(token_id).unwrap();
            if self.query_nft_proposal_lock(self.governor, token_id) {
                return Err(StakingError::NftLocked);
            }
            let delegations = self.voting_delegations.get(token_id);
            if let Some(d) = delegations {
                self.decrease_vote_weight(d.0, d.1)?
            }
            self.update_stake_accumulation(now)?;
            let cast_distribution = self.cast_distribution.get(token_id);
            self.decrease_registry_weights(cast_distribution.unwrap(), data.stake_weight);
            let last_claim = self
                .last_reward_claim
                .get(token_id)
                .unwrap_or(data.block_created);

            let reward = self.calculate_reward_share(now, last_claim, data.vote_weight);
            debug_println!("{}{:?}", "reward earned ", reward);
            self.staked_token_balance -= data.vote_weight;
            self.unstake_requests.insert(
                token_id,
                &UnstakeRequest {
                    time: now,
                    token_value: data.vote_weight + reward,
                    owner: caller,
                },
            );
            self.burn_psp34(caller, token_id)?;
            Ok(())
        }
        #[ink(message)]
        pub fn complete_request(&mut self, token_id: u128) -> Result<(), StakingError> {
            let now = Self::env().block_timestamp();
            let caller = Self::env().caller();
            let data = self.unstake_requests.get(token_id).unwrap();
            if now < data.time + WITHDRAW_DELAY {
                return Err(StakingError::InvalidTimeWindow);
            }
            if data.owner != caller {
                return Err(StakingError::Unauthorized);
            }
            self.transfer_psp22_from(&Self::env().account_id(), &caller, data.token_value)?;
            self.unstake_requests.remove(token_id);
            Ok(())
        }
    }
}
