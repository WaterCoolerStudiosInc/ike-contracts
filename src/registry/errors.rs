use ink::prelude::string::String;

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum RegistryError {
    InvalidInput,
    TooManyAgents,
    AgentNotFound,
    AgentDisabled,
    ActiveAgent,
    InvalidPermissions,
    InvalidRole,
    NoChange,
    /// An interaction with ink! environment has failed
    // NOTE: We're representing the `ink::env::Error` as `String` b/c the
    // type does not have Encode/Decode implemented.
    InkEnvError(String),
}