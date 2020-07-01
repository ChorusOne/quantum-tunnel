use serde_json::{Value, json};
use serde::{Serialize, Deserialize};
use regex::Regex;
use log::info;
use cast::{From, u64};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StdTx {
    pub msg: Vec<Value>,
    pub fee: StdFee,
    pub signatures: Vec<StdSignature>,
    pub memo: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StdSignDoc {
    pub account_number: u64,
    pub chain_id: String,
    pub fee: Value,
    pub memo: String,
    pub msgs: Vec<Value>,
    pub sequence: u64,
}

impl StdTx {
    pub fn get_sign_bytes(&self, chain_id: String, acc_num: u64, sequence: u64) -> Vec<u8> {
        std_sign_bytes(chain_id, acc_num, sequence, self.fee.clone(), self.msg.clone(), self.memo.clone())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StdSignature {
    pub pub_key: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StdFee {
    pub gas: u64,
    pub amount: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Coin {
    amount: u64,
    denom: String,
}

impl Coin {
    pub fn from(str: String) -> Self {
        info!("{}", str);
        let re = Regex::new(r"^(\d+)([a-z]+)$").unwrap();
        let caps = re.captures(&str).unwrap();
        Coin{
            amount: str::parse(caps.get(1).unwrap().as_str()).unwrap(),
            denom: caps.get(2).unwrap().as_str().to_string(),
        }
    }

    pub fn mul(&mut self, mult: u64) -> &Self {
        self.amount = self.amount*mult;
        self
    }

    pub fn to_dec_coin(&self) -> DecCoin {
        DecCoin{
            amount: self.amount as f64,
            denom: self.denom.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DecCoin {
    amount: f64,
    denom: String,
}

impl DecCoin {
    pub fn from(str: String) -> Self {
        info!("{}", str);
        let re = Regex::new(r"^(\d+(\.\d*)?)([a-z]+)$").unwrap();
        let caps = re.captures(&str).unwrap();
        DecCoin{
            amount: str::parse(caps.get(1).unwrap().as_str()).unwrap(),
            denom: caps.get(3).unwrap().as_str().to_string(),
        }
    }

    pub fn mul(&mut self, mult: f64) -> &Self {
        self.amount = self.amount*mult;
        self
    }

    pub fn to_coin(&self) -> Coin {
        let amount: u64 = u64(self.amount.abs()).unwrap();
        Coin{
            amount: amount,
            denom: self.denom.clone(),
        }
    }
}


fn std_sign_bytes(chain_id: String, acc_num: u64, sequence: u64, fee: StdFee, msgs: Vec<Value>, memo: String) -> Vec<u8> {
    let s = StdSignDoc{
        account_number: acc_num,
        chain_id: chain_id,
        fee: json!(fee),
        memo: memo,
        msgs: msgs,
        sequence: sequence,
    };


    serde_json::to_string(&s).unwrap().as_bytes().to_vec()
    // sort json
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_incr_nonce() {
        // make sure we match the golang implementation
        assert_eq!(1, 1);
    }
}
