use base64_serde::base64_serde_type;
use k256::Secp256k1;
use serde::{Deserialize, Serialize};
use signatory::public_key::PublicKeyed;
use signature::Signer;

base64_serde_type!(Base64Standard, base64::STANDARD);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StdSignature {
    #[serde(with = "Base64Standard")]
    pub_key: Vec<u8>,
    #[serde(with = "Base64Standard")]
    signature: Vec<u8>,
}

impl StdSignature {
    pub fn sign(signer: signatory_secp256k1::EcdsaSigner, bytes_to_sign: Vec<u8>) -> Self {
        let sig: signatory::ecdsa::FixedSignature<Secp256k1> =
            signer.sign(bytes_to_sign.as_slice());
        StdSignature {
            pub_key: tendermint_light_client::PublicKey::from(signer.public_key().unwrap())
                .to_amino_bytes()
                .to_vec(),
            signature: sig.as_ref().to_vec(),
        }
    }
}
