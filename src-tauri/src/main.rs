#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use codec::Compact;
use frame_support::Serialize;
use kitchensink_runtime::{
    AccountId, BalancesCall, Runtime as KitchensinkRuntime, RuntimeCall, Signature,
};
use pallet_staking::BalanceOf;
use secrets::{traits::AsContiguousBytes, Secret};
use sodiumoxide::crypto::pwhash::{self, Salt};
use sp_core::Pair;
use sp_keyring::AccountKeyring;
use sp_runtime::{app_crypto::Ss58Codec, generic::Era};
use substrate_api_client::{
    compose_extrinsic, rpc::WsRpcClient, Api, ExtrinsicSigner, GenericAdditionalParams,
    GetAccountInformation, GetHeader, PlainTipExtrinsicParams, SubmitAndWatch, XtStatus,
};
use tauri::{async_runtime::RwLock, State};

mod keystore;

static SALT: Salt = Salt([
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
    0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
]);

// TODO follow this to create pallet trait https://github.com/litentry/litentry-parachain/blob/8b7f31b764f988b77bda6b27d4e4a796c95923bc/tee-worker/core-primitives/node-api/api-client-extensions/src/pallet_teerex.rs

struct Session {
    derived_key: RwLock<String>,
}

#[tokio::main]
async fn main() {
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    tauri::Builder::default()
        .manage(Session {
            derived_key: RwLock::new(String::from("")),
        })
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            unlock,
            create_account,
            balance,
            get_accounts
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn unlock<'a>(master_password: &str, session: State<'a, Session>) -> Result<(), ()> {
    let mut derived_key = session.derived_key.write().await;

    let mut key = [0u8; 32];
    pwhash::derive_key(
        &mut key,
        master_password.as_bytes(),
        &SALT,
        pwhash::OPSLIMIT_INTERACTIVE,
        pwhash::MEMLIMIT_INTERACTIVE,
    )
    .unwrap();

    let key = key.to_vec();
    let key = String::from_utf8_lossy(&key);
    *derived_key = String::from(key);

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
    let derived_key = session.derived_key.read().await;

    Secret::<[u8; 32]>::random(|password| {
        let password = password.as_bytes();
        let password = hex::encode(password);

        let account = keystore::add_keypair(name, &password, &*derived_key).unwrap();

        println!("account: {:?}", account);

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

        // Get the nonce of the signer account (online).
        let signer_nonce = api.get_nonce().unwrap();
        println!("[+] {}'s Account Nonce is {}\n", name, signer_nonce);

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
        api.set_additional_params(tx_params);

        // Compose the extrinsic (offline).
        let call = RuntimeCall::Balances(BalancesCall::transfer {
            dest: multi_addr,
            value: 500,
        });
        let xt = api.compose_extrinsic_offline(call, signer_nonce);

        // Send and watch extrinsic until in block (online).
        let block_hash = api
            .submit_and_watch_extrinsic_until(xt, XtStatus::InBlock)
            .unwrap()
            .block_hash
            .unwrap();
        println!("[+] Extrinsic got included in block {:?}", block_hash);

        let address = format!("{:?}", address);
        Ok(Account {
            password: account.password,
            address,
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
async fn balance<'a>(account: &str, session: State<'a, Session>) -> Result<Wallet, String> {
    // causes problems
    // thread 'main' panicked at 'env_logger::init should not be called after logger initialized: SetLoggerError(())', /Users/michael.assaf/.cargo/registry/src/github.com-1ecc6299db9ec823/env_logger-0.10.0/src/lib.rs:1154:16
    // note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    // fatal runtime error: failed to initiate panic, error 5
    // env_logger::init();

    let derived_key = session.derived_key.read().await;

    if account.is_empty() {
        return Ok(Wallet {
            address: String::from(""),
            balance: String::from(""),
        });
    }

    let client = WsRpcClient::new("ws://127.0.0.1:9944").unwrap();
    let pair = keystore::verify_and_fetch_keypair(&account, &*derived_key).unwrap();

    let mut api =
        Api::<_, _, PlainTipExtrinsicParams<KitchensinkRuntime>, KitchensinkRuntime>::new(client)
            .unwrap();
    api.set_signer(ExtrinsicSigner::<_, Signature, KitchensinkRuntime>::new(
        pair.clone(),
    ));

    let account: AccountId = pair.public().into();

    let balance = api.get_account_data(&account).unwrap().unwrap().free;

    let address = pair.public().to_ss58check();

    Ok(Wallet {
        address,
        balance: balance.to_string(),
    })
}

#[tauri::command]
async fn get_accounts() -> Result<Vec<String>, String> {
    let keystore = keystore::get_available_keypairs().await?;

    Ok(keystore)
}

fn init_balances() -> (Compact<u128>, Compact<u128>) {
    let free: BalanceOf<KitchensinkRuntime> = 0;
    let free: Compact<u128> = Compact::from(free);

    let reserved: BalanceOf<KitchensinkRuntime> = 0;
    let reserved: Compact<u128> = Compact::from(reserved);

    (free, reserved)
}
