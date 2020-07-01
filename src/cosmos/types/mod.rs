mod tm;
mod msg;
mod stdtx;

pub type TMHeader = tm::TMHeader;
pub type MsgCreateWasmClient = msg::MsgCreateWasmClient;
pub type MsgUpdateWasmClient = msg::MsgUpdateWasmClient;
pub type StdTx = stdtx::StdTx;
pub type Coin = stdtx::Coin;
pub type StdFee = stdtx::StdFee;
pub type DecCoin = stdtx::DecCoin;
pub type TMUpdateClientPayload = tm::TMUpdateClientPayload;
pub type TMCreateClientPayload = tm::TMCreateClientPayload;
