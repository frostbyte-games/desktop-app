#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use codec::Compact;
use kitchensink_runtime::{BalancesCall, Runtime as KitchensinkRuntime, RuntimeCall, Signature};
use pallet_staking::BalanceOf;
use secrets::{traits::AsContiguousBytes, Secret};
use sp_core::crypto::Ss58Codec;
use sp_core::{sr25519, Pair};
use sp_keyring::AccountKeyring;
use sp_runtime::{generic::Era, AccountId32, MultiAddress};
use substrate_api_client::{
    compose_extrinsic, rpc::JsonrpseeClient, Api, ExtrinsicSigner, GenericAdditionalParams,
    GetAccountInformation, GetHeader, PlainTipExtrinsicParams, SubmitAndWatch, XtStatus,
};

use node_primitives::{AccountId, AccountIndex};

use serde::{Deserialize, Serialize};
use std::{env, fs};

#[derive(Serialize, Deserialize)]
struct Keypair {
    seed: String,
    public_key: String,
    address: String,
}

#[tauri::command]
fn create_account(name: &str, password: &str) -> Result<(String, String), String> {
    let password = password.as_bytes();
    if password.len() != 32 {
        return Err("Error: Password length must be 32 bytes".to_string());
    }
    let mut password: [u8; 32] = password.try_into().unwrap();
    Secret::from(&mut password, |s| {
        let s = s.as_bytes();
        let s = std::str::from_utf8(s).unwrap();
        let keypair = sr25519::Pair::generate_with_phrase(Some(s));
        let signer = sr25519::Pair::from_string(&keypair.1, None).unwrap();

        // let seed = String::from_utf8(keypair.2.to_vec()).unwrap();

        // Convert the keypair to a struct
        let keypair_struct = Keypair {
            seed: String::from(String::from_utf8_lossy(keypair.2.as_bytes())),
            public_key: signer.public().to_string(),
            address: signer.public().to_ss58check(),
        };

        // Serialize the struct as a JSON object
        let keypair_json = serde_json::to_string(&keypair_struct).unwrap();

        // Write the JSON object to a file on disk
        // check if directory exists if it doesnt, create it
        let app_data_dir = match env::var("APPDATA") {
            Ok(val) => val,
            Err(_) => match env::var("HOME") {
                Ok(val) => val,
                Err(e) => {
                    return Err(format!("Error: {}", e));
                }
            },
        };

        let path = format!("{}/frostbyte", app_data_dir);

        if !std::path::Path::new(&path).exists() {
            fs::create_dir(&path).unwrap();
        }
        fs::write(format!("{}/keystore.json", path), keypair_json).unwrap();

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

        let address: MultiAddress<AccountId, AccountIndex> =
            get_signer_multi_addr(signer.public().into());

        let (free, reserved) = init_balances();

        let xt = compose_extrinsic!(
            &api,
            "Balances",
            "set_balance",
            Box::new(address.clone()),
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
            dest: address,
            value: 500,
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

        let pub_key_string = format!("{:?}", signer.public());
        Ok((pub_key_string, keypair.1))
    })
}

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

    format!("{}", balance)
}

#[tokio::main]
async fn main() {
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![create_account, balance])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_balances() -> (Compact<u128>, Compact<u128>) {
    let free: BalanceOf<KitchensinkRuntime> = 0;
    let free: Compact<u128> = Compact::from(free);

    let reserved: BalanceOf<KitchensinkRuntime> = 0;
    let reserved: Compact<u128> = Compact::from(reserved);

    (free, reserved)
}

fn get_signer_multi_addr(signer: AccountId32) -> MultiAddress<AccountId, AccountIndex> {
    MultiAddress::Id(signer)
}
