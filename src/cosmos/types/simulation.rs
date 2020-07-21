use crate::cosmos::types::tm::TMHeader;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub(crate) header: TMHeader,
    pub(crate) next_validators: Vec<tendermint::validator::Info>,
}
