use tiny_hderive::bip32::ExtendedPrivKey;
use std::error::Error;
use std::vec::Vec;
use log::info;

use bip39::{Mnemonic, Language, Seed};

pub fn seed_from_mnemonic(words: String) -> Result<Vec<u8>, Box<dyn Error>> {
    info!("{}", words);
    let mnemonic = Mnemonic::from_phrase(words, Language::English)?;
    let seed = Seed::new(&mnemonic, "");
    Ok(seed.as_bytes().to_vec())
}

pub fn privkey_from_seed(seed: Vec<u8>) -> Vec<u8> {
    // Seed should be generated from your BIP39 phrase first!
    let ext = ExtendedPrivKey::derive(seed.as_slice(), "m/44'/118'/0'/0/0").unwrap();
    ext.secret().to_vec()

}
