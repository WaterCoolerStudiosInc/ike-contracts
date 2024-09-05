#![cfg_attr(not(feature = "std"), no_std, no_main)]
mod data;
pub mod errors;
mod nomination_agent_utils;
pub mod traits;

#[ink::contract]
mod vault {
    use crate::data::*;
    use crate::errors::VaultError;
    use crate::traits::*;

    use ink::{
        codegen::EmitEvent,
        contract_ref,
        env::Error as InkEnvError,
        prelude::{format, string::String, vec::Vec},
        reflect::ContractEventBase,
        ToAccountId,
    };
    use psp22::{PSP22Burnable, PSP22};
    use registry::RegistryRef;
    use share_token::{ShareToken, TokenRef};

    /// Errors returned by the contract's methods.
    impl From<InkEnvError> for VaultError {
        fn from(e: InkEnvError) -> Self {
            VaultError::InkEnvError(format!("{:?}", e))
        }
    }

    /// Alias for wrapper around all events in this contract generated by ink!.
    type Event = <Vault as ContractEventBase>::Type;

    #[ink(event)]
    pub struct Staked {
        #[ink(topic)]
        staker: AccountId,
        azero: Balance,
        new_shares: u128,
        virtual_shares: u128,
    }
    #[ink(event)]
    pub struct Referral {
        #[ink(topic)]
        referral_id: AccountId,
        staker: AccountId,
        azero: Balance,
    }
    #[ink(event)]
    pub struct Compounded {
        caller: AccountId,
        azero: Balance,
        incentive: Balance,
        virtual_shares: u128,
    }
    #[ink(event)]
    pub struct UnlockRequested {
        #[ink(topic)]
        staker: AccountId,
        unlock_id: u128,
        shares: u128,
        azero: u128,
        virtual_shares: u128,
    }
    #[ink(event)]
    pub struct UnlockRedeemed {
        #[ink(topic)]
        staker: AccountId,
        azero: u128,
        unlock_id: u64,
    }
    #[ink(event)]
    pub struct FeesWithdrawn {
        shares: u128,
    }
    #[ink(event)]
    pub struct FeesAdjusted {
        new_fee: u16,
        virtual_shares: u128,
    }
    #[ink(event)]
    pub struct IncentiveAdjusted {
        new_incentive: u16,
    }
    #[ink(event)]
    pub struct RoleAdjustFeeTransferred {
        new_account: AccountId,
    }
    #[ink(event)]
    pub struct RoleFeeToTransferred {
        new_account: AccountId,
    }
    #[ink(event)]
    pub struct NewHash {
        code_hash: [u8; 32],
    }
    #[ink(event)]
    pub struct SetHashDisabled {}

    #[ink(storage)]
    pub struct Vault {
        pub data: VaultData,
    }

    impl Vault {
        #[ink(constructor)]
        pub fn new(
            share_token_hash: Hash,
            registry_code_hash: Hash,
            nomination_agent_hash: Hash,
        ) -> Self {
            Self::custom_era(
                share_token_hash,
                registry_code_hash,
                nomination_agent_hash,
                DAY,
            )
        }

        #[ink(constructor)]
        pub fn custom_era(
            share_token_hash: Hash,
            registry_code_hash: Hash,
            nomination_agent_hash: Hash,
            era: u64,
        ) -> Self {
            let caller = Self::env().caller();
            let now = Self::env().block_timestamp();

            let registry_ref =
                RegistryRef::new(caller, caller, caller, caller, nomination_agent_hash)
                    .endowment(0)
                    .code_hash(registry_code_hash)
                    .salt_bytes(&[9_u8.to_le_bytes().as_ref(), caller.as_ref()].concat()[..4])
                    .instantiate();
            let share_token_ref = TokenRef::new(
                Some(String::from("Ike Liquid Staked AZERO")),
                Some(String::from("sA0")),
            )
            .endowment(0)
            .code_hash(share_token_hash)
            .salt_bytes(&[7_u8.to_le_bytes().as_ref(), caller.as_ref()].concat()[..4])
            .instantiate();

            Self {
                data: VaultData::new(
                    caller,
                    TokenRef::to_account_id(&share_token_ref),
                    registry_ref,
                    now,
                    era,
                ),
            }
        }

        fn emit_event<EE>(emitter: EE, event: Event)
        where
            EE: EmitEvent<Vault>,
        {
            emitter.emit_event(event);
        }

        fn transfer_shares_from(
            &self,
            from: &AccountId,
            to: &AccountId,
            amount: u128,
        ) -> Result<(), VaultError> {
            let mut token: contract_ref!(PSP22) = self.data.shares_contract.into();
            if let Err(e) = token.transfer_from(*from, *to, amount, Vec::new()) {
                return Err(VaultError::TokenError(e));
            }
            Ok(())
        }

        fn mint_shares(&mut self, amount: u128, to: AccountId) -> Result<(), VaultError> {
            let mut token: contract_ref!(ShareToken) = self.data.shares_contract.into();
            self.data.total_shares_minted += amount;
            if let Err(e) = token.mint(to, amount) {
                return Err(VaultError::TokenError(e));
            }
            Ok(())
        }

        fn burn_shares(&mut self, amount: u128) -> Result<(), VaultError> {
            let mut token: contract_ref!(PSP22Burnable) = self.data.shares_contract.into();
            self.data.total_shares_minted -= amount;
            if let Err(e) = token.burn(amount) {
                return Err(VaultError::TokenError(e));
            }
            Ok(())
        }
    }

    impl RateProvider for Vault {
        /// Calculate the value of sA0 shares in terms of AZERO with TARGET_DECIMALS precision
        #[ink(message)]
        fn get_rate(&mut self) -> u128 {
            // Because both RATE_DECIMALS and sA0.decimals() are 12,
            // no further adjustment is necessary
            self.get_azero_from_shares(1e12 as u128)
        }
    }

    impl IVault for Vault {
        /// Allow users to convert AZERO into sA0
        /// Mints the caller sA0 based on the redemption ratio
        ///
        /// Minimum AZERO amount is required to stake
        /// AZERO must be transferred via transferred_value
        #[ink(message, payable)]
        fn stake(&mut self) -> Result<Balance, VaultError> {
            let caller = Self::env().caller();
            let azero = Self::env().transferred_value();

            // Verify minimum AZERO is being staked
            if azero < 1_000_000 {
                return Err(VaultError::MinimumStake);
            }

            // Update fees before calculating redemption ratio and minting shares
            self.data.update_fees(Self::env().block_timestamp());

            let new_shares = self.get_shares_from_azero(azero);
            self.mint_shares(new_shares, caller)?;

            self.data.delegate_bonding(azero)?;

            Self::emit_event(
                Self::env(),
                Event::Staked(Staked {
                    staker: caller,
                    azero,
                    new_shares,
                    virtual_shares: self.data.total_shares_virtual, // updated in update_fees()
                }),
            );

            Ok(new_shares)
        }

        #[ink(message, payable)]
        fn stake_with_referral(&mut self, referral_id: AccountId) -> Result<Balance, VaultError> {
            let new_shares = self.stake()?;
            Self::emit_event(
                Self::env(),
                Event::Referral(Referral {
                    referral_id,
                    staker: Self::env().caller(),
                    azero: Self::env().transferred_value(),
                }),
            );
            Ok(new_shares)
        }

        /// Allow user to begin the unlock process converting shares into AZERO
        ///
        /// Transfers `shares` to the vault contract
        /// Calculates AZERO value of shares
        /// Creates `UnlockRequest` for the user
        /// Delegates unbonding of the associated AZERO
        /// Burns the associated shares tokens
        #[ink(message)]
        fn request_unlock(&mut self, shares: u128) -> Result<(), VaultError> {
            let caller = Self::env().caller();
            let now = Self::env().block_timestamp();

            self.transfer_shares_from(&caller, &Self::env().account_id(), shares)?;

            // Update fees before calculating redemption ratio and burning shares
            self.data.update_fees(now);

            let azero = self.get_azero_from_shares(shares);

            // Update user's unlock requests
            let mut user_unlock_requests = self
                .data
                .user_unlock_requests
                .get(caller)
                .unwrap_or_default();
            user_unlock_requests.push(UnlockRequest {
                creation_time: now,
                azero,
            });
            self.data
                .user_unlock_requests
                .insert(caller, &user_unlock_requests);

            // Allocate unlock quantity across nomination pools
            self.data.delegate_unbonding(azero)?;

            self.burn_shares(shares)?;

            Self::emit_event(
                Self::env(),
                Event::UnlockRequested(UnlockRequested {
                    staker: caller,
                    unlock_id: (user_unlock_requests.len() - 1) as u128,
                    shares,
                    azero,
                    virtual_shares: self.data.total_shares_virtual, // updated in update_fees()
                }),
            );

            Ok(())
        }

        /// Attempts to claim unbonded AZERO from all validators
        #[ink(message)]
        fn delegate_withdraw_unbonded(&mut self) -> Result<(), VaultError> {
            self.data.delegate_withdraw_unbonded()?;

            Ok(())
        }

        /// Allows a user to withdraw staked AZERO
        ///
        /// Returns original deposit amount plus interest to depositor address
        /// Queries the redeemable amount by user AccountId and Claim Vector index
        /// Associated unlock request must have been completed
        /// Deletes the user's unlock request
        #[ink(message)]
        fn redeem(&mut self, user: AccountId, unlock_id: u64) -> Result<(), VaultError> {
            let now = Self::env().block_timestamp();

            let mut user_unlock_requests =
                self.data.user_unlock_requests.get(user).unwrap_or_default();

            // Ensure user specified a valid unlock request index
            if unlock_id >= user_unlock_requests.len() as u64 {
                return Err(VaultError::InvalidUserUnlockRequest);
            }

            let creation_time = user_unlock_requests[unlock_id as usize].creation_time;
            let azero = user_unlock_requests[unlock_id as usize].azero;

            // Ensure unbond has completed
            if now < creation_time + self.data.cooldown_period {
                return Err(VaultError::CooldownPeriod);
            }

            // Delete completed user unlock request
            user_unlock_requests.remove(unlock_id as usize);
            self.data
                .user_unlock_requests
                .insert(user, &user_unlock_requests);

            // Send AZERO to user
            Self::env().transfer(user, azero)?;

            Self::emit_event(
                Self::env(),
                Event::UnlockRedeemed(UnlockRedeemed {
                    staker: user,
                    azero,
                    unlock_id,
                }),
            );

            Ok(())
        }

        /// Alternative method for a user to withdraw staked AZERO
        ///
        /// This should be called instead of `redeem()` when insufficient AZERO exists in the Vault and
        /// validator(s) have unbonded AZERO which can be claimed
        #[ink(message)]
        fn redeem_with_withdraw(
            &mut self,
            user: AccountId,
            unlock_id: u64,
        ) -> Result<(), VaultError> {
            // Claim all unbonded AZERO into Vault
            self.data.delegate_withdraw_unbonded()?;

            self.redeem(user, unlock_id)?;

            Ok(())
        }

        /// Compound earned interest for all validators
        ///
        /// Can be called by anyone
        /// Caller receives an AZERO incentive based on the total AZERO amount compounded
        #[ink(message)]
        fn compound(&mut self) -> Result<Balance, VaultError> {
            let caller = Self::env().caller();

            // Delegate compounding to all agents
            let (compounded, incentive) = self.data.delegate_compound()?;

            // Send AZERO incentive to caller
            if incentive > 0 {
                Self::env().transfer(caller, incentive)?;
            }

            Self::emit_event(
                Self::env(),
                Event::Compounded(Compounded {
                    caller,
                    azero: compounded,
                    incentive,
                    virtual_shares: self.get_current_virtual_shares(),
                }),
            );

            Ok(incentive)
        }

        /// Claim fees by inflating sA0 supply
        ///
        /// Caller must have the fee to role (`role_fee_to`)
        /// Mints virtual shares as sA0 to the caller
        /// Effectively serves as a compounding for protocol fee
        /// sets total_shares_virtual to 0
        #[ink(message)]
        fn withdraw_fees(&mut self) -> Result<(), VaultError> {
            let caller = Self::env().caller();
            let now = Self::env().block_timestamp();
            let role_fee_to = self.data.role_fee_to; // shadow

            if caller != role_fee_to {
                return Err(VaultError::InvalidPermissions);
            }

            self.data.update_fees(now);

            let shares = self.data.total_shares_virtual;
            self.mint_shares(shares, role_fee_to)?;
            self.data.total_shares_virtual = 0;

            Self::emit_event(Self::env(), Event::FeesWithdrawn(FeesWithdrawn { shares }));

            Ok(())
        }

        /// Upgrade the contract by the ink env set_code_hash function
        ///
        /// The set code role (`role_set_code`) must be set
        /// Caller must have the set code role (`role_set_code`)
        /// See ink documentation for details https://paritytech.github.io/ink/ink_env/fn.set_code_hash.html
        #[ink(message)]
        fn set_code(&mut self, code_hash: [u8; 32]) -> Result<(), VaultError> {
            let caller = Self::env().caller();
            let role_set_code = self.data.role_set_code; // shadow

            if role_set_code.is_none() || caller != role_set_code.unwrap() {
                return Err(VaultError::InvalidPermissions);
            }

            ink::env::set_code_hash(&code_hash)?;

            Self::emit_event(Self::env(), Event::NewHash(NewHash { code_hash }));

            Ok(())
        }

        #[ink(message)]
        fn disable_set_code(&mut self) -> Result<(), VaultError> {
            let caller = Self::env().caller();
            let role_set_code = self.data.role_set_code; // shadow

            if role_set_code.is_none() {
                return Err(VaultError::NoChange);
            }
            if caller != role_set_code.unwrap() {
                return Err(VaultError::InvalidPermissions);
            }

            self.data.role_set_code = None;

            Self::emit_event(Self::env(), Event::SetHashDisabled(SetHashDisabled {}));

            Ok(())
        }

        /// Update the protocol fee
        ///
        /// Caller must have the adjust fee role (`role_adjust_fee`)
        /// Updates the total_shares_virtual accumulator at the old fee level first
        #[ink(message, selector = 13)]
        fn adjust_fee(&mut self, new_fee: u16) -> Result<(), VaultError> {
            let caller = Self::env().caller();
            let now = Self::env().block_timestamp();

            if caller != self.data.role_adjust_fee {
                return Err(VaultError::InvalidPermissions);
            }
            if self.data.fee_percentage == new_fee {
                return Err(VaultError::NoChange);
            }
            if new_fee >= BIPS {
                return Err(VaultError::InvalidPercent);
            }

            self.data.update_fees(now);
            self.data.fee_percentage = new_fee;

            Self::emit_event(
                Self::env(),
                Event::FeesAdjusted(FeesAdjusted {
                    new_fee,
                    virtual_shares: self.data.total_shares_virtual, // updated in update_fees()
                }),
            );

            Ok(())
        }

        /// Update the compound incentive
        ///
        /// Caller must have the adjust fee role (`role_adjust_fee`)
        #[ink(message)]
        fn adjust_incentive(&mut self, new_incentive: u16) -> Result<(), VaultError> {
            let caller = Self::env().caller();

            if caller != self.data.role_adjust_fee {
                return Err(VaultError::InvalidPermissions);
            }
            if self.data.incentive_percentage == new_incentive {
                return Err(VaultError::NoChange);
            }
            if new_incentive >= BIPS {
                return Err(VaultError::InvalidPercent);
            }

            self.data.incentive_percentage = new_incentive;

            Self::emit_event(
                Self::env(),
                Event::IncentiveAdjusted(IncentiveAdjusted { new_incentive }),
            );

            Ok(())
        }

        #[ink(message)]
        fn get_role_adjust_fee(&self) -> AccountId {
            self.data.role_adjust_fee
        }

        /// Transfers adjust fee role to a new account
        ///
        /// Caller must have the adjust fee role (`role_adjust_fee`)
        #[ink(message)]
        fn transfer_role_adjust_fee(&mut self, new_account: AccountId) -> Result<(), VaultError> {
            let caller = Self::env().caller();
            let role_adjust_fee = self.data.role_adjust_fee; // shadow

            if caller != role_adjust_fee {
                return Err(VaultError::InvalidPermissions);
            }
            if role_adjust_fee == new_account {
                return Err(VaultError::NoChange);
            }

            self.data.role_adjust_fee = new_account;

            Self::emit_event(
                Self::env(),
                Event::RoleAdjustFeeTransferred(RoleAdjustFeeTransferred { new_account }),
            );

            Ok(())
        }

        #[ink(message)]
        fn get_role_fee_to(&self) -> AccountId {
            self.data.role_fee_to
        }

        /// Transfers fee to role to a new account
        ///
        /// Caller must have the fee to role (`role_fee_to`)
        #[ink(message)]
        fn transfer_role_fee_to(&mut self, new_account: AccountId) -> Result<(), VaultError> {
            let caller = Self::env().caller();
            let role_fee_to = self.data.role_fee_to; // shadow

            if caller != role_fee_to {
                return Err(VaultError::InvalidPermissions);
            }
            if role_fee_to == new_account {
                return Err(VaultError::NoChange);
            }

            self.data.role_fee_to = new_account;

            Self::emit_event(
                Self::env(),
                Event::RoleFeeToTransferred(RoleFeeToTransferred { new_account }),
            );

            Ok(())
        }

        #[ink(message)]
        fn get_role_set_code(&self) -> Option<AccountId> {
            self.data.role_set_code
        }

        /// Returns the total amount of bonded AZERO
        #[ink(message)]
        fn get_total_pooled(&self) -> Balance {
            self.data.total_pooled
        }

        /// Shares effectively in circulation by the protocol including:
        ///     1) sA0 that has already been minted
        ///     2) sA0 that could be minted (virtual) representing accumulating protocol fees
        #[ink(message)]
        fn get_total_shares(&self) -> u128 {
            self.data.total_shares_minted + self.get_current_virtual_shares()
        }

        /// Protocol fees (sA0) which can be minted and withdrawn at the current block timestamp
        #[ink(message)]
        fn get_current_virtual_shares(&self) -> u128 {
            let now = Self::env().block_timestamp();
            self.data.get_virtual_shares_at_time(now)
        }

        #[ink(message)]
        fn get_fee_percentage(&self) -> u16 {
            self.data.fee_percentage
        }

        #[ink(message)]
        fn get_incentive_percentage(&self) -> u16 {
            self.data.incentive_percentage
        }

        #[ink(message)]
        fn get_share_token_contract(&self) -> AccountId {
            self.data.shares_contract
        }

        #[ink(message)]
        fn get_registry_contract(&self) -> AccountId {
            RegistryRef::to_account_id(&self.data.registry_contract)
        }

        /// Calculate the value of AZERO in terms of sA0 shares
        #[ink(message)]
        fn get_shares_from_azero(&self, azero: Balance) -> u128 {
            let total_pooled_ = self.data.total_pooled; // shadow
            if total_pooled_ == 0 {
                // This happens upon initial stake
                // Also known as 1:1 redemption ratio
                azero
            } else {
                self.data
                    .pro_rata(azero, self.get_total_shares(), total_pooled_)
            }
        }

        /// Calculate the value of sA0 shares in terms of AZERO
        #[ink(message)]
        fn get_azero_from_shares(&self, shares: u128) -> Balance {
            let total_shares = self.get_total_shares();
            if total_shares == 0 {
                // This should never happen
                0
            } else {
                self.data
                    .pro_rata(shares, self.data.total_pooled, total_shares)
            }
        }

        /// Returns the unlock requests for a given user
        #[ink(message)]
        fn get_unlock_requests(&self, user: AccountId) -> Vec<UnlockRequest> {
            self.data.user_unlock_requests.get(user).unwrap_or_default()
        }

        #[ink(message)]
        fn get_weight_imbalances(&self, total_pooled: u128) -> (u128, u128, Vec<u128>, Vec<i128>) {
            let (total_weight, agents) = self.data.registry_contract.get_agents();
            self.data
                .get_weight_imbalances(&agents, total_weight, total_pooled)
        }
    }
}
