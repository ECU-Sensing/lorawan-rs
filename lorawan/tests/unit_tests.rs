#![no_std]

use lorawan::{
    config::device::{AESKey, DevAddr, DeviceConfig, SessionState},
    crypto::{self, Direction},
    lorawan::region::{DataRate, Region, US915},
};

mod mock;
use mock::MockRadio;

#[test]
fn test_device_config() {
    let dev_eui = [0x01; 8];
    let app_eui = [0x02; 8];
    let app_key = AESKey::new([0x03; 16]);

    let config = DeviceConfig::new_otaa(dev_eui, app_eui, app_key);

    assert_eq!(&config.dev_eui, &dev_eui);
    assert_eq!(&config.app_eui, &app_eui);
    assert_eq!(config.app_key.as_bytes(), &[0x03; 16]);
    assert!(config.dev_addr.is_none());
}

#[test]
fn test_session_state() {
    let dev_addr = DevAddr::new([0x01, 0x02, 0x03, 0x04]);
    let nwk_skey = AESKey::new([0x01; 16]);
    let app_skey = AESKey::new([0x02; 16]);

    let session = SessionState::new_abp(dev_addr, nwk_skey, app_skey);

    assert_eq!(session.dev_addr, dev_addr);
    assert_eq!(session.nwk_skey.as_bytes(), &[0x01; 16]);
    assert_eq!(session.app_skey.as_bytes(), &[0x02; 16]);
    assert_eq!(session.fcnt_up, 0);
    assert_eq!(session.fcnt_down, 0);
}

#[test]
fn test_crypto_encrypt_decrypt() {
    let key = AESKey::new([0x01; 16]);
    let dev_addr = DevAddr::new([0x01, 0x02, 0x03, 0x04]);
    let fcnt = 1;
    let payload = b"Hello LoRaWAN";

    // Test encryption
    let encrypted = crypto::encrypt_payload(&key, dev_addr, fcnt, Direction::Up, payload);

    // Test decryption
    let decrypted = crypto::encrypt_payload(&key, dev_addr, fcnt, Direction::Up, &encrypted);

    assert_eq!(&decrypted[..], payload);
}

#[test]
fn test_crypto_mic() {
    let key = AESKey::new([0x01; 16]);
    let dev_addr = DevAddr::new([0x01, 0x02, 0x03, 0x04]);
    let fcnt = 1;
    let data = b"Test Data";

    let mic = crypto::compute_mic(&key, data, dev_addr, fcnt, Direction::Up);

    assert_eq!(mic.len(), 4);
}

#[test]
fn test_crypto_join() {
    let app_key = AESKey::new([0x01; 16]);
    let app_nonce = [0x01, 0x02, 0x03];
    let net_id = [0x04, 0x05, 0x06];
    let dev_nonce = 0x0708;

    let (nwk_skey, app_skey) = crypto::derive_session_keys(&app_key, &app_nonce, &net_id, dev_nonce);

    assert_eq!(nwk_skey.as_bytes().len(), 16);
    assert_eq!(app_skey.as_bytes().len(), 16);
}

#[test]
fn test_us915_region() {
    let mut region = US915::new();

    // Test default configuration
    assert_eq!(region.get_data_rate(), DataRate::SF10BW125);
    assert_eq!(region.get_enabled_channels().len(), 72);

    // Test sub-band configuration
    region.set_sub_band(2);
    assert_eq!(region.get_enabled_channels().len(), 9); // 8 125kHz + 1 500kHz

    // Test TTN configuration
    region.set_sub_band(2); // TTN uses sub-band 2
    assert_eq!(region.get_enabled_channels().len(), 9); // 8 125kHz + 1 500kHz

    // Test RX windows
    let channel = region.get_next_channel().unwrap();
    let (rx1_freq, rx1_dr) = region.rx1_window(&channel);
    assert!(rx1_freq < channel.frequency);
    assert_eq!(rx1_dr, region.get_data_rate());

    let (rx2_freq, rx2_dr) = region.rx2_window();
    assert_eq!(rx2_freq, 923_300_000);
    assert_eq!(rx2_dr, DataRate::SF12BW125);
}
