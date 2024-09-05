use ink::{
    env::{
        call::{build_call, ExecutionInput, Selector},
        DefaultEnvironment,
        Environment,
    },
    primitives::AccountId,
};

// TODO: Import these from ../nomination_agent/lib.rs::RuntimeError
#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum RuntimeError {
    CallRuntimeFailed,
    Unauthorized,
}

type Balance = <DefaultEnvironment as Environment>::Balance;
const DEPOSIT_SELECTOR: Selector = Selector::new([0, 0, 0, 1]);
const UNBOND_SELECTOR: Selector = Selector::new([0, 0, 0, 2]);
const WITHDRAW_SELECTOR: Selector = Selector::new([0, 0, 0, 3]);
const COMPOUND_SELECTOR: Selector = Selector::new( [0, 0, 0, 4]);
const QUERY_STAKED_VALUE_SELECTOR: Selector = Selector::new([0, 0, 0, 12]);

pub fn make_call(
    nomination_agent_instance: AccountId,
    selector: Selector,
    transferred_value: u128,
) -> Result<(), RuntimeError> {
    build_call::<DefaultEnvironment>()
        .call(nomination_agent_instance)
        .exec_input(ExecutionInput::new(selector))
        .transferred_value(transferred_value)
        .returns::<Result<(), RuntimeError>>()
        .invoke()
}
pub fn call_deposit(
    nomination_agent_instance: AccountId,
    transferred_value: u128,
) -> Result<(), RuntimeError> {
    make_call(nomination_agent_instance, DEPOSIT_SELECTOR, transferred_value)
}

pub fn call_unbond(nomination_agent_instance: AccountId, amount: u128) -> Result<(), RuntimeError> {
    build_call::<DefaultEnvironment>()
        .call(nomination_agent_instance)
        .exec_input(ExecutionInput::new(UNBOND_SELECTOR).push_arg(amount))
        .transferred_value(0)
        .returns::<Result<(), RuntimeError>>()
        .invoke()
}

pub fn call_withdraw_unbonded(nomination_agent_instance: AccountId) -> Result<(), RuntimeError> {
    make_call(nomination_agent_instance, WITHDRAW_SELECTOR, 0_u128)
}

pub fn call_compound(nomination_agent_instance: AccountId) -> Result<Balance, RuntimeError> {
    let call_result: Result<Balance, RuntimeError> = build_call::<DefaultEnvironment>()
        .call(nomination_agent_instance)
        .exec_input(ExecutionInput::new(COMPOUND_SELECTOR))
        .transferred_value(0)
        .returns::<Result<Balance, RuntimeError>>()
        .invoke();
    call_result
}

pub fn query_staked_value(nomination_agent_instance: AccountId) -> Balance {
    let call_result: Balance = build_call::<DefaultEnvironment>()
        .call(nomination_agent_instance)
        .exec_input(ExecutionInput::new(QUERY_STAKED_VALUE_SELECTOR))
        .transferred_value(0)
        .returns::<Balance>()
        .invoke();
    call_result
}
