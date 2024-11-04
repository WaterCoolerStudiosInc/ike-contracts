#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub mod errors;
pub mod traits;

#[ink::contract]
pub mod registry {

    use crate::errors::RegistryError;
    use crate::traits::IRegistry;
    use ink::{
        contract_ref,
        env::Error as InkEnvError,
        prelude::{format, vec::Vec},
        storage::Mapping,
        ToAccountId,
    };
    use nomination_agent::{nomination_agent::NominationAgentRef, traits::INominationAgent};

    impl From<InkEnvError> for RegistryError {
        fn from(e: InkEnvError) -> Self {
            RegistryError::InkEnvError(format!("{:?}", e))
        }
    }

    pub const MAX_AGENTS: usize = 50;

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Role {
        admin: AccountId,
        account: AccountId,
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum RoleType {
        // Permission to add new agents
        AddAgent,
        // Permission to update agent weights
        UpdateAgents,
        // Permission to disable active agents
        DisableAgent,
        // Permission to remove deprecated agents
        RemoveAgent,
        // Permission to set code hash aka "upgrade" logic
        SetCodeHash,
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Agent {
        pub address: AccountId,
        pub weight: u128,
        pub disabled: bool,
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct WeightUpdate {
        pub agent: AccountId,
        pub weight: u128,
        pub increase: bool,
    }

    #[ink(event)]
    pub struct AgentAdded {
        #[ink(topic)]
        agent: AccountId,
    }
    #[ink(event)]
    pub struct AgentUpdated {
        #[ink(topic)]
        agent: AccountId,
        old_weight: u128,
        new_weight: u128,
    }
    #[ink(event)]
    pub struct AgentDisabled {
        #[ink(topic)]
        agent: AccountId,
        old_weight: u128,
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
        pub total_weight: u128,
        // Permissions for adding agents, updating weights, and removing agents
        pub roles: Mapping<RoleType, Role>,
        // Used for instantiating agents
        pub vault: AccountId,
        pub nomination_agent_hash: Hash,
        pub nomination_agent_counter: u128,
    }

    impl Registry {
        #[ink(constructor)]
        pub fn new(
            role_add: AccountId,
            role_update: AccountId,
            role_disable: AccountId,
            role_remove: AccountId,
            role_set_code_hash: AccountId,
            nomination_agent_hash: Hash,
        ) -> Self {
            let mut initial_roles = Mapping::default();
            initial_roles.insert(
                RoleType::AddAgent,
                &Role {
                    admin: role_add,
                    account: role_add,
                },
            );
            initial_roles.insert(
                RoleType::UpdateAgents,
                &Role {
                    admin: role_update,
                    account: role_update,
                },
            );
            initial_roles.insert(
                RoleType::DisableAgent,
                &Role {
                    admin: role_disable,
                    account: role_disable,
                },
            );
            initial_roles.insert(
                RoleType::RemoveAgent,
                &Role {
                    admin: role_remove,
                    account: role_remove,
                },
            );
            initial_roles.insert(
                RoleType::SetCodeHash,
                &Role {
                    admin: role_set_code_hash,
                    account: role_set_code_hash,
                },
            );

            Self {
                agents: Vec::new(),
                total_weight: 0,
                roles: initial_roles,
                vault: Self::env().caller(),
                nomination_agent_hash,
                nomination_agent_counter: 0,
            }
        }
    }

    impl IRegistry for Registry {
        /// Add a new nomination agent
        ///
        /// Caller must have the AddAgent role.
        /// Cannot add the same nomination agent twice.
        #[ink(message, payable, selector = 1)]
        fn add_agent(
            &mut self,
            admin: AccountId,
            validator: AccountId,
        ) -> Result<AccountId, RegistryError> {
            let caller = Self::env().caller();
            let nominator_bond = Self::env().transferred_value();

            if caller != self.roles.get(RoleType::AddAgent).unwrap().account {
                return Err(RegistryError::InvalidPermissions);
            }

            if self.agents.len() >= MAX_AGENTS {
                return Err(RegistryError::TooManyAgents);
            }

            let nomination_agent_counter = self.nomination_agent_counter; // shadow

            let agent_ref = NominationAgentRef::new(
                self.vault,
                admin,
                validator,
            )
            .endowment(nominator_bond)
            .code_hash(self.nomination_agent_hash)
            .salt_bytes(nomination_agent_counter.to_le_bytes())
            .instantiate();

            let agent_address = NominationAgentRef::to_account_id(&agent_ref);

            self.nomination_agent_counter = nomination_agent_counter + 1;

            self.agents.push(Agent {
                address: agent_address,
                weight: 0,
                disabled: false,
            });

            Self::env().emit_event(AgentAdded {
                agent: agent_address,
            });

            Ok(agent_address)
        }

        /// Update weight of existing nomination agents
        ///
        /// Caller must have the UpdateAgents role.
        /// Cannot update a disabled agent.
        #[ink(message, selector = 2)]
        fn update_agents(
            &mut self,
            weight_updates: Vec<WeightUpdate>,
        ) -> Result<(), RegistryError> {
            let caller = Self::env().caller();

            if caller != self.roles.get(RoleType::UpdateAgents).unwrap().account {
                return Err(RegistryError::InvalidPermissions);
            }

            for update in weight_updates.iter() {
                if let Some(index) = self.agents.iter().position(|a| a.address == update.agent) {
                    if self.agents[index].disabled {
                        return Err(RegistryError::AgentDisabled);
                    }

                    let old_weight = self.agents[index].weight;
                    let new_weight;

                    if update.increase {
                        new_weight = old_weight + update.weight;
                        self.total_weight += update.weight;
                    } else {
                        if update.weight > old_weight {
                            return Err(RegistryError::InvalidInput);
                        }
                        new_weight = old_weight - update.weight;
                        self.total_weight -= update.weight;
                    };

                    self.agents[index].weight = new_weight;

                    Self::env().emit_event(AgentUpdated {
                        agent: update.agent,
                        old_weight,
                        new_weight,
                    });
                } else {
                    return Err(RegistryError::AgentNotFound);
                }
            }

            Ok(())
        }

        /// Begins the process of offboarding a nomination agent
        /// Permanently removes weight allocation from a given agent.
        /// This allows dynamic registry weights and offboarding to co-exist.
        ///
        /// Caller must have the DisableAgent role.
        #[ink(message, selector = 5)]
        fn disable_agent(&mut self, agent: AccountId) -> Result<(), RegistryError> {
            let caller = Self::env().caller();

            if caller != self.roles.get(RoleType::DisableAgent).unwrap().account {
                return Err(RegistryError::InvalidPermissions);
            }

            if let Some(index) = self.agents.iter().position(|a| a.address == agent) {
                if self.agents[index].disabled {
                    return Err(RegistryError::NoChange);
                }

                let old_weight = self.agents[index].weight;
                self.agents[index].weight = 0;
                self.agents[index].disabled = true;
                self.total_weight -= old_weight;

                Self::env().emit_event(AgentDisabled { agent, old_weight });
            } else {
                return Err(RegistryError::AgentNotFound);
            }

            Ok(())
        }

        /// Removes a nomination agent
        /// This is intended to remove fully deprecated agents to save gas during iteration.
        ///
        /// Caller must have the RemoveAgent role.
        /// Agent must have no AZERO staked (excludes initial bond).
        /// Agent must have no AZERO unbonding.
        #[ink(message, selector = 3)]
        fn remove_agent(&mut self, agent: AccountId) -> Result<(), RegistryError> {
            let caller = Self::env().caller();

            if caller != self.roles.get(RoleType::RemoveAgent).unwrap().account {
                return Err(RegistryError::InvalidPermissions);
            }

            if let Some(index) = self.agents.iter().position(|a| a.address == agent) {
                let mut agent_contract: contract_ref!(INominationAgent) = agent.into();
                // Do not delete agents with AZERO staked
                if agent_contract.get_staked_value() > 0 {
                    return Err(RegistryError::ActiveAgent);
                }
                // Do not delete agents with AZERO unbonding
                if agent_contract.get_unbonding_value() > 0 {
                    return Err(RegistryError::ActiveAgent);
                }
                let weight = self.agents[index].weight;
                if weight > 0 {
                    self.total_weight -= weight;
                }
                self.agents.remove(index);
                agent_contract
                    .destroy()
                    .expect("Agent begins the destruction process");

                Self::env().emit_event(AgentDeleted { agent });
            } else {
                return Err(RegistryError::AgentNotFound);
            }
            Ok(())
        }

        #[ink(message, selector = 4)]
        fn get_agents(&self) -> (u128, Vec<Agent>) {
            (self.total_weight, self.agents.clone())
        }

        /// ================================ Update Role Methods ================================

        /// Transfers role to a new account
        ///
        /// Caller must be the admin for the role
        #[ink(message)]
        fn transfer_role(
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

                Self::env().emit_event(RoleAccountChanged {
                    role_type,
                    new_account,
                });
            } else {
                return Err(RegistryError::InvalidRole);
            }

            Ok(())
        }

        /// Transfers administration of role to a new account
        ///
        /// Caller must be the admin for the role
        #[ink(message)]
        fn transfer_role_admin(
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

                Self::env().emit_event(RoleAdminChanged {
                    role_type,
                    new_account,
                });
            } else {
                return Err(RegistryError::InvalidRole);
            }

            Ok(())
        }

        /// ================================ Code Hash Methods ================================

        /// "Upgrade" the Registry contract logic
        ///
        /// Caller must have the SetCodeHash role.
        #[ink(message)]
        fn set_code(&mut self, code_hash: [u8; 32]) -> Result<(), RegistryError> {
            let caller = Self::env().caller();

            if caller != self.roles.get(RoleType::SetCodeHash).unwrap().account {
                return Err(RegistryError::InvalidPermissions);
            }

            ink::env::set_code_hash(&code_hash)?;

            Ok(())
        }

        /// "Upgrade" the logic of all nomination agent contracts
        ///
        /// Caller must have the SetCodeHash role.
        #[ink(message)]
        fn set_agent_code(&mut self, nomination_agent_hash: [u8; 32]) -> Result<(), RegistryError> {
            let caller = Self::env().caller();

            if caller != self.roles.get(RoleType::SetCodeHash).unwrap().account {
                return Err(RegistryError::InvalidPermissions);
            }

            for agent in self.agents.iter() {
                let mut agent_contract: contract_ref!(INominationAgent) = agent.address.into();
                agent_contract
                    .set_code(nomination_agent_hash)
                    .expect("Agent code hash is updated");
            }

            self.nomination_agent_hash = Hash::from(nomination_agent_hash);

            Ok(())
        }

        #[ink(message)]
        fn get_max_agents(&self) -> u32 {
            MAX_AGENTS as u32
        }

        /// ================================ View Only Role Methods ================================

        #[ink(message)]
        fn get_role(&self, role_type: RoleType) -> AccountId {
            self.roles.get(role_type).unwrap().account
        }

        #[ink(message)]
        fn get_role_admin(&self, role_type: RoleType) -> AccountId {
            self.roles.get(role_type).unwrap().admin
        }
    }
}
