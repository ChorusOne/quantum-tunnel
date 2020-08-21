use crate::cosmos::types::Coins;
use crate::substrate::types::{CreateSignedBlockWithAuthoritySet, SignedBlockWithAuthoritySet};
use serde::{Deserialize, Serialize};

pub trait StdMsg {
    fn get_type() -> String
    where
        Self: Sized;
}

/// Payload to initialize substrate light client
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MsgCreateWasmClient {
    pub client_id: String,
    pub header: CreateSignedBlockWithAuthoritySet,
    pub trusting_period: String,
    pub unbonding_period: String,
    pub max_clock_drift: String,
    pub address: String,
    #[serde(with = "crate::utils::from_str")]
    pub wasm_id: u32,
}

impl StdMsg for MsgCreateWasmClient {
    fn get_type() -> String {
        "ibc/client/MsgCreateWasmClient".to_owned()
    }
}

/// Payload to update substrate light client
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MsgUpdateWasmClient {
    pub client_id: String,
    pub header: SignedBlockWithAuthoritySet,
    pub address: String,
}

impl StdMsg for MsgUpdateWasmClient {
    fn get_type() -> String {
        "ibc/client/MsgUpdateWasmClient".to_owned()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MsgSend {
    pub from_address: String,
    pub to_address: String,
    pub amount: Coins,
}

impl StdMsg for MsgSend {
    fn get_type() -> String {
        "cosmos-sdk/MsgSend".to_owned()
    }
}
