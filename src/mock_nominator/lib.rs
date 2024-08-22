#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub mod errors;
pub mod traits;

#[ink::contract]
mod mock_nominator {
    use crate::errors::RuntimeError;
    use crate::traits::INominationAgent;
    use ink::env::Error as EnvError;

    const BIPS: u128 = 10000;

    #[ink(storage)]
    pub struct NominationAgent {
        vault: AccountId,
        registry: AccountId,
        admin: AccountId,
        validator: AccountId,
        staked: u128,
        unbonding: u128,
        creation_bond: u128,
    }

    impl From<EnvError> for RuntimeError {
        fn from(e: EnvError) -> Self {
            match e {
                EnvError::CallRuntimeFailed => RuntimeError::CallRuntimeFailed,
                _ => panic!("Unexpected error from `pallet-contracts`."),
            }
        }
    }

    impl NominationAgent {
        #[ink(constructor)]
        pub fn deploy_hash() -> Self {
            let account_id = Self::env().account_id();
            Self {
                vault: account_id,
                registry: account_id,
                admin: account_id,
                validator: account_id,
                staked: 0,
                unbonding: 0,
                creation_bond: 0,
            }
        }

        #[ink(constructor, payable)]
        pub fn new(
            vault: AccountId,
            admin: AccountId,
            validator: AccountId,
            creation_bond: u128,
            existential_deposit: u128,
        ) -> Self {
            // Mock spending AZERO to create agent
            Self::env().transfer(
                AccountId::from([0u8; 32]),
                creation_bond + existential_deposit,
            ).unwrap();

            Self {
                vault,
                registry: Self::env().caller(),
                admin,
                validator,
                staked: 0,
                unbonding: 0,
                creation_bond,
            }
        }
    }

    impl INominationAgent for NominationAgent {
        #[ink(message, payable, selector = 1)]
        fn deposit(&mut self) -> Result<(), RuntimeError> {
            if Self::env().caller() != self.vault {
                return Err(RuntimeError::Unauthorized);
            }
            self.staked += Self::env().transferred_value();
            return Ok(());
        }

        #[ink(message, selector = 2)]
        fn start_unbond(&mut self, amount: u128) -> Result<(), RuntimeError> {
            if Self::env().caller() != self.vault {
                return Err(RuntimeError::Unauthorized);
            }
            self.staked -= amount;
            self.unbonding += amount;
            return Ok(());
        }

        #[ink(message, selector = 3)]
        fn withdraw_unbonded(&mut self) -> Result<(), RuntimeError> {
            if Self::env().caller() != self.vault {
                return Err(RuntimeError::Unauthorized);
            }
            if self.unbonding > 0 {
                Self::env().transfer(self.vault, self.unbonding)?;
                self.unbonding = 0;
            }
            return Ok(());
        }

        #[ink(message, selector = 4)]
        fn compound(&mut self, incentive_percentage: u16) -> Result<(Balance, Balance), RuntimeError> {
            let vault = self.vault; // shadow

            if Self::env().caller() != vault {
                return Err(RuntimeError::Unauthorized);
            }

            let balance = Self::env().balance();

            // Gracefully return when nomination agent has no rewards
            if balance == 0 {
                return Ok((0, 0));
            }

            let incentive = balance * incentive_percentage as u128 / BIPS;
            let compound_amount = balance - incentive;
            self.staked += compound_amount;

            if incentive > 0 {
                Self::env().transfer(vault, incentive)?;
            }

            Ok((compound_amount, incentive))
        }

        #[ink(message, selector = 12)]
        fn get_staked_value(&self) -> Balance {
            self.staked
        }

        #[ink(message, selector = 13)]
        fn get_unbonding_value(&self) -> Balance {
            self.unbonding
        }

        #[ink(message)]
        fn get_vault(&self) -> AccountId {
            self.vault
        }

        #[ink(message)]
        fn get_admin(&self) -> AccountId {
            self.admin
        }

        #[ink(message)]
        fn get_validator(&self) -> AccountId {
            self.validator
        }

        #[ink(message, selector = 101)]
        fn destroy(&mut self) -> Result<(), RuntimeError> {
            if Self::env().caller() != self.registry {
                return Err(RuntimeError::Unauthorized);
            }
            if self.staked > 0 || self.unbonding > 0 {
                return Err(RuntimeError::Active);
            }
            self.creation_bond = 0;
            Ok(())
        }

        #[ink(message, selector = 102)]
        fn admin_withdraw_bond(&mut self, to: AccountId) -> Result<u128, RuntimeError> {
            if Self::env().caller() != self.admin {
                return Err(RuntimeError::Unauthorized);
            }
            if self.creation_bond > 0 {
                return Err(RuntimeError::Active);
            }
            // Requires funds are sent via test environment to succeed
            let balance = Self::env().balance();
            Self::env().transfer(to, balance).unwrap();
            Ok(balance)
        }

        #[ink(message, selector = 999)]
        fn set_code(&mut self, code_hash: [u8; 32]) -> Result<(), RuntimeError> {
            if Self::env().caller() != self.registry {
                return Err(RuntimeError::Unauthorized);
            }
            ink::env::set_code_hash(&code_hash)?;
            Ok(())
        }
    }
}
