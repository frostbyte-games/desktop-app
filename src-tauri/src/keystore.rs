use frame_support::{Deserialize, Serialize};
use kitchensink_runtime::AccountId;
use node_primitives::AccountIndex;
use openssl::symm::{decrypt, encrypt, Cipher};
use sp_core::sr25519::{self, Public, Signature};
use sp_core::Pair;
use sp_runtime::app_crypto::{RuntimePublic, Ss58Codec};
use sp_runtime::{AccountId32, MultiAddress};
use std::io::Read;
use std::path::Path;
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

pub fn verify_and_fetch_keypair(
    account_name: &str,
    master_password: &str,
) -> Option<sr25519::Pair> {
    let app_dir_path = get_base_home_path().unwrap();
    let path = format!("{}/.frostbyte/keystore/{}.json", app_dir_path, account_name);

    let path = Path::new(&path);
    if !path.exists() {
        println!("Keystore file does not exist");
        None
    } else {
        let decrypted_data = decrypt_file(&path, master_password).unwrap();
        let decrypted_data = String::from_utf8(decrypted_data).unwrap();
        let keystore: Keystore = serde_json::from_str(&decrypted_data).unwrap();
        println!("Keystore: {:?}", keystore);
        let public_key = Public::from_string(&keystore.public_key).unwrap();
        public_key.verify(&keystore.message, &keystore.signature);

        let pair = sr25519::Pair::from_seed(&keystore.seed);

        Some(pair)
    }
}

pub fn get_available_keypairs() -> Result<Vec<String>, String> {
    let app_dir_path = get_base_home_path()?;
    let path = format!("{}/.frostbyte", app_dir_path);

    let path = Path::new(&path);
    if !path.exists() {
        fs::create_dir(&path).unwrap();
        return Ok(vec![]);
    }

    let path = match get_path("keystore", false) {
        Ok(path) => path,
        Err(err) => match err {
            FileErrors::DoesNotExist => return Err(String::from("File not found")),
        },
    };

    let keystores: Vec<String> = std::fs::read_dir(&path)
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension() == Some("json".as_ref()) {
                Some(path.file_stem().unwrap().to_str().unwrap().to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(keystores)
}

pub fn add_keypair(name: &str, password: &str, master_password: &str) -> Result<Account, String> {
    let account_gen = sr25519::Pair::generate_with_phrase(Some(&password));
    let pair: sr25519::Pair = account_gen.0;

    let account = Account {
        address: pair.public().into(),
        password: password.to_string(),
        mnemonic: account_gen.1,
    };

    let message = b"Frostbyte is awesome!";
    let signature = pair.sign(message);
    let public_key = pair.public().to_ss58check();
    let public_key = public_key.as_bytes();
    let public_key = String::from_utf8(public_key.to_vec()).unwrap();
    let keystore = Keystore {
        public_key,
        seed: account_gen.2,
        signature,
        message: message.to_vec(),
    };
    let keypair_json = serde_json::to_string(&keystore).unwrap();

    encrypt_file(name, master_password, keypair_json)?;

    Ok(account)
}

/// Encrypts data using AES-256-CBC algorithm and a key derived from the master_password and writes it to a file
///
/// # Parameters
/// * `name: &str` - the name of the file to be written
/// * `master_password: &str` - the master password used to derive the key
/// * `data: String` - the data to be encrypted and written to the file
///
/// # Returns
/// * `Result<(), String>` - returns Ok if the file is successfully written, otherwise returns an error string
fn encrypt_file(name: &str, master_password: &str, data: String) -> Result<(), String> {
    let path = get_path("keystore", true).unwrap();

    let file_path = format!("{}/{}.json", path, name);
    fs::write(&file_path, data).unwrap();
    let data = fs::read(&file_path).unwrap();

    let cipher = Cipher::aes_256_cbc();
    let encrypted_data = encrypt(cipher, master_password.as_bytes(), None, &data).unwrap();
    fs::write(file_path, &encrypted_data).unwrap();

    Ok(())
}

fn decrypt_file(file_path: &Path, master_password: &str) -> Result<Vec<u8>, String> {
    let mut data = vec![];
    let mut file = match std::fs::File::open(file_path) {
        Ok(file) => file,
        Err(e) => return Err(format!("{}", e)),
    };
    file.read_to_end(&mut data).map_err(|e| format!("{}", e))?;

    let cipher = Cipher::aes_256_cbc();
    let decrypted_data =
        decrypt(cipher, master_password.as_bytes(), None, &data).map_err(|e| format!("{}", e))?;

    Ok(decrypted_data)
}

#[derive(Debug)]
pub enum FileErrors {
    DoesNotExist,
}

pub fn get_path(relative_path: &str, create: bool) -> Result<String, FileErrors> {
    let path = format!("{}/{}", get_frostbyte_base_path().unwrap(), relative_path);
    if !std::path::Path::new(&path).exists() {
        match create {
            true => fs::create_dir(&path).unwrap(),
            false => return Err(FileErrors::DoesNotExist),
        }
    }

    Ok(path)
}

pub fn get_frostbyte_base_path() -> Result<String, String> {
    let path = format!("{}/.frostbyte", get_base_home_path().unwrap());
    if !std::path::Path::new(&path).exists() {
        fs::create_dir(&path).unwrap();
    }

    Ok(path)
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
