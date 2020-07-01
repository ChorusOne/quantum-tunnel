use crate::substrate::types::SignedBlockWithAuthoritySet;
use serde::{Deserialize, Serialize};
use crate::cosmos::types::Coins;

pub trait StdMsg {
    fn get_type() -> String;
}

// ClientID        string         `json:"client_id" yaml:"client_id"`
// Header          json.RawMessage `json:"header" yaml:"header"`
// TrustingPeriod  time.Duration  `json:"trusting_period" yaml:"trusting_period"`
// UnbondingPeriod time.Duration  `json:"unbonding_period" yaml:"unbonding_period"`
// MaxClockDrift	time.Duration  `json:"max_clock_drift" yaml:"max_clock_drift"`
// Signer          sdk.AccAddress `json:"address" yaml:"address"`
// WasmId          int          `json:"wasm_id" yaml:"wasm_id"`

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MsgCreateWasmClient {
    pub client_id: String,
    pub header: SignedBlockWithAuthoritySet,
    pub trusting_period: String,
    pub unbonding_period: String,
    pub max_clock_drift: String,
    pub address: String,
    pub wasm_id: u32,
}

impl StdMsg for MsgCreateWasmClient {
    fn get_type() -> String {
        "cosmos-sdk/MsgCreateWasmClient".to_owned()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MsgUpdateWasmClient {
    pub client_id: String,
    pub header: SignedBlockWithAuthoritySet,
    pub address: String,
}

impl StdMsg for MsgUpdateWasmClient {
    fn get_type() -> String {
        "cosmos-sdk/MsgUpdateWasmClient".to_owned()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MsgSend {
    pub from_address: String,
    pub to_address: String,
    pub amount: Coins
}

impl StdMsg for MsgSend {
    fn get_type() -> String {
        "cosmos-sdk/MsgSend".to_owned()
    }
}

//{"account_number":"0","chain_id":"test","fee":{"amount":[{"amount":"150","denom":"atom"}],"gas":"100000"},"memo":"oh hai","msgs":[{"type":"cosmos-sdk/MsgSend","value":{"amount":[{"amount":"25","denom":"stake"}],"from_address":"cosmos1a2wjatdh7k80a33qatlgqldmadxxxe3ce573d6","to_address":"cosmos1w6w5afvnqraw5w3g0kshf4kvq6d87tdy0nyxaa"}}],"sequence":"0"}
