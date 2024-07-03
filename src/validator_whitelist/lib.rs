//
#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod validator_whitelist {
    use ink::{
        env::{debug_println, DefaultEnvironment},
        prelude::{string::String, vec::Vec},
        storage::Mapping,
    };
    use psp22::{PSP22Error, PSP22};
    use psp34::{Id, PSP34Error, PSP34};
    use registry::{registry::RegistryError, Registry};

    #[ink(storage)]
    pub struct ValidatorWhitelist {
        admin: AccountId,
        registry: AccountId,
        gov_nft: AccountId,
        validators: Vec<AccountId>,
        token_stake_amount: u128,
        create_deposit: u128,
    }
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum WhitelistError {
        Invalid,
        Unauthorized,
        InvalidTimeWindow,
        NFTError(PSP34Error),
        TokenError(PSP22Error),
    }

    impl ValidatorWhitelist {
        fn transfer_psp34(
            &self,
            from: &AccountId,
            to: &AccountId,
            amount: Balance,
        ) -> Result<(), WhitelistError> {
            let mut token: contract_ref!(PSP34) = self.gov_nft.into();
            if let Err(e) = token.transfer_from(*from, *to, amount, Vec::new()) {
                return Err(WhitelistError::TokenError(e));
            }
            Ok(())
        }
        #[ink(constructor)]
        pub fn new(
            _admin: AccountId,
            _registry: AccountId,
            _gov_nft: AccountId,
            treasury: AccountId,
        ) -> Self {
            Self {
                admin: _admin,
                treasury: treasury,
                registry: _registry,
                gov_nft: _gov_nft,
                validators: Vec::new(),
                token_stake_amount: 100000_u128,
                create_deposit: 100000_u128,
            }
        }
        #[ink(message)]
        pub fn join_whitelist(&mut self) -> Result<(), WhitelistError> {
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
        // Step 2. Initialize Agent call with poolid and Account
        #[ink(message)]
        pub fn init_add_validator(&mut self, validator: AccountId) -> Result<(), WhitelistError> {
            Ok(())
        }
        #[ink(message)]
        pub fn finalize_add_validator(
            &mut self,
            validator: AccountId,
        ) -> Result<(), WhitelistError> {
            Ok(())
        }
        #[ink(message)]
        pub fn remove_validator(
            &mut self,
            validator: AccountId,
            slash: bool,
        ) -> Result<(), WhitelistError> {
            Ok(())
        }
    }
}
