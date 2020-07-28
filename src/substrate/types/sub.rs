use parity_scale_codec::Encode;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde::Deserialize;
use sp_finality_grandpa::AuthorityList;

use sp_runtime::{traits::BlakeTwo256, OpaqueExtrinsic};

pub type BlockNumber = u32;
pub type Header = sp_runtime::generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = sp_runtime::generic::Block<Header, OpaqueExtrinsic>;
pub type SignedBlock = sp_runtime::generic::SignedBlock<Block>;

#[derive(Deserialize, Clone, Debug)]
pub struct SignedBlockWithAuthoritySet {
    pub block: SignedBlock,
    pub authority_set: AuthorityList,
    pub set_id: u64,
}

/// As name suggests this structure is only used
/// while creating substrate light client
#[derive(Deserialize, Clone, Debug)]
pub struct CreateSignedBlockWithAuthoritySet {
    pub block: SignedBlock,
    pub authority_set: AuthorityList,
    pub set_id: u64,
    pub max_headers_allowed_to_store: u64,
    pub max_headers_allowed_between_justifications: u64
}

impl Serialize for CreateSignedBlockWithAuthoritySet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("SignedBlockWithAuthoritySet", 3)?;
        let block_hex = "0x".to_owned() + &hex::encode(&self.block.encode());
        state.serialize_field("block", &(block_hex))?;
        let set_hex = "0x".to_owned() + &hex::encode(&self.authority_set.encode());
        state.serialize_field("authority_set", &(set_hex))?;
        state.serialize_field("set_id", &self.set_id)?;
        state.serialize_field("max_headers_allowed_to_store", &self.max_headers_allowed_to_store)?;
        state.serialize_field("max_headers_allowed_between_justifications", &self.max_headers_allowed_between_justifications)?;
        state.end()
    }
}


impl SignedBlockWithAuthoritySet {
    pub fn from_parts(
        block: SignedBlock,
        authority_set: AuthorityList,
        authority_set_id: u64,
    ) -> Self {
        SignedBlockWithAuthoritySet {
            block: block,
            authority_set: authority_set,
            set_id: authority_set_id,
        }
    }
}

impl Serialize for SignedBlockWithAuthoritySet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("SignedBlockWithAuthoritySet", 3)?;
        let block_hex = "0x".to_owned() + &hex::encode(&self.block.encode());
        state.serialize_field("block", &(block_hex))?;
        let set_hex = "0x".to_owned() + &hex::encode(&self.authority_set.encode());
        state.serialize_field("authority_set", &(set_hex))?;
        state.serialize_field("set_id", &self.set_id)?;
        state.end()
    }
}
