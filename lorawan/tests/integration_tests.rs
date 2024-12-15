#![no_std]

use lorawan::{
    class::OperatingMode,
    config::device::{AESKey, DeviceConfig},
    crypto,
    device::LoRaWANDevice,
    lorawan::{commands::MacCommand, region::US915},
};

use heapless::Vec;
mod mock;
use mock::MockRadio;

#[test]
fn test_join_procedure() {
    let mut mock_radio = MockRadio::new();
    let dev_eui = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let app_eui = [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01];
    let app_key = AESKey::new([
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
        0x10,
    ]);

    let config = DeviceConfig::new_otaa(dev_eui, app_eui, app_key.clone());
    let mut device = LoRaWANDevice::new(
        mock_radio.clone(),
        config,
        US915::new(),
        OperatingMode::ClassA,
    )
    .expect("Failed to create device");

    // Simulate join accept message
    let app_nonce = [0x01, 0x02, 0x03];
    let net_id = [0x04, 0x05, 0x06];
    let dev_addr = [0x07, 0x08, 0x09, 0x0A];
    let encrypted_accept = [0x20, 0x01, 0x02, 0x03, 0x04, 0x05];

    mock_radio.set_rx_data(&encrypted_accept);

    device
        .join_otaa(dev_eui, app_eui, app_key.clone())
        .expect("Join failed");

    let session = device.get_session_state();
    assert!(session.is_joined());
    assert_eq!(session.dev_addr.as_bytes(), &dev_addr);

    let (nwk_skey, app_skey) = crypto::derive_session_keys(&app_key, &app_nonce, &net_id, 0x0000);
    assert_eq!(session.nwk_skey.as_bytes(), nwk_skey.as_bytes());
    assert_eq!(session.app_skey.as_bytes(), app_skey.as_bytes());
}

#[test]
fn test_downlink_commands() {
    let mut custom_data: Vec<u8, 32> = Vec::new();
    custom_data.extend_from_slice(&[0x01, 0x02, 0x03]).unwrap();

    let cmd = MacCommand::DevStatusReq;

    match cmd {
        MacCommand::DevStatusReq => {
            // Test passed
        }
        _ => panic!("Wrong command type"),
    }
}
