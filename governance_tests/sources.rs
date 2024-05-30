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