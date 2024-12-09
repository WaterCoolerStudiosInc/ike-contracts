//
#![cfg_attr(not(feature = "std"), no_std, no_main)]
mod traits;
pub use traits::ValidatorWhitelist;
#[ink::contract]
mod validator_whitelist {
    use governance_nft::GovernanceNFT;
    use ink::{
        contract_ref,
        env::{
            call::{build_call, ExecutionInput, Selector},
            DefaultEnvironment,
        },
        prelude::vec::Vec,
        storage::Mapping,
    };

    use psp22::PSP22Error;
    use psp34::PSP34Error;
    use registry::traits::IRegistry;
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

    #[ink(storage)]
    pub struct ValidatorWhitelist {
        pub admin: AccountId,
        pub registry: AccountId,
        pub treasury: AccountId,
        pub gov_nft: AccountId,
        pub deployed_validators: Vec<Validator>,
        pub token_stake_amount: u128,
        pub create_deposit: u128,
        pub existential_deposit: u128,
        pub max_applicants: u16,
    }
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]

    pub enum RuntimeError {
        CallRuntimeFailed,
        Unauthorized,
    }
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum WhitelistError {
        InvalidCreateDeposit,
        Unauthorized,
        InvalidStake,
        InvalidTimeWindow,
        AlreadyOnList,
        RegistryError,
        NFTError(PSP34Error),
        TokenError(PSP22Error),
        InternalError(RuntimeError),
    }
    const ADD_SELECTOR: Selector = Selector::new([0, 0, 0, 1]);
    impl ValidatorWhitelist {
        fn transfer_psp34(
            &self,
            from: &AccountId,
            to: &AccountId,
            id: Balance,
        ) -> Result<(), WhitelistError> {
            let mut token: contract_ref!(GovernanceNFT) = self.gov_nft.into();
            if let Err(e) = token.transfer_from(*from, *to, id, Vec::new()) {
                return Err(WhitelistError::NFTError(e));
            }
            Ok(())
        }

        fn query_weight(&self, id: u128) -> u128 {
            let mut nft: contract_ref!(GovernanceNFT) = self.gov_nft.into();
            let data = nft.get_governance_data(id).unwrap();
            data.vote_weight
        }

        /**
        * admin: AccountId,
           validator: AccountId,
           pool_id: u32,
           pool_create_amount: Balance,
           existential_deposit: Balance,
        */
        fn call_add_agent(
            &self,
            admin: AccountId,
            validator: AccountId,
            pool_create_amount: u128,
            existential_deposit: u128,
        ) -> Result<AccountId, RuntimeError> {
            let transfer_amount = pool_create_amount + existential_deposit;
            build_call::<DefaultEnvironment>()
                .call(self.registry)
                .exec_input(
                    ExecutionInput::new(ADD_SELECTOR)
                        .push_arg(admin)
                        .push_arg(validator)
                        .push_arg(pool_create_amount)
                        .push_arg(existential_deposit),
                )
                .transferred_value(transfer_amount)
                .returns::<Result<AccountId, RuntimeError>>()
                .invoke()
        }
        /**
        *  if let Err(e) = call_withdraw_unbonded(a.address) {
               return Err(VaultError::InternalError(e));
           }
        */
        fn call_remove_validator(&self, agent: AccountId) -> Result<(), WhitelistError> {
            let mut registry: contract_ref!(IRegistry) = self.registry.into();
            if let Err(_) = registry.disable_agent(agent) {
                return Err(WhitelistError::RegistryError);
            }
            Ok(())
        }
        #[ink(constructor)]
        pub fn new(
            _admin: AccountId,
            _registry: AccountId,
            _gov_nft: AccountId,
            _treasury: AccountId,
        ) -> Self {
            Self {
                admin: _admin,
                treasury: _treasury,
                registry: _registry,
                gov_nft: _gov_nft,
                deployed_validators: Vec::new(),
                token_stake_amount: 100000_u128,
                create_deposit: 100_000_000_000_000_u128,
                existential_deposit: 500_u128,
                max_applicants: 100_u16,
            }
        }

        #[ink(message)]
        pub fn update_deposits(
            &mut self,
            new_create: Option<u128>,
            new_existential: Option<u128>,
        ) -> Result<(), WhitelistError> {
            let caller = Self::env().caller();
            if caller != self.admin {
                return Err(WhitelistError::Unauthorized);
            }
            if new_create.is_some() {
                self.create_deposit = new_create.unwrap();
            }
            if new_existential.is_some() {
                self.existential_deposit = new_existential.unwrap();
            }
            Ok(())
        }

        #[ink(message)]
        pub fn join(
            &mut self,
            id: u128,
            validator: AccountId,
        ) -> Result<(), WhitelistError> {
            let nft_weight = self.query_weight(id);
            let caller: ink::primitives::AccountId = Self::env().caller();
            let azero = Self::env().transferred_value();
            if azero != self.create_deposit + self.existential_deposit {
                return Err(WhitelistError::InvalidCreateDeposit);
            }
            if nft_weight < self.token_stake_amount {
                return Err(WhitelistError::InvalidStake);
            }

            if self
                .deployed_validators
                .clone()
                .into_iter()
                .find(|p| p.validator == validator)
                .is_some()
            {
                return Err(WhitelistError::AlreadyOnList);
            }
            self.transfer_psp34(&caller, &Self::env().account_id(), id)?;
            let new_agent = self
                .call_add_agent(
                    validator,
                    caller,
                    self.create_deposit,
                    self.existential_deposit,
                )
                .unwrap();
            self.deployed_validators.push(Validator {
                validator: validator,
                agent: new_agent,
                admin: caller,
                nft_id: id,
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

        #[ink(message, selector = 2)]
        pub fn remove_validator_by_agent(
            &mut self,
            agent: AccountId,
            slash: bool,
        ) -> Result<(), WhitelistError> {
            let validator_info = self
                .deployed_validators
                .clone()
                .into_iter()
                .find(|p| p.agent == agent)
                .unwrap();

            self.call_remove_validator(agent)?;
            if slash {
                self.transfer_psp34(
                    &Self::env().account_id(),
                    &self.treasury,
                    validator_info.nft_id,
                )?;
            } else {
                self.transfer_psp34(
                    &validator_info.admin,
                    &Self::env().account_id(),
                    validator_info.nft_id,
                )?;
            }
            let filtered: Vec<Validator> = self
                .deployed_validators
                .clone()
                .into_iter()
                .filter(|v| v.agent != agent)
                .collect();
            self.deployed_validators = filtered;

            Ok(())
        }
    }
}
