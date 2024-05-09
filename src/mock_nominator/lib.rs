#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod mock_nominator {
    use ink::env::Error as EnvError;

    const BIPS: u128 = 10000;

    /// A trivial contract with a single message, that uses `call-runtime` API
    /// for performing native token transfer.
    #[ink(storage)]
    pub struct RuntimeCaller {
        vault: AccountId,
        mock_fail: bool,
        staked: u128,
        unbonded: u128,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum RuntimeError {
        CallRuntimeFailed,
        Unauthorized,
        InvalidWithdraw,
        InsufficientFunds,
    }

    impl From<EnvError> for RuntimeError {
        fn from(e: EnvError) -> Self {
            match e {
                EnvError::CallRuntimeFailed => RuntimeError::CallRuntimeFailed,
                _ => panic!("Unexpected error from `pallet-contracts`."),
            }
        }
    }

    impl RuntimeCaller {
        /// The constructor is `payable`, so that during instantiation it can be
        /// given some tokens that will be further transferred with
        /// `transfer_through_runtime` message.
        #[ink(constructor, payable)]
        pub fn new(vault_: AccountId, mock_fail_: bool) -> Self {
            RuntimeCaller {
                vault: vault_,
                mock_fail: mock_fail_,
                staked: 0,
                unbonded: 0,
            }
        }

        /// need to do a check for minimum value
        ///
        #[ink(message, payable, selector = 1)]
        pub fn deposit(&mut self) -> Result<(), RuntimeError> {
            if Self::env().caller() != self.vault {
                return Err(RuntimeError::Unauthorized);
            }
            if self.mock_fail {
                return Err(RuntimeError::CallRuntimeFailed);
            } else {
                self.staked += Self::env().transferred_value();
                return Ok(());
            }
        }

        #[ink(message, selector = 2)]
        pub fn start_unbond(&mut self, amount: u128) -> Result<(), RuntimeError> {
            if Self::env().caller() != self.vault {
                return Err(RuntimeError::Unauthorized);
            }
            if self.mock_fail {
                return Err(RuntimeError::CallRuntimeFailed);
            } else {
                self.staked -= amount;
                self.unbonded += amount;
                return Ok(());
            }
        }
        #[ink(message, selector = 3)]
        pub fn withdraw_unbonded(&mut self) -> Result<(), RuntimeError> {
            if Self::env().caller() != self.vault {
                return Err(RuntimeError::Unauthorized);
            }
            if self.mock_fail {
                return Err(RuntimeError::CallRuntimeFailed);
            } else {
                if self.unbonded > 0 {
                    Self::env().transfer(self.vault, self.unbonded)?;
                    self.unbonded = 0;
                }

                return Ok(());
            }
        }
        #[ink(message, selector = 4)]
        pub fn compound(&mut self, incentive_percentage: u16) -> Result<(Balance, Balance), RuntimeError> {
            let vault = self.vault; // shadow

            if Self::env().caller() != vault {
                return Err(RuntimeError::Unauthorized);
            }

            if self.mock_fail {
                return Err(RuntimeError::CallRuntimeFailed);
            } else {
                let balance = Self::env().balance();

                // Gracefully return when nomination pool had nothing to claim
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
        }
        #[ink(message, payable, selector = 5)]
        pub fn add_stake(&mut self) -> Balance {
            self.staked += Self::env().transferred_value();
            self.staked
        }
        #[ink(message, payable)]
        pub fn remove_stake(&mut self, amount: u128) -> Result<Balance, RuntimeError> {
            self.staked -= amount;
            Self::env().transfer(Self::env().caller(), amount)?;
            Ok(self.staked)
        }
        #[ink(message, selector = 12)]
        pub fn get_staked_value(&self) -> Balance {
            self.staked
        }
        #[ink(message, selector = 13)]
        pub fn get_unbonded_value(&self) -> Balance {
            self.unbonded
        }
    }
}
