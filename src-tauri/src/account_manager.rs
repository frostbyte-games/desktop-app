use std::{fs, path::Path};

use codec::Compact;
use frame_support::Serialize;
use sp_core::sr25519;
use tauri::async_runtime::RwLock;

use crate::{
    file_manager::{get_base_home_path, get_path, FileErrors},
    keystore,
};
use kitchensink_runtime::{Runtime as KitchensinkRuntime, Signature};
use secrets::{traits::AsContiguousBytes, Secret};
use sp_keyring::AccountKeyring;
use sp_runtime::app_crypto::Ss58Codec;
use substrate_api_client::{
    compose_extrinsic, pallet_staking_config::BalanceOf, rpc::WsRpcClient, Api, ExtrinsicSigner,
    PlainTipExtrinsicParams, SubmitAndWatch, XtStatus,
};

#[derive(Serialize)]
pub struct Account {
    password: String,
    address: String,
    mnemonic: String,
}

pub struct AccountManager {
    pub active: RwLock<Option<sr25519::Pair>>,
    pub accounts: RwLock<Vec<String>>,
}

impl AccountManager {
    pub fn new() -> Self {
        let accounts = Self::get_available_keypairs().unwrap();

        Self {
            active: RwLock::new(None),
            accounts: RwLock::new(accounts),
        }
    }

    pub async fn set_active(&self, account_name: &str, master_password: &str) {
        match keystore::verify_and_fetch_keypair(account_name, master_password) {
            Some(pair) => {
                let mut active = self.active.write().await;
                *active = Some(pair);
            }
            None => (),
        }
    }

    pub async fn refresh_accounts(&self) {
        let mut accounts = self.accounts.write().await;
        *accounts = Self::get_available_keypairs().unwrap();
    }

    pub fn create_account(&self, name: &str, master_password: &str) -> Result<Account, String> {
        Secret::<[u8; 32]>::random(|password| {
            let password = password.as_bytes();
            let password = hex::encode(password);

            let account = keystore::add_keypair(name, &password, master_password).unwrap();

            let client = WsRpcClient::new("ws://127.0.0.1:9944").unwrap();
            let alice_signer = AccountKeyring::Alice.pair();
            let mut api =
                Api::<_, _, PlainTipExtrinsicParams<KitchensinkRuntime>, KitchensinkRuntime>::new(
                    client,
                )
                .unwrap();
            api.set_signer(ExtrinsicSigner::<_, Signature, KitchensinkRuntime>::new(
                alice_signer.clone(),
            ));

            let (free, reserved) = Self::init_balances();

            let address = account.address.to_ss58check();
            let multi_addr = keystore::get_signer_multi_addr(account.address);

            let xt = compose_extrinsic!(
                &api,
                "Balances",
                "set_balance",
                Box::new(&multi_addr),
                Box::new(free.clone()),
                Box::new(reserved.clone())
            );

            let xt_hash = api
                .submit_and_watch_extrinsic_until(xt, XtStatus::InBlock)
                .unwrap();
            println!("[+] Extrinsic hash: {:?}", xt_hash);

            Ok(Account {
                password: account.password,
                address: format!("{:?}", address),
                mnemonic: account.mnemonic,
            })
        })
    }

    fn init_balances() -> (Compact<u128>, Compact<u128>) {
        let free: BalanceOf<KitchensinkRuntime> = 0;
        let free: Compact<u128> = Compact::from(free);

        let reserved: BalanceOf<KitchensinkRuntime> = 0;
        let reserved: Compact<u128> = Compact::from(reserved);

        (free, reserved)
    }

    fn get_available_keypairs() -> Result<Vec<String>, String> {
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
}
