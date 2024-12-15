//! LoRaWAN cryptographic operations
//!
//! This module provides cryptographic functions for LoRaWAN security:
//! - Message Integrity Code (MIC) computation
//! - Payload encryption/decryption
//! - Join accept encryption
//! - Session key derivation

use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes128;
use heapless::Vec;

use crate::config::device::{AESKey, DevAddr};

/// MIC size in bytes
pub const MIC_SIZE: usize = 4;

/// Block size for AES-128
const BLOCK_SIZE: usize = 16;

/// Direction identifiers for cryptographic operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    /// Uplink (device to network)
    Up = 0,
    /// Downlink (network to device)
    Down = 1,
}

/// Compute Message Integrity Code (MIC) for a LoRaWAN message
///
/// # Arguments
/// * `key` - AES key for MIC computation
/// * `data` - Data to compute MIC for
/// * `dev_addr` - Device address
/// * `fcnt` - Frame counter
/// * `dir` - Message direction
pub fn compute_mic(
    key: &AESKey,
    data: &[u8],
    dev_addr: DevAddr,
    fcnt: u32,
    dir: Direction,
) -> [u8; MIC_SIZE] {
    let cipher = Aes128::new_from_slice(key.as_bytes()).unwrap();
    let mut b0 = [0u8; BLOCK_SIZE];
    b0[0] = 0x49; // MIC block identifier
    b0[5] = dir as u8;
    b0[6..10].copy_from_slice(dev_addr.as_bytes());
    b0[10..14].copy_from_slice(&fcnt.to_le_bytes());
    b0[15] = data.len() as u8;

    // Initialize CMAC with first block
    let mut x = b0;
    cipher.encrypt_block((&mut x).into());

    // Process data blocks
    let k = (data.len() + BLOCK_SIZE - 1) / BLOCK_SIZE;
    for i in 0..k {
        let start = i * BLOCK_SIZE;
        let end = (start + BLOCK_SIZE).min(data.len());
        
        // XOR with previous block
        for j in 0..end.saturating_sub(start) {
            x[j] ^= data[start + j];
        }
        
        // If this is the last block and it's not full, pad with zeros (already done by initialization)
        if i == k - 1 && end.saturating_sub(start) < BLOCK_SIZE {
            x[end.saturating_sub(start)] ^= 0x80; // Add padding bit
        }
        
        // Encrypt block
        cipher.encrypt_block((&mut x).into());
    }

    // Return first 4 bytes as MIC
    let mut mic = [0u8; MIC_SIZE];
    mic.copy_from_slice(&x[..MIC_SIZE]);
    mic
}

/// Encrypt or decrypt payload using AES-128 in CTR mode
///
/// # Arguments
/// * `key` - AES key for encryption/decryption
/// * `dev_addr` - Device address
/// * `fcnt` - Frame counter
/// * `dir` - Message direction
/// * `payload` - Data to encrypt/decrypt
pub fn encrypt_payload(
    key: &AESKey,
    dev_addr: DevAddr,
    fcnt: u32,
    dir: Direction,
    payload: &[u8],
) -> Vec<u8, 256> {
    let cipher = <Aes128 as KeyInit>::new_from_slice(key.as_bytes()).unwrap();
    let mut result = Vec::new();

    let k = (payload.len() + 15) / 16;

    for i in 0..k {
        let mut a = [0u8; BLOCK_SIZE];
        a[0] = 0x01; // Data encryption
        a[5] = dir as u8;
        a[6..10].copy_from_slice(dev_addr.as_bytes());
        a[10..14].copy_from_slice(&fcnt.to_le_bytes());
        a[15] = i as u8;

        let mut s = a;
        cipher.encrypt_block((&mut s).into());

        let start = i * 16;
        let end = (start + 16).min(payload.len());
        for j in start..end {
            result.push(payload[j] ^ s[j - start]).unwrap();
        }
    }

    result
}

/// Encrypt join accept message
///
/// # Arguments
/// * `key` - AES key for encryption
/// * `data` - Join accept data to encrypt
pub fn encrypt_join_accept(key: &AESKey, data: &[u8]) -> Vec<u8, 256> {
    let cipher = Aes128::new_from_slice(key.as_bytes()).unwrap();
    let mut result = Vec::new();

    for chunk in data.chunks(16) {
        let mut block = [0u8; BLOCK_SIZE];
        block[..chunk.len()].copy_from_slice(chunk);
        cipher.encrypt_block((&mut block).into());
        for &b in &block[..chunk.len()] {
            result.push(b).unwrap();
        }
    }

    result
}

/// Derive network and application session keys from join accept
///
/// # Arguments
/// * `app_key` - Application key
/// * `app_nonce` - Application nonce from join accept
/// * `net_id` - Network ID from join accept
/// * `dev_nonce` - Device nonce from join request
pub fn derive_session_keys(
    app_key: &AESKey,
    app_nonce: &[u8; 3],
    net_id: &[u8; 3],
    dev_nonce: u16,
) -> (AESKey, AESKey) {
    let cipher = Aes128::new_from_slice(app_key.as_bytes()).unwrap();

    // Generate Network Session Key
    let mut nwk_skey = [0u8; BLOCK_SIZE];
    nwk_skey[0] = 0x01;
    nwk_skey[1..4].copy_from_slice(app_nonce);
    nwk_skey[4..7].copy_from_slice(net_id);
    nwk_skey[7..9].copy_from_slice(&dev_nonce.to_le_bytes());
    cipher.encrypt_block((&mut nwk_skey).into());

    // Generate Application Session Key
    let mut app_skey = [0u8; BLOCK_SIZE];
    app_skey[0] = 0x02;
    app_skey[1..4].copy_from_slice(app_nonce);
    app_skey[4..7].copy_from_slice(net_id);
    app_skey[7..9].copy_from_slice(&dev_nonce.to_le_bytes());
    cipher.encrypt_block((&mut app_skey).into());

    (AESKey::new(nwk_skey), AESKey::new(app_skey))
}

/// Compute Message Integrity Code (MIC) for a LoRaWAN join request
///
/// # Arguments
/// * `key` - Application key for MIC computation
/// * `data` - Join request data to compute MIC for
pub fn compute_join_request_mic(key: &AESKey, data: &[u8]) -> [u8; MIC_SIZE] {
    let cipher = Aes128::new_from_slice(key.as_bytes()).unwrap();
    let mut b0 = [0u8; BLOCK_SIZE];
    b0[0] = 0x49; // MIC block identifier
    b0[1..].copy_from_slice(&data[..data.len().min(BLOCK_SIZE - 1)]);

    // Initialize CMAC with first block
    let mut x = b0;
    cipher.encrypt_block((&mut x).into());

    // Process remaining data blocks if any
    if data.len() > BLOCK_SIZE - 1 {
        let remaining = &data[BLOCK_SIZE - 1..];
        let k = (remaining.len() + BLOCK_SIZE - 1) / BLOCK_SIZE;
        
        for i in 0..k {
            let start = i * BLOCK_SIZE;
            let end = (start + BLOCK_SIZE).min(remaining.len());
            
            // XOR with previous block
            for j in 0..end.saturating_sub(start) {
                x[j] ^= remaining[start + j];
            }
            
            // If this is the last block and it's not full, pad with zeros (already done by initialization)
            if i == k - 1 && end.saturating_sub(start) < BLOCK_SIZE {
                x[end.saturating_sub(start)] ^= 0x80; // Add padding bit
            }
            
            // Encrypt block
            cipher.encrypt_block((&mut x).into());
        }
    } else {
        // If all data fit in first block, just add padding
        x[data.len()] ^= 0x80;
        cipher.encrypt_block((&mut x).into());
    }

    // Return first 4 bytes as MIC
    let mut mic = [0u8; MIC_SIZE];
    mic.copy_from_slice(&x[..MIC_SIZE]);
    mic
}
