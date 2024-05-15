#![cfg_attr(not(feature = "std"), no_std, no_main)]
mod traits;
pub use crate::registry::RegistryRef;
pub use traits::Registry;
#[ink::contract]
pub mod registry {

    use ink::{
        env::{debug_println, Error as InkEnvError},
        prelude::{format, string::String, vec::Vec},
        storage::Mapping,
    };

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum RegistryError {
        InvalidInput,
        DuplicateAgent,
        AgentNotFound,
        ActiveAgent,
        InvalidPermissions,
        InvalidRole,
        NoChange,
        /// An interaction with ink! environment has failed
        // NOTE: We're representing the `ink::env::Error` as `String` b/c the
        // type does not have Encode/Decode implemented.
        InkEnvError(String),
    }

    impl From<InkEnvError> for RegistryError {
        fn from(e: InkEnvError) -> Self {
            RegistryError::InkEnvError(format!("{:?}", e))
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Role {
        admin: AccountId,
        account: AccountId,
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub enum RoleType {
        // Permission to add new agents
        AddAgent,
        // Permission to update agent weights
        UpdateAgents,
        // Permission to remove deprecated agents
        RemoveAgent,
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Agent {
        pub address: AccountId,
        pub weight: u64,
    }

    #[ink(event)]
    pub struct AgentAdded {
        #[ink(topic)]
        agent: AccountId,
        weight: u64,
    }
    #[ink(event)]
    pub struct AgentUpdated {
        #[ink(topic)]
        agent: AccountId,
        old_weight: u64,
        new_weight: u64,
    }
    #[ink(event)]
    pub struct AgentDeleted {
        #[ink(topic)]
        agent: AccountId,
    }
    #[ink(event)]
    pub struct RoleAccountChanged {
        role_type: RoleType,
        new_account: AccountId,
    }
    #[ink(event)]
    pub struct RoleAdminChanged {
        role_type: RoleType,
        new_account: AccountId,
    }

    #[ink(storage)]
    pub struct Registry {
        // List of all nomination agents including their deployment and relative weight
        pub agents: Vec<Agent>,
        // Sum of nomination agent relative weights
        pub total_weight: u64,
        // Permissions for adding agents, updating weights, and removing agents
        pub roles: Mapping<RoleType, Role>,
    }

    impl Registry {
        #[ink(constructor)]
        pub fn new(
            role_add: AccountId,
            role_update: AccountId,
            role_remove: AccountId,
        ) -> Self {
            let mut initial_roles = Mapping::default();
            initial_roles.insert(RoleType::AddAgent, &Role { admin: role_add, account: role_add });
            initial_roles.insert(RoleType::UpdateAgents, &Role { admin: role_update, account: role_update });
            initial_roles.insert(RoleType::RemoveAgent, &Role { admin: role_remove, account: role_remove });

            Self {
                agents: Vec::new(),
                total_weight: 0,
                roles: initial_roles,
            }
        }

        /// Add a new nomination agent
        ///
        /// Caller must have the AddAgent role.
        /// Cannot add the same nomination agent twice.
        #[ink(message)]
        pub fn add_agent(
            &mut self,
            account: AccountId,
            new_weight: u64,
        ) -> Result<(), RegistryError> {
            let caller = Self::env().caller();

            if caller != self.roles.get(RoleType::AddAgent).unwrap().account {
                return Err(RegistryError::InvalidPermissions);
            }

            // Ensure agent does not already exist
            if self.agents.iter().any(|a| a.address == account) {
                return Err(RegistryError::DuplicateAgent);
            }

            debug_println!("adding new agent");
            self.agents.push(Agent {
                address: account,
                weight: new_weight,
            });
            self.total_weight += new_weight;

            Self::env().emit_event(
                AgentAdded {
                    agent: account,
                    weight: new_weight,
                }
            );

            Ok(())
        }

        /// Update existing nomination agents
        ///
        /// Caller must have the UpdateAgents role.
        #[ink(message)]
        pub fn update_agents(
            &mut self,
            accounts: Vec<AccountId>,
            new_weights: Vec<u64>,
        ) -> Result<(), RegistryError> {
            let caller = Self::env().caller();

            if caller != self.roles.get(RoleType::UpdateAgents).unwrap().account {
                return Err(RegistryError::InvalidPermissions);
            }

            if accounts.len() != new_weights.len() {
                return Err(RegistryError::InvalidInput);
            }

            for (args_index, &account) in accounts.iter().enumerate() {
                if let Some(index) = self.agents.iter().position(|a| a.address == account) {
                    let old_weight = self.agents[index].weight;
                    let new_weight = new_weights[args_index];

                    self.total_weight -= old_weight;
                    self.total_weight += new_weight;

                    self.agents[index] = Agent {
                        address: account,
                        weight: new_weight,
                    };

                    Self::env().emit_event(
                        AgentUpdated {
                            agent: account,
                            old_weight,
                            new_weight,
                        }
                    );
                } else {
                    return Err(RegistryError::AgentNotFound);
                }
            }

            Ok(())
        }

        /// Removes a nomination agent
        ///
        /// Caller must have the RemoveAgent role.
        /// This is intended to remove fully deprecated agents to save gas during iteration.
        /// Agent must have a weight set of 0.
        /// Agent should have sufficient time to unbond all staked AZERO.
        #[ink(message)]
        pub fn remove_agent(
            &mut self,
            account: AccountId,
        ) -> Result<(), RegistryError> {
            let caller = Self::env().caller();

            if caller != self.roles.get(RoleType::RemoveAgent).unwrap().account {
                return Err(RegistryError::InvalidPermissions);
            }

            if let Some(index) = self.agents.iter().position(|a| a.address == account) {
                let weight = self.agents[index].weight;

                // Do not delete agents with active weight (and possible bonded AZERO)
                if weight > 0 {
                    return Err(RegistryError::ActiveAgent);
                }

                self.total_weight -= weight;
                self.agents.remove(index);

                Self::env().emit_event(
                    AgentDeleted {
                        agent: account,
                    }
                );
            } else {
                return Err(RegistryError::AgentNotFound);
            }

            Ok(())
        }

        #[ink(message)]
        pub fn get_agents(
            &self,
        ) -> (u64, Vec<Agent>) {
            (self.total_weight, self.agents.clone())
        }

        /// ================================ Update Role Methods ================================

        /// Transfers role to a new account
        ///
        /// Caller must be the admin for the role
        #[ink(message)]
        pub fn transfer_role(
            &mut self,
            role_type: RoleType,
            new_account: AccountId,
        ) -> Result<(), RegistryError> {
            if let Some(mut role) = self.roles.get(role_type.clone()) {
                if Self::env().caller() != role.admin {
                    return Err(RegistryError::InvalidPermissions);
                }
                if role.account == new_account {
                    return Err(RegistryError::NoChange);
                }

                // Update role account
                role.account = new_account;
                self.roles.insert(role_type.clone(), &role);

                Self::env().emit_event(
                    RoleAccountChanged {
                        role_type,
                        new_account,
                    }
                );
            } else {
                return Err(RegistryError::InvalidRole);
            }

            Ok(())
        }

        /// Transfers administration of role to a new account
        ///
        /// Caller must be the admin for the role
        #[ink(message)]
        pub fn transfer_role_admin(
            &mut self,
            role_type: RoleType,
            new_account: AccountId,
        ) -> Result<(), RegistryError> {
            if let Some(mut role) = self.roles.get(role_type.clone()) {
                if Self::env().caller() != role.admin {
                    return Err(RegistryError::InvalidPermissions);
                }
                if role.admin == new_account {
                    return Err(RegistryError::NoChange);
                }

                // Update role admin
                role.admin = new_account;
                self.roles.insert(role_type.clone(), &role);

                Self::env().emit_event(
                    RoleAdminChanged {
                        role_type,
                        new_account,
                    }
                );
            } else {
                return Err(RegistryError::InvalidRole);
            }

            Ok(())
        }

        /// ================================ View Only Role Methods ================================

        #[ink(message)]
        pub fn get_role(&self, role_type: RoleType) -> AccountId {
            self.roles.get(role_type).unwrap().account
        }

        #[ink(message)]
        pub fn get_role_admin(&self, role_type: RoleType) -> AccountId {
            self.roles.get(role_type).unwrap().admin
        }
    }
}
