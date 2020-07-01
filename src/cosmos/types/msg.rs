use crate::substrate::types::SignedBlockWithAuthoritySet;
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MsgUpdateWasmClient {
    pub client_id: String,
    pub header: SignedBlockWithAuthoritySet,
    pub address: String,
}
