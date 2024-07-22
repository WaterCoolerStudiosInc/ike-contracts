use ink::{
    prelude::string::String,
};
use psp22::PSP22Error;
use crate::nomination_agent_utils::RuntimeError;

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum VaultError {
    Duplication,
    InvalidPercent,
    InvalidBatchUnlockRequest,
    InvalidUserUnlockRequest,
    CooldownPeriod,
    InvalidPermissions,
    NoChange,
    ZeroDepositing,
    ZeroUnbonding,
    ZeroTotalWeight,
    ZeroCompounding,
    MinimumStake,
    /// An interaction with ink! environment has failed
    // NOTE: We're representing the `ink::env::Error` as `String` b/c the
    // type does not have Encode/Decode implemented.
    InkEnvError(String),
    InternalError(RuntimeError),
    TokenError(PSP22Error),
    InternalTokenError,
}
