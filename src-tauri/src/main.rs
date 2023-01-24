#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use codec::Compact;
use frame_support::Serialize;
use frame_system::offchain::Signer;
use kitchensink_runtime::{
    AccountId, BalancesCall, Runtime as KitchensinkRuntime, RuntimeCall, Signature,
};
use pallet_staking::BalanceOf;
use secrets::{traits::AsContiguousBytes, Secret};
use sp_core::Pair;
use sp_keyring::AccountKeyring;
use sp_runtime::{app_crypto::Ss58Codec, generic::Era, AccountId32};
use std::env;
use substrate_api_client::{
    compose_extrinsic, rpc::JsonrpseeClient, Api, ExtrinsicSigner, GenericAdditionalParams,
    GetAccountInformation, GetHeader, PlainTipExtrinsicParams, SubmitAndWatch, XtStatus,
};

mod keystore;

#[tokio::main]
async fn main() {
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            create_account,
            balance,
            get_accounts
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn create_account(name: &str, master_password: &str) -> Result<(String, String, String), String> {
    Secret::<[u8; 32]>::random(|password| {
        let password = password.as_bytes();
        let password = hex::encode(password);
        let account = keystore::add_keypair(name, &password, master_password).unwrap();

        println!("account: {:?}", account);

        // create account on chain
        let client = JsonrpseeClient::with_default_url().unwrap();
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
        Ok((account.password, address, account.mnemonic))
    })
}

#[derive(Serialize)]
struct Wallet {
    address: String,
    balance: String,
}

#[tauri::command]
fn balance(account: &str) -> Wallet {
    // causes problems
    // thread 'main' panicked at 'env_logger::init should not be called after logger initialized: SetLoggerError(())', /Users/michael.assaf/.cargo/registry/src/github.com-1ecc6299db9ec823/env_logger-0.10.0/src/lib.rs:1154:16
    // note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    // fatal runtime error: failed to initiate panic, error 5
    // env_logger::init();

    if account.is_empty() {
        return Wallet {
            address: String::from(""),
            balance: String::from(""),
        };
    }

    let client = JsonrpseeClient::with_default_url().unwrap();

    let pair = keystore::verify_and_fetch_keypair(&account).unwrap();

    let mut api =
        Api::<_, _, PlainTipExtrinsicParams<KitchensinkRuntime>, KitchensinkRuntime>::new(client)
            .unwrap();
    api.set_signer(ExtrinsicSigner::<_, Signature, KitchensinkRuntime>::new(
        pair.clone(),
    ));

    let account: AccountId = pair.public().into();

    let balance = api.get_account_data(&account).unwrap().unwrap().free;

    let address = pair.public().to_ss58check();

    return Wallet {
        address,
        balance: balance.to_string(),
    };
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
