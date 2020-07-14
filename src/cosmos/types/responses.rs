use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AccountQueryResponse {
    #[serde(with = "crate::utils::from_str")]
    pub height: u64,
    pub result: AccountQueryResponseResult,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AccountQueryResponseResult {
    pub value: AccountQueryResponseResultValue,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AccountQueryResponseResultValue {
    #[serde(default = "default_pubkey")]
    pub public_key: String,
    pub address: String,
    #[serde(with = "crate::utils::from_str")]
    pub account_number: u64,
    #[serde(with = "crate::utils::from_str", default = "default_sequence")]
    pub sequence: u64,
}

/// Define the default sequence when no sequence exists (new account).
fn default_sequence() -> u64 {
    0
}

/// Define the default pubkey when no pubkey exists (new account).
fn default_pubkey() -> String {
    "".to_owned()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TxRpcResponse {
    #[serde(with = "crate::utils::from_str")]
    pub height: u64,
    pub txhash: String,
    #[serde(default = "default_code")]
    pub code: u64,
    pub raw_log: String,
}

/// Define the default code when no sequence exists (successful tx).
fn default_code() -> u64 {
    0
}
