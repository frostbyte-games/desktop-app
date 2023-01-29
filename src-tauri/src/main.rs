#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{fs, path::Path};

use account_manager::AccountManager;
use codec::Compact;
use frame_support::Serialize;
use kitchensink_runtime::{AccountId, Runtime as KitchensinkRuntime, Signature};
use pallet_staking::BalanceOf;
use pwhash::bcrypt;
use secrets::{traits::AsContiguousBytes, Secret};
use sp_core::Pair;
use sp_keyring::AccountKeyring;
use sp_runtime::app_crypto::Ss58Codec;
use substrate_api_client::{
    compose_extrinsic, rpc::WsRpcClient, Api, ExtrinsicSigner, GetAccountInformation,
    PlainTipExtrinsicParams, SubmitAndWatch, XtStatus,
};
use tauri::{async_runtime::RwLock, State};

mod account_manager;
mod keystore;

// TODO follow this to create pallet trait https://github.com/litentry/litentry-parachain/blob/8b7f31b764f988b77bda6b27d4e4a796c95923bc/tee-worker/core-primitives/node-api/api-client-extensions/src/pallet_teerex.rs

struct Session {
    password: RwLock<String>,
}

#[tokio::main]
async fn main() {
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    tauri::Builder::default()
        .manage(Session {
            password: RwLock::new(String::from("")),
        })
        .manage(AccountManager::new())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            unlock,
            set_active_account,
            create_account,
            balance,
            get_accounts
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Serialize)]
enum UnlockErrors {
    InvalidPassword,
}

#[tauri::command]
async fn unlock<'a>(
    master_password: String,
    session: State<'a, Session>,
) -> Result<(), UnlockErrors> {
    let master_password_path = format!("{}/.master", keystore::get_frostbyte_base_path().unwrap());

    if Path::new(&master_password_path).exists() {
        let hash = fs::read_to_string(master_password_path).unwrap();
        if !bcrypt::verify(&master_password, &hash) {
            return Err(UnlockErrors::InvalidPassword);
        }
    } else {
        let hash = bcrypt::hash(&master_password).unwrap();
        fs::write(master_password_path, hash).unwrap();
    }

    let mut password = session.password.write().await;
    *password = master_password;

    Ok(())
}

#[derive(Serialize)]
struct Account {
    password: String,
    address: String,
    mnemonic: String,
}

#[tauri::command]
async fn create_account<'a>(name: &str, session: State<'a, Session>) -> Result<Account, String> {
    let master_password = session.password.read().await;

    Secret::<[u8; 32]>::random(|password| {
        let password = password.as_bytes();
        let password = hex::encode(password);

        let account = keystore::add_keypair(name, &password, &*master_password).unwrap();

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

        let (free, reserved) = init_balances();

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

#[derive(Serialize)]
struct Wallet {
    address: String,
    balance: String,
}

#[tauri::command]
async fn balance<'a>(account_manager: State<'a, AccountManager>) -> Result<Wallet, String> {
    // causes problems
    // thread 'main' panicked at 'env_logger::init should not be called after logger initialized: SetLoggerError(())', /Users/michael.assaf/.cargo/registry/src/github.com-1ecc6299db9ec823/env_logger-0.10.0/src/lib.rs:1154:16
    // note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    // fatal runtime error: failed to initiate panic, error 5
    // env_logger::init();

    let active_account = account_manager.active.read().await;
    println!("{:?}", &active_account.is_none());
    let active_account = match &*active_account {
        None => {
            return Ok(Wallet {
                address: String::from(""),
                balance: String::from(""),
            })
        }
        Some(active) => active,
    };

    let client = WsRpcClient::new("ws://127.0.0.1:9944").unwrap();
    let mut api =
        Api::<_, _, PlainTipExtrinsicParams<KitchensinkRuntime>, KitchensinkRuntime>::new(client)
            .unwrap();
    api.set_signer(ExtrinsicSigner::<_, Signature, KitchensinkRuntime>::new(
        active_account.clone(),
    ));

    let account: AccountId = active_account.public().into();

    println!("{:?}", account);

    let balance = api.get_account_data(&account).unwrap().unwrap().free;

    let address = active_account.public().to_ss58check();

    Ok(Wallet {
        address,
        balance: balance.to_string(),
    })
}

#[tauri::command]
fn get_accounts(account_manager: State<AccountManager>) -> Result<Vec<String>, String> {
    Ok(account_manager.accounts.clone())
}

#[tauri::command]
async fn set_active_account<'a>(
    account_name: &str,
    account_manager: State<'a, AccountManager>,
    session: State<'a, Session>,
) -> Result<(), ()> {
    let master_password = session.password.read().await;
    println!("{}", account_name);
    account_manager
        .set_active(account_name, &*master_password)
        .await;

    Ok(())
}

fn init_balances() -> (Compact<u128>, Compact<u128>) {
    let free: BalanceOf<KitchensinkRuntime> = 0;
    let free: Compact<u128> = Compact::from(free);

    let reserved: BalanceOf<KitchensinkRuntime> = 0;
    let reserved: Compact<u128> = Compact::from(reserved);

    (free, reserved)
}
