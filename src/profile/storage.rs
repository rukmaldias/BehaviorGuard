use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use rand::RngCore;
use zeroize::Zeroize;

use crate::error::{BgError, Result};
use super::enrollment::BaselineProfile;

const MAGIC: &[u8; 8] = b"BGPROF01";

/// Encrypts and decrypts the `BaselineProfile` blob for on-device storage.
///
/// The caller supplies the 32-byte key (from Android Keystore or a
/// hardware-backed key derivation). Raw profile data never appears in the
/// serialised bytes — only AES-256-GCM ciphertext.
pub struct ProfileStore;

impl ProfileStore {
    /// Serialises and encrypts a `BaselineProfile`.
    /// Returns `BGPROF01 || 12-byte nonce || GCM ciphertext`.
    pub fn seal(profile: &BaselineProfile, key: &[u8; 32]) -> Result<Vec<u8>> {
        let mut plaintext =
            serde_json::to_vec(profile).map_err(|e| BgError::Serialise(e.to_string()))?;

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|_| BgError::Crypto)?;

        plaintext.zeroize();

        let mut out = Vec::with_capacity(8 + 12 + ciphertext.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    /// Decrypts and deserialises a blob produced by `seal`.
    pub fn open(blob: &[u8], key: &[u8; 32]) -> Result<BaselineProfile> {
        if blob.len() < 8 + 12 + 16 {
            return Err(BgError::Crypto);
        }
        if &blob[..8] != MAGIC {
            return Err(BgError::Crypto);
        }
        let nonce = Nonce::from_slice(&blob[8..20]);
        let ciphertext = &blob[20..];

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let mut plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| BgError::Crypto)?;

        let profile = serde_json::from_slice(&plaintext)
            .map_err(|e| BgError::Serialise(e.to_string()))?;

        plaintext.zeroize();
        Ok(profile)
    }
}
