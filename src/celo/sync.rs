use celo_light_client::{Address, FromBytes, Header, SerializedPublicKey, Validator};

use hyper::client::{Client, HttpConnector};
use hyper::http::Request;
use hyper::Body;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::json;

pub struct SyncClient {
    client: Client<HttpConnector, Body>,
    uri: String,
}

impl SyncClient {
    pub fn new(uri: String) -> Self {
        Self {
            client: Client::new(),
            uri,
        }
    }

    pub async fn get_block_header_by_number(
        &self,
        hex_num: &str,
    ) -> Result<Header, Box<dyn std::error::Error>> {
        let req = json!({
            "jsonrpc": "2.0",
            "method": "eth_getBlockByNumber",
            "params": [hex_num, true],
            "id": 1,
        });

        return self.fetch(req).await;
    }

    pub async fn get_current_validators(
        &self,
        hex_num: &str,
    ) -> Result<Vec<Validator>, Box<dyn std::error::Error>> {
        // NOTE: This call assumes that public keys and validator adressess are ordered lists (thus
        // can be zipped)
        let req = json!({
            "jsonrpc": "2.0",
            "method": "istanbul_getValidators",
            "params": [hex_num],
            "id": 1,
        });

        let validators: Vec<String> = self.fetch(req).await?;

        let req = json!({
            "jsonrpc": "2.0",
            "method": "istanbul_getValidatorsBLSPublicKeys",
            "params": [hex_num],
            "id": 1,
        });

        let keys: Vec<String> = self.fetch(req).await?;

        validators
            .iter()
            .zip(keys.iter())
            .map(|(address, public_key)| {
                Ok(Validator {
                    address: Address::from_bytes(&hex::decode(&address[2..])?)?.to_owned(),
                    public_key: SerializedPublicKey::from_bytes(&hex::decode(&public_key[2..])?)?
                        .to_owned(),
                })
            })
            .collect()
    }

    async fn fetch<T: DeserializeOwned>(
        &self,
        body: serde_json::Value,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let req = Request::builder()
            .method("POST")
            .uri(&self.uri)
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request builder");

        #[derive(Deserialize)]
        struct Container<T> {
            result: T,
        }

        let response = (&self.client).request(req).await?;
        let buf = hyper::body::to_bytes(response).await?;
        let container: Container<T> = serde_json::from_slice(&buf)?;

        Ok(container.result)
    }
}
