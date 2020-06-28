use serde::{Deserialize, Serialize};
use tendermint::block::signed_header::SignedHeader;
use tendermint_light_client::{ClientId, LightValidator, LightValidatorSet};

/// TMHeader serializes to the same form as TMHeader in wormhole, but is using Tendermint types,
/// not tendermint_light_client types - although structurewise, these are compatible.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TMHeader {
    pub signed_header: SignedHeader,
    pub validator_set: std::vec::Vec<tendermint::validator::Info>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TMCreateClientPayload {
    pub header: TMHeader,
    pub trusting_period: u64,
    pub max_clock_drift: u64,
    pub unbonding_period: u64,
    pub client_id: ClientId,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TMUpdateClientPayload {
    pub header: TMHeader,
    pub client_id: ClientId,
    pub next_validator_set: LightValidatorSet<LightValidator>,
}
