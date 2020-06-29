use serde::{Deserialize, Serialize};

use sp_finality_grandpa::AuthorityList;
use sp_runtime::{traits::BlakeTwo256, OpaqueExtrinsic};

pub type BlockNumber = u32;
pub type Hash = sp_core::H256;
pub type Header = sp_runtime::generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = sp_runtime::generic::Block<Header, OpaqueExtrinsic>;
pub type SignedBlock = sp_runtime::generic::SignedBlock<Block>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignedBlockWithAuthoritySet {
    pub block: SignedBlock,
    pub authority_set: String,
    pub set_id: u64,
}

impl SignedBlockWithAuthoritySet {
    pub fn from_parts(block: SignedBlock, authority_set: String, authority_set_id: u64) -> Self {
        SignedBlockWithAuthoritySet{
            block: block,
            authority_set: authority_set,
            set_id: authority_set_id,
        }
    }
}
