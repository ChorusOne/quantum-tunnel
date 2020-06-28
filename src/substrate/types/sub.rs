use serde::{Deserialize, Serialize};

use sp_finality_grandpa::AuthorityList;
use sp_runtime::{traits::BlakeTwo256, OpaqueExtrinsic};

pub type BlockNumber = u32;
pub type Header = sp_runtime::generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = sp_runtime::generic::Block<Header, OpaqueExtrinsic>;
pub type SignedBlock = sp_runtime::generic::SignedBlock<Block>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignedBlockWithAuthoritySet {
    pub block: SignedBlock,
    pub authority_set: AuthorityList,
    pub set_id: u64,
}
