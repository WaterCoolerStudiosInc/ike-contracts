use drink::{
    chain_api::ChainApi,
    runtime::MinimalRuntime,
    session::{contract_transcode::ContractMessageTranscoder, Session, NO_ARGS},
    AccountId32,
};
use std::{error::Error, rc::Rc};

// Publicize all sources module methods (hash_*, transcoder_*, bytes_*)
pub use crate::sources::*;
use crate::tests::TokenTransfer;
pub const SECOND: u64 = 1_000;
pub const DAY: u64 = SECOND * 86400;
pub const YEAR: u64 = DAY * 365_25 / 100;
pub const BIPS: u128 = 10000;
#[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct Proposal {
    pub creation_timestamp: u64,
    pub creator_id: u128,
    pub prop_id: String,
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
    TransferFunds(TokenTransfer),
    UpdateStakingRewards(u128),
    AddCouncilMember(AccountId32),
    RemoveCouncilMember(AccountId32),
    ThresholdChange(u16),
    FeeChange(u16),
    VoteDelayUpdate(u64),
    VotePeriodUpdate(u64),
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

pub fn query_proposal(
    mut sess: Session<MinimalRuntime>,
    governance: AccountId32,
    prop_id: u128,
) -> Result<(Proposal, Session<MinimalRuntime>), Box<dyn Error>> {
    sess.set_transcoder(governance.clone(), &transcoder_governance().unwrap());
    sess.call_with_address(
        governance.clone(),
        "get_proposal_by_id",
        &[prop_id.to_string()],
        None,
    )?;

    let prop: Result<Proposal, drink::errors::LangError> = sess.last_call_return().unwrap();
    Ok((prop.unwrap(), sess))
}
