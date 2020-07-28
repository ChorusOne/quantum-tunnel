mod sub;
use parity_scale_codec::Decode;
use serde::{Deserialize, Serialize};
use sp_finality_grandpa::{AuthorityId, AuthorityWeight, VersionedAuthorityList};

pub type SignedBlockWithAuthoritySet = sub::SignedBlockWithAuthoritySet;
pub type CreateSignedBlockWithAuthoritySet = sub::CreateSignedBlockWithAuthoritySet;
pub type SignedBlock = sub::SignedBlock;
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
    pub result: String, // scale-encoded AuthorityList
}

impl AuthSetRpcResponse {
    pub fn get_authset(&self) -> AuthorityList {
        let bytes = hex::decode(&self.result[2..]).unwrap();
        (VersionedAuthorityList::decode(&mut bytes.as_slice()).unwrap()).into()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthSetIdRpcResponse {
    pub result: String,
}

impl AuthSetIdRpcResponse {
    pub fn as_u64(&self) -> u64 {
        match u64::from_str_radix(&self.result[2..], 16) {
            Ok(val) => val,
            Err(e) => panic!(e),
        }
    }
}
