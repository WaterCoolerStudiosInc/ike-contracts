#![cfg_attr(not(feature = "std"), no_std, no_main)]

mod data;

#[ink::contract]
mod nomination_agent {
    use crate::data::{BondExtra, MultiAddress, NominationCall, RuntimeCall};
    use ink::env::Error as EnvError;

    const BIPS: u128 = 10000;

    #[ink(storage)]
    pub struct NominationAgent {
        vault: AccountId,
        admin: AccountId,
        validator: AccountId,
        pool_id: u32,
        staked: u128,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum RuntimeError {
        CallRuntimeFailed,
        Unauthorized,
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
            vault_: AccountId,
            admin_: AccountId,
            validator_: AccountId,
            pool_id_: u32,
            pool_create_amount: Balance,
            existential_deposit: Balance,
        ) -> Self {
            let account_id = Self::env().account_id();

            if Self::env().transferred_value() != pool_create_amount + existential_deposit {
                panic!("Insufficient transferred value");
            }

            let nomination_agent = NominationAgent {
                vault: vault_,
                admin: admin_,
                validator: validator_,
                pool_id: pool_id_,
                staked: pool_create_amount,
            };

            // Create nomination pool
            nomination_agent.env()
                .call_runtime(&RuntimeCall::NominationPools(
                    NominationCall::Create {
                        amount: pool_create_amount,
                        root: MultiAddress::Id(account_id),
                        nominator: MultiAddress::Id(account_id),
                        bouncer: MultiAddress::Id(account_id),
                    }
                )).unwrap();

            // Nominate to validator
            nomination_agent.env()
                .call_runtime(&RuntimeCall::NominationPools(
                    NominationCall::Nominate {
                        pool_id: pool_id_,
                        validators: [validator_].to_vec(),
                    }
                )).unwrap();

            nomination_agent
        }

        #[ink(message, payable, selector = 1)]
        pub fn deposit(&mut self) -> Result<(), RuntimeError> {
            let deposit_amount = Self::env().transferred_value();

            // Restricted to vault
            if Self::env().caller() != self.vault {
                return Err(RuntimeError::Unauthorized);
            }

            if self.staked == 0 {
                // Join nomination pool
                self.env()
                    .call_runtime(&RuntimeCall::NominationPools(
                        NominationCall::Join {
                            amount: deposit_amount,
                            pool_id: self.pool_id,
                        }
                    ))?;
            } else {
                // Bond extra AZERO to nomination pool
                self.env()
                    .call_runtime(&RuntimeCall::NominationPools(
                        NominationCall::BondExtra {
                            extra: BondExtra::FreeBalance {
                                balance: deposit_amount,
                            }
                        }
                    ))?;
            }

            self.staked += deposit_amount;

            Ok(())
        }

        #[ink(message, selector = 2)]
        pub fn start_unbond(&mut self, amount: u128) -> Result<(), RuntimeError> {
            // Restricted to vault
            if Self::env().caller() != self.vault {
                return Err(RuntimeError::Unauthorized);
            }

            self.staked -= amount;

            // Trigger un-bonding process
            self.env()
                .call_runtime(&RuntimeCall::NominationPools(
                    NominationCall::Unbond {
                        member_account: MultiAddress::Id(Self::env().account_id()),
                        unbonding_points: amount,
                    }
                ))?;

            Ok(())
        }

        #[ink(message, selector = 3)]
        pub fn withdraw_unbonded(&mut self) -> Result<(), RuntimeError> {
            let vault = self.vault;

            // Restricted to vault
            if Self::env().caller() != vault {
                return Err(RuntimeError::Unauthorized);
            }

            let before = Self::env().balance();
            if let Err(e) = self.env()
                .call_runtime(&RuntimeCall::NominationPools(
                    NominationCall::WithdrawUnbonded {
                        member_account: MultiAddress::Id(Self::env().account_id()),
                        num_slashing_spans: 1,
                    }
                )) {
                ink::env::debug_println!("Ignoring NominationCall::WithdrawUnbonded error {:?}", e);
                return Ok(());
            };
            let after = Self::env().balance();

            let withdrawn = after - before;

            // Transfer withdrawn AZERO to vault
            if withdrawn > 0 {
                Self::env().transfer(vault, withdrawn)?;
            }

            Ok(())
        }

        #[ink(message, selector = 4)]
        pub fn compound(&mut self, incentive_percentage: u16) -> Result<(Balance, Balance), RuntimeError> {
            let vault = self.vault; // shadow

            // Restricted to vault
            if Self::env().caller() != vault {
                return Err(RuntimeError::Unauthorized);
            }

            // Claim available AZERO
            self.env()
                .call_runtime(&RuntimeCall::NominationPools(
                    NominationCall::ClaimPayout {}
                ))?;

            let balance = Self::env().balance();

            // Gracefully return when nomination pool had nothing to claim
            if balance == 0 {
                return Ok((0, 0));
            }

            let incentive = balance * incentive_percentage as u128 / BIPS;
            let compound_amount = balance - incentive;
            self.staked += compound_amount;

            // Bond AZERO to nomination pool
            self.env()
                .call_runtime(&RuntimeCall::NominationPools(
                    NominationCall::BondExtra {
                        extra: BondExtra::FreeBalance {
                            balance: compound_amount,
                        }
                    }
                ))?;

            // Send incentive AZERO to vault which will handle distribution to caller
            if incentive > 0 {
                Self::env().transfer(vault, incentive)?;
            }

            Ok((compound_amount, incentive))
        }

        #[ink(message, selector = 12)]
        pub fn get_staked_value(&self) -> Balance {
            self.staked
        }

        #[ink(message)]
        pub fn get_vault(&self) -> AccountId {
            self.vault
        }

        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin
        }

        #[ink(message)]
        pub fn get_validator(&self) -> AccountId {
            self.validator
        }

        #[ink(message)]
        pub fn get_pool_id(&self) -> u32 {
            self.pool_id
        }

        #[ink(message, selector = 99)]
        pub fn set_code(&mut self, code_hash: [u8; 32]) -> Result<(), RuntimeError> {
            // Restricted to admin
            if Self::env().caller() != self.admin {
                return Err(RuntimeError::Unauthorized);
            }

            ink::env::set_code_hash(&code_hash)?;
            ink::env::debug_println!("Switched code hash to {:?}.", code_hash);

            Ok(())
        }
    }
}
