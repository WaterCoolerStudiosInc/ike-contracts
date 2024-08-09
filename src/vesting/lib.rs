#![cfg_attr(not(feature = "std"), no_std, no_main)]

mod errors;

#[ink::contract]
pub mod vesting {

    use crate::errors::VestingError;
    use ink::{
        contract_ref,
        env::{Error as InkEnvError},
        prelude::{format, vec::Vec},
        storage::Mapping,
    };
    use psp22::PSP22;

    impl From<InkEnvError> for VestingError {
        fn from(e: InkEnvError) -> Self {
            VestingError::InkEnvError(format!("{:?}", e))
        }
    }

    #[ink(event)]
    pub struct Claim {
        #[ink(topic)]
        recipient: AccountId,
        amount: u128,
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Schedule {
        pub amount: u128,
        pub cliff: u128,
        pub offset: u64,
        pub duration: u64,
    }

    #[ink(storage)]
    pub struct Vesting {
        pub token: AccountId,
        pub admin: Option<AccountId>,
        pub deployment_time: u64,
        pub schedules: Mapping<AccountId, Schedule>,
        pub funding_required: u128,
        pub active: bool,
    }

    impl Vesting {
        #[ink(constructor)]
        pub fn new(
            token: AccountId,
        ) -> Self {
            Self {
                token,
                admin: Some(Self::env().caller()),
                deployment_time: Self::env().block_timestamp(),
                schedules: Mapping::default(),
                funding_required: 0,
                active: false,
            }
        }

        fn token_balance_of(&self, account: AccountId) -> u128 {
            let token: contract_ref!(PSP22) = self.token.into();
            token.balance_of(account)
        }

        fn token_transfer_to(&self, to: AccountId, amount: u128) -> Result<(), VestingError> {
            let mut token: contract_ref!(PSP22) = self.token.into();
            if let Err(e) = token.transfer(to, amount, Vec::new()) {
                return Err(VestingError::TokenError(e));
            }
            Ok(())
        }

        #[ink(message)]
        pub fn get_admin(&self) -> Option<AccountId> {
            self.admin
        }

        #[ink(message)]
        pub fn get_deployment_time(&self) -> u64 {
            self.deployment_time
        }

        #[ink(message)]
        pub fn get_schedule(&self, recipient: AccountId) -> Option<Schedule> {
            self.schedules.get(recipient)
        }

        /// Adds schedules for recipients
        ///
        /// Caller must be the current admin
        /// Can only call before vesting is active
        /// Can only have one schedule per recipient
        #[ink(message)]
        pub fn add_recipients(
            &mut self,
            recipients: Vec<AccountId>,
            schedules: Vec<Schedule>,
        ) -> Result<(), VestingError> {
            let admin = self.admin.ok_or(VestingError::NoAdmin).unwrap();

            if self.env().caller() != admin {
                return Err(VestingError::AdminOnly);
            }

            // Cannot add recipient after activation
            if self.active == true {
                return Err(VestingError::Active);
            }

            if recipients.len() != schedules.len() {
                return Err(VestingError::InvalidInput);
            }

            let mut additional_funding_required = 0u128;

            for (recipient, schedule) in recipients.iter().zip(schedules) {
                if self.schedules.contains(recipient) {
                    return Err(VestingError::RecipientAlreadyExists);
                }

                additional_funding_required += schedule.amount + schedule.cliff;

                self.schedules.insert(recipient, &schedule);
            }

            let new_funding_required = self.funding_required + additional_funding_required;
            let funding = self.token_balance_of(self.env().account_id());

            if new_funding_required > funding {
                return Err(VestingError::InsufficientFunding);
            }

            self.funding_required = new_funding_required;

            Ok(())
        }

        /// Remove schedules for recipients
        ///
        /// Caller must be the current admin
        /// Can only call before vesting is active
        #[ink(message)]
        pub fn remove_recipients(
            &mut self,
            recipients: Vec<AccountId>,
        ) -> Result<(), VestingError> {
            let admin = self.admin.ok_or(VestingError::NoAdmin).unwrap();

            if self.env().caller() != admin {
                return Err(VestingError::AdminOnly);
            }

            // Cannot remove recipient after activation
            if self.active == true {
                return Err(VestingError::Active);
            }

            let mut removed_funding_required = 0u128;

            for recipient in recipients.iter() {
                let schedule = self.schedules.get(recipient)
                    .ok_or(VestingError::RecipientDoesNotExist)
                    .unwrap();

                removed_funding_required += schedule.amount + schedule.cliff;

                self.schedules.remove(recipient);
            }

            self.funding_required -= removed_funding_required;

            Ok(())
        }

        /// Allows claiming of vested tokens
        /// Disables adding/removing recipients
        ///
        /// Caller must be the current admin
        #[ink(message)]
        pub fn activate(
            &mut self,
        ) -> Result<(), VestingError> {
            let admin = self.admin.ok_or(VestingError::NoAdmin).unwrap();

            if self.env().caller() != admin {
                return Err(VestingError::AdminOnly);
            }

            if self.active == true {
                return Err(VestingError::NoChange);
            }

            self.active = true;

            Ok(())
        }

        /// Transfers admin role to another user
        ///
        /// Caller must be the current admin
        #[ink(message)]
        pub fn admin_transfer(
            &mut self,
            to: AccountId,
        ) -> Result<(), VestingError> {
            let admin = self.admin.ok_or(VestingError::NoAdmin).unwrap();

            if self.env().caller() != admin {
                return Err(VestingError::AdminOnly);
            }

            if admin == to {
                return Err(VestingError::NoChange);
            }

            self.admin = Some(to);

            Ok(())
        }

        /// Removes the admin role and effectively disables associated methods:
        ///   * add_recipients()
        ///   * remove_recipients()
        ///   * activate()
        ///   * admin_transfer()
        ///   * admin_abort()
        ///
        /// Caller must be the current admin
        /// Contract must have been activated
        #[ink(message)]
        pub fn admin_relinquish(
            &mut self,
        ) -> Result<(), VestingError> {
            let admin = self.admin.ok_or(VestingError::NoAdmin).unwrap();

            if self.env().caller() != admin {
                return Err(VestingError::AdminOnly);
            }

            if !self.active {
                return Err(VestingError::NotActive);
            }

            self.admin = None;

            Ok(())
        }

        /// Transfers all unclaimed tokens (vested or not) to the admin
        ///
        /// Caller must be the current admin
        /// Can be disabled by removing the admin via `admin_relinquish()`
        #[ink(message)]
        pub fn admin_abort(
            &mut self,
        ) -> Result<(), VestingError> {
            let admin = self.admin.ok_or(VestingError::NoAdmin).unwrap();

            if self.env().caller() != admin {
                return Err(VestingError::AdminOnly);
            }

            let balance = self.token_balance_of(self.env().account_id());

            self.token_transfer_to(admin, balance)?;

            Ok(())
        }

        #[ink(message)]
        pub fn claim(
            &mut self,
        ) -> Result<u128, VestingError> {
            let now = self.env().block_timestamp();
            let recipient = self.env().caller();

            // Vesting must have been activated
            if self.active == false {
                return Err(VestingError::NotActive);
            }

            let mut schedule = self.schedules.get(recipient)
                .ok_or(VestingError::RecipientDoesNotExist)
                .unwrap();

            let start = self.deployment_time + schedule.offset;
            let end = start + schedule.duration;

            // Ensure vesting schedule has begun
            if now < start {
                return Err(VestingError::TooEarly);
            }

            let mut payable: u128 = 0;

            // Vest cliff if not already vested
            if schedule.cliff > 0 {
                payable += schedule.cliff;
                schedule.cliff = 0;
            }

            if now < end {
                // Vest amount proportional to elapsed time
                let time_elapsed = now - start;
                let amount_proportional = time_elapsed as u128 * schedule.amount / schedule.duration as u128;
                payable += amount_proportional;
                schedule.amount -= amount_proportional;
                schedule.offset += time_elapsed;
                schedule.duration -= time_elapsed;
            } else {
                // Vest full remaining amount
                payable += schedule.amount;
                schedule.amount = 0;
                schedule.offset += schedule.duration;
                schedule.duration = 0;
            }

            if payable == 0 {
                return Err(VestingError::NoChange);
            }

            self.schedules.insert(recipient, &schedule);

            self.token_transfer_to(recipient, payable)?;

            Self::env().emit_event(
                Claim {
                    recipient,
                    amount: payable,
                }
            );

            Ok(payable)
        }
    }
}
