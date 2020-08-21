use crate::cosmos::types::StdSignature;
use cast::u64;
use log::info;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

//TODO: add amino prost encoding for all this. or better still, wait for protobuf...
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StdTx {
    pub msg: Vec<Value>,
    pub fee: StdFee,
    pub signatures: Vec<StdSignature>,
    pub memo: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StdSignDoc {
    #[serde(with = "crate::utils::from_str")]
    pub account_number: u64,
    pub chain_id: String,
    pub fee: Value,
    pub memo: String,
    pub msgs: Vec<Value>,
    #[serde(with = "crate::utils::from_str")]
    pub sequence: u64,
}

impl StdTx {
    pub fn get_sign_bytes(&self, chain_id: String, acc_num: u64, sequence: u64) -> Vec<u8> {
        std_sign_bytes(
            chain_id,
            acc_num,
            sequence,
            self.fee.clone(),
            self.msg.clone(),
            self.memo.clone(),
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StdFee {
    #[serde(with = "crate::utils::from_str")]
    pub gas: u64,
    pub amount: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Coin {
    #[serde(with = "crate::utils::from_str")]
    amount: u64,
    denom: String,
}

pub type Coins = Vec<Coin>;

impl Coin {
    #[allow(dead_code)] // Used only in test
    pub fn from(str: String) -> Self {
        info!("{}", str);
        let re = Regex::new(r"^(\d+)([a-z]+)$").unwrap();
        let caps = re.captures(&str).unwrap();
        Coin {
            amount: str::parse(caps.get(1).unwrap().as_str()).unwrap(),
            denom: caps.get(2).unwrap().as_str().to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DecCoin {
    amount: f64,
    denom: String,
}

impl DecCoin {
    pub fn from(str: String) -> Self {
        info!("{}", str);
        let re = Regex::new(r"^(\d+(\.\d*)?)([a-z]+)$").unwrap();
        let caps = re.captures(&str).unwrap();
        DecCoin {
            amount: str::parse(caps.get(1).unwrap().as_str()).unwrap(),
            denom: caps.get(3).unwrap().as_str().to_string(),
        }
    }

    pub fn mul(&mut self, mult: f64) -> &Self {
        self.amount = self.amount * mult;
        self
    }

    pub fn to_coin(&self) -> Coin {
        let amount: u64 = u64(self.amount.abs()).unwrap();
        Coin {
            amount: amount,
            denom: self.denom.clone(),
        }
    }
}

fn std_sign_bytes(
    chain_id: String,
    acc_num: u64,
    sequence: u64,
    fee: StdFee,
    msgs: Vec<Value>,
    memo: String,
) -> Vec<u8> {
    let s = StdSignDoc {
        account_number: acc_num,
        chain_id,
        fee: json!(fee),
        memo,
        msgs,
        sequence,
    };

    serde_json::to_vec(&s).unwrap()
    // sort json
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cosmos::types::msg::MsgSend;
    use crate::cosmos::types::StdMsg;
    use std::str::from_utf8;

    #[test]
    fn test_serialize_msgsend() {
        // make sure we match the golang implementation
        let msg = MsgSend {
            from_address: "cosmos1a2wjatdh7k80a33qatlgqldmadxxxe3ce573d6".to_owned(),
            to_address: "cosmos1w6w5afvnqraw5w3g0kshf4kvq6d87tdy0nyxaa".to_owned(),
            amount: vec![Coin::from("25stake".to_string())],
        };

        let m = vec![serde_json::json!({"type": MsgSend::get_type(), "value": msg})];
        let f = StdFee {
            gas: 100000,
            amount: vec![Coin::from("150atom".to_string())],
        };

        let tx = StdTx {
            msg: m,
            fee: f,
            signatures: vec![],
            memo: "oh hai".to_owned(),
        };

        assert_eq!(from_utf8(tx.get_sign_bytes("test".to_owned(), 0, 0).as_slice()).unwrap(), r#"{"account_number":"0","chain_id":"test","fee":{"amount":[{"amount":"150","denom":"atom"}],"gas":"100000"},"memo":"oh hai","msgs":[{"type":"cosmos-sdk/MsgSend","value":{"amount":[{"amount":"25","denom":"stake"}],"from_address":"cosmos1a2wjatdh7k80a33qatlgqldmadxxxe3ce573d6","to_address":"cosmos1w6w5afvnqraw5w3g0kshf4kvq6d87tdy0nyxaa"}}],"sequence":"0"}"#.to_string())
    }
}
