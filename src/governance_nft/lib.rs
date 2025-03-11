#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub mod traits;
pub use crate::governance_nft::GovernanceNFT;
pub use crate::governance_nft::GovernanceNFTRef;

#[ink::contract]
mod governance_nft {
    use ink::{
        env::debug_println,
        prelude::{string::String, vec::Vec},
        storage::Mapping,
    };
    use psp34::{metadata, Id, PSP34Data, PSP34Error, PSP34Event, PSP34Metadata, PSP34};

    use crate::traits::IGovernanceNFT;

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        operator: AccountId,
        #[ink(topic)]
        id: Option<Id>,
        approved: bool,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        id: Id,
    }

    #[ink(event)]
    pub struct AttributeSet {
        id: Id,
        key: Vec<u8>,
        data: Vec<u8>,
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct GovernanceData {
        pub block_created: u64,
        pub stake_weight: u128,
        pub vote_weight: u128,
    }

    #[ink(storage)]
    pub struct GovernanceNFT {
        data: PSP34Data,
        metadata: metadata::Data,
        admin: AccountId,
        governance: AccountId,
        mint_count: u128,
        token_governance_data: Mapping<u128, GovernanceData>,
        lock_transfer: bool,
    }

    impl GovernanceNFT {
        #[ink(constructor)]
        pub fn new(governance: AccountId) -> Self {
            Self {
                data: PSP34Data::new(),
                metadata: metadata::Data::default(),
                admin: governance,
                governance,
                mint_count: 0_u128,
                token_governance_data: Mapping::default(),
                lock_transfer: true,
            }
        }

        // A helper function translating a vector of PSP34Events into the proper
        // ink event types (defined internally in this contract) and emitting them.
        fn emit_events(&self, events: ink::prelude::vec::Vec<PSP34Event>) {
            for event in events {
                match event {
                    PSP34Event::Approval {
                        owner,
                        operator,
                        id,
                        approved,
                    } => self.env().emit_event(Approval {
                        owner,
                        operator,
                        id,
                        approved,
                    }),
                    PSP34Event::Transfer { from, to, id } => {
                        self.env().emit_event(Transfer { from, to, id })
                    }
                    PSP34Event::AttributeSet { id, key, data } => {
                        self.env().emit_event(AttributeSet { id, key, data })
                    }
                }
            }
        }

        fn only_admin(&self) -> Result<(), PSP34Error> {
            if self.env().caller() != self.admin {
                return Err(PSP34Error::Custom(String::from("Unauthorized")));
            }
            Ok(())
        }
    }

    impl IGovernanceNFT for GovernanceNFT {
        #[ink(message, selector = 91)]
        fn lock_transfer(&mut self) -> Result<(), PSP34Error> {
            if self.env().caller() != self.governance {
                return Err(PSP34Error::Custom(String::from("Unauthorized")));
            }
            self.lock_transfer = true;
            Ok(())
        }

        #[ink(message, selector = 47)]
        fn unlock_transfer(&mut self) -> Result<(), PSP34Error> {
            if self.env().caller() != self.governance {
                return Err(PSP34Error::Custom(String::from("Unauthorized")));
            }
            self.lock_transfer = false;
            Ok(())
        }

        #[ink(message, selector = 69)]
        fn is_collection_locked(&self) -> bool {
            self.lock_transfer
        }

        #[ink(message, selector = 31337)]
        fn get_governance_data(&self, id: u128) -> Option<GovernanceData> {
            self.token_governance_data.get(id)
        }

        fn get_admin(&self) -> AccountId {
            self.admin
        }

        #[ink(message, selector = 89)]
        fn increment_weights(
            &mut self,
            id: u128,
            stake_weight: u128,
            vote_weight: u128,
        ) -> Result<(), PSP34Error> {
            self.only_admin()?;

            let mut curr = self
                .token_governance_data
                .get(id)
                .ok_or(PSP34Error::TokenNotExists)?;
            debug_println!("{:?}", curr);
            if vote_weight > 0 {
                curr.vote_weight += vote_weight;
            }
            if stake_weight > 0 {
                curr.stake_weight += stake_weight;
            }
            debug_println!("{:?}", curr);
            self.token_governance_data.insert(id, &curr);

            Ok(())
        }

        #[ink(message, selector = 99)]
        fn decrement_vote_weight(&mut self, id: u128, vote_weight: u128) -> Result<(), PSP34Error> {
            self.only_admin()?;

            let mut curr = self
                .token_governance_data
                .get(id)
                .ok_or(PSP34Error::TokenNotExists)?;

            if curr.vote_weight < vote_weight {
                return Err(PSP34Error::Custom(String::from("Insufficient vote weight")));
            }
            curr.vote_weight -= vote_weight;
            self.token_governance_data.insert(id, &curr);

            Ok(())
        }

        #[ink(message, selector = 1337)]
        fn mint(
            &mut self,
            to: AccountId,
            stake_weight: u128,
            vote_weight: u128,
        ) -> Result<u128, PSP34Error> {
            self.only_admin()?;

            self.mint_count += 1;
            let curr_id = Id::U128(self.mint_count);
            let g_metadata = GovernanceData {
                block_created: self.env().block_timestamp(),
                stake_weight,
                vote_weight,
            };
            self.token_governance_data
                .insert(self.mint_count, &g_metadata);

            let events = self.data.mint(to, curr_id)?;
            self.emit_events(events);

            Ok(self.mint_count)
        }

        #[ink(message, selector = 8057)]
        fn burn(&mut self, account: AccountId, id: u128) -> Result<(), PSP34Error> {
            self.only_admin()?;

            self.token_governance_data.remove(id);
            let events = self.data.burn(self.env().caller(), account, Id::U128(id))?;
            self.emit_events(events);

            Ok(())
        }

        #[ink(message, selector = 8888)]
        fn set_admin(&mut self, new_admin: AccountId) -> Result<(), PSP34Error> {
            self.only_admin()?;
            self.admin = new_admin;
            Ok(())
        }

        #[ink(message)]
        fn owner_of_id(&self, id: u128) -> Option<AccountId> {
            self.data.owner_of(&Id::U128(id))
        }
    }

    impl PSP34 for GovernanceNFT {
        #[ink(message)]
        fn collection_id(&self) -> Id {
            self.data.collection_id(self.env().account_id())
        }

        #[ink(message)]
        fn total_supply(&self) -> u128 {
            self.data.total_supply()
        }

        #[ink(message)]
        fn balance_of(&self, owner: AccountId) -> u32 {
            self.data.balance_of(owner)
        }

        #[ink(message)]
        fn allowance(&self, owner: AccountId, operator: AccountId, id: Option<Id>) -> bool {
            self.data.allowance(owner, operator, id.as_ref())
        }

        #[ink(message)]
        fn transfer(
            &mut self,
            to: AccountId,
            id: Id,
            data: ink::prelude::vec::Vec<u8>,
        ) -> Result<(), PSP34Error> {
            if self.lock_transfer && self.env().caller() != self.admin {
                // Only admin can transfer when locked
                return Err(PSP34Error::Custom(String::from("Token transfer is locked")));
            }
            let events = self.data.transfer(self.env().caller(), to, id, data)?;
            self.emit_events(events);
            Ok(())
        }

        #[ink(message)]
        fn approve(
            &mut self,
            operator: AccountId,
            id: Option<Id>,
            approved: bool,
        ) -> Result<(), PSP34Error> {
            let events = self
                .data
                .approve(self.env().caller(), operator, id, approved)?;
            self.emit_events(events);
            Ok(())
        }

        #[ink(message)]
        fn owner_of(&self, id: Id) -> Option<AccountId> {
            self.data.owner_of(&id)
        }
    }

    impl PSP34Metadata for GovernanceNFT {
        #[ink(message)]
        fn get_attribute(&self, id: Id, key: Vec<u8>) -> Option<Vec<u8>> {
            self.metadata.get_attribute(id, key)
        }
    }
}
