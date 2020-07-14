use std::error::Error;
use std::vec::Vec;
use tiny_hderive::bip32::ExtendedPrivKey;

use bip39::{Language, Mnemonic, Seed};

pub fn seed_from_mnemonic(words: String) -> Result<Vec<u8>, Box<dyn Error>> {
    let mnemonic = Mnemonic::from_phrase(&words.trim(), Language::English)?;
    let seed = Seed::new(&mnemonic, "");
    Ok(seed.as_bytes().to_vec())
}

pub fn privkey_from_seed(seed: Vec<u8>) -> Vec<u8> {
    // TODO: pass path as a config variable so we are compatible with other chains.
    let ext = ExtendedPrivKey::derive(seed.as_slice(), "m/44'/118'/0'/0/0").unwrap();
    ext.secret().to_vec()
}
