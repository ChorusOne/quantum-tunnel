use crate::cosmos::types::tm::TMHeader;
use serde::{Deserialize, Serialize};

/// Simulation message read by `simulation_recv_handler`
#[derive(Serialize, Deserialize)]
pub struct Message {
    pub(crate) header: TMHeader,
    pub(crate) next_validators: Vec<tendermint::validator::Info>,
}
