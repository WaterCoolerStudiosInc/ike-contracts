#![cfg_attr(not(feature = "std"), no_std, no_main)]

mod data;
pub mod errors;
pub mod traits;

#[ink::contract]
pub mod nomination_agent {
    use crate::data::{MultiAddress, RewardDestination, RuntimeCall, StakingCall};
    use crate::errors::RuntimeError;
    use crate::traits::INominationAgent;
    use ink::env::Error as EnvError;

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
        #[ink(constructor, payable)]
        pub fn new(
            vault: AccountId,
            admin: AccountId,
            validator: AccountId,
        ) -> Self {
            let creation_bond = Self::env().transferred_value();

            let nomination_agent = NominationAgent {
                vault,
                registry: Self::env().caller(),
                admin,
                validator,
                staked: 0,
                unbonding: 0,
                creation_bond,
            };

            nomination_agent
                .env()
                .call_runtime(&RuntimeCall::Staking(StakingCall::Bond {
                    value: creation_bond,
                    payee: RewardDestination::Stash,
                }))
                .unwrap();

            nomination_agent
                .env()
                .call_runtime(&RuntimeCall::Staking(StakingCall::Nominate {
                    targets: [MultiAddress::Id(validator)].to_vec(),
                }))
                .unwrap();

            nomination_agent
        }
    }

    impl INominationAgent for NominationAgent {
        #[ink(message, payable, selector = 1)]
        fn deposit(&mut self) -> Result<(), RuntimeError> {
            let deposit_amount = Self::env().transferred_value();

            // Restricted to vault
            if Self::env().caller() != self.vault {
                return Err(RuntimeError::Unauthorized);
            }

            self.staked += deposit_amount;

            // Bond extra AZERO
            self.env()
                .call_runtime(&RuntimeCall::Staking(StakingCall::BondExtra {
                    max_additional: deposit_amount,
                }))?;

            Ok(())
        }

        #[ink(message, selector = 2)]
        fn start_unbond(&mut self, amount: u128) -> Result<(), RuntimeError> {
            // Restricted to vault
            if Self::env().caller() != self.vault {
                return Err(RuntimeError::Unauthorized);
            }

            self.staked -= amount;
            self.unbonding += amount;

            let balance_before = Self::env().balance();

            // Unbond AZERO
            self.env()
                .call_runtime(&RuntimeCall::Staking(StakingCall::Unbond {
                    value: amount,
                }))?;

            let withdrawn = Self::env().balance() - balance_before;

            if withdrawn > 0 {
                // Typically this should be 0
                // If unlocking requests equal `staking.maxUnlockingChunks`, some might be withdrawn
                ink::env::debug_println!("Withdrawn {:?} AZERO", withdrawn);
                self.unbonding -= withdrawn;
                Self::env().transfer(self.vault, withdrawn)?;
            }

            Ok(())
        }

        #[ink(message, selector = 3)]
        fn withdraw_unbonded(&mut self) -> Result<(), RuntimeError> {
            let vault = self.vault; // shadow

            // Restricted to vault
            if Self::env().caller() != vault {
                return Err(RuntimeError::Unauthorized);
            }

            let balance_before = Self::env().balance();

            if let Err(e) = self.env().call_runtime(&RuntimeCall::Staking(
                StakingCall::WithdrawUnbonded {
                    num_slashing_spans: 0,
                },
            )) {
                ink::env::debug_println!("Ignoring StakingCall::WithdrawUnbonded error {:?}", e);
                return Ok(());
            };

            let withdrawn = Self::env().balance() - balance_before;
            ink::env::debug_println!("Withdrawn {:?} AZERO", withdrawn);

            // Transfer withdrawn AZERO to vault
            if withdrawn > 0 {
                self.unbonding -= withdrawn;
                Self::env().transfer(vault, withdrawn)?;
            }

            Ok(())
        }

        #[ink(message, selector = 4)]
        fn compound(&mut self) -> Result<Balance, RuntimeError> {
            // Restricted to vault
            if Self::env().caller() != self.vault {
                return Err(RuntimeError::Unauthorized);
            }

            let balance_before = Self::env().balance();

            // Gracefully return when no funds are available
            if balance_before == 0 {
                return Ok(0);
            }

            // Attempt bonding
            self.env()
                .call_runtime(&RuntimeCall::Staking(StakingCall::BondExtra {
                    max_additional: balance_before,
                }))
                .ok();

            let compounded = balance_before - Self::env().balance();

            if compounded > 0 {
                self.staked += compounded;
            }

            Ok(compounded)
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

        /// Step 1 of 2 in finalizing the agent's lifecycle
        /// Performs the following actions:
        ///     1) Removes the validator nomination
        ///     2) Begins unbonding the initial bond
        ///
        /// Can only be called by registry
        /// Must have no protocol funds staked
        /// Must have no protocol funds unbonding
        #[ink(message, selector = 101)]
        fn destroy(&mut self) -> Result<(), RuntimeError> {
            // Restricted to registry
            if Self::env().caller() != self.registry {
                return Err(RuntimeError::Unauthorized);
            }

            if self.staked > 0 || self.unbonding > 0 {
                return Err(RuntimeError::Active);
            }

            // Chill
            self.env()
                .call_runtime(&RuntimeCall::Staking(StakingCall::Chill))?;

            // Unbond initial bond
            self.env()
                .call_runtime(&RuntimeCall::Staking(StakingCall::Unbond {
                    value: self.creation_bond,
                }))?;

            self.creation_bond = 0;

            Ok(())
        }

        /// Step 2 of 2 in finalizing the agent's lifecycle
        /// Performs the following actions:
        ///     1) Withdraws the (now unbonded) initial bond
        ///     2) Transfers the initial bond to any account of choice
        ///
        /// Can only be called by admin
        /// Must be called after `destroy()`
        #[ink(message, selector = 102)]
        fn admin_withdraw_bond(&mut self, to: AccountId) -> Result<u128, RuntimeError> {
            // Restricted to admin
            if Self::env().caller() != self.admin {
                return Err(RuntimeError::Unauthorized);
            }

            if self.creation_bond > 0 {
                return Err(RuntimeError::Active);
            }

            self.env()
                .call_runtime(&RuntimeCall::Staking(StakingCall::WithdrawUnbonded {
                    num_slashing_spans: 0,
                }))
                .ok();

            let balance = Self::env().balance();

            Self::env().transfer(to, balance)?;

            Ok(balance)
        }

        /// Allows the Registry to effectively "upgrade" the contract logic
        ///
        /// Can only be called by registry
        #[ink(message, selector = 999)]
        fn set_code(&mut self, code_hash: [u8; 32]) -> Result<(), RuntimeError> {
            // Restricted to registry
            if Self::env().caller() != self.registry {
                return Err(RuntimeError::Unauthorized);
            }

            ink::env::set_code_hash(&code_hash)?;

            Ok(())
        }
    }
}
