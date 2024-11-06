use drink::{
    chain_api::ChainApi,
    runtime::MinimalRuntime,
    session::{contract_transcode::ContractMessageTranscoder, Session, NO_ARGS},
    AccountId32,
};
use std::{error::Error, rc::Rc};

// Publicize all sources module methods (hash_*, transcoder_*, bytes_*)
pub use crate::sources::*;

pub const SECOND: u64 = 1_000;
pub const DAY: u64 = 86400 * SECOND;
pub const YEAR: u64 = DAY * 365_25 / 100; // https://docs.alephzero.org/aleph-zero/use/stake/staking-rewards
pub const BIPS: u128 = 10000;

#[derive(Debug, scale::Decode)]
pub struct Agent {
    pub address: AccountId32,
    pub weight: u128,
    pub disabled: bool,
}

#[derive(Debug, scale::Decode, scale::Encode, serde::Deserialize, serde::Serialize)]
pub struct WeightUpdate {
    pub agent: AccountId32,
    pub weight: u128,
    pub increase: bool,
}

pub fn update_days(
    mut sess: Session<MinimalRuntime>,
    days: u64,
) -> Session<MinimalRuntime> {
    let current_time = sess.chain_api().get_timestamp();
    let time_update = days * DAY;
    sess.chain_api().set_timestamp(current_time + time_update);
    sess
}
pub fn update_in_milliseconds(
    mut sess: Session<MinimalRuntime>,
    milliseconds: u64,
) -> Session<MinimalRuntime> {
    let current_time = sess.chain_api().get_timestamp();
    sess.chain_api().set_timestamp(current_time + milliseconds);
    sess
}

pub fn call_add_agent(
    sess: Session<MinimalRuntime>,
    registry: &AccountId32,
    sender: &AccountId32,
    admin: &AccountId32,
    validator: &AccountId32,
    pool_create_amount: u128,
) -> Result<(AccountId32, Session<MinimalRuntime>), Box<dyn Error>> {
    let sess: Session<MinimalRuntime> = call_function(
        sess,
        &registry,
        &sender,
        String::from("IRegistry::add_agent"),
        Some([
            admin.to_string(),
            validator.to_string(),
        ].to_vec()),
        Some(pool_create_amount),
        transcoder_registry(),
    )?;

    let (_, agents, sess) = get_agents(sess, &registry)?;

    Ok((agents[agents.len() - 1].address.clone(), sess))
}
pub fn call_update_agents(
    sess: Session<MinimalRuntime>,
    registry: &AccountId32,
    sender: &AccountId32,
    updates: Vec<WeightUpdate>,
) -> Result<Session<MinimalRuntime>, Box<dyn Error>> {
    let sess: Session<MinimalRuntime> = call_function(
        sess,
        &registry,
        &sender,
        String::from("IRegistry::update_agents"),
        Some(vec![serde_json::to_string(&updates).unwrap()]),
        None,
        transcoder_registry(),
    )?;
    Ok(sess)
}
pub fn call_disable_agent(
    sess: Session<MinimalRuntime>,
    registry: &AccountId32,
    sender: &AccountId32,
    agent: &AccountId32,
) -> Result<Session<MinimalRuntime>, Box<dyn Error>> {
    let sess: Session<MinimalRuntime> = call_function(
        sess,
        &registry,
        &sender,
        String::from("IRegistry::disable_agent"),
        Some([agent.to_string()].to_vec()),
        None,
        transcoder_registry(),
    )?;
    Ok(sess)
}
pub fn call_remove_agent(
    sess: Session<MinimalRuntime>,
    registry: &AccountId32,
    sender: &AccountId32,
    agent: &AccountId32,
) -> Result<Session<MinimalRuntime>, Box<dyn Error>> {
    let sess: Session<MinimalRuntime> = call_function(
        sess,
        &registry,
        &sender,
        String::from("IRegistry::remove_agent"),
        Some([agent.to_string()].to_vec()),
        None,
        transcoder_registry(),
    )?;
    Ok(sess)
}
pub fn call_stake(
    sess: Session<MinimalRuntime>,
    vault: &AccountId32,
    token: &AccountId32,
    sender: &AccountId32,
    amount: u128,
) -> Result<(u128, Session<MinimalRuntime>), Box<dyn Error>> {
    let mut sess: Session<MinimalRuntime> = call_function(
        sess,
        &vault,
        &sender,
        String::from("IVault::stake"),
        None,
        Some(amount),
        transcoder_vault(),
    )?;

    sess.set_actor(sender.clone());
    sess.set_transcoder(token.clone(), &transcoder_share_token().unwrap());
    let _res2 = sess
        .call_with_address(
            token.clone(),
            "PSP22::balance_of",
            &[sender.to_string()],
            None,
        )
        .unwrap();
    let balance: Result<u128, drink::errors::LangError> = sess.last_call_return().unwrap();
    Ok((balance.unwrap(), sess))
}
pub fn call_request_unlock(
    mut sess: Session<MinimalRuntime>,
    vault: &AccountId32,
    token: &AccountId32,
    sender: &AccountId32,
    amount: u128,
) -> Result<(u128, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.set_transcoder(token.clone(), &transcoder_share_token().unwrap());
    sess.set_actor(sender.clone());

    sess.chain_api().add_tokens(sender.clone(), 1000000);

    sess.set_transcoder(vault.clone(), &transcoder_vault().unwrap());

    println!("Calling: request_unlock()");
    sess.call_with_address(vault.clone(), "IVault::request_unlock", &[amount.to_string()], None)?;

    sess.set_transcoder(token.clone(), &transcoder_share_token().unwrap());
    sess.call_with_address(
        token.clone(),
        "PSP22::balance_of",
        &[sender.to_string()],
        None,
    )
    .unwrap();
    let balance: Result<u128, drink::errors::LangError> = sess.last_call_return().unwrap();
    Ok((balance.unwrap(), sess))
}

#[allow(dead_code)]
pub enum RoleType {
    AddAgent,
    UpdateAgents,
    DisableAgent,
    RemoveAgent,
    SetCodeHash,
}
pub fn get_role(
    mut sess: Session<MinimalRuntime>,
    registry: &AccountId32,
    role_type: &RoleType,
) -> Result<(AccountId32, Session<MinimalRuntime>), Box<dyn Error>> {
    let role_string = match role_type {
        RoleType::AddAgent => "AddAgent",
        RoleType::UpdateAgents => "UpdateAgents",
        RoleType::DisableAgent => "DisableAgent",
        RoleType::RemoveAgent => "RemoveAgent",
        RoleType::SetCodeHash => "SetCodeHash",
    };
    sess.call_with_address(registry.clone(), "IRegistry::get_role", &[role_string], None)?;

    let role: Result<AccountId32, drink::errors::LangError> = sess.last_call_return().unwrap();
    Ok((role.unwrap(), sess))
}
pub fn get_role_admin(
    mut sess: Session<MinimalRuntime>,
    registry: &AccountId32,
    role_type: &RoleType,
) -> Result<(AccountId32, Session<MinimalRuntime>), Box<dyn Error>> {
    let role_string = match role_type {
        RoleType::AddAgent => "AddAgent",
        RoleType::UpdateAgents => "UpdateAgents",
        RoleType::DisableAgent => "DisableAgent",
        RoleType::RemoveAgent => "RemoveAgent",
        RoleType::SetCodeHash => "SetCodeHash",
    };
    sess.call_with_address(registry.clone(), "IRegistry::get_role_admin", &[role_string], None)?;

    let admin: Result<AccountId32, drink::errors::LangError> = sess.last_call_return().unwrap();
    Ok((admin.unwrap(), sess))
}
pub fn transfer_role(
    sess: Session<MinimalRuntime>,
    registry: &AccountId32,
    sender: &AccountId32,
    role_type: &RoleType,
    new_account: &AccountId32,
) -> Result<Session<MinimalRuntime>, Box<dyn Error>> {
    let role_string = match role_type {
        RoleType::AddAgent => "AddAgent",
        RoleType::UpdateAgents => "UpdateAgents",
        RoleType::DisableAgent => "DisableAgent",
        RoleType::RemoveAgent => "RemoveAgent",
        RoleType::SetCodeHash => "SetCodeHash",
    };
    let sess = call_function(
        sess,
        &registry,
        &sender,
        String::from("IRegistry::transfer_role"),
        Some([role_string.to_string(), new_account.to_string()].to_vec()),
        None,
        transcoder_registry(),
    )?;
    Ok(sess)
}
pub fn transfer_role_admin(
    sess: Session<MinimalRuntime>,
    registry: &AccountId32,
    sender: &AccountId32,
    role_type: &RoleType,
    new_account: &AccountId32,
) -> Result<Session<MinimalRuntime>, Box<dyn Error>> {
    let role_string = match role_type {
        RoleType::AddAgent => "AddAgent",
        RoleType::UpdateAgents => "UpdateAgents",
        RoleType::DisableAgent => "DisableAgent",
        RoleType::RemoveAgent => "RemoveAgent",
        RoleType::SetCodeHash => "SetCodeHash",
    };
    let sess = call_function(
        sess,
        &registry,
        &sender,
        String::from("IRegistry::transfer_role_admin"),
        Some([role_string.to_string(), new_account.to_string()].to_vec()),
        None,
        transcoder_registry(),
    )?;
    Ok(sess)
}
pub fn get_role_adjust_fee(
    mut sess: Session<MinimalRuntime>,
    vault: &AccountId32,
) -> Result<(AccountId32, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.call_with_address(vault.clone(), "IVault::get_role_adjust_fee", NO_ARGS, None)?;

    let adjust_fee: Result<AccountId32, drink::errors::LangError> = sess.last_call_return().unwrap();
    Ok((adjust_fee.unwrap(), sess))
}
pub fn get_role_fee_to(
    mut sess: Session<MinimalRuntime>,
    vault: &AccountId32,
) -> Result<(AccountId32, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.call_with_address(vault.clone(), "IVault::get_role_fee_to", NO_ARGS, None)?;

    let fee_to: Result<AccountId32, drink::errors::LangError> = sess.last_call_return().unwrap();
    Ok((fee_to.unwrap(), sess))
}
pub fn get_role_set_code(
    mut sess: Session<MinimalRuntime>,
    vault: &AccountId32,
) -> Result<(Option<AccountId32>, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.call_with_address(vault.clone(), "IVault::get_role_set_code", NO_ARGS, None)?;

    let set_code: Result<Option<AccountId32>, drink::errors::LangError> = sess.last_call_return().unwrap();
    Ok((set_code.unwrap(), sess))
}
pub fn get_agents(
    mut sess: Session<MinimalRuntime>,
    registry: &AccountId32,
) -> Result<(u128, Vec<Agent>, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.call_with_address(registry.clone(), "IRegistry::get_agents", NO_ARGS, None)?;

    let result: Result<(u128, Vec<Agent>), drink::errors::LangError> = sess.last_call_return().unwrap();
    let (total_weight, agents) = result.unwrap();

    Ok((total_weight, agents, sess))
}
pub fn get_current_virtual_shares(
    sess: Session<MinimalRuntime>,
    vault: &AccountId32,
) -> Result<(u128, Session<MinimalRuntime>), Box<dyn Error>> {
    let sess: Session<MinimalRuntime> = call_function(
        sess,
        vault,
        &AccountId32::new([1u8; 32]),
        String::from("IVault::get_current_virtual_shares"),
        None,
        None,
        transcoder_vault(),
    )
    .unwrap();
    let virtual_shares: Result<u128, drink::errors::LangError> = sess.last_call_return().unwrap();
    Ok((virtual_shares.unwrap(), sess))
}
pub fn get_azero_from_shares(
    sess: Session<MinimalRuntime>,
    vault: &AccountId32,
    shares: u128,
) -> Result<(u128, Session<MinimalRuntime>), Box<dyn Error>> {
    let sess: Session<MinimalRuntime> = call_function(
        sess,
        vault,
        &AccountId32::new([1u8; 32]),
        String::from("IVault::get_azero_from_shares"),
        Some([shares.clone().to_string()].to_vec()),
        None,
        transcoder_vault(),
    )
        .unwrap();
    let azero: Result<u128, drink::errors::LangError> = sess.last_call_return().unwrap();
    Ok((azero.unwrap(), sess))
}
pub fn get_total_pooled(
    sess: Session<MinimalRuntime>,
    vault: &AccountId32,
) -> Result<(u128, Session<MinimalRuntime>), Box<dyn Error>> {
    let sess: Session<MinimalRuntime> = call_function(
        sess,
        vault,
        &AccountId32::new([1u8; 32]),
        String::from("IVault::get_total_pooled"),
        None,
        None,
        transcoder_vault(),
    )
    .unwrap();
    let total_pooled: Result<u128, drink::errors::LangError> = sess.last_call_return().unwrap();
    Ok((total_pooled.unwrap(), sess))
}
pub fn query_nominator_balance(
    sess: Session<MinimalRuntime>,
    nominator: &AccountId32,
) -> Result<(u128, u128, Session<MinimalRuntime>), Box<dyn Error>> {
    let sess = call_function(
        sess,
        &nominator,
        &AccountId32::new([1u8; 32]),
        String::from("INominationAgent::get_unbonding_value"),
        None,
        None,
        transcoder_nominator(),
    )
    .unwrap();
    let unbonded: Result<u128, drink::errors::LangError> = sess.last_call_return().unwrap();
    let unbond = unbonded.unwrap();
    let sess = call_function(
        sess,
        &nominator,
        &AccountId32::new([1u8; 32]),
        String::from("INominationAgent::get_staked_value"),
        None,
        None,
        transcoder_nominator(),
    )
    .unwrap();
    let staked: Result<u128, drink::errors::LangError> = sess.last_call_return().unwrap();
    let stake = staked.unwrap();

    Ok((stake, unbond, sess))
}
pub fn query_token_balance(
    mut sess: Session<MinimalRuntime>,
    token: &AccountId32,
    user: &AccountId32,
) -> Result<(u128, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.set_transcoder(token.clone(), &transcoder_share_token().unwrap());
    sess.call_with_address(
        token.clone(),
        "PSP22::balance_of",
        &[user.to_string()],
        None,
    )?;

    let balance: Result<u128, drink::errors::LangError> = sess.last_call_return().unwrap();
    Ok((balance.unwrap(), sess))
}
pub fn call_redeem(
    mut sess: Session<MinimalRuntime>,
    vault: &AccountId32,
    sender: &AccountId32,
    index: u64,
) -> Result<(u128, Session<MinimalRuntime>), Box<dyn Error>> {
    let prev_balance = sess.chain_api().balance(&sender);

    let mut sess = call_function(
        sess,
        &vault,
        &sender,
        String::from("IVault::redeem"),
        Some([sender.clone().to_string(), index.to_string()].to_vec()),
        None,
        transcoder_vault(),
    )?;

    let updated_balance = sess.chain_api().balance(&sender);
    let gained = updated_balance - prev_balance;

    Ok((gained, sess))
}

pub fn call_redeem_with_withdraw(
    mut sess: Session<MinimalRuntime>,
    vault: &AccountId32,
    sender: &AccountId32,
    index: u64,
) -> Result<(u128, Session<MinimalRuntime>), Box<dyn Error>> {
    let prev_balance = sess.chain_api().balance(&sender);

    let mut sess = call_function(
        sess,
        &vault,
        &sender,
        String::from("IVault::redeem_with_withdraw"),
        Some([sender.clone().to_string(), index.to_string()].to_vec()),
        None,
        transcoder_vault(),
    )?;

    let updated_balance = sess.chain_api().balance(&sender);
    let gained = updated_balance - prev_balance;

    Ok((gained, sess))
}

pub fn call_withdraw_fees(
    sess: Session<MinimalRuntime>,
    vault: &AccountId32,
    sender: &AccountId32,
) -> Result<Session<MinimalRuntime>, Box<dyn Error>> {
    let sess = call_function(
        sess,
        &vault,
        &sender,
        String::from("IVault::withdraw_fees"),
        None,
        None,
        transcoder_vault(),
    )?;
    Ok(sess)
}

pub fn call_function(
    mut sess: Session<MinimalRuntime>,
    contract: &AccountId32,
    sender: &AccountId32,
    func_name: String,
    args: Option<Vec<String>>,
    value: Option<u128>,
    transcoder: Option<Rc<ContractMessageTranscoder>>,
) -> Result<Session<MinimalRuntime>, Box<dyn Error>> {
    println!("Calling: {}()", func_name);
    if let Some(args) = args {
        sess.set_actor(sender.clone());
        sess.set_transcoder(contract.clone(), &transcoder.unwrap());
        sess.call_with_address(contract.clone(), &func_name, &args, value)?;
    } else {
        sess.set_actor(sender.clone());
        sess.set_transcoder(contract.clone(), &transcoder.unwrap());
        sess.call_with_address(contract.clone(), &func_name, NO_ARGS, value)?;
    }

    // Print debug logs
    let encoded = &sess.last_call_result().unwrap().debug_message;
    let decoded = encoded.iter().map(|b| *b as char).collect::<String>();
    let messages: Vec<String> = decoded.split('\n').map(|s| s.to_string()).collect();
    for line in messages {
        if line.len() > 0 {
            println!("LOG: {}", line);
        }
    }

    Ok(sess)
}
