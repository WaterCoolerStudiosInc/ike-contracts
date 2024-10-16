use drink::{
    chain_api::ChainApi,
    runtime::MinimalRuntime,
    session::{contract_transcode::ContractMessageTranscoder, Session, NO_ARGS},
    AccountId32 as AccountId,
};
use hex_literal;
use serde::{Deserialize, Serialize};
use sp_core::{Encode, Pair};
use std::{error::Error, fmt, rc::Rc};
// Publicize all sources module methods (hash_*, transcoder_*, bytes_*)
pub use crate::sources::*;
pub const SECOND: u64 = 1_000;
pub const DAY: u64 = SECOND * 86400;
pub const YEAR: u64 = DAY * 365_25 / 100;
pub const BIPS: u128 = 10000;

#[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct TokenTransfer {
    pub token: AccountId,
    pub amount: u128,
    pub to: AccountId,
}

#[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct Proposal {
    pub creation_timestamp: u64,
    pub creator_id: u128,
    pub prop_id: u128,
    pub prop_type: PropType,
    pub pro_vote_count: u128,
    pub con_vote_count: u128,
    pub vote_start: u64,
    pub vote_end: u64,
}

#[derive(Debug, PartialEq, Eq, scale::Encode, Clone, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum PropType {
    TransferFunds(AccountId, u128, AccountId),
    NativeTokenTransfer(AccountId, u128),
    ChangeStakingRewardRate(u128),
    AddCouncilMember(AccountId),
    ReplaceCouncilMember(AccountId, AccountId),
    RemoveCouncilMember(AccountId),
    ChangeMultiSigThreshold(u16),
    FeeChange(u16),
    CompoundIncentiveChange(u16),
    AcceptanceWeightUpdate(u128),
    VoteDelayUpdate(u64),
    VotePeriodUpdate(u64),
    UpdateRejectThreshhold(u128),
    UpdateExecThreshhold(u128),
    SetCodeHash([u8; 32]),
    UnlockTransfer(),
    LockTransfer(),
}
#[derive(Debug, PartialEq, Eq, scale::Encode, Clone, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum Vote {
    Pro,
    Con,
}
impl fmt::Display for Vote {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl fmt::Display for PropType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PropType::TransferFunds(token,amount,recipient)=>write!(f, "TransferFunds({},{},{})",token,amount,recipient),
            PropType::NativeTokenTransfer(address,amount) => write!(f, "NativeTokenTransfer({},{})", address,amount),
            PropType::ReplaceCouncilMember(address1,address2)=>write!(f, "ReplaceCouncilMember({},{})", address1,address2),
            PropType::RemoveCouncilMember(address)=>write!(f,"RemoveCouncilMember({})",address),
            PropType::AddCouncilMember(address)=>write!(f,"AddCouncilMember({})",address),
             _ =>write!(f, "{:?}", self)
        }
        
    }
}
fn sign(hash: [u8; 32], pk: &str) -> [u8; 65] {
    // Use Dan's seed
    // `subkey inspect //Dan --scheme Ecdsa --output-type json | jq .secretSeed`

    let pair = sp_core::ecdsa::Pair::from_legacy_string(pk, None);

    let signature = pair.sign_prehashed(&hash);
    signature.0
}

pub fn update_days(mut sess: Session<MinimalRuntime>, days: u64) -> Session<MinimalRuntime> {
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
pub fn call_function(
    mut sess: Session<MinimalRuntime>,
    contract: &AccountId,
    sender: &AccountId,
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

pub fn query_governance_get_proposal_by_nft(
    mut sess: Session<MinimalRuntime>,
    governance: &AccountId,
    nft_id: u128,
) -> Result<(Proposal, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.set_transcoder(governance.clone(), &transcoder_governance().unwrap());
    sess.call_with_address(
        governance.clone(),
        "get_proposal_by_nft",
        &[nft_id.to_string()],
        None,
    )?;

    let proposal: Result<Proposal, drink::errors::LangError> = sess.last_call_return().unwrap();
    Ok((proposal.unwrap(), sess))
}
pub fn query_governance_get_all_proposals(
    mut sess: Session<MinimalRuntime>,
    governance: &AccountId,
) -> Result<(Vec<Proposal>, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.set_transcoder(governance.clone(), &transcoder_governance().unwrap());
    sess.call_with_address(governance.clone(), "get_all_proposals", NO_ARGS, None)?;

    let proposals: Result<Vec<Proposal>, drink::errors::LangError> =
        sess.last_call_return().unwrap();
    Ok((proposals.unwrap(), sess))
}
pub fn query_owner(
    mut sess: Session<MinimalRuntime>,
    governance_nft: AccountId,
    nft_id: u128,
) -> Result<(Option<AccountId>, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.set_transcoder(
        governance_nft.clone(),
        &transcoder_governance_nft().unwrap(),
    );
    sess.call_with_address(
        governance_nft.clone(),
        "owner_of_id",
        &[nft_id.to_string()],
        None,
    )?;

    let owner: Result<Option<AccountId>, drink::errors::LangError> =
        sess.last_call_return().unwrap();
    //println!("{:?}",&prop.clone().unwrap());
    Ok((owner.unwrap(), sess))
}
pub fn query_governance_vote_period(
    mut sess: Session<MinimalRuntime>,
    governance:AccountId,
   
) -> Result<(u64, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.set_transcoder(
        governance.clone(),
        &transcoder_governance().unwrap(),
    );
    sess.call_with_address(governance.clone(), "get_voting_period", NO_ARGS, None)?;

    let value: Result<u64, drink::errors::LangError> = sess.last_call_return().unwrap();
    //println!("{:?}",&prop.clone().unwrap());
    Ok((value.unwrap(), sess))
}
pub fn query_governance_vote_delay(
    mut sess: Session<MinimalRuntime>,
    governance: AccountId,
  
) -> Result<(u64, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.set_transcoder(
        governance.clone(),
        &transcoder_governance().unwrap(),
    );
    sess.call_with_address(governance.clone(), "get_voting_delay", NO_ARGS, None)?;

    let value: Result<u64, drink::errors::LangError> = sess.last_call_return().unwrap();
    //println!("{:?}",&prop.clone().unwrap());
    Ok((value.unwrap(), sess))
}
pub fn query_governance_acceptance_threshold(
    mut sess: Session<MinimalRuntime>,
    governance: AccountId,
) -> Result<(Option<u128>, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.set_transcoder(
        governance.clone(),
        &transcoder_governance().unwrap(),
    );
    sess.call_with_address(
        governance.clone(),
        "get_acceptance_threshold",
        NO_ARGS,
        None,
    )?;

    let value: Result<u128, drink::errors::LangError> = sess.last_call_return().unwrap();
    //println!("{:?}",&prop.clone().unwrap());
    Ok((Some(value.unwrap()), sess))
}

pub fn query_governance_rejection_threshold(
    mut sess: Session<MinimalRuntime>,
    governance: AccountId,
) -> Result<(Option<u128>, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.set_transcoder(
        governance.clone(),
        &transcoder_governance().unwrap(),
    );
    sess.call_with_address(
        governance.clone(),
        "get_rejection_threshold",
        NO_ARGS,
        None,
    )?;

    let value: Result<u128, drink::errors::LangError> = sess.last_call_return().unwrap();
    //println!("{:?}",&prop.clone().unwrap());
    Ok((Some(value.unwrap()), sess))
}

pub fn query_governance_execution_threshold(
    mut sess: Session<MinimalRuntime>,
    governance: AccountId,
) -> Result<(Option<u128>, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.set_transcoder(
        governance.clone(),
        &transcoder_governance().unwrap(),
    );
    sess.call_with_address(
        governance.clone(),
        "get_execution_threshold",
        NO_ARGS,
        None,
    )?;

    let value: Result<u128, drink::errors::LangError> = sess.last_call_return().unwrap();
    //println!("{:?}",&prop.clone().unwrap());
    Ok((Some(value.unwrap()), sess))
}
pub fn query_token_balance(
    mut sess: Session<MinimalRuntime>,
    token: &AccountId,
    user: &AccountId,
) -> Result<(u128, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.set_transcoder(token.clone(), &transcoder_governance_token().unwrap());
    sess.call_with_address(
        token.clone(),
        "PSP22::balance_of",
        &[user.to_string()],
        None,
    )?;

    let balance: Result<u128, drink::errors::LangError> = sess.last_call_return().unwrap();
    Ok((balance.unwrap(), sess))
}
pub fn query_allowance(
    mut sess: Session<MinimalRuntime>,
    governance_nft: &AccountId,
    owner: &AccountId,
    operator: &AccountId,
) -> Result<(bool, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.set_transcoder(
        governance_nft.clone(),
        &transcoder_governance_nft().unwrap(),
    );
    sess.call_with_address(
        governance_nft.clone(),
        "PSP34::allowance",
        &[
            owner.to_string(),
            operator.to_string(),
            String::from("None"),
        ],
        None,
    )?;

    let result: Result<bool, drink::errors::LangError> = sess.last_call_return().unwrap();
    //println!("{:?}",&prop.clone().unwrap());
    Ok((result.unwrap(), sess))
}

pub fn gov_token_transfer(
    mut sess: Session<MinimalRuntime>,
    gov_token: &AccountId,
    sender: &AccountId,
    to: &AccountId,
    amount: u128,
) -> Result<Session<MinimalRuntime>, Box<dyn Error>> {
    let sess: Session<MinimalRuntime> = call_function(
        sess,
        &gov_token,
        &sender,
        String::from("PSP22::transfer"),
        Some(vec![to.to_string(), amount.to_string(), "[]".to_string()]),
        None,
        transcoder_governance_token(),
    )?;
    Ok(sess)
}

#[derive(Debug, Deserialize, Serialize, PartialEq, scale::Decode)]
pub struct Schedule {
    pub amount: u128,
    pub cliff: u128,
    pub offset: u64,
    pub duration: u64,
}

pub fn query_vesting_get_admin(
    mut sess: Session<MinimalRuntime>,
    vesting: &AccountId,
) -> Result<(Option<AccountId>, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.call_with_address(vesting.clone(), "get_admin", NO_ARGS, None)?;

    let admin: Result<Option<AccountId>, drink::errors::LangError> =
        sess.last_call_return().unwrap();
    Ok((admin.unwrap(), sess))
}

pub fn query_vesting_get_schedule(
    mut sess: Session<MinimalRuntime>,
    vesting: &AccountId,
    recipient: &AccountId,
) -> Result<(Option<Schedule>, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.call_with_address(
        vesting.clone(),
        "get_schedule",
        &[recipient.to_string()],
        None,
    )?;

    let schedule: Result<Option<Schedule>, drink::errors::LangError> =
        sess.last_call_return().unwrap();
    Ok((schedule.unwrap(), sess))
}

pub fn vesting_add_recipients(
    sess: Session<MinimalRuntime>,
    vesting: &AccountId,
    sender: &AccountId,
    recipients: Vec<&AccountId>,
    schedules: Vec<&Schedule>,
) -> Result<Session<MinimalRuntime>, Box<dyn Error>> {
    let sess: Session<MinimalRuntime> = call_function(
        sess,
        &vesting,
        &sender,
        String::from("add_recipients"),
        Some(vec![
            serde_json::to_string(&recipients).unwrap(),
            serde_json::to_string(&schedules).unwrap(),
        ]),
        None,
        transcoder_vesting(),
    )?;
    Ok(sess)
}

pub fn vesting_remove_recipients(
    sess: Session<MinimalRuntime>,
    vesting: &AccountId,
    sender: &AccountId,
    recipients: Vec<&AccountId>,
) -> Result<Session<MinimalRuntime>, Box<dyn Error>> {
    let sess: Session<MinimalRuntime> = call_function(
        sess,
        &vesting,
        &sender,
        String::from("remove_recipients"),
        Some(vec![serde_json::to_string(&recipients).unwrap()]),
        None,
        transcoder_vesting(),
    )?;
    Ok(sess)
}

pub fn vesting_activate(
    sess: Session<MinimalRuntime>,
    vesting: &AccountId,
    sender: &AccountId,
) -> Result<Session<MinimalRuntime>, Box<dyn Error>> {
    let sess: Session<MinimalRuntime> = call_function(
        sess,
        &vesting,
        &sender,
        String::from("activate"),
        None,
        None,
        transcoder_vesting(),
    )?;
    Ok(sess)
}

pub fn vesting_claim(
    sess: Session<MinimalRuntime>,
    vesting: &AccountId,
    sender: &AccountId,
) -> Result<Session<MinimalRuntime>, Box<dyn Error>> {
    let sess: Session<MinimalRuntime> = call_function(
        sess,
        &vesting,
        &sender,
        String::from("claim"),
        None,
        None,
        transcoder_vesting(),
    )?;
    Ok(sess)
}
