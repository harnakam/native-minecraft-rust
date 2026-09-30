//! Online-mode login crypto and transport encryption helpers.

use crate::codec::login::EncryptionRequest;
use aes::cipher::{
    generic_array::GenericArray, BlockDecryptMut, BlockEncryptMut, InvalidLength, KeyIvInit,
};
use aes::Aes128;
use cfb8::{Decryptor, Encryptor};
use num_bigint::BigInt;
use rand::{thread_rng, RngCore};
use rsa::{pkcs1v15::Pkcs1v15Encrypt, pkcs8::DecodePublicKey, RsaPublicKey};
use sha1::{Digest, Sha1};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CryptoError {
    InvalidPublicKey,
    RsaEncrypt,
    InvalidCipherLength,
}

impl From<InvalidLength> for CryptoError {
    fn from(_value: InvalidLength) -> Self {
        Self::InvalidCipherLength
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoginEncryption {
    pub shared_secret: [u8; 16],
    pub encrypted_shared_secret: Vec<u8>,
    pub encrypted_verify_token: Vec<u8>,
    pub server_hash: String,
}

pub fn build_login_encryption_response(
    request: &EncryptionRequest,
) -> Result<LoginEncryption, CryptoError> {
    let mut shared_secret = [0u8; 16];
    thread_rng().fill_bytes(&mut shared_secret);
    build_login_encryption_response_with_secret(request, shared_secret)
}

pub fn build_login_encryption_response_with_secret(
    request: &EncryptionRequest,
    shared_secret: [u8; 16],
) -> Result<LoginEncryption, CryptoError> {
    let public_key = RsaPublicKey::from_public_key_der(&request.public_key)
        .map_err(|_| CryptoError::InvalidPublicKey)?;
    let mut rng = thread_rng();
    let encrypted_shared_secret = public_key
        .encrypt(&mut rng, Pkcs1v15Encrypt, &shared_secret)
        .map_err(|_| CryptoError::RsaEncrypt)?;
    let encrypted_verify_token = public_key
        .encrypt(&mut rng, Pkcs1v15Encrypt, &request.verify_token)
        .map_err(|_| CryptoError::RsaEncrypt)?;

    Ok(LoginEncryption {
        shared_secret,
        encrypted_shared_secret,
        encrypted_verify_token,
        server_hash: compute_server_hash(&request.server_id, &request.public_key, &shared_secret),
    })
}

fn compute_server_hash(server_id: &str, public_key: &[u8], shared_secret: &[u8; 16]) -> String {
    let mut digest = Sha1::new();
    digest.update(server_id.as_bytes());
    digest.update(shared_secret);
    digest.update(public_key);
    let digest = digest.finalize();
    BigInt::from_signed_bytes_be(&digest).to_str_radix(16)
}

type Aes128Cfb8Encryptor = Encryptor<Aes128>;
type Aes128Cfb8Decryptor = Decryptor<Aes128>;

pub struct TransportEncryption {
    encryptor: Aes128Cfb8Encryptor,
    decryptor: Aes128Cfb8Decryptor,
}

impl TransportEncryption {
    pub fn new(shared_secret: [u8; 16]) -> Result<Self, CryptoError> {
        Ok(Self {
            encryptor: Aes128Cfb8Encryptor::new_from_slices(&shared_secret, &shared_secret)?,
            decryptor: Aes128Cfb8Decryptor::new_from_slices(&shared_secret, &shared_secret)?,
        })
    }

    pub fn encrypt(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            let mut block = GenericArray::clone_from_slice(&[*byte]);
            self.encryptor.encrypt_block_mut(&mut block);
            *byte = block[0];
        }
    }

    pub fn decrypt(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            let mut block = GenericArray::clone_from_slice(&[*byte]);
            self.decryptor.decrypt_block_mut(&mut block);
            *byte = block[0];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{build_login_encryption_response_with_secret, TransportEncryption};
    use crate::codec::login::EncryptionRequest;
    use rand::{thread_rng, RngCore};
    use rsa::{pkcs8::EncodePublicKey, RsaPrivateKey, RsaPublicKey};

    #[test]
    fn builds_encryption_response() {
        let private_key = RsaPrivateKey::new(&mut thread_rng(), 1024).expect("key should generate");
        let public_key = RsaPublicKey::from(&private_key)
            .to_public_key_der()
            .expect("public key should encode");

        let request = EncryptionRequest {
            server_id: "".to_owned(),
            public_key: public_key.as_ref().to_vec(),
            verify_token: vec![1, 2, 3, 4],
        };

        let response = build_login_encryption_response_with_secret(&request, [9u8; 16])
            .expect("response should build");

        assert_eq!(response.shared_secret, [9u8; 16]);
        assert!(!response.encrypted_shared_secret.is_empty());
        assert!(!response.encrypted_verify_token.is_empty());
    }

    #[test]
    fn roundtrips_transport_encryption() {
        let mut secret = [0u8; 16];
        thread_rng().fill_bytes(&mut secret);

        let mut encrypt = TransportEncryption::new(secret).expect("cipher should initialize");
        let mut decrypt = TransportEncryption::new(secret).expect("cipher should initialize");
        let mut bytes = b"minecraft".to_vec();

        encrypt.encrypt(&mut bytes);
        decrypt.decrypt(&mut bytes);

        assert_eq!(bytes, b"minecraft");
    }
}
