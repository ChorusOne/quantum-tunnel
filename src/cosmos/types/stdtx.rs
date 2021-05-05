use cast::u64;
use regex::Regex;
use crate::utils::prost_serialize;
use crate::cosmos::proto::cosmos::{
    base::v1beta1::Coin,
    tx::v1beta1::{AuthInfo, TxBody, SignDoc},
};

#[derive(Clone, Debug)]
pub struct DecCoin {
    amount: f64,
    denom: String,
}

impl DecCoin {
    pub fn from(str: String) -> Self {
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
            amount: format!("{}", amount),
            denom: self.denom.clone(),
        }
    }
}

pub fn std_sign_bytes(
    body: &TxBody,
    auth_info: &AuthInfo,
    chain_id: String,
    account_number: u64
) -> Result<Vec<u8>, prost::EncodeError> {
    let sign_doc = SignDoc {
        body_bytes: prost_serialize(body)?,
        auth_info_bytes: prost_serialize(auth_info)?,
        chain_id,
        account_number,
    };

    Ok(prost_serialize(&sign_doc)?)
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
