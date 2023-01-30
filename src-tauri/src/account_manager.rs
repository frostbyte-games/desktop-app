use codec::Compact;
use frame_support::Serialize;
use node_primitives::AccountIndex;
use sp_core::sr25519;
use tauri::async_runtime::RwLock;

use crate::{keystore, ClientApi};
use kitchensink_runtime::{Runtime as KitchensinkRuntime, Signature};
use secrets::{traits::AsContiguousBytes, Secret};
use sp_keyring::AccountKeyring;
use sp_runtime::{app_crypto::Ss58Codec, AccountId32, MultiAddress};
use substrate_api_client::{
    compose_extrinsic, pallet_staking_config::BalanceOf, ExtrinsicSigner, SubmitAndWatch, XtStatus,
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

    pub fn create_account(
        &self,
        api: &mut ClientApi,
        name: &str,
        master_password: &str,
    ) -> Result<Account, String> {
        Secret::<[u8; 32]>::random(|password| {
            let password = password.as_bytes();
            let password = hex::encode(password);

            let account = keystore::generate_keypair(name, &password, master_password).unwrap();

            // TODO: Replace with admin stash account
            let alice_signer = AccountKeyring::Alice.pair();
            api.set_signer(ExtrinsicSigner::<_, Signature, KitchensinkRuntime>::new(
                alice_signer.clone(),
            ));

            let (free, reserved) = Self::init_balances();

            let address = account.address.to_ss58check();
            let multi_addr: MultiAddress<AccountId32, AccountIndex> =
                MultiAddress::Id(account.address);

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
        let path = keystore::get_keystore_path().unwrap();

        let keystores: Vec<String> = std::fs::read_dir(&path)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.extension() == Some("account".as_ref()) {
                    Some(path.file_stem().unwrap().to_str().unwrap().to_string())
                } else {
                    None
                }
            })
            .collect();

        Ok(keystores)
    }
}
