use serde::{Serialize, Deserialize};
use signature::{Signature, Signer};
use signatory::public_key::PublicKeyed;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StdSignature {
    pub_key: tendermint_light_client::PublicKey,
    signature: tendermint_light_client::Signature,
}

impl StdSignature {
    pub fn sign(signer: signatory_secp256k1::EcdsaSigner, bytes_to_sign: Vec<u8>) -> Self {
        let sig = signer.sign(bytes_to_sign.as_slice());
        StdSignature {
            pub_key: tendermint_light_client::PublicKey::from(signer.public_key().unwrap()),
            signature: tendermint_light_client::Signature::Secp256k1(sig),
        }
    }
}
