#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{fs, path::Path};

use account_manager::AccountManager;
use codec::Compact;
use frame_support::Serialize;
use kitchensink_runtime::{AccountId, Runtime as KitchensinkRuntime, Signature};
use node_primitives::Balance;
use pwhash::bcrypt;
use sp_core::Pair;
use sp_runtime::app_crypto::Ss58Codec;
use substrate_api_client::{
    compose_extrinsic, pallet_staking_config::BalanceOf, rpc::WsRpcClient, Api, ExtrinsicSigner,
    GetAccountInformation, PlainTipExtrinsicParams, SubmitAndWatch, XtStatus,
};
use tauri::{async_runtime::RwLock, State};

mod account_manager;
mod file_manager;
mod keystore;

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
            get_accounts,
            transfer
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
    let master_password_path = format!(
        "{}/.master",
        file_manager::get_frostbyte_base_path().unwrap()
    );

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

#[tauri::command]
async fn create_account<'a>(
    name: &str,
    account_manager: State<'a, AccountManager>,
    session: State<'a, Session>,
) -> Result<account_manager::Account, String> {
    let master_password = session.password.read().await;
    let account = account_manager
        .create_account(name, &master_password)
        .unwrap();
    account_manager.refresh_accounts().await;

    Ok(account)
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

    let balance = match api.get_account_data(&account).unwrap() {
        Some(balance) => balance.free,
        None => 0,
    };

    let address = active_account.public().to_ss58check();

    Ok(Wallet {
        address,
        balance: balance.to_string(),
    })
}

#[tauri::command]
async fn get_accounts<'a>(
    account_manager: State<'a, AccountManager>,
) -> Result<Vec<String>, String> {
    let accounts = &*account_manager.accounts.read().await;
    Ok(accounts.to_owned())
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

#[tauri::command]
async fn transfer<'a>(
    amount: &str,
    to: &str,
    account_manager: State<'a, AccountManager>,
) -> Result<(), ()> {
    println!("HERE");
    let active_account = account_manager.active.read().await;
    let active_account = match &*active_account {
        None => return Err(()),
        Some(active) => active,
    };

    let client = WsRpcClient::new("ws://127.0.0.1:9944").unwrap();
    let mut api =
        Api::<_, _, PlainTipExtrinsicParams<KitchensinkRuntime>, KitchensinkRuntime>::new(client)
            .unwrap();
    api.set_signer(ExtrinsicSigner::<_, Signature, KitchensinkRuntime>::new(
        active_account.clone(),
    ));

    let address = AccountId::from_string(to).unwrap();
    let multi_addr = keystore::get_signer_multi_addr(address);

    println!("over here");

    let balance = match amount.parse::<u128>() {
        Ok(balance) => {
            let free: BalanceOf<KitchensinkRuntime> = balance;
            Compact::from(free)
        }
        Err(error) => return Err(()),
    };

    let xt = compose_extrinsic!(
        &api,
        "Balances",
        "transfer",
        Box::new(&multi_addr),
        Box::new(balance)
    );

    let xt_hash = api
        .submit_and_watch_extrinsic_until(xt, XtStatus::InBlock)
        .unwrap();
    println!("[+] Extrinsic hash: {:?}", xt_hash);

    Ok(())
}
