use base64_serde::base64_serde_type;
use serde::{Deserialize, Serialize};
use k256::ecdsa::{Signature, SigningKey, signature::Signer};
use k256::elliptic_curve::SecretKey;
use k256::EncodedPoint as Secp256k1;

base64_serde_type!(Base64Standard, base64::STANDARD);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StdSignature {
    #[serde(with = "Base64Standard")]
    pub_key: Vec<u8>,
    #[serde(with = "Base64Standard")]
    signature: Vec<u8>,
}

impl StdSignature {
    pub fn sign(signer: SigningKey, bytes_to_sign: Vec<u8>) -> Self {
        let secret_key = SecretKey::from(&signer);
        let public_key = tendermint_light_client::PublicKey::from(Secp256k1::from_secret_key(&secret_key, true));
        let sig: Signature = signer.sign(bytes_to_sign.as_slice());

        StdSignature {
            pub_key: public_key
                .to_amino_bytes()
                .to_vec(),
            signature: sig.as_ref().to_vec(),
        }
    }
}
