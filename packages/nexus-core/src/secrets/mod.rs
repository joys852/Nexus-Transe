//! Encrypted API key storage (AES-256-GCM). Set NEXUS_MASTER_KEY (32-byte base64).

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::Result;

const NONCE_LEN: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecretsStore {
    pub providers: HashMap<String, EncryptedSecret>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedSecret {
    pub ciphertext_b64: String,
    pub nonce_b64: String,
    pub label: String,
}

pub struct SecretVault {
    cipher: Aes256Gcm,
    path: PathBuf,
}

impl SecretVault {
    pub fn open(data_dir: &std::path::Path) -> Result<Self> {
        let key = master_key_from_env()?;
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| {
            crate::NexusError::Config(format!("invalid master key: {e}"))
        })?;
        Ok(Self {
            cipher,
            path: data_dir.join("secrets.enc.json"),
        })
    }

    pub fn load(&self) -> Result<SecretsStore> {
        if !self.path.exists() {
            return Ok(SecretsStore::default());
        }
        let text = std::fs::read_to_string(&self.path)?;
        Ok(serde_json::from_str(&text)?)
    }

    pub fn save(&self, store: &SecretsStore) -> Result<()> {
        let text = serde_json::to_string_pretty(store)?;
        std::fs::write(&self.path, text)?;
        Ok(())
    }

    pub fn set_api_key(&self, store: &mut SecretsStore, provider: &str, key: &str) -> Result<()> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, key.as_bytes())
            .map_err(|e| crate::NexusError::Config(e.to_string()))?;
        store.providers.insert(
            provider.into(),
            EncryptedSecret {
                ciphertext_b64: STANDARD.encode(ciphertext),
                nonce_b64: STANDARD.encode(nonce_bytes),
                label: provider.into(),
            },
        );
        self.save(store)
    }

    pub fn get_api_key(&self, store: &SecretsStore, provider: &str) -> Result<Option<String>> {
        let Some(enc) = store.providers.get(provider) else {
            return Ok(None);
        };
        let ciphertext = STANDARD.decode(&enc.ciphertext_b64)?;
        let nonce_bytes = STANDARD.decode(&enc.nonce_b64)?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        let plain = self
            .cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| crate::NexusError::Config(e.to_string()))?;
        Ok(Some(String::from_utf8(plain)?))
    }
}

fn master_key_from_env() -> Result<[u8; 32]> {
    let raw = std::env::var("NEXUS_MASTER_KEY").unwrap_or_else(|_| {
        tracing::warn!("NEXUS_MASTER_KEY unset — using dev-only default (not for production)");
        STANDARD.encode([0u8; 32])
    });
    let bytes = STANDARD.decode(raw.trim())?;
    if bytes.len() != 32 {
        return Err(crate::NexusError::Config(
            "NEXUS_MASTER_KEY must be 32 bytes base64".into(),
        ));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}
