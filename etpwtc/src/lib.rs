#![no_std]

#[cfg(test)]
mod tests;

use chacha20poly1305::{
    aead::{heapless::Vec, AeadMutInPlace, KeyInit},
    ChaCha20Poly1305, Nonce,
};

#[derive(Debug, PartialEq)]
pub enum EndecError {
    InvalidKeyLength,
    InsufficientBufferCapacity,
    DecryptionFailed,
}

pub struct Endec {
    scratch: Vec<u8, 64>, // enough for anybody
    context: u8,          // prevents code replay,
    counter: [u8; 12],    // generates unique nonces
}

pub struct Message<'a> {
    pub nonce: [u8; 12],
    pub ciphertext: &'a [u8],
}

impl Endec {
    pub fn new(associated_data: u8) -> Self {
        Endec {
            scratch: Vec::new(),
            context: associated_data,
            counter: [0; 12],
        }
    }

    pub fn enc(&mut self, key: &[u8; 32], plaintext: &[u8]) -> Result<Message, EndecError> {
        let nonce = Nonce::clone_from_slice(&self.counter);

        self.scratch.clear();
        self.scratch
            .extend_from_slice(plaintext)
            .map_err(|_| EndecError::InsufficientBufferCapacity)?;

        let mut cipher =
            ChaCha20Poly1305::new_from_slice(key).map_err(|_| EndecError::InvalidKeyLength)?;

        cipher
            .encrypt_in_place(&nonce, &[self.context], &mut self.scratch)
            .map_err(|_| EndecError::InsufficientBufferCapacity)?;

        let message = Message {
            nonce: self.counter.clone(),
            ciphertext: &self.scratch.as_slice(),
        };
        increment_nonce(&mut self.counter);

        Ok(message)
    }

    pub fn dec(&mut self, key: &[u8; 32], message: Message) -> Result<&[u8], EndecError> {
        let nonce = Nonce::clone_from_slice(&message.nonce);

        self.scratch.clear();
        self.scratch
            .extend_from_slice(message.ciphertext)
            .map_err(|_| EndecError::InsufficientBufferCapacity)?;

        let mut cipher =
            ChaCha20Poly1305::new_from_slice(key).map_err(|_| EndecError::InvalidKeyLength)?;

        cipher
            .decrypt_in_place(&nonce, &[self.context], &mut self.scratch)
            .map_err(|_| EndecError::DecryptionFailed)?;

        Ok(&self.scratch.as_slice())
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
