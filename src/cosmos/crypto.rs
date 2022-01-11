use std::error::Error;

use bip39::{Language, Mnemonic};
use bitcoin::network::constants::Network;
use bitcoin::secp256k1;
use bitcoin::util::bip32;
use std::str::FromStr;

pub fn seed_from_mnemonic(words: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let mnemonic = Mnemonic::parse_in(Language::English, words)?;
    Ok(mnemonic.to_seed("").to_vec())
}

pub fn privkey_from_seed(seed: Vec<u8>) -> Vec<u8> {
    // TODO: pass path as a config variable so we are compatible with other chains.
    let dpath = bip32::DerivationPath::from_str("m/44'/118'/0'/0/0").unwrap();
    let ext = bip32::ExtendedPrivKey::new_master(Network::Bitcoin, seed.as_slice()).unwrap()
        .derive_priv(&secp256k1::Secp256k1::signing_only(), &dpath).unwrap();
    ext.private_key.to_bytes()
}

pub fn keys_from_mnemonic(
    mnemonic_words: &str,
) -> Result<bip32::ExtendedPrivKey, Box<dyn Error>> {
    let mnemonic = Mnemonic::parse_in(Language::English, mnemonic_words)?;
    let dpath = bip32::DerivationPath::from_str("m/44'/118'/0'/0/0").unwrap();
    let xpriv = bip32::ExtendedPrivKey::new_master(Network::Bitcoin, &mnemonic.to_seed("")[..])?
        .derive_priv(&secp256k1::Secp256k1::signing_only(), &dpath)?;
    Ok(xpriv)
}
