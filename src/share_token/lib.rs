#![cfg_attr(not(feature = "std"), no_std, no_main)]
mod traits;

pub use traits::ShareToken;
pub use crate::token::TokenRef;

#[ink::contract]
mod token {

    use ink::prelude::{string::String, vec::Vec};
    use psp22::{PSP22Burnable, PSP22Data, PSP22Error, PSP22Event, PSP22Metadata, PSP22};
    #[ink(storage)]
    pub struct Token {
        data: PSP22Data, // (1)
        owner: AccountId,
        operator: AccountId,
        name: Option<String>,
        symbol: Option<String>,
        decimals: u8,
    }

    impl Token {
        #[ink(constructor)]
        pub fn new(name: Option<String>, symbol: Option<String>) -> Self {
            let caller = Self::env().caller();
            Self {
                owner: caller,
                operator: caller,
                data: PSP22Data::new(0, caller),
                name,
                symbol,
                decimals: 12_u8,
            }
        }
        #[ink(message, selector = 7777)]
        pub fn mint(&mut self, to: AccountId, value: u128) -> Result<(), PSP22Error> {
            if Self::env().caller() != self.owner {
                return Err(PSP22Error::Custom(String::from("Caller is not Owner")));
            }
            let events = self.data.mint(to, value)?;
            self.emit_events(events);
            Ok(())
        }
        #[ink(message)]
        pub fn get_owner(&self) -> AccountId {
            self.owner
        }
        // A helper function translating a vector of PSP22Events into the proper
        // ink event types (defined internally in this contract) and emitting them.
        // (5)
        fn emit_events(&self, events: Vec<PSP22Event>) {
            for event in events {
                match event {
                    PSP22Event::Transfer { from, to, value } => {
                        self.env().emit_event(Transfer { from, to, value })
                    }
                    PSP22Event::Approval {
                        owner,
                        spender,
                        amount,
                    } => self.env().emit_event(Approval {
                        owner,
                        spender,
                        amount,
                    }),
                }
            }
        }
    }

    // (3)
    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        amount: u128,
    }

    // (3)
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: u128,
    }
    impl PSP22Burnable for Token {
        #[ink(message)]
        fn burn(&mut self, value: u128) -> Result<(), PSP22Error> {
            if Self::env().caller() != self.owner {
                return Err(PSP22Error::Custom(String::from("Caller is not Owner")));
            }
            let events = self.data.burn(self.env().caller(), value)?;
            self.emit_events(events);
            Ok(())
        }
    }
    // (4)
    impl PSP22 for Token {
        #[ink(message)]
        fn total_supply(&self) -> u128 {
            self.data.total_supply()
        }

        #[ink(message)]
        fn balance_of(&self, owner: AccountId) -> u128 {
            self.data.balance_of(owner)
        }

        #[ink(message)]
        fn allowance(&self, owner: AccountId, spender: AccountId) -> u128 {
            self.data.allowance(owner, spender)
        }

        #[ink(message)]
        fn transfer(
            &mut self,
            to: AccountId,
            value: u128,
            _data: Vec<u8>,
        ) -> Result<(), PSP22Error> {
            let events = self.data.transfer(self.env().caller(), to, value)?;
            self.emit_events(events);
            Ok(())
        }

        #[ink(message)]
        fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: u128,
            _data: Vec<u8>,
        ) -> Result<(), PSP22Error> {
            let caller = self.env().caller();

            if caller == self.operator {
                let events = self.data.transfer(from, to, value)?;
                self.emit_events(events);
            } else {
                let events = self.data.transfer_from(caller, from, to, value)?;
                self.emit_events(events);
            }
            
            Ok(())
        }

        #[ink(message)]
        fn approve(&mut self, spender: AccountId, value: u128) -> Result<(), PSP22Error> {
            let events = self.data.approve(self.env().caller(), spender, value)?;
            self.emit_events(events);
            Ok(())
        }

        #[ink(message)]
        fn increase_allowance(
            &mut self,
            spender: AccountId,
            delta_value: u128,
        ) -> Result<(), PSP22Error> {
            let events = self
                .data
                .increase_allowance(self.env().caller(), spender, delta_value)?;
            self.emit_events(events);
            Ok(())
        }

        #[ink(message)]
        fn decrease_allowance(
            &mut self,
            spender: AccountId,
            delta_value: u128,
        ) -> Result<(), PSP22Error> {
            let events = self
                .data
                .decrease_allowance(self.env().caller(), spender, delta_value)?;
            self.emit_events(events);
            Ok(())
        }
    }

    // (6)
    impl PSP22Metadata for Token {
        #[ink(message)]
        fn token_name(&self) -> Option<String> {
            self.name.clone()
        }
        #[ink(message)]
        fn token_symbol(&self) -> Option<String> {
            self.symbol.clone()
        }
        #[ink(message)]
        fn token_decimals(&self) -> u8 {
            self.decimals
        }
    }

    // (7)
}
