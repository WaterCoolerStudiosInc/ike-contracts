use ink::{
    env::{
        call::{build_call, ExecutionInput, Selector},
        DefaultEnvironment,
    },
    primitives::AccountId,
};

const QUERY_STAKED_VALUE_SELECTOR: Selector = Selector::new([0, 0, 0, 12]);
const QUERY_UNBONDED_VALUE_SELECTOR: Selector = Selector::new([0, 0, 0, 13]);

pub fn query_staked_value(nomination_agent_instance: AccountId) -> u128 {
    build_call::<DefaultEnvironment>()
        .call(nomination_agent_instance)
        .exec_input(ExecutionInput::new(QUERY_STAKED_VALUE_SELECTOR))
        .transferred_value(0)
        .returns::<u128>()
        .invoke()
}

pub fn query_unbonded_value(nomination_agent_instance: AccountId) -> u128 {
    build_call::<DefaultEnvironment>()
        .call(nomination_agent_instance)
        .exec_input(ExecutionInput::new(QUERY_UNBONDED_VALUE_SELECTOR))
        .transferred_value(0)
        .returns::<u128>()
        .invoke()
}
