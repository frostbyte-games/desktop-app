use frame_support::{Deserialize, Serialize};
use kitchensink_runtime::AccountId;
use node_primitives::AccountIndex;
use openssl::symm::{decrypt, encrypt, Cipher};
use sodiumoxide::crypto::pwhash::{self, Salt};
use sp_core::sr25519::{self, Public, Signature};
use sp_core::ByteArray;
use sp_core::Pair;
use sp_runtime::app_crypto::{RuntimePublic, Ss58Codec};
use sp_runtime::{AccountId32, MultiAddress};
use std::io::{Read, Write};
use std::{env, fs};

#[derive(Serialize, Deserialize, Debug)]
pub struct Keystore {
    pub public_key: String,
    seed: [u8; schnorrkel::keys::MINI_SECRET_KEY_LENGTH], // this should match sp_core::sr25519::Seed
    signature: Signature,
    message: Vec<u8>,
}

#[derive(Debug)]
pub struct Account {
    pub address: AccountId32,
    pub password: String,
    pub mnemonic: String,
}

pub fn get_signer_multi_addr(signer: AccountId32) -> MultiAddress<AccountId, AccountIndex> {
    MultiAddress::Id(signer)
}

pub fn verify_and_fetch_keypair(keystore: &Keystore) -> Option<sr25519::Pair> {
    let public_key = Public::from_slice(&keystore.public_key.as_bytes()).unwrap();
    public_key.verify(&keystore.message, &keystore.signature);

    let pair = sr25519::Pair::from_seed(&keystore.seed);
    Some(pair)
}

pub async fn get_keypairs(master_password: &str) -> Result<Keystore, String> {
    let app_dir_path = get_base_home_path()?;
    let path = format!("{}/frostbyte/keystore.json", app_dir_path);

    // check if file exists
    if !std::path::Path::new(&path).exists() {
        return Err("Create your first account!".to_string());
    }

    let decrypted_data = decrypt_file(&path, master_password)
        .map_err(|err| format!("Decryption failed with error: {}", err))?;

    let decrypted_data = String::from_utf8(decrypted_data).unwrap();

    let accounts: Keystore = serde_json::from_str(&decrypted_data).unwrap();

    Ok(accounts)
}

pub fn add_keypair(password: &str, master_password: &str) -> Result<Account, String> {
    // Derive key from password
    let salt = pwhash::gen_salt();

    let mut key = [0u8; 32];
    pwhash::derive_key(
        &mut key,
        master_password.as_bytes(),
        &salt,
        pwhash::OPSLIMIT_INTERACTIVE,
        pwhash::MEMLIMIT_INTERACTIVE,
    )
    .unwrap();
    let key = key.to_vec();
    let key = String::from_utf8_lossy(&key);
    let account_gen = sr25519::Pair::generate_with_phrase(Some(&key));
    let pair: sr25519::Pair = account_gen.0;

    let account = Account {
        address: pair.public().into(),
        password: password.to_string(),
        mnemonic: account_gen.1,
    };

    let message = b"Frostbyte is awesome!";
    let signature = pair.sign(message);
    let public_keys = pair.public().to_ss58check();
    let public_keys = public_keys.as_bytes();
    let public_keys = String::from_utf8(public_keys.to_vec()).unwrap();
    let keystore = Keystore {
        public_key: public_keys,
        seed: account_gen.2,
        signature,
        message: message.to_vec(),
    };
    let keypair_json = serde_json::to_string(&keystore).unwrap();

    encrypt_file(master_password, keypair_json)?;

    Ok(account)
}

static SALT: Salt = Salt([
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
    0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
]);

fn encrypt_file(master_password: &str, data: String) -> Result<(), String> {
    // Write the JSON object to a file on disk
    // check if directory exists if it doesnt, create it
    let app_data_dir = get_base_home_path()?;

    let path = format!("{}/frostbyte", app_data_dir);
    if !std::path::Path::new(&path).exists() {
        fs::create_dir(&path).unwrap();
    }

    let file_path = format!("{}/keystore.json", path);
    fs::write(&file_path, data).unwrap();
    let data = fs::read(&file_path).unwrap();

    // Derive key from password
    // let salt = pwhash::gen_salt();

    let mut key = [0u8; 32];
    pwhash::derive_key(
        &mut key,
        master_password.as_bytes(),
        &SALT,
        pwhash::OPSLIMIT_INTERACTIVE,
        pwhash::MEMLIMIT_INTERACTIVE,
    )
    .unwrap();

    // Use key in encryption
    let cipher = Cipher::aes_256_cbc();
    let encrypted_data = encrypt(cipher, &key, None, &data).unwrap();
    fs::write(file_path, &encrypted_data).unwrap();

    Ok(())
}

fn decrypt_file(file_path: &str, master_password: &str) -> Result<Vec<u8>, String> {
    // let salt = pwhash::gen_salt();

    let mut key = [0u8; 32];
    pwhash::derive_key(
        &mut key,
        master_password.as_bytes(),
        &SALT,
        pwhash::OPSLIMIT_INTERACTIVE,
        pwhash::MEMLIMIT_INTERACTIVE,
    )
    .map_err(|e| format!("{:?}", e))?;

    let mut data = vec![];
    let mut file = match std::fs::File::open(file_path) {
        Ok(file) => file,
        Err(e) => return Err(format!("{}", e)),
    };
    file.read_to_end(&mut data).map_err(|e| format!("{}", e))?;

    let cipher = Cipher::aes_256_cbc();
    let decrypted_data = decrypt(cipher, &key, None, &data).map_err(|e| format!("{}", e))?;

    Ok(decrypted_data)
}
fn get_base_home_path() -> Result<String, String> {
    match env::var("APPDATA") {
        Ok(val) => return Ok(val),
        Err(_) => match env::var("HOME") {
            Ok(val) => return Ok(val),
            Err(e) => {
                return Err(format!("Error: {}", e));
            }
        },
    }
}
