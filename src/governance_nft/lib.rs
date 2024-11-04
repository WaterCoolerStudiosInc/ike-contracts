#![cfg_attr(not(feature = "std"), no_std, no_main)]
mod traits;
pub use crate::governance_nft::GovernanceNFTRef;
pub use traits::GovernanceNFT;

//pub use psp34::{Id, PSP34Data, PSP34Event};
//pub use psp34::PSP34Error;
//pub use psp34::{PSP34Burnable, PSP34Metadata, PSP34Mintable, PSP34};

// An example code of a smart contract using PSP34Data struct to implement
// the functionality of PSP34 fungible token.
//
// Any contract can be easily enriched to act as PSP34 token by:
// (1) adding PSP34Data to contract storage
// (2) properly initializing it
// (3) defining the correct AttributeSet, Transfer and Approval events
// (4) implementing PSP34 trait based on PSP34Data methods
// (5) properly emitting resulting events
//
// Implemented the optional PSP34Mintable (6), PSP34Burnable (7), and PSP34Metadata (8) extensions
// and included unit tests (8).

#[ink::contract]
mod governance_nft {
    use ink::{
        env::debug_println,
        prelude::{string::String, vec::Vec},
        storage::Mapping,
    };
    use psp34::{metadata, Id, PSP34Data, PSP34Error, PSP34Event, PSP34Metadata, PSP34};

    #[cfg(feature = "enumerable")]
    use psp34::PSP34Enumerable;

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
        data: PSP34Data, // (1)
        metadata: metadata::Data,
        admin: AccountId,
        mint_count: u128,
        token_governance_data: Mapping<u128, GovernanceData>,
        lock_transfer: bool,
    }

    impl GovernanceNFT {
        #[ink(constructor)]
        pub fn new(_admin: AccountId) -> Self {
            Self {
                data: PSP34Data::new(), // (2)
                metadata: metadata::Data::default(),
                admin: _admin,
                mint_count: 0_u128,
                token_governance_data: Mapping::default(), // (8)
                lock_transfer: true,
            }
        }

        // A helper function translating a vector of PSP34Events into the proper
        // ink event types (defined internally in this contract) and emitting them.
        // (5)
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

        #[ink(message)]
        pub fn lock_transfer(&mut self) -> Result<(), PSP34Error> {
            if self.env().caller() != self.admin {
                return Err(PSP34Error::Custom(String::from("Unauthorized")));
            }
            self.lock_transfer = true;
            Ok(())
        }

        #[ink(message)]
        pub fn unlock_transfer(&mut self) -> Result<(), PSP34Error> {
            if self.env().caller() != self.admin {
                return Err(PSP34Error::Custom(String::from("Unauthorized")));
            }
            self.lock_transfer = false;
            Ok(())
        }

        #[ink(message, selector = 69)]
        pub fn is_collection_locked(&self) -> bool {
            self.lock_transfer
        }

        #[ink(message, selector = 7)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            id: Id,
            data: ink::prelude::vec::Vec<u8>,
        ) -> Result<(), PSP34Error> {
            let events = self.data.transfer(from, to, id, data)?;
            if self.lock_transfer == true {
                return Err(PSP34Error::Custom(String::from("Token transfer is locked")));
            }
            self.emit_events(events);
            Ok(())
        }
        #[ink(message, selector = 31337)]
        pub fn get_governance_data(&self, id: u128) -> Option<GovernanceData> {
            self.token_governance_data.get(id)
        }
        pub fn get_admin(&self) -> AccountId {
            self.admin
        }

        #[ink(message, selector = 88)]
        pub fn increment_weight(&mut self, id: u128, weight: u128) -> Result<(), PSP34Error> {
            if self.env().caller() != self.admin {
                return Err(PSP34Error::Custom(String::from("Unauthorized")));
            }
            let mut curr = self.token_governance_data.get(id).unwrap();
            curr.vote_weight += weight;
            debug_println!("VOTE WEIGHT {}", curr.vote_weight);
            self.token_governance_data.insert(id, &curr);

            Ok(())
        }
        #[ink(message, selector = 89)]
        pub fn increment_stake_weight(&mut self, id: u128, weight: u128) -> Result<(), PSP34Error> {
            if self.env().caller() != self.admin {
                return Err(PSP34Error::Custom(String::from("Unauthorized")));
            }
            let mut curr = self.token_governance_data.get(id).unwrap();
            curr.stake_weight += weight;
            debug_println!("VOTE WEIGHT {}", curr.vote_weight);
            self.token_governance_data.insert(id, &curr);

            Ok(())
        }
        #[ink(message, selector = 99)]
        pub fn decrement_weight(&mut self, id: u128, weight: u128) -> Result<(), PSP34Error> {
            if self.env().caller() != self.admin {
                return Err(PSP34Error::Custom(String::from("Unauthorized")));
            }
            let mut curr = self.token_governance_data.get(id).unwrap();
            assert!(curr.vote_weight >= weight);
            curr.vote_weight -= weight;
            debug_println!("VOTE WEIGHT {}", curr.vote_weight);
            self.token_governance_data.insert(id, &curr);
            Ok(())
        }
        #[ink(message, selector = 1337)]
        pub fn mint(
            &mut self,
            to: AccountId,
            weight: u128,
            vote_weight: u128,
        ) -> Result<u128, PSP34Error> {
            if self.env().caller() != self.admin {
                return Err(PSP34Error::Custom(String::from("Unauthorized")));
            }

            self.mint_count += 1;
            let curr_id = Id::U128(self.mint_count);
            let g_metadata = GovernanceData {
                block_created: self.env().block_timestamp(),
                stake_weight: weight,
                vote_weight: vote_weight,
                
            };

            self.token_governance_data
                .insert(self.mint_count, &g_metadata);
            let events = self.data.mint(to, curr_id)?;

            self.emit_events(events);

            Ok(self.mint_count)
        }

        #[ink(message, selector = 8057)]
        pub fn burn(&mut self, account: AccountId, id: u128) -> Result<(), PSP34Error> {
            // Add security, restrict usage of the message
            if self.env().caller() != self.admin {
                return Err(PSP34Error::Custom(String::from("Unauthorized")));
            }

            let _id = Id::U128(id);

            let events = self.data.burn(account, self.env().caller(), _id)?;
            self.emit_events(events);
            Ok(())
        }
        #[ink(message, selector = 8888)]
        pub fn set_admin(&mut self, new_admin: AccountId) -> Result<(), PSP34Error> {
            if self.env().caller() != self.admin {
                return Err(PSP34Error::Custom(String::from("Unauthorized")));
            }
            self.admin = new_admin;
            Ok(())
        }
        #[ink(message)]
        pub fn owner_of_id(&self, id: u128) -> Option<AccountId> {
            self.data.owner_of(&Id::U128(id))
        }
    }

    // (3)
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

    // (3)
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        id: Id,
    }

    // (3)
    #[ink(event)]
    pub struct AttributeSet {
        id: Id,
        key: Vec<u8>,
        data: Vec<u8>,
    }

    // (4)
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
            let events = self.data.transfer(self.env().caller(), to, id, data)?;
            if self.lock_transfer == true {
                return Err(PSP34Error::Custom(String::from("Token transfer is locked")));
            }
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

    // (7)

    // (8)
    impl PSP34Metadata for GovernanceNFT {
        #[ink(message)]
        fn get_attribute(&self, id: Id, key: Vec<u8>) -> Option<Vec<u8>> {
            self.metadata.get_attribute(id, key)
        }
    }

    // (9)
}
