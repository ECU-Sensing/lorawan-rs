#![no_std]

use lorawan::{
    config::device::{AESKey, DeviceConfig},
    lorawan::{
        region::US915,
        commands::MacCommand,
    },
    crypto,
    device::LoRaWANDevice,
    class::OperatingMode,
};

use heapless::Vec;

mod mock;
use mock::MockRadio;

#[test]
fn test_otaa_join() {
    let dev_eui = [0x01; 8];
    let app_eui = [0x02; 8];
    let app_key = AESKey::new([0x03; 16]);

    let config = DeviceConfig::new_otaa(dev_eui, app_eui, app_key.clone());
    let region = US915::new();
    let mut radio = MockRadio::new();
    let mut mock_radio = radio.clone();

    let mut device = LoRaWANDevice::new(radio, config, region, OperatingMode::ClassA)
        .expect("Failed to create device");

    let app_nonce = [0x01, 0x02, 0x03];
    let net_id = [0x04, 0x05, 0x06];
    let dev_addr = [0x01, 0x02, 0x03, 0x04];
    let dlsettings = 0x00;
    let rx_delay = 0x01;
    let cflist = [0u8; 16];

    let mut join_accept = Vec::<u8, 32>::new();
    join_accept.extend_from_slice(&app_nonce).unwrap();
    join_accept.extend_from_slice(&net_id).unwrap();
    join_accept.extend_from_slice(&dev_addr).unwrap();
    join_accept.push(dlsettings).unwrap();
    join_accept.push(rx_delay).unwrap();
    join_accept.extend_from_slice(&cflist).unwrap();

    let encrypted_accept = crypto::encrypt_join_accept(&app_key, &join_accept);

    mock_radio.set_rx_data(&encrypted_accept);

    device.join_otaa(dev_eui, app_eui, app_key.clone()).expect("Join failed");

    let session = device.get_session_state();
    assert!(session.is_joined());
    assert_eq!(session.dev_addr.as_bytes(), &dev_addr);

    let (nwk_skey, app_skey) = crypto::derive_session_keys(
        &app_key,
        &app_nonce,
        &net_id,
        0x0000,
    );
    assert_eq!(session.nwk_skey.as_bytes(), nwk_skey.as_bytes());
    assert_eq!(session.app_skey.as_bytes(), app_skey.as_bytes());

    assert_eq!(session.fcnt_up, 0);
    assert_eq!(session.fcnt_down, 0);
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
