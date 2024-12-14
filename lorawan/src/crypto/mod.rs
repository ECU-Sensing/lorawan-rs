use aes::Aes128;
use aes::cipher::{BlockEncrypt, KeyInit};
use cmac::{Cmac, Mac};
use heapless::Vec;

use crate::config::device::{AESKey, DevAddr, EUI64};

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

/// Compute Message Integrity Code (MIC)
pub fn compute_mic(key: &AESKey, data: &[u8], dev_addr: DevAddr, fcnt: u32, dir: Direction) -> [u8; MIC_SIZE] {
    let mut cmac = Cmac::<Aes128>::new_from_slice(key).unwrap();
    
    // Block B0
    let mut b0 = [0u8; BLOCK_SIZE];
    b0[0] = 0x49; // Data MIC
    b0[5] = dir as u8;
    b0[6..10].copy_from_slice(&dev_addr);
    b0[10..14].copy_from_slice(&fcnt.to_le_bytes());
    b0[15] = data.len() as u8;

    cmac.update(&b0);
    cmac.update(data);

    let result = cmac.finalize().into_bytes();
    let mut mic = [0u8; MIC_SIZE];
    mic.copy_from_slice(&result[..MIC_SIZE]);
    mic
}

/// Compute join request MIC
pub fn compute_join_request_mic(key: &AESKey, data: &[u8]) -> [u8; MIC_SIZE] {
    let mut cmac = Cmac::<Aes128>::new_from_slice(key).unwrap();
    cmac.update(data);
    let result = cmac.finalize().into_bytes();
    let mut mic = [0u8; MIC_SIZE];
    mic.copy_from_slice(&result[..MIC_SIZE]);
    mic
}

/// Generate session keys from join accept
pub fn generate_session_keys(
    app_key: &AESKey,
    app_nonce: &[u8; 3],
    net_id: &[u8; 3],
    dev_nonce: u16,
) -> (AESKey, AESKey) { // (NwkSKey, AppSKey)
    let mut cipher = Aes128::new_from_slice(app_key).unwrap();

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

    (nwk_skey, app_skey)
}

/// Encrypt/decrypt payload
pub fn encrypt_payload(
    key: &AESKey,
    dev_addr: DevAddr,
    fcnt: u32,
    dir: Direction,
    payload: &[u8],
) -> Vec<u8, 256> {
    let mut cipher = Aes128::new_from_slice(key).unwrap();
    let mut result = Vec::new();
    
    let k = (payload.len() + 15) / 16;
    
    for i in 0..k {
        let mut a = [0u8; BLOCK_SIZE];
        a[0] = 0x01; // Data encryption
        a[5] = dir as u8;
        a[6..10].copy_from_slice(&dev_addr);
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

/// Encrypt join accept
pub fn encrypt_join_accept(key: &AESKey, data: &[u8]) -> Vec<u8, 256> {
    let mut cipher = Aes128::new_from_slice(key).unwrap();
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