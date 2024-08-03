#![no_std]

#[cfg(test)]
mod tests;

pub use chacha20poly1305::aead::heapless;
use chacha20poly1305::{
    aead::{heapless::Vec, AeadMutInPlace, KeyInit},
    ChaCha20Poly1305, Nonce,
};

#[derive(Debug, PartialEq)]
pub enum EndecError {
    InvalidKeyLength,
    InsufficientBufferCapacity,
    IncorrectArraySize,
    DecryptionFailed,
}

pub struct Endec {
    context: u8,       // prevents code replay,
    counter: [u8; 12], // generates unique nonces
}

#[derive(Clone)]
pub struct Secret<const N: usize> {
    pub nonce: [u8; 12],
    pub len: usize,
    pub ciphertext: [u8; N],
}

impl Endec {
    pub fn make_key(bytes: &[u8]) -> [u8; 32] {
        let mut key = [0; 32];

        for i in 0..32 {
            key[i] = bytes[i % bytes.len()];
        }

        key
    }

    pub fn new(associated_data: u8) -> Self {
        Endec {
            context: associated_data,
            counter: [0; 12],
        }
    }

    pub fn enc<const N: usize>(
        &mut self,
        key: &[u8; 32],
        plaintext: &[u8],
    ) -> Result<Secret<N>, EndecError> {
        let nonce = Nonce::clone_from_slice(&self.counter);

        let mut scratch: Vec<u8, N> =
            Vec::from_slice(plaintext).map_err(|_| EndecError::InsufficientBufferCapacity)?;

        let mut cipher =
            ChaCha20Poly1305::new_from_slice(key).map_err(|_| EndecError::InvalidKeyLength)?;

        cipher
            .encrypt_in_place(&nonce, &[self.context], &mut scratch)
            .map_err(|_| EndecError::InsufficientBufferCapacity)?;

        let len = scratch.len();
        unsafe {
            scratch.set_len(N);
        }

        let message = Secret {
            nonce: self.counter.clone(),
            len,
            ciphertext: scratch
                .into_array()
                .map_err(|_| EndecError::IncorrectArraySize)?,
        };
        increment_nonce(&mut self.counter);

        Ok(message)
    }

    pub fn dec<const N: usize>(
        &mut self,
        key: &[u8; 32],
        message: &Secret<N>,
    ) -> Result<Vec<u8, N>, EndecError> {
        let nonce = Nonce::clone_from_slice(&message.nonce);

        let mut scratch = Vec::from_slice(&message.ciphertext)
            .map_err(|_| EndecError::InsufficientBufferCapacity)?;
        scratch.truncate(message.len);

        let mut cipher =
            ChaCha20Poly1305::new_from_slice(key).map_err(|_| EndecError::InvalidKeyLength)?;

        cipher
            .decrypt_in_place(&nonce, &[self.context], &mut scratch)
            .map_err(|_| EndecError::DecryptionFailed)?;

        Ok(scratch)
    }
}

fn increment_nonce<const N: usize>(slice: &mut [u8; N]) {
    let mut i = 0usize;
    let mut carry = 1u64;
    while carry > 0 && i < N {
        carry += slice[i] as u64;
        slice[i] = carry as u8;
        carry /= 256;
        i += 1;
    }
}
