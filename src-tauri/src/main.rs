#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use kitchensink_runtime::{Runtime as KitchensinkRuntime, Signature};
use sp_keyring::AccountKeyring;
use substrate_api_client::{
    rpc::JsonrpseeClient, Api, ExtrinsicSigner, GetAccountInformation, PlainTipExtrinsicParams,
};

#[tauri::command]
fn balance() -> String {
    // causes problems
    // thread 'main' panicked at 'env_logger::init should not be called after logger initialized: SetLoggerError(())', /Users/michael.assaf/.cargo/registry/src/github.com-1ecc6299db9ec823/env_logger-0.10.0/src/lib.rs:1154:16
    // note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    // fatal runtime error: failed to initiate panic, error 5
    // env_logger::init();

    let client = JsonrpseeClient::with_default_url().unwrap();
    let signer = AccountKeyring::Alice.pair();
    let mut api =
        Api::<_, _, PlainTipExtrinsicParams<KitchensinkRuntime>, KitchensinkRuntime>::new(client)
            .unwrap();
    api.set_signer(ExtrinsicSigner::<_, Signature, KitchensinkRuntime>::new(
        signer.clone(),
    ));

    let balance = api
        .get_account_data(&AccountKeyring::Alice.to_account_id())
        .unwrap()
        .unwrap()
        .free;
    println!("[+] Account balance: {}", balance);

    format!("{}", balance)
}

#[tokio::main]
async fn main() {
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![balance])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
