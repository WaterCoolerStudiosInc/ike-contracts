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
        registry: AccountId,
        admin: AccountId,
        validator: AccountId,
        pool_id: u32,
        staked: u128,
        unbonding: u128,
        creation_bond: u128,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum RuntimeError {
        CallRuntimeFailed,
        Unauthorized,
        Active,
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
        pub fn new(
            vault: AccountId,
            admin: AccountId,
            validator: AccountId,
            pool_id: u32,
            creation_bond: u128,
            existential_deposit: u128,
        ) -> Self {
            // Mock spending AZERO to create nomination pool
            Self::env().transfer(
                AccountId::from([0u8; 32]),
                creation_bond + existential_deposit,
            ).unwrap();

            RuntimeCaller {
                vault,
                registry: Self::env().caller(),
                admin,
                validator,
                pool_id,
                staked: 0,
                unbonding: 0,
                creation_bond,
            }
        }

        /// need to do a check for minimum value
        ///
        #[ink(message, payable, selector = 1)]
        pub fn deposit(&mut self) -> Result<(), RuntimeError> {
            if Self::env().caller() != self.vault {
                return Err(RuntimeError::Unauthorized);
            }
            self.staked += Self::env().transferred_value();
            return Ok(());
        }

        #[ink(message, selector = 2)]
        pub fn start_unbond(&mut self, amount: u128) -> Result<(), RuntimeError> {
            if Self::env().caller() != self.vault {
                return Err(RuntimeError::Unauthorized);
            }
            self.staked -= amount;
            self.unbonding += amount;
            return Ok(());
        }

        #[ink(message, selector = 3)]
        pub fn withdraw_unbonded(&mut self) -> Result<(), RuntimeError> {
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
        pub fn compound(&mut self, incentive_percentage: u16) -> Result<(Balance, Balance), RuntimeError> {
            let vault = self.vault; // shadow

            if Self::env().caller() != vault {
                return Err(RuntimeError::Unauthorized);
            }

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
            self.unbonding
        }

        #[ink(message, selector = 100)]
        pub fn destroy(&mut self) -> Result<(), RuntimeError> {
            // Stub
            if Self::env().caller() != self.registry {
                return Err(RuntimeError::Unauthorized);
            }
            if self.staked > 0 || self.unbonding > 0 {
                return Err(RuntimeError::Active);
            }
            self.creation_bond = 0;
            Ok(())
        }

        #[ink(message, selector = 101)]
        pub fn admin_withdraw_bond(&mut self, to: AccountId) -> Result<(), RuntimeError> {
            // Stub
            if Self::env().caller() != self.admin {
                return Err(RuntimeError::Unauthorized);
            }
            if self.creation_bond > 0 {
                return Err(RuntimeError::Active);
            }
            // Requires funds are sent via test environment to succeed
            Self::env().transfer(to, Self::env().balance()).unwrap();
            Ok(())
        }
    }
}
