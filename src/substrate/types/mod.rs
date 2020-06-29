mod sub;
use sp_finality_grandpa::{AuthorityId, AuthorityWeight};
use serde::{Deserialize, Serialize};

pub type SignedBlockWithAuthoritySet = sub::SignedBlockWithAuthoritySet;
pub type Number = sub::BlockNumber;
pub type Hash = sub::Hash;
pub type Header = sub::Header;
pub type SignedBlock = sub::SignedBlock;
pub type BlockNumber = sub::BlockNumber;
pub type AuthorityList = Vec<(AuthorityId, AuthorityWeight)>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HashRpcResponse {
    pub result: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockRpcResponse {
    pub result: SignedBlock,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthSetRpcResponse {
    pub result: String,  // scale-encoded AuthorityList
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthSetIdRpcResponse {
    pub result: String,
}

impl AuthSetIdRpcResponse {
    pub fn as_u64(&self) -> u64 {
        match u64::from_str_radix(&self.result[2..], 16) {
            Ok(val) => val,
            Err(e) => panic!(e)
        }
    }
}
