#[allow(dead_code)]
#[derive(Clone, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
pub enum PoolState {
    #[codec(index = 0)]
    Open,
    #[codec(index = 1)]
    Blocked,
    #[codec(index = 2)]
    Destroying,
}
