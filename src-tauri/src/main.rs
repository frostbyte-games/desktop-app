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
    env_logger::init();

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

    format!("Your balance is {} snowflakes!", balance)
}

#[tokio::main]
async fn main() {
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![balance])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
