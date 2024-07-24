use drink::session::contract_transcode::ContractMessageTranscoder;
use serde::Deserialize;
use serde_json::from_str;
use std::{
    fs::{read, read_to_string},
    path::PathBuf,
    rc::Rc,
};

// Fetch deployed hashes

#[derive(Deserialize)]
struct Source {
    hash: String,
}
#[derive(Deserialize)]
struct Artifact {
    source: Source,
}
pub fn hash_governance_nft() -> String {
    let json = read_to_string("../deployments/governance_nft/governance_nft.json").unwrap();
    let artifact: Artifact = from_str(&json).unwrap();
    artifact.source.hash
}
pub fn transcoder_governance_nft() -> Option<Rc<ContractMessageTranscoder>> {
    Some(Rc::new(
        ContractMessageTranscoder::load(PathBuf::from(
            "../deployments/governance_nft/governance_nft.json",
        ))
            .expect("Failed to create transcoder"),
    ))
}
pub fn bytes_governance_nft() -> Vec<u8> {
    read("../deployments/governance_nft/governance_nft.wasm")
        .expect("Failed to find or read contract file")
}

pub fn hash_governance_token() -> String {
    let json = read_to_string("../deployments/governance_token/governance_token.json").unwrap();
    let artifact: Artifact = from_str(&json).expect("Should extract hash from share_token.json");
    artifact.source.hash
}

// Transcoders for making contract calls

pub fn transcoder_governance_token() -> Option<Rc<ContractMessageTranscoder>> {
    Some(Rc::new(
        ContractMessageTranscoder::load(PathBuf::from(
            "../deployments/governance_token/governance_token.json",
        ))
            .expect("Failed to create transcoder"),
    ))
}
pub fn bytes_governance_token() -> Vec<u8> {
    read("../deployments/governance_token/governance_token.wasm")
        .expect("Failed to find or read contract file")
}

pub fn hash_governance_staking() -> String {
    let json = read_to_string("../deployments/governance_staking/governance_staking.json").unwrap();
    let artifact: Artifact = from_str(&json).expect("Should extract hash from share_token.json");
    artifact.source.hash
}

// Transcoders for making contract calls

pub fn transcoder_governance_staking() -> Option<Rc<ContractMessageTranscoder>> {
    Some(Rc::new(
        ContractMessageTranscoder::load(PathBuf::from(
            "../deployments/governance_staking/governance_staking.json",
        ))
            .expect("Failed to create transcoder"),
    ))
}
pub fn bytes_governance_staking() -> Vec<u8> {
    read("../deployments/governance_staking/governance_staking.wasm")
        .expect("Failed to find or read contract file")
}

pub fn hash_governance() -> String {
    let json = read_to_string("../deployments/governance/governance.json").unwrap();
    let artifact: Artifact = from_str(&json).expect("Should extract hash from share_token.json");
    artifact.source.hash
}

// Transcoders for making contract calls

pub fn transcoder_governance() -> Option<Rc<ContractMessageTranscoder>> {
    Some(Rc::new(
        ContractMessageTranscoder::load(PathBuf::from(
            "../deployments/governance/governance.json",
        ))
            .expect("Failed to create transcoder"),
    ))
}
pub fn bytes_governance() -> Vec<u8> {
    read("../deployments/governance/governance.wasm")
        .expect("Failed to find or read contract file")
}

pub fn hash_registry() -> String {
    let json = read_to_string("../deployments/registry/registry.json").unwrap();
    let artifact: Artifact = from_str(&json).unwrap();
    artifact.source.hash
}
pub fn hash_share_token() -> String {
    let json = read_to_string("../deployments/share_token/share_token.json").unwrap();
    let artifact: Artifact = from_str(&json).expect("Should extract hash from share_token.json");
    artifact.source.hash
}

pub fn hash_nominator() -> String {
    let json = read_to_string("../deployments/mock_nominator/mock_nominator.json").unwrap();
    let artifact: Artifact = from_str(&json).expect("Should extract hash from mock_nominator.json");
    artifact.source.hash
}
pub fn hash_multisig() -> String {
    let json = read_to_string("../deployments/multisig/multisig.json").unwrap();
    let artifact: Artifact = from_str(&json).expect("Should extract hash from multisig.json");
    artifact.source.hash
}

// Transcoders for making contract calls

pub fn transcoder_registry() -> Option<Rc<ContractMessageTranscoder>> {
    Some(Rc::new(
        ContractMessageTranscoder::load(PathBuf::from(
            "../deployments/registry/registry.json",
        ))
            .expect("Failed to create transcoder"),
    ))
}
pub fn transcoder_share_token() -> Option<Rc<ContractMessageTranscoder>> {
    Some(Rc::new(
        ContractMessageTranscoder::load(PathBuf::from(
            "../deployments/share_token/share_token.json",
        ))
            .expect("Failed to create transcoder"),
    ))
}
pub fn transcoder_vault() -> Option<Rc<ContractMessageTranscoder>> {
    Some(Rc::new(
        ContractMessageTranscoder::load(PathBuf::from(
            "../deployments/vault/vault.json",
        ))
            .expect("Failed to create transcoder"),
    ))
}
pub fn transcoder_nominator() -> Option<Rc<ContractMessageTranscoder>> {
    Some(Rc::new(
        ContractMessageTranscoder::load(PathBuf::from(
            "../deployments/mock_nominator/mock_nominator.json",
        ))
            .expect("Failed to create transcoder"),
    ))
}
pub fn transcoder_multisig() -> Option<Rc<ContractMessageTranscoder>> {
    Some(Rc::new(
        ContractMessageTranscoder::load(PathBuf::from(
            "../deployments/multisig/multisig.json",
        ))
            .expect("Failed to create transcoder"),
    ))
}
// Bytes for instantiating contracts
pub fn bytes_multisig() -> Vec<u8> {
    read("../deployments/multisig/multisig.wasm")
        .expect("Failed to find or read contract file")
}
pub fn bytes_registry() -> Vec<u8> {
    read("../deployments/registry/registry.wasm")
        .expect("Failed to find or read contract file")
}
pub fn bytes_share_token() -> Vec<u8> {
    read("../deployments/share_token/share_token.wasm")
        .expect("Failed to find or read contract file")
}
pub fn bytes_vault() -> Vec<u8> {
    read("../deployments/vault/vault.wasm")
        .expect("Failed to find or read contract file")
}
pub fn bytes_nominator() -> Vec<u8> {
    read("../deployments/mock_nominator/mock_nominator.wasm")
        .expect("Failed to find or read contract file")
}
