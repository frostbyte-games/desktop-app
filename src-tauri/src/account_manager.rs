use sp_core::sr25519;
use tauri::async_runtime::RwLock;

use crate::keystore;

pub struct AccountManager {
    pub active: RwLock<Option<sr25519::Pair>>,
    pub accounts: Vec<String>,
}

impl AccountManager {
    pub fn new() -> Self {
        let accounts = keystore::get_available_keypairs().unwrap();

        Self {
            active: RwLock::new(None),
            accounts,
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
}
