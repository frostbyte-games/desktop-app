#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{fs, path::Path};

use account_manager::AccountManager;
use frame_support::Serialize;
use kitchensink_runtime::{
    AccountId, BalancesCall, Runtime as KitchensinkRuntime, RuntimeCall, Signature,
};
use node_primitives::{Hash, Index};
use pwhash::bcrypt;
use sp_core::{sr25519, Pair};
use sp_runtime::{app_crypto::Ss58Codec, generic::Era, MultiSignature};
use substrate_api_client::{
    rpc::WsRpcClient, Api, ExtrinsicSigner, GenericAdditionalParams, GenericExtrinsicParams,
    GetAccountInformation, GetHeader, PlainTip, PlainTipExtrinsicParams, SubmitAndWatch, XtStatus,
};
use tauri::{async_runtime::RwLock, State};

mod account_manager;
mod file_manager;
mod keystore;

pub type ClientApi = Api<
    ExtrinsicSigner<sr25519::Pair, MultiSignature, KitchensinkRuntime>,
    WsRpcClient,
    GenericExtrinsicParams<PlainTip<u128>, Index, Hash>,
    KitchensinkRuntime,
>;

struct Session {
    client: RwLock<ClientApi>,
    password: RwLock<String>,
}

#[tokio::main]
async fn main() {
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    tauri::Builder::default()
        .manage(Session {
            client: RwLock::new(
                Api::<_, _, PlainTipExtrinsicParams<KitchensinkRuntime>, KitchensinkRuntime>::new(
                    WsRpcClient::new("ws://127.0.0.1:9944").unwrap(),
                )
                .unwrap(),
            ),
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
    let mut api = session.client.write().await;
    let account = account_manager
        .create_account(&mut api, name, &master_password)
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
async fn balance<'a>(
    account_manager: State<'a, AccountManager>,
    session: State<'a, Session>,
) -> Result<Wallet, String> {
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

    let api = session.client.read().await;

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

    let mut api = session.client.write().await;

    let active_account = match &*account_manager.active.read().await {
        Some(active_account) => active_account.to_owned(),
        None => return Err(()),
    };

    api.set_signer(ExtrinsicSigner::<_, Signature, KitchensinkRuntime>::new(
        active_account,
    ));

    Ok(())
}

#[tauri::command]
async fn transfer<'a>(amount: &str, to: &str, session: State<'a, Session>) -> Result<(), ()> {
    let amount = match amount.parse::<u128>() {
        Ok(amount) => amount,
        Err(_error) => return Err(()),
    };

    let mut api = session.client.write().await;

    // Information for Era for mortal transactions (online).
    let last_finalized_header_hash = api.get_finalized_head().unwrap().unwrap();
    let header = api
        .get_header(Some(last_finalized_header_hash))
        .unwrap()
        .unwrap();
    let period = 5;
    let tx_params = GenericAdditionalParams::new()
        .era(
            Era::mortal(period, header.number.into()),
            last_finalized_header_hash,
        )
        .tip(0);

    // Set the additional params.
    api.set_additional_params(tx_params);

    // Get the nonce of the signer account (online).
    let signer_nonce = api.get_nonce().unwrap();

    // Compose the extrinsic (offline).
    let address = AccountId::from_string(to).unwrap();
    let recipient = keystore::get_signer_multi_addr(address);
    let call = RuntimeCall::Balances(BalancesCall::transfer_keep_alive {
        dest: recipient,
        value: amount,
    });
    let xt = api.compose_extrinsic_offline(call, signer_nonce);
    println!("[+] Composed Extrinsic:\n {:?}\n", xt);

    // Send and watch extrinsic until in block (online).
    let block_hash = api
        .submit_and_watch_extrinsic_until(xt, XtStatus::InBlock)
        .unwrap()
        .block_hash
        .unwrap();
    println!("[+] Extrinsic got included in block {:?}", block_hash);
    Ok(())
}
