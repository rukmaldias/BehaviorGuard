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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::FEATURE_DIM;
    use crate::profile::enrollment::BaselineProfile;
    use approx::assert_abs_diff_eq;

    fn test_key() -> [u8; 32] {
        [0xAB; 32]
    }

    fn sample_profile() -> BaselineProfile {
        BaselineProfile {
            mean: [1.5; FEATURE_DIM],
            std: [0.3; FEATURE_DIM],
            session_count: 5,
        }
    }

    #[test]
    fn seal_open_round_trip() {
        let profile = sample_profile();
        let key = test_key();
        let blob = ProfileStore::seal(&profile, &key).unwrap();
        let restored = ProfileStore::open(&blob, &key).unwrap();
        assert_eq!(restored.session_count, profile.session_count);
        for i in 0..FEATURE_DIM {
            assert_abs_diff_eq!(restored.mean[i], profile.mean[i], epsilon = 1e-6);
            assert_abs_diff_eq!(restored.std[i], profile.std[i], epsilon = 1e-6);
        }
    }

    #[test]
    fn blob_has_correct_magic() {
        let blob = ProfileStore::seal(&sample_profile(), &test_key()).unwrap();
        assert_eq!(&blob[..8], b"BGPROF01");
    }

    #[test]
    fn open_with_wrong_key_fails() {
        let blob = ProfileStore::seal(&sample_profile(), &test_key()).unwrap();
        let wrong_key = [0x00u8; 32];
        assert!(ProfileStore::open(&blob, &wrong_key).is_err());
    }

    #[test]
    fn open_truncated_blob_fails() {
        let blob = ProfileStore::seal(&sample_profile(), &test_key()).unwrap();
        assert!(ProfileStore::open(&blob[..10], &test_key()).is_err());
    }

    #[test]
    fn open_wrong_magic_fails() {
        let mut blob = ProfileStore::seal(&sample_profile(), &test_key()).unwrap();
        blob[0] = 0xFF; // corrupt magic
        assert!(ProfileStore::open(&blob, &test_key()).is_err());
    }

    #[test]
    fn open_bit_flip_in_ciphertext_fails() {
        let mut blob = ProfileStore::seal(&sample_profile(), &test_key()).unwrap();
        // Flip a bit in the ciphertext (after magic + nonce)
        let last = blob.len() - 1;
        blob[last] ^= 0x01;
        assert!(ProfileStore::open(&blob, &test_key()).is_err());
    }

    #[test]
    fn two_seals_produce_different_nonces() {
        let key = test_key();
        let blob1 = ProfileStore::seal(&sample_profile(), &key).unwrap();
        let blob2 = ProfileStore::seal(&sample_profile(), &key).unwrap();
        // Nonces are at bytes [8..20] — should differ due to random generation
        assert_ne!(&blob1[8..20], &blob2[8..20]);
    }
}
