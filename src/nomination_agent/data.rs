use ink::prelude::vec::Vec;
use ink::primitives::AccountId;

#[allow(dead_code)]
#[derive(Clone, scale::Encode)]
pub enum MultiAddress<AccountId, AccountIndex> {
    // It's an account ID (pubkey).
    Id(AccountId),
    // It's an account index.
    Index(#[codec(compact)] AccountIndex),
    // It's some arbitrary raw bytes.
    Raw(Vec<u8>),
    // It's a 32 byte representation.
    Address32([u8; 32]),
    // Its a 20 byte representation.
    Address20([u8; 20]),
}

#[allow(dead_code)]
#[derive(scale::Encode)]
pub enum RewardDestination<_0> {
    #[codec(index = 0)]
    Staked,
    #[codec(index = 1)]
    Stash,
    #[codec(index = 2)]
    Controller,
    #[codec(index = 3)]
    Account(_0),
    #[codec(index = 4)]
    None,
}

#[derive(scale::Encode)]
pub enum StakingCall {
    #[codec(index = 0)]
    Bond {
        #[codec(compact)]
        value: u128,
        payee: RewardDestination<AccountId>,
    },
    #[codec(index = 1)]
    BondExtra {
        #[codec(compact)]
        max_additional: u128,
    },
    #[codec(index = 2)]
    Unbond {
        #[codec(compact)]
        value: u128,
    },
    #[codec(index = 3)]
    WithdrawUnbonded { num_slashing_spans: u32 },
    #[codec(index = 5)]
    Nominate {
        targets: Vec<MultiAddress<AccountId, ()>>,
    },
    #[codec(index = 6)]
    Chill,
}

#[derive(scale::Encode)]
pub enum RuntimeCall {
    #[codec(index = 8)]
    Staking(StakingCall),
}
