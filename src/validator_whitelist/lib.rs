//
#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod validator_whitelist {
    use ink::{
        env::{debug_println, DefaultEnvironment},
        prelude::{string::String, vec::Vec},
        primitives::AccountId,
        storage::Mapping,
    };
    use psp22::{PSP22Error, PSP22};
    use psp34::{Id, PSP34Error, PSP34};
    use registry::{registry::RegistryError, Registry};
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    struct Validator {
        validator: AccountId,
        admin: AccountId,
        stake: u128,
    }
    #[ink(storage)]
    pub struct ValidatorWhitelist {
        admin: AccountId,
        registry: AccountId,
        treasury: AccountId,
        gov_nft: AccountId,
        queued_validators: Mapping<AccountId, Validator>,
        deployed_validators: Mapping<AccountId, Validator>,
        token_stake_amount: u128,
        create_deposit: u128,
        existential_deposit: u128,
        max_applicants: u16,
    }
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum WhitelistError {
        Invalid,
        Unauthorized,
        InvalidStake,
        InvalidTimeWindow,
        AlreadyOnList,
        NFTError(PSP34Error),
        TokenError(PSP22Error),
    }
    const ADD_SELECTOR: Selector = Selector::new([0, 0, 0, 1]);
    impl ValidatorWhitelist {
        fn transfer_psp34(
            &self,
            from: &AccountId,
            to: &AccountId,
            id: Balance,
        ) -> Result<(), WhitelistError> {
            let mut token: contract_ref!(PSP34) = self.gov_nft.into();
            if let Err(e) = token.transfer_from(*from, *to, id, Vec::new()) {
                return Err(WhitelistError::TokenError(e));
            }
            Ok(())
        }
        fn query_weight(&self, id: u128) -> u128 {
            let mut nft: contract_ref!(GovernanceNFT) = self.gov_nft.into();
            let data = nft.get_governance_data(id);
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
            pool_id: u32,
            pool_create_amount: u128,
            existential_deposit: u128,
        ) -> Result<AccountId, WhitelistError> {
            let transfer_amount = pool_create_amount + existential_deposit;
            build_call::<DefaultEnvironment>()
                .call(self.registry)
                .exec_input(
                    ExecutionInput::new(ADD_SELECTOR)
                        .push_arg(admin)
                        .push_arg(validator)
                        .push_arg(pool_create_amount)
                        .push_ard(existential_deposit),
                )
                .transferred_value(transferred_value)
                .returns::<Result<AccountId, RuntimeError>>()
                .invoke()
        }
        fn call_remove_validator(&self, agent: AccountId) -> Result<(), WhitelistError> {
            let mut registry: contract_ref!(Registry) = self.registry.into();
            registry.remove_agent(agent)?;
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
                validators: Vec::new(),
                token_stake_amount: 100000_u128,
                create_deposit: 100_000_000_000_000_u128,
                existential_deposit: 500_u128,
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
        pub fn join_whitelist(
            &mut self,
            id: u128,
            validator: Account,
        ) -> Result<(), WhitelistError> {
            let nft_weight = self.query_weight(id);
            let caller = Self::env().caller();
            if nft_weight < self.token_stake_amount {
                return Err(WhitelistError::InvalidStake);
            }

            if self
                .queued_validators
                .clone()
                .into_iter()
                .find(|p| p.account == caller)
                .is_some()
            {
                return Err(WhitelistError::AlreadyOnList);
            }
            self.transfer_psp34(caller, Self::env().account_id(), id)?;
            self.validators.insert(
                caller,
                Validator {
                    validator: validator,
                    admin: caller,
                    stake: id,
                },
            );
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
        #[ink(message)]
        pub fn init_add_validator(&mut self, validator: AccountId) -> Result<(), WhitelistError> {
            let caller = Self::env().caller();
            if caller != self.admin {
                return Err(WhitelistError::Unauthorized);
            }
            let v = self
                .queued_validators
                .clone()
                .into_iter()
                .find(|p| p.account == validator);

            if v.is_none() {
                return Err(WhitelistError::AlreadyOnList);
            }

            let new_agent = self.call_add_agent(
                v.unwrap().admin,
                v.unwrap().validator,
                self.create_deposit,
                self.existential_deposit,
            );
            self.deployed_validators.insert(new_agent, v.unwrap());
            Ok(())
        }
        #[ink(message)]
        pub fn reject_application(
            &mut self,
            validator: AccountId,
            slash: bool,
        ) -> Result<(), WhitelistError> {
            Ok(())
        }
        #[ink(message)]
        pub fn remove_validator_by_agent(
            &mut self,
            agent: AccountId,
            slash: bool,
        ) -> Result<(), WhitelistError> {
            self.call_remove_validator(validator)?;
            if slash {
                self.transfer_psp34(&Self::env().account_id(), &self.treasury, id)?;
            } else {
                let info = self.deployed_validators.get(agent);
                self.transfer_psp34(info.admin, &Self::env().account_id(), id)?;
            }
            self.deployed_validators.remove(agent);
            Ok(())
        }
    }
}
