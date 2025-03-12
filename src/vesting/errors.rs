use psp22::PSP22Error;

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum VestingError {
    RecipientDoesNotExist,
    RecipientAlreadyExists,
    Active,
    NotActive,
    TooEarly,
    NoChange,
    NoAdmin,
    AdminOnly,
    InsufficientFunding,
    InvalidInput,
    TokenError(PSP22Error),
    EnvError,
}
