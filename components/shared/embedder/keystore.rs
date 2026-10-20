/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fmt::Debug;
use std::sync::{Arc, OnceLock, RwLock};

use malloc_size_of_derive::MallocSizeOf;
use serde::{Deserialize, Serialize};

/// Identifier for a key managed by the host embedder's keystore rather than SpiderMonkey heap memory.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, MallocSizeOf)]
pub struct HostKeyHandle {
    pub key_id: String,
    pub algorithm: String,
}

/// Trait implemented by the host application to provide hardware or host-enclave cryptographic
/// operations (including post-quantum ML-DSA signatures and ML-KEM / X-Wing key encapsulation)
/// without exposing secret key material to the script thread.
pub trait EmbedderKeystoreProvider: Send + Sync + Debug {
    /// Sign `data` using the host-managed key identified by `key_id` and `algorithm` (e.g. "ML-DSA-65", "Ed25519").
    fn sign(&self, key_id: &str, algorithm: &str, data: &[u8]) -> Result<Vec<u8>, String>;

    /// Verify `signature` over `data` using the host-managed key identified by `key_id` and `algorithm`.
    fn verify(&self, key_id: &str, algorithm: &str, data: &[u8], signature: &[u8]) -> Result<bool, String>;

    /// Encapsulate a shared secret to `key_id` (returns `(ciphertext, shared_secret)`).
    fn encapsulate(&self, key_id: &str, algorithm: &str) -> Result<(Vec<u8>, Vec<u8>), String>;

    /// Decapsulate `ciphertext` using the host-managed secret key `key_id` (e.g. "ML-KEM-768", "X-Wing").
    fn decapsulate(&self, key_id: &str, algorithm: &str, ciphertext: &[u8]) -> Result<Vec<u8>, String>;
}

static GLOBAL_KEYSTORE_PROVIDER: OnceLock<RwLock<Option<Arc<dyn EmbedderKeystoreProvider>>>> = OnceLock::new();

pub fn set_embedder_keystore_provider(provider: Arc<dyn EmbedderKeystoreProvider>) {
    let lock = GLOBAL_KEYSTORE_PROVIDER.get_or_init(|| RwLock::new(None));
    if let Ok(mut guard) = lock.write() {
        *guard = Some(provider);
    }
}

pub fn get_embedder_keystore_provider() -> Option<Arc<dyn EmbedderKeystoreProvider>> {
    let lock = GLOBAL_KEYSTORE_PROVIDER.get()?;
    lock.read().ok()?.clone()
}
